//! Closed native StepIn handlers. Source AIDs are read from compiled map attributes.
use super::{Callback, Trigger};
use serde::Deserialize;
use std::collections::BTreeSet;

pub const EMBEDDED: &str =
    include_str!("../../../../../../content/quests/action-marker-bindings.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    bindings: Vec<Binding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    action: u16,
    handler: String,
    callbacks: Vec<Callback>,
}

/// Called by the production callback loader; unknown AIDs cannot create handlers.
pub fn load(source: &str) -> Result<Vec<Callback>, String> {
    let document: Document = serde_json::from_str(source).map_err(|e| e.to_string())?;
    if document.schema != 1 || document.bindings.len() > 1 {
        return Err("native StepIn binding bounds".into());
    }
    let mut actions = BTreeSet::new();
    let mut callbacks = Vec::new();
    for binding in document.bindings {
        if binding.action != 25018
            || binding.handler != "cursed_crystal_stairs"
            || !actions.insert(binding.action)
            || binding.callbacks.len() != 4
            || binding.callbacks.iter().any(|callback| {
                callback.trigger != Trigger::StepIn
                    || callback.selector.action != Some(binding.action)
                    || callback.selector.ordinal.is_some()
                    || callback.selector.position.is_none()
                    || callback.teleport.is_none()
                    || !callback.storage.is_empty()
            })
        {
            return Err(format!(
                "unknown or invalid native StepIn action {}",
                binding.action
            ));
        }
        let source = serde_json::json!({"schema": 1, "callbacks": binding.callbacks});
        callbacks.extend(super::parse(&source.to_string())?);
    }
    Ok(callbacks)
}
