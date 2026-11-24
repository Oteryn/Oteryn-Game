//! Create a screenshot directory with its protected DACL in the creation call.
#![allow(unsafe_code)]
use std::{io, mem::size_of, os::windows::ffi::OsStrExt, path::Path, ptr::null_mut};
use windows_sys::Win32::{
    Foundation::LocalFree,
    Security::{
        Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, SECURITY_ATTRIBUTES,
    },
    Storage::FileSystem::CreateDirectoryW,
};

pub(crate) fn create_private_directory(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        return Ok(());
    }
    let path = std::path::absolute(path)?;
    if let Some(parent) = path.parent() {
        create_private_directory(parent)?;
    }
    let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    wide.push(0);
    let owner = crate::win_mutex::current_user_sid()?;
    let sddl: Vec<_> = format!("O:{owner}D:P(A;OICI;FA;;;{owner})")
        .encode_utf16()
        .chain([0])
        .collect();
    // SAFETY: both strings are terminated and the descriptor lives through creation.
    unsafe {
        let mut descriptor = null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            null_mut(),
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        let created = CreateDirectoryW(wide.as_ptr(), &attributes);
        let error = io::Error::last_os_error();
        LocalFree(descriptor);
        if created == 0 {
            if error.kind() != io::ErrorKind::AlreadyExists {
                return Err(error);
            }
            if !path.is_dir() {
                return Err(io::ErrorKind::NotADirectory.into());
            }
        }
        Ok(())
    }
}
