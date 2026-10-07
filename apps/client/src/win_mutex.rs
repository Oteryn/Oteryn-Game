//! Per-user install mutexes (CLIENT-INSTALLER-0 §2.1, Concurrency). This module holds the
//! client's only unsafe code (CP D901): the Windows mutex and token-SID calls behind safe
//! functions, every handle closed by `Drop`.
#![allow(unsafe_code)]

use std::fmt::{self, Display, Formatter};
use std::io;
use std::ptr;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_FILE_NOT_FOUND, GetLastError, HANDLE, HLOCAL, LocalFree,
};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, OpenMutexW, OpenProcessToken,
};

/// `SYNCHRONIZE` access right; defined here so the `Win32_Storage_FileSystem` feature is not needed.
const SYNCHRONIZE: u32 = 0x0010_0000;

/// An open kernel handle, closed on drop.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: `self.0` is a valid handle this value exclusively owns; it is closed once, here.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// A named mutex this process holds open; the name stays in use until the value is dropped.
pub struct NamedMutex {
    _handle: OwnedHandle,
}

/// The client could not start beside an install or uninstall.
#[derive(Debug)]
pub enum InstanceError {
    /// An install or uninstall of this user holds `Global\OterynClientSetup-<SID>`.
    SetupInProgress,
    /// The current user's SID or a mutex could not be obtained.
    Os(io::Error),
}

impl Display for InstanceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetupInProgress => formatter.write_str(
                "an Oteryn install or uninstall is in progress; start Oteryn again when it has finished",
            ),
            Self::Os(error) => write!(formatter, "cannot check for a running install: {error}"),
        }
    }
}

impl std::error::Error for InstanceError {}

/// `Global\OterynClientSetup-<SID>`: the install/uninstall transaction mutex.
#[must_use]
pub fn setup_mutex_name(sid: &str) -> String {
    format!("Global\\OterynClientSetup-{sid}")
}

/// `Global\OterynClient-<SID>`: held by every running client of the user.
#[must_use]
pub fn client_mutex_name(sid: &str) -> String {
    format!("Global\\OterynClient-{sid}")
}

/// Takes the client mutex, then checks the transaction mutex. While an install or uninstall
/// holds it, the client mutex is released again and `SetupInProgress` is returned.
pub fn acquire_client_instance() -> Result<NamedMutex, InstanceError> {
    let sid = current_user_sid().map_err(InstanceError::Os)?;
    let client = create(&client_mutex_name(&sid)).map_err(InstanceError::Os)?;
    if exists(&setup_mutex_name(&sid)).map_err(InstanceError::Os)? {
        drop(client);
        return Err(InstanceError::SetupInProgress);
    }
    Ok(client)
}

/// Creates or opens the named mutex without owning it.
pub fn create(name: &str) -> io::Result<NamedMutex> {
    let wide = wide(name);
    // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the call; null security
    // attributes select the default descriptor.
    let handle = unsafe { CreateMutexW(ptr::null(), 0, wide.as_ptr()) };
    if handle.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(NamedMutex {
        _handle: OwnedHandle(handle),
    })
}

/// Whether any process currently holds the named mutex open.
pub fn exists(name: &str) -> io::Result<bool> {
    let wide = wide(name);
    // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the call.
    let handle = unsafe { OpenMutexW(SYNCHRONIZE, 0, wide.as_ptr()) };
    if !handle.is_null() {
        drop(OwnedHandle(handle));
        return Ok(true);
    }
    // SAFETY: reads the calling thread's last-error value set by `OpenMutexW` just above.
    let error = unsafe { GetLastError() };
    if error == ERROR_FILE_NOT_FOUND {
        return Ok(false);
    }
    Err(io::Error::from_raw_os_error(error as i32))
}

/// The current process token's user SID in string form (`S-1-5-21-…`).
pub fn current_user_sid() -> io::Result<String> {
    let mut token: HANDLE = ptr::null_mut();
    // SAFETY: the pseudo-handle of `GetCurrentProcess` needs no closing; `token` receives a new
    // handle on success.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = OwnedHandle(token);
    let mut length = 0u32;
    // SAFETY: a size query: a null buffer of length 0 makes the call report the needed length.
    unsafe { GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut length) };
    // u64 storage keeps the buffer aligned for the pointer inside `TOKEN_USER`.
    let mut buffer = vec![0u64; (length as usize).div_ceil(8).max(1)];
    let capacity = u32::try_from(buffer.len() * 8).map_err(io::Error::other)?;
    // SAFETY: `buffer` is writable for `capacity` bytes and outlives the call.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            capacity,
            &mut length,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: on success the buffer starts with a `TOKEN_USER` whose SID points into the buffer,
    // which stays alive and unmodified until after `ConvertSidToStringSidW` below.
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut text: *mut u16 = ptr::null_mut();
    // SAFETY: `sid` is valid (above); `text` receives a `LocalAlloc` string on success.
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `text` is a NUL-terminated UTF-16 string from `ConvertSidToStringSidW`.
    let length = (0..).take_while(|&i| unsafe { *text.add(i) } != 0).count();
    // SAFETY: the `length` units before the NUL are initialised and stay valid until `LocalFree`.
    let value = String::from_utf16(unsafe { std::slice::from_raw_parts(text, length) });
    // SAFETY: `text` was allocated by `ConvertSidToStringSidW` and is freed once, here.
    unsafe { LocalFree(text.cast::<core::ffi::c_void>() as HLOCAL) };
    value.map_err(io::Error::other)
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
