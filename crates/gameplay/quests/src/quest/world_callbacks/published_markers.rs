//! Preserve published main world definitions while binding newly authored terms.
use super::{BTreeSet, Callback, Document};

pub(super) fn keys() -> Result<BTreeSet<String>, String> {
    let published: Document = serde_json::from_str(include_str!(
        "../../../../../../content/quests/world-callbacks/l6-item-rewards.json"
    ))
    .map_err(|error| error.to_string())?;
    Ok(published
        .callbacks
        .into_iter()
        .map(|callback| callback.key)
        .collect())
}

pub(super) fn needs_marker(callback: &Callback, published: &BTreeSet<String>) -> bool {
    let new_value_terms = callback.use_range != 1
        || callback.disguise.is_some()
        || callback.selector.ordinal.is_some()
        || !callback.present.is_empty()
        || !callback.required_travel.is_empty()
        || !callback.effects.carried_inputs.is_empty()
        || callback.effects.scene.is_some()
        || callback
            .effects
            .item_reward
            .as_ref()
            .is_some_and(|reward| !reward.backpack_outputs.is_empty());
    (callback.effects.item_reward.is_some()
        && (!published.contains(&callback.key) || new_value_terms))
        || callback.completes
        || callback.premium_required
}
