//! Open and inspect the same Windows file handle before writing chat.
#![allow(unsafe_code)]

use super::export_policy::validate_destination;
use std::{
    ffi::c_void,
    fs::File,
    io,
    mem::{size_of, zeroed},
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    path::Path,
    ptr::{null, null_mut},
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
        BY_HANDLE_FILE_INFORMATION, CreateFileW, FILE_APPEND_DATA, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_READ_ATTRIBUTES,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TYPE_DISK, FILE_WRITE_DATA,
        GetFileInformationByHandle, GetFileType, OPEN_ALWAYS, READ_CONTROL,
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

fn open_with_descriptor(path: &Path, append: bool, descriptor: &str) -> io::Result<File> {
    let path = std::path::absolute(path)?;
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
        let handle = CreateFileW(
            path.as_ptr(),
            write | FILE_READ_ATTRIBUTES | READ_CONTROL,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &attributes,
            OPEN_ALWAYS,
            FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        );
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        Ok(File::from_raw_handle(handle))
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
