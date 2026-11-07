//! Authored event rules require the verified native selector, not a donor label.
use super::Callback;
use serde_json::Value;
use std::sync::OnceLock;

pub(super) fn valid(callback: &Callback) -> bool {
    if callback
        .effects
        .world_event
        .as_ref()
        .is_none_or(|binding| !binding.valid())
        || !callback.sources.iter().any(|source| {
            source
                .strip_prefix("oteryn-policy:CONTRACT-NOTES.md#")
                .is_some_and(|anchor| !anchor.is_empty())
        })
    {
        return false;
    }
    let Some(position) = callback.selector.position else {
        return false;
    };
    let Ok(z) = u16::try_from(-i16::from(position.floor)) else {
        return false;
    };
    static BINDINGS: OnceLock<Option<Vec<Value>>> = OnceLock::new();
    let bindings = BINDINGS.get_or_init(|| {
        let document: Value = serde_json::from_str(include_str!(
            "../../../../../../content/quests/native-object-bindings.json"
        ))
        .ok()?;
        if document["schema"] != 1 {
            return None;
        }
        document["bindings"]
            .as_array()
            .filter(|rows| rows.len() <= 256)
            .cloned()
    });
    bindings.as_ref().is_some_and(|bindings| {
        bindings.iter().any(|row| {
            row["source"]["policy"] == "content/quests/world-event-object-policy.json"
                && row["position"] == serde_json::json!([position.x, position.y, z])
                && row["action"].as_u64() == callback.selector.action.map(u64::from)
                && row["unique"].as_u64() == callback.selector.unique.map(u64::from)
                && row["item"]
                    .as_u64()
                    .is_none_or(|item| callback.selector.appearance.map(u64::from) == Some(item))
        })
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn wevents_authored_native_event_source_requires_exact_selector() {
        let callbacks = super::super::load().expect("all native callbacks must load");
        let mut callback = callbacks
            .into_iter()
            .find(|c| c.key.ends_with("insect-restoration-water"))
            .expect("water binding");
        assert!(super::valid(&callback));
        callback.selector.action = Some(52674);
        assert!(!super::valid(&callback));
        callback.selector.action = Some(52670);
        callback.effects.world_event = None;
        assert!(!super::valid(&callback));
    }
}
