//! Positive NPC height facts from the two pinned donor appearance loaders.
//! Canary src/items/items.cpp:235 uses protobuf height presence, not an XML default.
use serde::Deserialize;
use std::sync::OnceLock;

pub const FIELD: &str = "oteryn:source.npc.walk_height_sha256";
const DATA: &str = include_str!("../../../../../../content/npcs/s10-walk-heights.json");

#[derive(Deserialize)]
struct Heights {
    schema: String,
    known_ranges: Vec<[u32; 2]>,
    height_ranges: Vec<[u32; 2]>,
    damaging_field_ids: Vec<u32>,
}

pub fn digest() -> &'static str {
    static DIGEST: OnceLock<String> = OnceLock::new();
    DIGEST.get_or_init(|| crate::content::world_project_sha256(DATA.as_bytes()))
}

fn data() -> Option<&'static Heights> {
    static HEIGHTS: OnceLock<Option<Heights>> = OnceLock::new();
    let data = HEIGHTS
        .get_or_init(|| {
            let data: Heights = serde_json::from_str(DATA).ok()?;
            if data.schema != "OTERYN_NPC_WALK_HEIGHTS/v1"
                || data.known_ranges.iter().any(|range| range[0] > range[1])
                || data
                    .known_ranges
                    .windows(2)
                    .any(|pair| pair[0][1] >= pair[1][0])
                || data.height_ranges.iter().any(|range| range[0] > range[1])
                || data
                    .height_ranges
                    .windows(2)
                    .any(|pair| pair[0][1] >= pair[1][0])
                || data
                    .damaging_field_ids
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                || data
                    .height_ranges
                    .iter()
                    .any(|range| !known(&data, range[0]) || !known(&data, range[1]))
            {
                return None;
            }
            Some(data)
        })
        .as_ref()?;
    Some(data)
}

pub fn has_height(id: u32) -> Option<bool> {
    let data = data()?;
    known(data, id).then(|| contains(&data.height_ranges, id))
}

pub fn damaging_field(id: u32) -> Option<bool> {
    let data = data()?;
    known(data, id).then(|| data.damaging_field_ids.binary_search(&id).is_ok())
}

fn known(data: &Heights, id: u32) -> bool {
    contains(&data.known_ranges, id)
}

fn contains(ranges: &[[u32; 2]], id: u32) -> bool {
    let index = ranges.partition_point(|range| range[1] < id);
    ranges.get(index).is_some_and(|range| range[0] <= id)
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_height_is_positive_knowledge_and_unknown_is_not_zero() {
        assert_eq!(super::has_height(103), Some(false));
        assert_eq!(super::has_height(0), None);
        assert_eq!(super::has_height(u32::MAX), None);
    }
}
