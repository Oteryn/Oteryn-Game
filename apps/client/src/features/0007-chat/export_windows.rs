//! Open and inspect the same Windows file handle before writing chat.
#![allow(unsafe_code)]

use super::export_policy::validate_destination;
use std::{
    ffi::c_void,
    fs::File,
    io,
    mem::{size_of, zeroed},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::{Path, PathBuf},
    ptr::null_mut,
};
use windows_sys::Win32::{
    Foundation::{INVALID_HANDLE_VALUE, LocalFree},
    Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL,
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
            GetSecurityInfo, SE_FILE_OBJECT,
        },
        DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetTokenInformation, IsWellKnownSid,
        OWNER_SECURITY_INFORMATION, PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
        WinBuiltinAdministratorsSid, WinLocalSystemSid,
    },
    Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, CREATE_NEW, CreateFileW, DELETE, FILE_APPEND_DATA,
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT, FILE_DISPOSITION_INFO,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_LIST_DIRECTORY,
        FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_DISK, FILE_WRITE_DATA,
        FileDispositionInfo, GetFileInformationByHandle, GetFileType, GetFinalPathNameByHandleW,
        OPEN_EXISTING, READ_CONTROL, SetFileInformationByHandle,
    },
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

// Both security descriptor APIs and SID string conversion allocate with LocalAlloc.
struct LocalAllocation(*mut c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this pointer was returned by the allocating Windows API.
            unsafe { LocalFree(self.0) };
        }
    }
}

pub(super) fn open(path: &Path, append: bool) -> io::Result<File> {
    with_user_sid(|sid| {
        let user = sid_string(sid)?;
        let file = open_with_descriptor(
            path,
            append,
            &format!("O:{user}D:P(A;;FA;;;{user})(A;;FA;;;SY)(A;;FA;;;BA)"),
        )?;
        validate_handle(&file, sid)?;
        Ok(file)
    })
}

fn with_user_sid<T>(use_sid: impl FnOnce(PSID) -> io::Result<T>) -> io::Result<T> {
    // SAFETY: the token handle and aligned output buffer are valid for these calls.
    unsafe {
        let mut token = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(io::Error::last_os_error());
        }
        let token = OwnedHandle::from_raw_handle(token);
        let mut bytes = 0;
        GetTokenInformation(token.as_raw_handle(), TokenUser, null_mut(), 0, &mut bytes);
        if bytes < size_of::<TOKEN_USER>() as u32 {
            return Err(io::Error::last_os_error());
        }
        let mut buffer = vec![0_usize; (bytes as usize).div_ceil(size_of::<usize>())];
        if GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buffer.as_mut_ptr().cast(),
            bytes,
            &mut bytes,
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        use_sid((*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid)
    }
}

fn sid_string(sid: PSID) -> io::Result<String> {
    // SAFETY: sid lives in the token buffer and the returned string is NUL terminated.
    unsafe {
        let mut string = null_mut();
        if ConvertSidToStringSidW(sid, &mut string) == 0 {
            return Err(io::Error::last_os_error());
        }
        let _allocation = LocalAllocation(string.cast());
        let len = (0..).take_while(|&index| *string.add(index) != 0).count();
        String::from_utf16(std::slice::from_raw_parts(string, len)).map_err(io::Error::other)
    }
}

// Hold each checked directory without delete sharing until the final
// open, so the path cannot be redirected between inspection and creation.
fn guard_parents(path: &Path) -> io::Result<Vec<File>> {
    let parents: Vec<_> = path.ancestors().skip(1).collect();
    parents
        .into_iter()
        .rev()
        .map(|parent| {
            let directory = File::options()
                .access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES)
                .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(parent)?;
            let attributes = directory.metadata()?.file_attributes();
            if attributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
                != FILE_ATTRIBUTE_DIRECTORY
            {
                return Err(io::ErrorKind::PermissionDenied.into());
            }
            Ok(directory)
        })
        .collect()
}

fn final_path(file: &File) -> io::Result<PathBuf> {
    let mut buffer = vec![0_u16; 260];
    loop {
        // SAFETY: the buffer holds the indicated number of writable UTF-16 units.
        let len = unsafe {
            GetFinalPathNameByHandleW(
                file.as_raw_handle(),
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                0,
            )
        } as usize;
        if len == 0 {
            return Err(io::Error::last_os_error());
        }
        if len < buffer.len() {
            return Ok(std::ffi::OsString::from_wide(&buffer[..len]).into());
        }
        buffer.resize(len, 0);
    }
}

fn directory_id(file: &File) -> io::Result<(u32, u32, u32)> {
    // SAFETY: the handle is live and the output structure is writable.
    unsafe {
        let mut info: BY_HANDLE_FILE_INFORMATION = zeroed();
        if GetFileInformationByHandle(file.as_raw_handle(), &mut info) == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    }
}

fn verify_parent(file: &File, guarded: &File) -> io::Result<()> {
    let actual = final_path(file)?;
    let parent = actual.parent().ok_or(io::ErrorKind::PermissionDenied)?;
    let opened = File::options()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(parent)?;
    // Every spelling of the export's parent must identify the pinned directory.
    if directory_id(&opened)? != directory_id(guarded)? {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

fn open_with_descriptor(path: &Path, append: bool, descriptor: &str) -> io::Result<File> {
    let path = std::path::absolute(path)?;
    let parents = guard_parents(&path)?;
    let guarded_parent = parents.last().ok_or(io::ErrorKind::InvalidInput)?;
    let mut path: Vec<u16> = path.as_os_str().encode_wide().collect();
    if path.contains(&0) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    path.push(0);
    let descriptor: Vec<u16> = descriptor.encode_utf16().chain([0]).collect();
    // SAFETY: the input strings are NUL terminated, and the security descriptor
    // and attributes stay alive through CreateFileW.
    unsafe {
        let mut security = LocalAllocation(null_mut());
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            descriptor.as_ptr(),
            1,
            &mut security.0,
            null_mut(),
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: security.0,
            bInheritHandle: 0,
        };
        let write = if append {
            FILE_APPEND_DATA
        } else {
            FILE_WRITE_DATA
        };
        let open = |access, disposition| {
            CreateFileW(
                path.as_ptr(),
                write | FILE_READ_ATTRIBUTES | READ_CONTROL | access,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                &attributes,
                disposition,
                FILE_FLAG_OPEN_REPARSE_POINT,
                null_mut(),
            )
        };
        let mut handle = open(DELETE, CREATE_NEW);
        let created = handle != INVALID_HANDLE_VALUE;
        if !created {
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::AlreadyExists {
                return Err(error);
            }
            handle = open(0, OPEN_EXISTING);
            if handle == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }
        }
        let file = File::from_raw_handle(handle);
        match verify_parent(&file, guarded_parent) {
            Ok(()) => Ok(file),
            Err(error) => {
                // Delete only our own newly created file, through its open handle.
                if created {
                    let disposition = FILE_DISPOSITION_INFO { DeleteFile: true };
                    if SetFileInformationByHandle(
                        handle,
                        FileDispositionInfo,
                        (&raw const disposition).cast(),
                        size_of::<FILE_DISPOSITION_INFO>() as u32,
                    ) == 0
                    {
                        return Err(io::Error::last_os_error());
                    }
                }
                Err(error)
            }
        }
    }
}

fn validate_handle(file: &File, user: PSID) -> io::Result<()> {
    // SAFETY: these APIs inspect a live handle. All ACL and SID pointers borrowed
    // from GetSecurityInfo remain valid while the descriptor allocation lives.
    unsafe {
        let mut information: BY_HANDLE_FILE_INFORMATION = zeroed();
        if GetFileInformationByHandle(file.as_raw_handle(), &mut information) == 0 {
            return Err(io::Error::last_os_error());
        }
        let regular = GetFileType(file.as_raw_handle()) == FILE_TYPE_DISK
            && information.dwFileAttributes
                & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT)
                == 0;
        if !regular {
            return validate_destination(false, false, false);
        }
        let mut descriptor = LocalAllocation(null_mut());
        let mut owner = null_mut();
        let mut dacl: *mut ACL = null_mut();
        let code = GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            null_mut(),
            &mut dacl,
            null_mut(),
            &mut descriptor.0,
        );
        if code != 0 {
            return Err(io::Error::from_raw_os_error(code as i32));
        }
        let trusted = |sid: PSID| {
            !sid.is_null()
                && (EqualSid(sid, user) != 0
                    || IsWellKnownSid(sid, WinLocalSystemSid) != 0
                    || IsWellKnownSid(sid, WinBuiltinAdministratorsSid) != 0)
        };
        let restricted = !dacl.is_null()
            && (0..u32::from((*dacl).AceCount)).all(|index| {
                let mut ace = null_mut();
                if GetAce(dacl, index, &mut ace) == 0 {
                    return false;
                }
                let header = &*ace.cast::<ACE_HEADER>();
                // An unfamiliar ACE is refused conservatively. A simple deny grants no access.
                match header.AceType {
                    0 if usize::from(header.AceSize) >= size_of::<ACCESS_ALLOWED_ACE>() => {
                        let ace = ace.cast::<ACCESS_ALLOWED_ACE>();
                        trusted((&raw mut (*ace).SidStart).cast())
                    }
                    1 => true,
                    _ => false,
                }
            });
        validate_destination(regular, trusted(owner), restricted)
    }
}

#[cfg(test)]
#[test]
fn windows_export_creates_private_file_and_refuses_broad_existing_dacl() -> io::Result<()> {
    let folder = tempfile::tempdir()?;
    let private = folder.path().join("private.log");
    open(&private, false)?.set_len(0)?;
    use std::io::Write;
    open(&private, true)?.write_all(b"private")?;
    let public = folder.path().join("public.log");
    drop(open_with_descriptor(&public, false, "D:P(A;;FA;;;WD)")?);
    for append in [true, false] {
        assert_eq!(
            open(&public, append).err().map(|error| error.kind()),
            Some(io::ErrorKind::PermissionDenied)
        );
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn checked_parent_denies_rename_until_create_finishes() -> io::Result<()> {
    let root = tempfile::tempdir()?;
    let parent = root.path().join("parent");
    let moved = root.path().join("moved");
    std::fs::create_dir(&parent)?;
    let guards = guard_parents(&parent.join("export.log"))?;
    assert_eq!(
        std::fs::rename(&parent, &moved)
            .err()
            .and_then(|error| error.raw_os_error()),
        Some(32),
        "checked directory must remain pinned"
    );
    drop(guards);
    std::fs::rename(parent, moved)
}

#[cfg(test)]
#[test]
fn windows_export_accepts_short_parent_name() -> io::Result<()> {
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
    let root = tempfile::tempdir()?;
    let parent = root.path().join("chat export directory");
    std::fs::create_dir(&parent)?;
    let long: Vec<_> = parent.as_os_str().encode_wide().chain([0]).collect();
    // SAFETY: long is terminated and the second call uses the requested capacity.
    let short = unsafe {
        let size = GetShortPathNameW(long.as_ptr(), null_mut(), 0);
        let mut buffer = vec![0; size as usize];
        let len = GetShortPathNameW(long.as_ptr(), buffer.as_mut_ptr(), size);
        if len == 0 || len >= size {
            return Err(io::Error::last_os_error());
        }
        PathBuf::from(std::ffi::OsString::from_wide(&buffer[..len as usize]))
    };
    assert_ne!(
        short.file_name(),
        parent.file_name(),
        "requires 8.3 aliases"
    );
    open(&short.join("chat.log"), false)?;
    assert!(parent.join("chat.log").is_file());
    Ok(())
}
