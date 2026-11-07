//! A bounded descriptor pins maintained authoring and existing sharded source observations.
//! The large native section is derived in memory, never needed as a repository file.
use super::super::{FilePin, PinnedGameplayBytes, invalid, qualify, read_bounded};
use super::{MAX_BYTES, StrictValue};
use crate::content::ContentError;
use serde::Deserialize;
use std::path::Path;

// QENGINE2's source-backed catalogue and per-field promotions exceed the old
// 200 kB NX-5 sample budget (~805 kB). Match the authoring tool's 8 MiB document
// limit; the descriptor and materialized section keep their separate bounds.
const MAX_AUTHORING_BYTES: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema: String,
    authoring: FilePin,
    source_manifest: FilePin,
    section_sha256: String,
}

pub(in crate::content::native_gameplay) fn materialize(
    supplied: PinnedGameplayBytes,
    locator: &Path,
) -> Result<PinnedGameplayBytes, ContentError> {
    // Historical inline packets keep their existing decoder and byte identity.
    let value: StrictValue = serde_json::from_slice(&supplied.bytes)
        .map_err(|_| invalid("cosmetics provisioning JSON"))?;
    if value.0["schema"] != "OTERYN_NATIVE_COSMETICS_INPUT/v1" {
        return Ok(supplied);
    }
    if supplied.bytes.len() > 16 * 1024 {
        return Err(invalid("cosmetics descriptor bounds"));
    }
    let input: Input = serde_json::from_value(value.0)
        .map_err(|_| invalid("cosmetics provisioning descriptor"))?;
    if input.schema != "OTERYN_NATIVE_COSMETICS_INPUT/v1" {
        return Err(invalid("cosmetics provisioning schema"));
    }
    let directory = locator.parent().unwrap_or(Path::new("."));
    let load = |directory: &Path, pin: &FilePin, max| {
        if pin.path.is_empty() {
            return Err(invalid("cosmetics empty source locator"));
        }
        let pinned = PinnedGameplayBytes {
            bytes: read_bounded(&directory.join(&pin.path), max)?,
            sha256: pin.sha256.clone(),
        };
        qualify(&pinned, max)?;
        Ok::<_, ContentError>(pinned.bytes)
    };
    let authoring = load(directory, &input.authoring, MAX_AUTHORING_BYTES)?;
    let authoring: StrictValue =
        serde_json::from_slice(&authoring).map_err(|_| invalid("cosmetics authoring JSON"))?;
    let source = load(directory, &input.source_manifest, 16 * 1024)?;
    let manifest: StrictValue =
        serde_json::from_slice(&source).map_err(|_| invalid("cosmetics source manifest JSON"))?;
    let source_locator = directory.join(&input.source_manifest.path);
    let source_directory = source_locator
        .parent()
        .ok_or_else(|| invalid("cosmetics source directory"))?;
    let files = manifest.0["files"]
        .as_array()
        .ok_or_else(|| invalid("cosmetics source shard list"))?;
    let mut total = source.len();
    let mut shards = Vec::new();
    for file in files {
        let name = file["path"]
            .as_str()
            .ok_or_else(|| invalid("cosmetics source shard name"))?;
        if !name.ends_with(".json")
            || !name
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || matches!(v, b'_' | b'.' | b'-'))
        {
            return Err(invalid("cosmetics source shard locator"));
        }
        let pin = FilePin {
            path: name.to_owned(),
            sha256: file["sha256"]
                .as_str()
                .ok_or_else(|| invalid("cosmetics source shard pin"))?
                .to_owned(),
        };
        let bytes = load(source_directory, &pin, 8 * 1024 * 1024)?;
        total = total
            .checked_add(bytes.len())
            .ok_or_else(|| invalid("cosmetics source size"))?;
        if total > 32 * 1024 * 1024 {
            return Err(invalid("cosmetics source aggregate bounds"));
        }
        shards.push(serde_json::json!({"path":name,"json":String::from_utf8(bytes).map_err(|_| invalid("cosmetics shard UTF8"))?}));
    }
    let packet = serde_json::json!({
        "schema":"OTERYN_NATIVE_COSMETICS/v1", "availability":"authored",
        "authoring":authoring.0,
        "source_manifest":String::from_utf8(source).map_err(|_| invalid("cosmetics manifest UTF8"))?,
        "source_shards":shards,
    });
    let mut bytes =
        serde_json::to_vec(&packet).map_err(|_| invalid("cosmetics section encoding"))?;
    bytes.push(b'\n');
    let result = PinnedGameplayBytes {
        bytes,
        sha256: input.section_sha256,
    };
    qualify(&result, MAX_BYTES)?;
    Ok(result)
}

#[cfg(test)]
mod tests;
