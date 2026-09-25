#[path = "src/icon.rs"]
mod icon;
use std::{env, fs, path::PathBuf, process::Command};
fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=assets/brand.png");
    let manifest = fs::read_to_string("Cargo.toml").unwrap();
    let display_name = manifest
        .lines()
        .find_map(|line| line.strip_prefix("display-name = "))
        .map(|value| value.trim_matches('"'))
        .expect("display-name is required");
    println!("cargo:rustc-env=APP_DISPLAY_NAME={}", display_name);
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let mut ico = vec![0, 0, 1, 0, sizes.len() as u8, 0];
    let mut data = Vec::new();
    for size in sizes {
        let rgba = icon::rgba(size);
        let mask_len = (size.div_ceil(32) * 4 * size) as usize;
        let len = 40 + rgba.len() + mask_len;
        ico.extend_from_slice(&[size as u8, size as u8, 0, 0, 1, 0, 32, 0]);
        ico.extend_from_slice(&(len as u32).to_le_bytes());
        ico.extend_from_slice(&((6 + sizes.len() * 16 + data.len()) as u32).to_le_bytes());
        for v in [40, size, size * 2] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        data.extend_from_slice(&[1, 0, 32, 0]);
        for v in [0, (rgba.len() + mask_len) as u32, 0, 0, 0, 0] {
            data.extend_from_slice(&v.to_le_bytes());
        }
        for y in (0..size).rev() {
            for x in 0..size {
                let i = ((y * size + x) * 4) as usize;
                data.extend_from_slice(&[rgba[i + 2], rgba[i + 1], rgba[i], rgba[i + 3]]);
            }
        }
        data.resize(data.len() + mask_len, 0);
    }
    ico.extend(data);
    fs::write(out.join("app.ico"), ico).unwrap();
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let name = env::var("CARGO_PKG_NAME").unwrap();
        let version = env::var("CARGO_PKG_VERSION").unwrap();
        let numbers = format!("{},0", version.replace('.', ","));
        let rc = format!(
            r#"1 ICON "app.ico"
1 VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904B0"
  BEGIN
   VALUE "FileDescription", "{display_name}"
   VALUE "ProductName", "{display_name}"
   VALUE "FileVersion", "{version}"
   VALUE "ProductVersion", "{version}"
   VALUE "OriginalFilename", "{name}.exe"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x409, 1200
 END
END
"#
        );
        fs::write(out.join("app.rc"), rc).unwrap();
        let resource = out.join("app-resource.o");
        let compiler = env::var("WINDRES").unwrap_or_else(|_| "x86_64-w64-mingw32-windres".into());
        let status = Command::new(compiler)
            .current_dir(&out)
            .args(["-i", "app.rc", "-o"])
            .arg(&resource)
            .arg("-O")
            .arg("coff")
            .status()
            .expect("resource compiler missing; set WINDRES");
        assert!(status.success(), "resource compilation failed");
        println!("cargo:rustc-link-arg={}", resource.display());
    }
}
