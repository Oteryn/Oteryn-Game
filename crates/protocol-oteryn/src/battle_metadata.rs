//! Bounded read-only player filters on the existing ATTACK_V1 capability.
use crate::world_spatial_entities::EntityRef;
pub const COMMAND_TYPE_BATTLE_METADATA_QUERY: u32 = 736;
pub const MAX_ENTRIES: usize = 16;
pub const MAX_REQUEST_BYTES: usize = 1 + 24 * MAX_ENTRIES;
pub const MAX_REPLY_BYTES: usize = 1 + 26 * MAX_ENTRIES;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerMetadata {
    pub entity: EntityRef,
    pub vocation: Option<u8>,
    pub same_guild: Option<bool>,
}
fn valid_refs(refs: &[EntityRef]) -> bool {
    refs.len() <= MAX_ENTRIES
        && refs
            .iter()
            .enumerate()
            .all(|(i, entity)| entity.generation != 0 && !refs[..i].contains(entity))
}
fn read_ref(bytes: &[u8]) -> Option<EntityRef> {
    let entity = EntityRef {
        identity: bytes.get(..16)?.try_into().ok()?,
        generation: u64::from_be_bytes(bytes.get(16..24)?.try_into().ok()?),
    };
    (entity.generation != 0).then_some(entity)
}
fn write_ref(out: &mut Vec<u8>, entity: EntityRef) {
    out.extend(entity.identity);
    out.extend(entity.generation.to_be_bytes());
}
pub fn encode_query(refs: &[EntityRef]) -> Option<Vec<u8>> {
    if !valid_refs(refs) {
        return None;
    }
    let mut out = vec![u8::try_from(refs.len()).ok()?];
    for &entity in refs {
        write_ref(&mut out, entity);
    }
    Some(out)
}
pub fn decode_query(bytes: &[u8]) -> Option<Vec<EntityRef>> {
    let count = usize::from(*bytes.first()?);
    if count > MAX_ENTRIES || bytes.len() != 1 + 24 * count {
        return None;
    }
    let refs = bytes[1..]
        .as_chunks::<24>()
        .0
        .iter()
        .map(|bytes| read_ref(bytes))
        .collect::<Option<Vec<_>>>()?;
    valid_refs(&refs).then_some(refs)
}
pub fn encode_reply(rows: &[PlayerMetadata]) -> Option<Vec<u8>> {
    if !valid_refs(&rows.iter().map(|row| row.entity).collect::<Vec<_>>())
        || rows
            .iter()
            .any(|row| row.vocation.is_some_and(|vocation| vocation > 5))
    {
        return None;
    }
    let mut out = vec![u8::try_from(rows.len()).ok()?];
    for row in rows {
        write_ref(&mut out, row.entity);
        out.push(row.vocation.unwrap_or(255));
        out.push(match row.same_guild {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        });
    }
    Some(out)
}
pub fn decode_reply(bytes: &[u8]) -> Option<Vec<PlayerMetadata>> {
    let count = usize::from(*bytes.first()?);
    if count > MAX_ENTRIES || bytes.len() != 1 + 26 * count {
        return None;
    }
    let mut rows = Vec::with_capacity(count);
    for bytes in bytes[1..].as_chunks::<26>().0 {
        let entity = read_ref(bytes)?;
        if rows.iter().any(|row: &PlayerMetadata| row.entity == entity) {
            return None;
        }
        rows.push(PlayerMetadata {
            entity,
            vocation: match bytes[24] {
                255 => None,
                vocation @ 0..=5 => Some(vocation),
                _ => return None,
            },
            same_guild: match bytes[25] {
                0 => None,
                1 => Some(false),
                2 => Some(true),
                _ => return None,
            },
        });
    }
    Some(rows)
}
#[cfg(test)]
#[path = "battle_metadata/tests.rs"]
mod tests;
