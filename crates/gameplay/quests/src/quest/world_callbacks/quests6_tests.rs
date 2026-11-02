//! Child of quest::world_callbacks, using its real loader and evaluator.
//! Run RED before adding the per-quest callback records, GREEN afterwards.
//! These tests do not claim missing production UID/AID bindings are present.
use super::*;

fn named(suffix: &str) -> Result<Callback, String> {
    load()?
        .into_iter()
        .find(|c| c.key == format!("oteryn:quest-callback/{suffix}"))
        .ok_or_else(|| format!("missing source callback {suffix}"))
}

