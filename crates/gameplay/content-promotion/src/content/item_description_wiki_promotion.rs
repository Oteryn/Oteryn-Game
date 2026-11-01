//! Separate closed1515 Wiki-primary strings, sharing the atomic scalar setter.
use super::{ProjectV2Draft, item_description_promotion::apply_rows, world_project_sha256};
pub const ITEM_DESCRIPTION_WIKI_PACKET: &[u8] = include_bytes!(
    "../../../../../docs/agents/evidence/OTV2-20261002-item-description-wiki-promotion-v1.json"
);
pub const ITEM_DESCRIPTION_WIKI_PACKET_SHA256: &str =
    "9ba9269c1d346979434a256a5f76cad902662b36f4da3ad9cf0dc687dfc31ba9";
pub fn apply_item_description_wiki_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<usize, String> {
    if world_project_sha256(ITEM_DESCRIPTION_WIKI_PACKET) != ITEM_DESCRIPTION_WIKI_PACKET_SHA256 {
        return Err("Wiki description packet digest drift".into());
    }
    apply_rows(
        &mut draft.core.records,
        &draft.state.item_authoring,
        ITEM_DESCRIPTION_WIKI_PACKET,
        1513,
    )
}
