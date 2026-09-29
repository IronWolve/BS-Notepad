use std::{fs::File, io, path::Path};

#[cfg(target_os = "windows")]
pub(crate) fn windows_path(path: &Path) -> io::Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    let path = std::path::absolute(path)?;
    let mut text: Vec<u16> = path.as_os_str().encode_wide().collect();
    if text.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Filename contains a NUL character",
        ));
    }
    if !text.starts_with(&[92, 92, 63, 92]) {
        if text.starts_with(&[92, 92]) {
            text = "\\\\?\\UNC\\"
                .encode_utf16()
                .chain(text.into_iter().skip(2))
                .collect();
        } else {
            text = "\\\\?\\".encode_utf16().chain(text).collect();
        }
    }
    text.push(0);
    Ok(text)
}

pub fn create_temp(path: &Path, original: Option<&std::fs::Metadata>) -> io::Result<File> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::{fs::MetadataExt, io::FromRawHandle};
        use windows_sys::Win32::{
            Foundation::{LocalFree, GENERIC_WRITE, INVALID_HANDLE_VALUE},
            Security::{
                Authorization::{
                    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
                },
                SECURITY_ATTRIBUTES,
            },
            Storage::FileSystem::{
                CreateFileW, CREATE_NEW, FILE_ATTRIBUTE_ENCRYPTED, FILE_ATTRIBUTE_NORMAL,
            },
        };
        let path = windows_path(path)?;
        let descriptor_text: Vec<u16> = "D:P(A;;FA;;;OW)(A;;FA;;;SY)"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut descriptor = std::ptr::null_mut();
        // All temporary bytes are owner/system-only from creation, including on shared folders.
        unsafe {
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                descriptor_text.as_ptr(),
                SDDL_REVISION_1,
                &mut descriptor,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(io::Error::last_os_error());
            }
            let security = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor,
                bInheritHandle: 0,
            };
            let encrypted =
                original.is_some_and(|meta| meta.file_attributes() & FILE_ATTRIBUTE_ENCRYPTED != 0);
            let handle = CreateFileW(
                path.as_ptr(),
                GENERIC_WRITE,
                0,
                &security,
                CREATE_NEW,
                FILE_ATTRIBUTE_NORMAL
                    | if encrypted {
                        FILE_ATTRIBUTE_ENCRYPTED
                    } else {
                        0
                    },
                std::ptr::null_mut(),
            );
            let error = (handle == INVALID_HANDLE_VALUE).then(io::Error::last_os_error);
            LocalFree(descriptor);
            if let Some(error) = error {
                return Err(error);
            }
            Ok(File::from_raw_handle(handle))
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let _ = original;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
    }
}

pub fn preserve(path: &Path, destination: &File) -> io::Result<()> {
    let source = File::open(path)?;
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };
        let _ = destination;
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(source.as_raw_handle(), &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.nNumberOfLinks > 1 {
            return Err(io::Error::other("This file has hard links. Use Save a copy to avoid breaking their shared contents."));
        }
        // ReplaceFile preserves Windows ACLs, streams, creation time and storage attributes.
    }
    #[cfg(unix)]
    {
        use std::os::{fd::AsRawFd, unix::fs::MetadataExt};
        let old = source.metadata()?;
        let new = destination.metadata()?;
        if old.nlink() > 1 {
            return Err(io::Error::other("This file has hard links. Use Save a copy to avoid breaking their shared contents."));
        }
        if (old.uid(), old.gid()) != (new.uid(), new.gid())
            && unsafe { libc::fchown(destination.as_raw_fd(), old.uid(), old.gid()) } != 0
        {
            return Err(io::Error::other(format!(
                "Could not preserve file ownership. Use Save a copy. {}",
                io::Error::last_os_error()
            )));
        }
        copy_attributes(&source, destination)?;
        #[cfg(target_os = "macos")]
        if unsafe {
            libc::fcopyfile(
                source.as_raw_fd(),
                destination.as_raw_fd(),
                std::ptr::null_mut(),
                libc::COPYFILE_ACL,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        destination.set_permissions(old.permissions())?;
    }
    Ok(())
}

#[cfg(unix)]
fn copy_attributes(source: &File, destination: &File) -> io::Result<()> {
    use std::{ffi::CString, os::fd::AsRawFd};
    unsafe fn list(fd: i32, bytes: *mut libc::c_char, length: usize) -> isize {
        #[cfg(target_os = "linux")]
        {
            unsafe { libc::flistxattr(fd, bytes, length) }
        }
        #[cfg(target_os = "macos")]
        {
            unsafe { libc::flistxattr(fd, bytes, length, 0) }
        }
    }
    unsafe fn get(
        fd: i32,
        name: *const libc::c_char,
        bytes: *mut libc::c_void,
        length: usize,
    ) -> isize {
        #[cfg(target_os = "linux")]
        {
            unsafe { libc::fgetxattr(fd, name, bytes, length) }
        }
        #[cfg(target_os = "macos")]
        {
            unsafe { libc::fgetxattr(fd, name, bytes, length, 0, 0) }
        }
    }
    fn value(file: &File, name: &CString) -> io::Result<Vec<u8>> {
        let length = unsafe { get(file.as_raw_fd(), name.as_ptr(), std::ptr::null_mut(), 0) };
        if length < 0 {
            return Err(io::Error::last_os_error());
        }
        if length > 16 * 1024 * 1024 {
            return Err(io::Error::other(
                "Extended metadata is too large to preserve safely. Use Save a copy.",
            ));
        }
        let mut bytes = vec![0; length as usize];
        let read = unsafe {
            get(
                file.as_raw_fd(),
                name.as_ptr(),
                bytes.as_mut_ptr().cast(),
                bytes.len(),
            )
        };
        if read < 0 {
            return Err(io::Error::last_os_error());
        }
        if read as usize > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "File metadata changed during the save.",
            ));
        }
        bytes.truncate(read as usize);
        Ok(bytes)
    }
    let length = unsafe { list(source.as_raw_fd(), std::ptr::null_mut(), 0) };
    if length < 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ENOTSUP) {
            return Ok(());
        }
        return Err(error);
    }
    if length > 1024 * 1024 {
        return Err(io::Error::other(
            "Extended metadata list is too large. Use Save a copy.",
        ));
    }
    let mut names = vec![0u8; length as usize];
    let read = unsafe { list(source.as_raw_fd(), names.as_mut_ptr().cast(), names.len()) };
    if read < 0 {
        return Err(io::Error::last_os_error());
    }
    if read as usize > names.len() {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "File metadata changed during the save.",
        ));
    }
    names.truncate(read as usize);
    let compressed = names
        .split(|b| *b == 0)
        .any(|name| name == b"com.apple.decmpfs");
    for name in names.split(|b| *b == 0).filter(|name| !name.is_empty()) {
        // Compression metadata contains the old data stream; the newly written file is plain.
        if name == b"com.apple.decmpfs" || compressed && name == b"com.apple.ResourceFork" {
            continue;
        }
        let name = CString::new(name).map_err(io::Error::other)?;
        let bytes = value(source, &name)?;
        if value(destination, &name).ok().as_deref() == Some(bytes.as_slice()) {
            continue;
        }
        #[cfg(target_os = "linux")]
        let result = unsafe {
            libc::fsetxattr(
                destination.as_raw_fd(),
                name.as_ptr(),
                bytes.as_ptr().cast(),
                bytes.len(),
                0,
            )
        };
        #[cfg(target_os = "macos")]
        let result = unsafe {
            libc::fsetxattr(
                destination.as_raw_fd(),
                name.as_ptr(),
                bytes.as_ptr().cast(),
                bytes.len(),
                0,
                0,
            )
        };
        if result != 0 {
            return Err(io::Error::other(format!(
                "Could not preserve extended file metadata. Use Save a copy. {}",
                io::Error::last_os_error()
            )));
        }
    }
    Ok(())
}

fn retry(operation: impl Fn() -> io::Result<()>) -> io::Result<()> {
    let result = operation();
    #[cfg(target_os = "windows")]
    {
        let mut result = result;
        for delay in [20, 60, 120, 240] {
            if !result
                .as_ref()
                .err()
                .is_some_and(|e| matches!(e.raw_os_error(), Some(32 | 33)))
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(delay));
            result = operation();
        }
        result
    }
    #[cfg(not(target_os = "windows"))]
    result
}

pub fn replace(
    temp: &Path,
    target: &Path,
    existing: bool,
    private: bool,
) -> io::Result<Option<String>> {
    #[cfg(target_os = "windows")]
    if existing && !private {
        use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;
        let backup = temp.with_extension("previous");
        if backup.exists() {
            return Err(io::Error::other(
                "A save backup name is already in use; try again.",
            ));
        }
        let (target_wide, temp_wide, backup_wide) = (
            windows_path(target)?,
            windows_path(temp)?,
            windows_path(&backup)?,
        );
        let result = retry(|| {
            if unsafe {
                ReplaceFileW(
                    target_wide.as_ptr(),
                    temp_wide.as_ptr(),
                    backup_wide.as_ptr(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                )
            } != 0
            {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        });
        if let Err(error) = result {
            if backup.exists() && !target.exists() {
                if let Err(restore) = crate::workspace_files::rename_no_replace(&backup, target) {
                    return Err(io::Error::other(format!("Save failed: {error}. Original file is preserved at {}. Restore failed: {restore}", backup.display())));
                }
            }
            return Err(error);
        }
        return Ok(std::fs::remove_file(&backup).err().map(|error| {
            format!(
                "Saved. The previous copy remains at {}: {error}",
                backup.display()
            )
        }));
    }
    let _ = private;
    retry(|| {
        if existing {
            std::fs::rename(temp, target)
        } else {
            crate::workspace_files::rename_no_replace(temp, target)
        }
    })?;
    Ok(None)
}

#[cfg(target_os = "windows")]
pub fn cloud_placeholder(path: &Path) -> bool {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        FileAttributeTagInfo, GetFileInformationByHandleEx, FILE_ATTRIBUTE_TAG_INFO,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };
    let Ok(file) = std::fs::OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
    else {
        return false;
    };
    let mut info: FILE_ATTRIBUTE_TAG_INFO = unsafe { std::mem::zeroed() };
    if unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileAttributeTagInfo,
            (&mut info as *mut FILE_ATTRIBUTE_TAG_INFO).cast(),
            std::mem::size_of::<FILE_ATTRIBUTE_TAG_INFO>() as u32,
        )
    } == 0
    {
        return false;
    }
    // Cloud files are hydrated in place; they do not substitute a different pathname.
    info.ReparseTag & !0xf000 == 0x9000001a
}
