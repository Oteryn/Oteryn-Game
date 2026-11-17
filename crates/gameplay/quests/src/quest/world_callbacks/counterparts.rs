//! Sparse binding/presentation additions; source state definitions remain authoritative.
use super::{ActorKind, Callback, MagicEffect, NativeSelector};
use serde::Deserialize;
use std::collections::BTreeSet;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    counterparts: Vec<Counterpart>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Counterpart {
    callback: String,
    native: NativeSelector,
    #[serde(default)]
    entry_effect: Option<u16>,
    #[serde(default)]
    teleport_effect: Option<u16>,
}

pub(super) fn apply(callbacks: &mut Vec<Callback>) -> Result<(), String> {
    apply_text(
        callbacks,
        include_str!("../../../../../../content/quests/native-counterparts.json"),
    )
}
fn apply_text(callbacks: &mut Vec<Callback>, text: &str) -> Result<(), String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if document.schema != 1 || document.counterparts.len() > super::MAX_CALLBACKS {
        return Err("native counterpart bounds".into());
    }
    let mut staged = callbacks.clone();
    let mut keys = BTreeSet::new();
    for entry in document.counterparts {
        if !keys.insert(entry.callback.clone()) {
            return Err("duplicate native counterpart".into());
        }
        let callback = staged
            .iter_mut()
            .find(|c| c.key == entry.callback)
            .ok_or("unknown native counterpart callback")?;
        if callback.actor_kind != ActorKind::Player
            || callback.native_rule.is_some()
            || callback.selector.native.is_some()
            || (entry.entry_effect.is_some() && callback.effects.magic.is_some())
            || (entry.teleport_effect.is_some() && callback.effects.teleport_effect.is_some())
            || [entry.entry_effect, entry.teleport_effect]
                .into_iter()
                .flatten()
                .any(|effect| effect == 0 || effect > 255)
        {
            return Err(
                "native counterpart cannot replace a closed binding or feedback owner".into(),
            );
        }
        if let Some(effect) = entry.entry_effect {
            callback.effects.magic = Some(MagicEffect {
                position: entry.native.position,
                effect,
            });
        }
        if let Some(effect) = entry.teleport_effect {
            callback.effects.teleport_effect = Some(effect);
        }
        callback.selector.native = Some(entry.native);
        // Apply the same closed callback validation as the source registries.
        let wire = serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
        super::parse(&wire)?;
    }
    super::validate_registry(&staged)?;
    *callbacks = staged;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn original() -> Result<Vec<Callback>, String> {
        super::super::parse(super::super::EMBEDDED)
    }
    fn document() -> Result<serde_json::Value, String> {
        serde_json::from_str(include_str!(
            "../../../../../../content/quests/native-counterparts.json"
        ))
        .map_err(|e| e.to_string())
    }
    #[test]
    fn metadata_preserves_source_state_and_rejects_replacement() -> Result<(), String> {
        let before = original()?;
        let mut callbacks = before.clone();
        apply(&mut callbacks)?;
        for old in &before {
            let new = callbacks
                .iter()
                .find(|c| c.key == old.key)
                .ok_or("callback")?;
            assert_eq!(new.storage, old.storage);
            assert_eq!(new.teleport, old.teleport);
            assert_eq!(new.quest, old.quest);
            assert_eq!(new.trigger, old.trigger);
            assert_eq!(new.selector.unique, old.selector.unique);
            assert_eq!(new.selector.action, old.selector.action);
            assert_eq!(new.sources, old.sources);
        }
        let committed = callbacks.clone();
        assert!(apply(&mut callbacks).is_err());
        assert_eq!(callbacks, committed);
        Ok(())
    }
    #[test]
    fn bad_metadata_never_partially_changes_callback_registry() -> Result<(), String> {
        let base = original()?;
        let doc = document()?;
        let mut variants = Vec::new();
        let mut bad = doc.clone();
        bad["counterparts"][1]["callback"] = "oteryn:quest-callback/unknown".into();
        variants.push(bad);
        let mut bad = doc.clone();
        bad["counterparts"][1] = bad["counterparts"][0].clone();
        variants.push(bad);
        let mut bad = doc.clone();
        bad["counterparts"][1]["native"]["position"]["floor"] = 1.into();
        variants.push(bad);
        let mut bad = doc.clone();
        bad["counterparts"][1]["entry_effect"] = 256.into();
        variants.push(bad);
        let mut bad = doc.clone();
        bad["counterparts"][1]["storage"] = serde_json::json!([]);
        variants.push(bad);
        let mut bad = doc;
        bad["counterparts"][1]["teleport_effect"] = 11.into();
        variants.push(bad);
        for bad in variants {
            let mut callbacks = base.clone();
            assert!(apply_text(&mut callbacks, &bad.to_string()).is_err());
            assert_eq!(callbacks, base);
        }
        Ok(())
    }
}
