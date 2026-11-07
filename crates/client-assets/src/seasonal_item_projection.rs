//! Closed, generated Item-look interpretation for the two seasonal source bodies.
//! The generator checks the independent production client artifact, Native keyset
//! and current official appearance identity. This grants no item-use authority.
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
pub struct SeasonalItemProjection {
    pub content_generation: [u8; 32],
    pub definition_refs: [u32; 2],
    appearances: [u32; 2],
}
impl SeasonalItemProjection {
    pub fn appearance_id(&self, reference: u32) -> Option<u32> {
        self.definition_refs
            .iter()
            .position(|value| *value == reference)
            .map(|index| self.appearances[index])
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    schema: String,
    client_content_generation: String,
    items: Vec<RawItem>,
    sources: serde_json::Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawItem {
    definition_ref: u32,
    definition_key: String,
    definition_revision: String,
    appearance_id: u32,
    appearance_identity_sha256: String,
}
fn digest(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        bytes[index] = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(bytes)
}
fn decode(bytes: &[u8]) -> Option<SeasonalItemProjection> {
    if bytes.len() > 8192 {
        return None;
    }
    let raw: Raw = serde_json::from_slice(bytes).ok()?;
    if raw.schema != "OTERYN_SEASONAL_ITEM_APPEARANCE_PROJECTION/v1"
        || raw.items.len() != 2
        || !raw.sources.is_object()
    {
        return None;
    }
    let mut refs = [0; 2];
    let mut appearances = [0; 2];
    for (index, item) in raw.items.iter().enumerate() {
        if item.definition_ref == 0
            || !matches!(item.appearance_id, 2096 | 6393)
            || item.definition_key != format!("oteryn:item.tibia.i{}", item.appearance_id)
            || item.definition_revision.is_empty()
            || item.definition_revision.len() > 128
            || digest(&item.appearance_identity_sha256).is_none()
        {
            return None;
        }
        refs[index] = item.definition_ref;
        appearances[index] = item.appearance_id;
    }
    if refs[0] >= refs[1] || appearances[0] == appearances[1] {
        return None;
    }
    Some(SeasonalItemProjection {
        content_generation: digest(&raw.client_content_generation)?,
        definition_refs: refs,
        appearances,
    })
}
/// Sealed into the native client package, never learned from an incoming reference.
pub fn seasonal_item_projection() -> Option<&'static SeasonalItemProjection> {
    static PIN: OnceLock<Option<SeasonalItemProjection>> = OnceLock::new();
    PIN.get_or_init(|| {
        let bytes = include_bytes!("fixtures/seasonal-item-projection.json");
        if crate::text_sha256(bytes)
            != include_str!("fixtures/seasonal-item-projection.json.sha256").trim()
        {
            return None;
        }
        decode(bytes)
    })
    .as_ref()
}
