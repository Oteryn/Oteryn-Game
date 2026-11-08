use super::*;
pub(super) fn binding(callback: &Callback) -> Result<Option<[u8; 32]>, String> {
    callback
        .carried_use
        .as_ref()
        .map(|_| {
            let bytes = serde_json::to_vec(&(
                &callback.key,
                &callback.quest,
                callback.trigger,
                &callback.storage,
                &callback.item_on_target,
                &callback.carried_use,
            ))
            .map_err(|error| error.to_string())?;
            let mut digest = Sha256::new();
            digest.update(bytes);
            if callback.trigger == Trigger::UseCarriedOnTarget {
                digest.update(b"\0quest-carried-target-selector-v1\0");
                digest.update(
                    serde_json::to_vec(&callback.selector).map_err(|error| error.to_string())?,
                );
            }
            if let Some(level) = callback.minimum_level {
                digest.update(b"\0quest-carried-minimum-level-v1\0");
                digest.update(level.to_be_bytes());
            }
            if let Some(reward) = &callback.effects.item_reward {
                digest.update(b"\0quest-carried-target-item-reward-v1\0");
                digest.update(serde_json::to_vec(reward).map_err(|error| error.to_string())?);
            }
            Ok::<[u8; 32], String>(digest.finalize().into())
        })
        .transpose()
}
