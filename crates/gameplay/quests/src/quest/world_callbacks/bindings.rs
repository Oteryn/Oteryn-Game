//! Exact native placement aliases for source callbacks whose donor UIDs are absent.
use super::*;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    callback: String,
    expected: Selector,
    selector: Selector,
    sources: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    bindings: Vec<Binding>,
}

pub(super) fn apply(callbacks: &mut [Callback]) -> Result<(), String> {
    apply_from(
        callbacks,
        include_str!("../../../../../../content/quests/native-world-callback-bindings.json"),
    )
}
fn apply_from(callbacks: &mut [Callback], source: &str) -> Result<(), String> {
    let document: Document = serde_json::from_str(source).map_err(|e| e.to_string())?;
    let mut keys = BTreeSet::new();
    if document.schema != 1 || document.bindings.len() > 64 {
        return Err("native callback binding bounds".into());
    }
    for binding in document.bindings {
        let selector = &binding.selector;
        if !keys.insert(binding.callback.clone())
            || !binding.callback.starts_with("oteryn:quest-callback/")
            || binding.callback.len() > 160
            || binding.expected.position.is_some()
            || binding.expected.unique.is_none_or(|value| value == 0)
            || binding.expected.action.is_some()
            || binding.expected.appearance.is_some()
            || selector
                .position
                .is_none_or(|p| !(-15..=0).contains(&p.floor))
            || selector.unique.is_some()
            || selector.action.is_some()
            || selector.appearance.is_none_or(|value| value == 0)
            || binding.sources.is_empty()
            || binding.sources.len() > 8
            || binding
                .sources
                .iter()
                .any(|s| s.is_empty() || s.len() > 1024 || s.chars().any(char::is_control))
        {
            return Err(format!(
                "invalid native callback binding {}",
                binding.callback
            ));
        }
        let callback = callbacks
            .iter_mut()
            .find(|c| c.key == binding.callback)
            .ok_or_else(|| format!("native binding target missing {}", binding.callback))?;
        if callback.selector != binding.expected {
            return Err(format!(
                "native binding source changed {}",
                binding.callback
            ));
        }
        callback.selector = binding.selector;
        callback.sources.extend(binding.sources);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_bindings_preserve_effects_and_refuse_ambiguous_or_changed_sources()
    -> Result<(), String> {
        let source =
            include_str!("../../../../../../content/quests/native-world-callback-bindings.json");
        let document: serde_json::Value =
            serde_json::from_str(source).map_err(|e| e.to_string())?;
        let mut callbacks = super::super::parse(super::super::EMBEDDED)?;
        let original = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/formogar-hoist-3059")
            .ok_or("source")?
            .clone();
        apply_from(&mut callbacks, source)?;
        let bound = callbacks
            .iter()
            .find(|c| c.key == original.key)
            .ok_or("bound")?;
        assert_eq!(bound.storage, original.storage);
        assert_eq!(bound.teleport, original.teleport);
        assert!(bound.is_read_only());
        assert_eq!(bound.selector.unique, None);
        assert_eq!(
            bound.selector.position,
            Some(Position {
                x: 32157,
                y: 31125,
                floor: -9
            })
        );
        for case in 0..5 {
            let mut bad = document.clone();
            match case {
                0 => bad["bindings"][0]["expected"]["unique"] = 1.into(),
                1 => bad["bindings"][0]["selector"]["position"]["floor"] = (-16).into(),
                2 => bad["bindings"][0]["callback"] = "oteryn:quest-callback/unknown".into(),
                3 => bad["bindings"][0]["selector"]["unique"] = 3059.into(),
                _ => {
                    let duplicate = bad["bindings"][0].clone();
                    bad["bindings"]
                        .as_array_mut()
                        .ok_or("bindings")?
                        .push(duplicate);
                }
            }
            assert!(
                apply_from(
                    &mut super::super::parse(super::super::EMBEDDED)?,
                    &bad.to_string()
                )
                .is_err(),
                "case {case}"
            );
        }
        Ok(())
    }
}
