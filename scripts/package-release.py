#!/usr/bin/env python3
"""Package reviewed release files without runtime data or host archive metadata."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import stat
import subprocess
import zipfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def audit(name, data, private_markers):
    # UTF-16 covers strings embedded by Windows compilers and resource tools.
    for marker in private_markers:
        for encoding in ('utf-8', 'utf-16-le', 'utf-16-be'):
            if marker.encode(encoding) in data:
                raise ValueError(f'Private marker found in {name}; package withheld')
    for text in (data.decode('latin1'), data.decode('utf-16-le', errors='ignore')):
        if re.search(r'/(?:home|Users)/[A-Za-z0-9_.-]+|[A-Za-z]:[\\/]Users[\\/]', text):
            raise ValueError(f'Development home path found in {name}; package withheld')
        if re.search(r'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|\bgh[pousr]_[A-Za-z0-9]{30,}', text):
            raise ValueError(f'Credential pattern found in {name}; package withheld')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('platform', choices=('linux', 'windows', 'macos'))
    parser.add_argument('--macos-archive', type=Path)
    parser.add_argument('--private-marker', action='append', default=[], help='Additional private text to reject; never written into a package')
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    root = repo.parent
    cargo = (repo / 'Cargo.toml').read_text()
    name = re.search(r'^name = "([^"]+)"', cargo, re.M)[1]
    version = re.search(r'^version = "([^"]+)"', cargo, re.M)[1]
    display = re.search(r'^display-name = "([^"]+)"', cargo, re.M)[1]
    arch = 'arm64' if args.platform == 'macos' else 'x86_64'
    label = f'{name}-{version}-{args.platform}-{arch}'
    files = {}
    executable = set()
    if args.platform == 'macos':
        if not args.macos_archive:
            parser.error('--macos-archive is required for macos')
        bundle = f'{display}.app/Contents/'
        approved = {bundle + path for path in ('Info.plist', '_CodeSignature/CodeResources', 'Resources/AppIcon.icns', f'MacOS/{name}')}
        with zipfile.ZipFile(args.macos_archive) as source:
            entries = [entry for entry in source.infolist() if not entry.is_dir()]
            if len(entries) != len(approved) or {e.filename for e in entries} != approved:
                raise ValueError('Unexpected files in app bundle; review before packaging')
            for entry in entries:
                if stat.S_ISLNK(entry.external_attr >> 16):
                    raise ValueError('Symlinks are not in the approved bundle manifest')
                files[entry.filename] = source.read(entry)
        info = plistlib.loads(files[bundle + 'Info.plist'])
        if info.get('CFBundleShortVersionString') != version:
            raise ValueError('App bundle version does not match source')
        binary = files[bundle + f'MacOS/{name}']
        if binary[:4] != b'\xcf\xfa\xed\xfe' or int.from_bytes(binary[4:8], 'little') != 0x0100000C:
            raise ValueError('Expected an arm64 Mach-O executable')
        executable.add(bundle + f'MacOS/{name}')
        instructions = f'Place {display}.app in a writable folder. Requires macOS 14 or newer on Apple Silicon.\nThis package is ad-hoc signed, not notarized. Settings, logs and recovery data are created in {name}-data beside the app.\n'
    else:
        approved = [name] if args.platform == 'linux' else [name + '.exe', 'WebView2Loader.dll', 'installed.json', 'register-file-types.ps1']
        for filename in approved:
            path = root / 'deploy' / args.platform / filename
            if not path.is_file() or path.is_symlink():
                raise ValueError(f'Missing regular release file: {filename}')
            files[filename] = path.read_bytes()
        if args.platform == 'linux':
            if not files[name].startswith(b'\x7fELF\x02\x01') or int.from_bytes(files[name][18:20], 'little') != 62:
                raise ValueError('Expected an ELF executable')
            executable.add(name)
            versions = re.findall(rb'GLIBC_([0-9]+\.[0-9]+(?:\.[0-9]+)?)\x00', files[name])
            if not versions:
                raise ValueError('Cannot determine the required C library version')
            glibc = max(versions, key=lambda value: tuple(map(int, value.split(b'.')))).decode()
            instructions = f'Extract the complete folder, then run ./{name}.\nRequires x86-64 Linux with glibc {glibc} or newer, GTK 3, WebKitGTK 4.1 and its runtime dependencies.\nSettings, logs and recovery data are created beside the executable. Keep this folder writable.\n'
        else:
            if not files[name + '.exe'].startswith(b'MZ'):
                raise ValueError('Expected a Windows executable')
            binary = files[name + '.exe']
            offset = int.from_bytes(binary[60:64], 'little')
            if binary[offset:offset+6] != b'PE\x00\x00\x64\x86':
                raise ValueError('Expected an x86-64 Windows executable')
            installed = json.loads(files['installed.json'])
            if installed != {'name': name, 'version': version}:
                raise ValueError('Installation manifest does not match source')
            instructions = f'Extract the complete folder and run {name}.exe. Keep WebView2Loader.dll beside it.\nRequires x64 Windows and the WebView2 runtime. Keep this folder writable for settings, logs and recovery data.\nOptional: register-file-types.ps1 registers Markdown Open with entries for the current user; -Remove reverses registration.\n'
    files['Read Me.txt'] = (f'{display} {version}\n\n' + instructions + '\nThis archive contains application files only. Existing settings are not included.\n').encode()
    revision = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', 'HEAD'], text=True).strip()
    manifest = {'name': name, 'version': version, 'platform': args.platform, 'architecture': arch,
                'source_revision': revision, 'files': {key: digest(value) for key, value in sorted(files.items())}}
    files['release.json'] = (json.dumps(manifest, indent=2) + '\n').encode()
    markers = [str(Path.home()), str(root), *args.private_marker]
    for filename, data in files.items():
        if not data:
            raise ValueError(f'Empty release file: {filename}')
        audit(filename, data, markers)
    out = root / 'dists'
    scratch = root / 'tmp' / 'packages'
    out.mkdir(exist_ok=True)
    scratch.mkdir(parents=True, exist_ok=True)
    target = out / (label + '.zip')
    staged = scratch / (label + f'.{os.getpid()}.zip')
    with zipfile.ZipFile(staged, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
        for filename, data in sorted(files.items()):
            entry = zipfile.ZipInfo(label + '/' + filename, date_time=(1980, 1, 1, 0, 0, 0))
            entry.create_system = 3
            entry.external_attr = (stat.S_IFREG | (0o755 if filename in executable else 0o644)) << 16
            entry.compress_type = zipfile.ZIP_DEFLATED
            archive.writestr(entry, data)
    with zipfile.ZipFile(staged) as archive:
        if archive.testzip() is not None:
            raise ValueError('Archive integrity check failed')
        for filename, data in files.items():
            if archive.read(label + '/' + filename) != data:
                raise ValueError('Archive bytes changed unexpectedly')
    staged.replace(target)
    sha = digest(target.read_bytes())
    target.with_suffix('.zip.sha256').write_text(f'{sha}  {target.name}\n')
    print(f'{target.name}: {len(files)} reviewed files; SHA-256 {sha}')


if __name__ == '__main__':
    main()
