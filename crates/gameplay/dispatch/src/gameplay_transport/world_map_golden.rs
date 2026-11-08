//! Seeded wire recording with an independent v6-identity compatibility check.
use super::*;
use sha2::{Digest, Sha256};

pub(super) struct Golden {
    pub digest: Sha256,
    pub legacy: Sha256,
    pub updates: usize,
    pub bytes: usize,
    current_bundle: [u8; 32],
    legacy_bundle: [u8; 32],
}
impl Golden {
    pub fn new(current_bundle: [u8; 32], legacy_bundle: [u8; 32]) -> Self {
        Self {
            digest: Sha256::new(),
            legacy: Sha256::new(),
            updates: 0,
            bytes: 0,
            current_bundle,
            legacy_bundle,
        }
    }
    pub fn record(&mut self, update: &Option<MapUpdate>) {
        let (tag, revision, payload) = match update {
            None => (0u8, 0u64, &[][..]),
            Some(MapUpdate::Snapshot(snapshot)) => {
                (1, snapshot.revision, snapshot.payload.as_slice())
            }
            Some(MapUpdate::Delta(delta)) => (2, delta.to, delta.payload.as_slice()),
        };
        record(&mut self.digest, tag, revision, payload);
        let mut old = payload.to_vec();
        if !old.is_empty() {
            // Fixed-width protobuf bytes fields in MapViewHeaderV1. Verify their
            // tags/lengths and current identity before replacing only these 32 bytes.
            assert_eq!(old[0], 10);
            assert!(old[1] < 128);
            assert_eq!(&old[2..4], &[10, 32]);
            assert_eq!(&old[36..38], &[18, 32]);
            assert_eq!(&old[38..70], &self.current_bundle);
            old[38..70].copy_from_slice(&self.legacy_bundle);
        }
        record(&mut self.legacy, tag, revision, &old);
        self.updates += 1;
        self.bytes += payload.len();
    }
}
fn record(digest: &mut Sha256, tag: u8, revision: u64, payload: &[u8]) {
    digest.update([tag]);
    digest.update(revision.to_be_bytes());
    digest.update((payload.len() as u64).to_be_bytes());
    digest.update(payload);
}

/// Reconstitute v6 identity for a layerless fixture; no legacy reader is admitted.
/// No layers or known projectile overrides: both version tokens have equal length.
pub(super) fn legacy_digest(bytes: &[u8]) -> [u8; 32] {
    assert_eq!(&bytes[..6], b"OTWB\x08\x00");
    let size = u32::from_le_bytes(bytes[8..12].try_into().expect("manifest length")) as usize;
    let manifest = std::str::from_utf8(&bytes[16..16 + size]).expect("manifest UTF8");
    assert!(!manifest.contains("\"layers\":"));
    assert!(!manifest.contains("\"blocks_projectile\":"));
    assert_eq!(manifest.matches("OTERYN_WORLD_BUNDLE/v8").count(), 1);
    assert_eq!(manifest.matches("\"min_reader_version\":8").count(), 1);
    let old = manifest
        .replace("OTERYN_WORLD_BUNDLE/v8", "OTERYN_WORLD_BUNDLE/v6")
        .replace("\"min_reader_version\":8", "\"min_reader_version\":6");
    assert_eq!(old.len(), size);
    let mut body = bytes[..bytes.len() - 32].to_vec();
    body[4..6].copy_from_slice(&6u16.to_le_bytes());
    body[16..16 + size].copy_from_slice(old.as_bytes());
    let mut hash = Sha256::new();
    hash.update(b"OTERYN_WORLD_BUNDLE/v6\0");
    hash.update(body);
    hash.finalize().into()
}
