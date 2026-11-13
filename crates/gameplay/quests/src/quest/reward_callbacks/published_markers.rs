//! Published main callbacks predate the additional value-plan catalogue marker.
use super::{BTreeSet, Document, RewardCallback};

pub(super) fn keys() -> Result<BTreeSet<String>, String> {
    let mut keys = BTreeSet::new();
    // These documents contain callbacks published before value-plan markers.
    for source in [
        include_str!("../../../../../../content/quests/shared-reward-callbacks.json"),
        include_str!("../../../../../../content/quests/npc-callbacks/questpc7-programmes.json"),
        include_str!("../../../../../../content/quests/npc-callbacks/l13-outfits.json"),
        include_str!("../../../../../../content/quests/npc-callbacks/l6-stories.json"),
    ] {
        let document: Document = serde_json::from_str(source).map_err(|error| error.to_string())?;
        keys.extend(document.callbacks.into_iter().map(|callback| callback.key));
    }
    Ok(keys)
}

pub(super) fn needs_marker(callback: &RewardCallback, published: &BTreeSet<String>) -> bool {
    !published.contains(&callback.key)
        || !callback.required_receipts.is_empty()
        || callback.per_occurrence_mint
        || callback.carried_exchange
        || callback
            .give
            .as_ref()
            .is_some_and(|give| !give.backpack_outputs.is_empty())
}
