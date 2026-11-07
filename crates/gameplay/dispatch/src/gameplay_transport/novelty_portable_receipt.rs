//! Presentation-only projection for the finite source portable USE catalogue.
use super::*;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use serde_json::{Value, json};

pub(super) fn matches(
    prepared: &Value,
    receipt: &ConsumeTransformResult,
    native: &NativeGameplayState,
    node: &NodeIncarnationProof,
) -> Option<bool> {
    matches_binding(
        prepared,
        receipt,
        native,
        *node.fact().node_id().as_bytes(),
        node.fact().registration_revision(),
    )
}
fn matches_binding(
    prepared: &Value,
    receipt: &ConsumeTransformResult,
    native: &NativeGameplayState,
    node_id: [u8; 16],
    node_revision: u64,
) -> Option<bool> {
    let id = prepared["source_item_id"].as_u64()?;
    let key = format!("oteryn:item.tibia.i{id}");
    let target = oteryn_gameplay_items::furniture::portable_use_to(&key)?;
    let check = || -> Option<bool> {
        let source = native.item_policy_for_source_id(u32::try_from(id).ok()?)?;
        let after = native.item_policy_for_source_id(target)?;
        if !source.record().admission.materializable
            || !after.record().admission.materializable
            || receipt.recipe.as_deref()? != format!("oteryn:novelty-item.furniture-use.{key}.v1")
            || receipt.effects["portable_use"] != true
        {
            return Some(false);
        }
        let definition = &after.record().production_definition;
        let expected = json!({"key":definition.production_key,"revision":definition.revision_ref});
        if receipt.effects["rotate_after"] != expected {
            return Some(false);
        }
        let node_hex = node_id
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let expected_node = format!(
            "{}-{}-{}-{}-{}",
            &node_hex[..8],
            &node_hex[8..12],
            &node_hex[12..16],
            &node_hex[16..20],
            &node_hex[20..]
        );
        let expected_revision = node_revision.to_string();
        if receipt.effects["node_id"] != expected_node
            || receipt.effects["node_revision"].as_str() != Some(expected_revision.as_str())
        {
            return Some(false);
        }
        let decay =
            crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(after)
                .ok()?
                .decay;
        let mut public = receipt.effects.as_object()?.clone();
        if public.contains_key("house_after") || public.contains_key("house_temporal") {
            if public.remove("house_after")? != expected {
                return Some(false);
            }
            let temporal = public.remove("house_temporal")?;
            let temporal = temporal.as_object()?;
            if temporal.len() != 3
                || !temporal.contains_key("before")
                || !(temporal["before"].is_null() || temporal["before"].is_object())
                || temporal["duration"] != json!(decay.as_ref().map(|d| d.duration_millis))
                || temporal["target"]
                    != json!(
                        decay
                            .as_ref()
                            .and_then(|d| d.target.as_ref())
                            .map(|d| &d.production_key)
                    )
            {
                return Some(false);
            }
        } else if decay.is_some() {
            return Some(false);
        }
        for key in ["portable_use", "node_id", "node_revision", "rotate_after"] {
            public.remove(key)?;
        }
        // All source cue/text/position fields remain byte-for-byte value-equal.
        // Any unknown private or public property survives and refuses here.
        Some(prepared == &Value::Object(public))
    };
    Some(check().unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_all_eighty_portable_use_receipts_preserve_exact_public_projection()
    -> Result<(), Box<dyn std::error::Error>> {
        // A genuine version7 identifier, as required by the strict wire decoder.
        let world = crate::foundation::WorldId::decode(&[
            0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x71, 0x11, 0x81, 0x11, 0x11, 0x11, 0x11, 0x11,
            0x11, 0x11,
        ])
        .map_err(|e| format!("{e:?}"))?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active native")?;
        let native = active.native_gameplay().ok_or("native")?;
        let catalogue: Value = serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/native-gameplay/furniture-rules.json"
        )))?;
        let rows = catalogue["portable_use"]
            .as_array()
            .ok_or("portable source rows")?;
        assert_eq!(rows.len(), 80);
        for row in rows {
            let source = u32::try_from(row[0].as_u64().ok_or("source id")?)?;
            let policy = native
                .item_policy_for_source_id(source)
                .ok_or_else(|| format!("source {source}"))?;
            let def = &policy.record().production_definition;
            let used = crate::durability::item_mint::TypedDefinitionRef {
                family: def.family.clone(),
                production_key: def.production_key.clone(),
                revision_ref: def.revision_ref.clone(),
            };
            let plan = oteryn_gameplay_items::furniture::plan_portable_use(native, &used)
                .ok_or_else(|| format!("qualified plan {source}"))?;
            let typed: NoveltyEffects = serde_json::from_value(plan.effects.clone())?;
            let mut prepared = serde_json::to_value(typed)?;
            prepared["ground"] = json!({"x":7,"y":0,"floor":7});
            let target = oteryn_gameplay_items::furniture::portable_use_to(&used.production_key)
                .ok_or("source target")?;
            assert_eq!(u64::from(target), row[1].as_u64().ok_or("captured target")?);
            let target_policy = native
                .item_policy_for_source_id(target)
                .ok_or("native target")?;
            let after = &target_policy.record().production_definition;
            let decay =
                crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(
                    target_policy,
                )
                .map_err(|e| {
                    format!("portable source{source} target{target} native lifetime: {e:?}")
                })?
                .decay;
            let mut effects = prepared.clone();
            effects["portable_use"] = true.into();
            effects["node_id"] = "01010101-0101-0101-0101-010101010101".into();
            effects["node_revision"] = "1".into();
            effects["rotate_after"] =
                json!({"key":after.production_key,"revision":after.revision_ref});
            if let Some(decay) = decay {
                effects["house_after"] = effects["rotate_after"].clone();
                effects["house_temporal"] = json!({"before":null,"duration":decay.duration_millis,"target":decay.target.as_ref().map(|d|&d.production_key)});
            }
            let receipt = ConsumeTransformResult {
                disposition: UseDisposition::Committed,
                fresh: true,
                effects,
                recipe: Some(plan.recipe),
            };
            assert_eq!(
                matches_binding(&prepared, &receipt, native, [1; 16], 1),
                Some(true),
                "source {source}"
            );
            let mut carried = receipt.clone();
            let mut carried_prepared = prepared.clone();
            carried_prepared
                .as_object_mut()
                .ok_or("prepared object")?
                .remove("ground");
            carried
                .effects
                .as_object_mut()
                .ok_or("receipt object")?
                .remove("ground");
            assert_eq!(
                matches_binding(&carried_prepared, &carried, native, [1; 16], 1),
                Some(true),
                "carried or nested source {source}"
            );
            let mut wrong_recipe = receipt.clone();
            wrong_recipe.recipe =
                Some("oteryn:novelty-item.furniture-use.oteryn:item.tibia.i1.v1".into());
            assert_eq!(
                matches_binding(&prepared, &wrong_recipe, native, [1; 16], 1),
                Some(false)
            );
            for field in ["duration", "target"] {
                if receipt.effects.get("house_temporal").is_some() {
                    let mut wrong = receipt.clone();
                    wrong.effects["house_temporal"][field] = json!("wrong");
                    assert_eq!(
                        matches_binding(&prepared, &wrong, native, [1; 16], 1),
                        Some(false),
                        "source {source} changed temporal {field}"
                    );
                }
            }
            for field in [
                "ground",
                "speech",
                "cues",
                "node_id",
                "node_revision",
                "source_item_id",
                "rotate_after",
                "unknown_private",
            ] {
                let mut wrong = receipt.clone();
                wrong.effects[field] = json!("wrong");
                assert_eq!(
                    matches_binding(&prepared, &wrong, native, [1; 16], 1),
                    Some(false),
                    "source {source} changed {field}"
                );
            }
        }
        Ok(())
    }
}
