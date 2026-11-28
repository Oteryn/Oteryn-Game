//! Retain a paid hunting confirmation across client restarts.
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use oteryn_client_runtime::cui3::store;

pub(in crate::play) fn path(character: &str) -> Option<PathBuf> {
    let identity = oteryn_platform_client::native_login::parse_uuid_v7(character)?;
    let base = crate::play::PreferenceStore::configuration_path("store-confirmation.bin")?;
    Some(crate::play::PreferenceStore::character_path(&base, identity).with_extension("bin"))
}

pub(in crate::play) fn is_hunting(key: &str) -> bool {
    matches!(
        key,
        "store:xp-boost:1"
            | "store:xp-boost:2"
            | "store:xp-boost:3"
            | "store:xp-boost:4"
            | "store:xp-boost:5"
            | "store:permanent-prey-slot"
            | "store:prey-wildcards:1"
            | "store:prey-wildcards:5"
    )
}

pub(in crate::play) fn is_hunting_request(payload: &[u8]) -> bool {
    store::decode_purchase_request(payload).is_ok_and(|quote| is_hunting(&quote.offer_key))
}

pub(in crate::play) fn load(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    match bytes.len() {
        0 => Ok(None),
        1..=4096 if is_hunting_request(&bytes) => Ok(Some(bytes)),
        _ => Err(io::ErrorKind::InvalidData.into()),
    }
}

/// Return an earlier outstanding confirmation if another client has already saved it.
/// A lost reply can only be retried with the original bytes.
#[cfg(any(test, bevy_shell))]
pub(in crate::play) fn retain(path: &Path, proposed: &[u8]) -> io::Result<Option<Vec<u8>>> {
    let _lock = crate::atomic_file::try_lock(&path.with_extension("lock"))?;
    if let Some(existing) = load(path)? {
        return Ok((existing != proposed).then_some(existing));
    }
    crate::atomic_file::publish_durable(path, proposed)?;
    Ok(None)
}

pub(in crate::play) fn finish(path: &Path, completed: &[u8]) -> io::Result<()> {
    let _lock = crate::atomic_file::try_lock(&path.with_extension("lock"))?;
    if load(path)?.as_deref() == Some(completed) {
        crate::atomic_file::publish_durable(path, &[])?;
    }
    Ok(())
}
