//! Native authored allocations use the shared map binder and once-claim mint owner.
//! A claim's placements are alternatives sharing one durable per-character claim;
//! independent caches must have independent claim keys. Container contents are
//! explicit allocations, never inferred from recipe prose or donor evidence.
//!
//! `native_authored_v1` requires ready, per-character once claims. Each placement
//! pins `source_binding.project_position`, `source_binding.native_entry_ordinal`
//! and `appearance_tibia_id`. `reward.items` declares one root (or bounded
//! independent roots with an exactly bound `reward_callback`); optional
//! `reward.contents` declares 1..=20 direct children of that root container.
//! `achievement` names a native catalogue key; `quest_transition` names the
//! existing quest obligation. The durable owner checks container capacity,
//! weights, item stack facts, achievement existence and current session fencing.
//! No level, Premium or access requirement is inferred from the reward recipe;
//! those remain the quest admission owner's responsibility.
use super::*;

pub(super) fn candidate(
    definition: &serde_json::Map<String, Value>,
    admitted: &dyn Fn(&TypedDefinitionRef) -> bool,
) -> Result<Result<Candidate, LeftOut>, ClaimsError> {
    let encoded = definition.get("definition_profile").and_then(Value::as_str)
        == Some("native_authored_v1")
        && definition
            .keys()
            .all(|key| DEFINITION_FIELDS.contains(&key.as_str()) || key == "definition_profile")
        && definition.get("readiness").and_then(Value::as_str) == Some("ready")
        && definition.get("claim")
            == Some(&serde_json::json!({"per": "character", "repeat": {"kind": "once"}}));
    if !encoded {
        return Ok(Err(LeftOut::UnencodedField));
    }
    let revision = definition
        .get("identity")
        .and_then(|v| v.get("revision"))
        .and_then(Value::as_str)
        .ok_or(ClaimsError("record without an identity revision"))?;
    let placements = definition
        .get("placements")
        .and_then(Value::as_array)
        .ok_or(ClaimsError("record without placements"))?;
    if placements.is_empty() {
        return Ok(Err(LeftOut::UnencodedField));
    }
    let mut parsed = Vec::with_capacity(placements.len());
    for placement in placements {
        let Some(placement) = placement_candidate(placement)? else {
            return Ok(Err(LeftOut::UnencodedField));
        };
        if !admitted(&placement.item)
            || placement
                .extra_items
                .iter()
                .any(|entry| !admitted(&entry.item))
            || placement
                .contents
                .iter()
                .any(|child| !admitted(&child.item))
        {
            return Ok(Err(LeftOut::ItemNotAdmitted));
        }
        parsed.push(placement);
    }
    Ok(Ok(Candidate {
        revision: revision.to_owned(),
        placements: parsed,
    }))
}

fn placement_candidate(placement: &Value) -> Result<Option<CandidatePlacement>, ClaimsError> {
    if !only(
        placement,
        &[
            "appearance_tibia_id",
            "source_binding",
            "quest_transition",
            "reward",
            "achievement",
            "reward_callback",
        ],
    ) {
        return Ok(None);
    }
    let Some(reward) = placement.get("reward") else {
        return Ok(None);
    };
    if !only(reward, &["items", "contents"]) {
        return Ok(None);
    }
    let mut ordinary = placement.clone();
    let Some(fields) = ordinary.as_object_mut() else {
        return Ok(None);
    };
    fields.remove("achievement");
    fields.remove("reward_callback");
    let Some(items) = reward
        .get("items")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty() && items.len() <= 20)
    else {
        return Ok(None);
    };
    let callback = match placement.get("reward_callback") {
        None => None,
        Some(value) => {
            let Some(key) = value
                .as_str()
                .filter(|key| key.starts_with("oteryn:quest-reward/"))
            else {
                return Ok(None);
            };
            if ProductionKey::new(key).is_err()
                || placement.get("quest_transition").is_some()
                || placement.get("achievement").is_some()
            {
                return Ok(None);
            }
            Some(key.to_owned())
        }
    };
    if items.len() > 1 && (callback.is_none() || reward.get("contents").is_some()) {
        return Ok(None);
    }
    ordinary["reward"] = serde_json::json!({"items": [items[0]]});
    let binding = &placement["source_binding"];
    if !only(binding, &["project_position", "native_entry_ordinal"]) {
        return Ok(None);
    }
    let Some(ordinal) = binding
        .get("native_entry_ordinal")
        .and_then(Value::as_u64)
        .and_then(|v| u8::try_from(v).ok())
    else {
        return Ok(None);
    };
    ordinary["source_binding"] = serde_json::json!({"project_position": binding["project_position"], "legacy_unique_ids": []});
    let Some(mut result) = candidate_placement(&ordinary, false)? else {
        return Ok(None);
    };
    result.native_ordinal = Some(ordinal);
    result.reward_callback = callback;
    for item in &items[1..] {
        let mut leaf = ordinary.clone();
        leaf["reward"] = serde_json::json!({"items": [item]});
        let Some(item) = candidate_placement(&leaf, false)? else {
            return Ok(None);
        };
        result.extra_items.push(RewardClaimItem {
            item: item.item,
            count: item.count,
        });
    }
    result.achievement = match placement.get("achievement") {
        None => None,
        Some(value) => {
            let Some(key) = value.as_str().filter(|key| {
                key.strip_prefix("oteryn:achievement/")
                    .is_some_and(|slug| !slug.is_empty())
            }) else {
                return Ok(None);
            };
            if ProductionKey::new(key).is_err() {
                return Ok(None);
            }
            Some(key.to_owned())
        }
    };
    if let Some(contents) = reward.get("contents") {
        let Some(contents) = contents
            .as_array()
            .filter(|rows| !rows.is_empty() && rows.len() <= 20)
        else {
            return Ok(None);
        };
        if result.count != 1 {
            return Ok(None);
        }
        for child in contents {
            // Fluid subtype belongs to one vessel, never to its stack quantity.
            let fluid_subtype = match child.get("fluid_subtype") {
                None => None,
                Some(value) => {
                    let Some(fluid) = value
                        .as_u64()
                        .and_then(|value| u8::try_from(value).ok())
                        .filter(|value| {
                            *value != 0 && child.get("count").and_then(Value::as_u64) == Some(1)
                        })
                    else {
                        return Ok(None);
                    };
                    Some(fluid)
                }
            };
            let mut quantity = child.clone();
            let Some(fields) = quantity.as_object_mut() else {
                return Ok(None);
            };
            fields.remove("fluid_subtype");
            // All remaining fields retain the existing exact Item/count grammar.
            let mut leaf = ordinary.clone();
            leaf["reward"] = serde_json::json!({"items": [quantity]});
            let Some(child) = candidate_placement(&leaf, false)? else {
                return Ok(None);
            };
            result.contents.push(RewardContainerItem {
                fluid_subtype,
                fluid_subtype: None,
                item: child.item,
                count: child.count,
                key_number: None,
                text: None,
            });
        }
    }
    Ok(Some(result))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::super::*;

    fn definition() -> Value {
        serde_json::json!({
            "identity": {"key": "oteryn:reward-claim.authored.allocation", "revision": "1"},
            "definition_profile": "native_authored_v1",
            "readiness": "ready",
            "claim": {"per": "character", "repeat": {"kind": "once"}},
            "placements": [{
                "appearance_tibia_id": 2472,
                "source_binding": {"native_entry_ordinal": 1,
                    "project_position": {"x": 100, "y": 100, "z": 7}},
                "reward": {"items": [{"item": {"family": "Item", "key": "oteryn:item.tibia.i2854", "revision": "definition-r1"}, "count": 1}],
                    "contents": [{"item": {"family": "Item", "key": "oteryn:item.tibia.i3031", "revision": "definition-r1"}, "count": 100}]},
                "achievement": "oteryn:achievement/annihilator"
            }]
        })
    }

    struct Base;
    struct Tile;
    impl RewardClaimBase for Base {
        type Tile<'a> = Tile;
        fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
            (x == 100 && y == 100 && floor == -7).then_some(Tile)
        }
    }
    impl RewardClaimTile for Tile {
        fn depths(&self) -> &[u8] {
            &[0, 0]
        }
        fn unique(&self, _: u8) -> Option<UniqueEntry> {
            None
        }
        fn appearance(&self, ordinal: u8) -> Option<u16> {
            (ordinal == 1).then_some(2472)
        }
    }

    #[test]
    fn native_authored_fluid_child_keeps_subtype_and_rejects_ambiguous_counts() {
        let mut source = definition();
        let child = &mut source["placements"][0]["reward"]["contents"][0];
        child["item"]["key"] = "oteryn:item.tibia.i2874".into();
        child["count"] = 1.into();
        child["fluid_subtype"] = 1.into();
        let shard = serde_json::json!({"records":[{"definition":source}]}).to_string();
        let report = bind_reward_claims(&Base, &[&shard], &|_| true).expect("binding");
        assert_eq!(report.served.len(), 1);
        assert_eq!(
            report.served[0].placements[0].contents[0].fluid_subtype,
            Some(1)
        );
        for mutation in 0..6 {
            let mut invalid = source.clone();
            let child = &mut invalid["placements"][0]["reward"]["contents"][0];
            match mutation {
                0 => child["count"] = 2.into(),
                1 => child["fluid_subtype"] = 0.into(),
                2 => child["fluid_subtype"] = 256.into(),
                3 => child["fluid_subtype"] = true.into(),
                4 => child["fluid_subtype"] = serde_json::Value::Null,
                _ => child["key_number"] = 3012.into(),
            }
            let shard = serde_json::json!({"records":[{"definition":invalid}]}).to_string();
            assert!(
                bind_reward_claims(&Base, &[&shard], &|_| true)
                    .expect("binding")
                    .served
                    .is_empty(),
                "accepted fluid mutation {mutation}"
            );
        }
    }

    #[test]
    fn native_authored_binding_needs_exact_entry_and_appearance_without_legacy_uid() {
        for (ordinal, appearance, bound) in [
            (1, 2472, true),
            (0, 2472, false),
            (2, 2472, false),
            (1, 2473, false),
        ] {
            let mut source = definition();
            source["placements"][0]["source_binding"]["native_entry_ordinal"] = ordinal.into();
            source["placements"][0]["appearance_tibia_id"] = appearance.into();
            let shard = serde_json::json!({"records": [{"definition": source}]}).to_string();
            let report = bind_reward_claims(&Base, &[&shard], &|_| true).expect("binding report");
            assert_eq!(report.served.len(), usize::from(bound));
            assert_eq!(report.unbound.is_empty(), bound);
        }
    }

    #[test]
    fn native_authored_duplicate_placement_refuses_both_claim_authorities() {
        let first = definition();
        let mut second = first.clone();
        second["identity"]["key"] = "oteryn:reward-claim.authored.other".into();
        let shard = serde_json::json!({"records": [{"definition": first}, {"definition": second}]})
            .to_string();
        let report = bind_reward_claims(&Base, &[&shard], &|_| true).expect("binding report");
        assert!(report.served.is_empty());
        assert_eq!(report.unbound.len(), 2);
        assert!(
            report
                .unbound
                .iter()
                .all(|entry| entry.reason == UnboundReason::AmbiguousEntry)
        );
    }

    #[test]
    fn native_authored_allocations_and_achievement_reach_shared_binder() {
        let source = definition();
        let parsed = candidate(source.as_object().expect("definition"), &|_| true)
            .expect("decode")
            .expect("native authored reward");
        assert_eq!(parsed.placements[0].contents[0].count, 100);
        assert_eq!(
            parsed.placements[0].achievement.as_deref(),
            Some("oteryn:achievement/annihilator")
        );
        assert!(matches!(
            candidate(source.as_object().expect("definition"), &|item| item
                .key()
                .as_str()
                != "oteryn:item.tibia.i3031"),
            Ok(Err(LeftOut::ItemNotAdmitted))
        ));
    }

    #[test]
    fn native_authored_independent_outputs_require_one_bound_callback() {
        let mut source = definition();
        source["placements"][0]
            .as_object_mut()
            .expect("placement")
            .remove("achievement");
        source["placements"][0]["reward"]
            .as_object_mut()
            .expect("reward")
            .remove("contents");
        source["placements"][0]["reward"]["items"].as_array_mut().expect("items").push(
            serde_json::json!({"item":{"family":"Item","key":"oteryn:item.tibia.i3354","revision":"definition-r1"},"count":1}));
        assert!(matches!(
            candidate(source.as_object().expect("definition"), &|_| true),
            Ok(Err(_))
        ));
        source["placements"][0]["reward_callback"] = "oteryn:quest-reward/engclaim-corpse".into();
        let shard = serde_json::json!({"records":[{"definition":source.clone()}]}).to_string();
        let report = bind_reward_claims(&Base, &[&shard], &|_| true).expect("binding");
        assert_eq!(report.served[0].placements[0].placement.items.len(), 2);
        assert!(matches!(
            candidate(source.as_object().expect("definition"), &|item| item
                .key()
                .as_str()
                != "oteryn:item.tibia.i3354"),
            Ok(Err(LeftOut::ItemNotAdmitted))
        ));
        let first = report.served[0].authority_digest;
        source["placements"][0]["reward"]["items"][1]["count"] = 2.into();
        let changed = serde_json::json!({"records":[{"definition":source}]}).to_string();
        assert_ne!(
            first,
            bind_reward_claims(&Base, &[&changed], &|_| true)
                .expect("binding")
                .served[0]
                .authority_digest
        );
    }

    #[test]
    fn native_authored_rewards_refuse_unknown_and_partial_shapes() {
        for mutation in [
            "repeat",
            "profile",
            "child",
            "achievement",
            "count",
            "readiness",
        ] {
            let mut source = definition();
            match mutation {
                "repeat" => source["claim"]["repeat"]["kind"] = "daily".into(),
                "profile" => source["definition_profile"] = "unrecognized".into(),
                "child" => source["placements"][0]["reward"]["contents"][0]["random"] = true.into(),
                "achievement" => source["placements"][0]["achievement"] = "client-selected".into(),
                "count" => source["placements"][0]["reward"]["contents"][0]["count"] = 0.into(),
                _ => source["readiness"] = "waiting_native_bindings".into(),
            }
            assert!(
                !matches!(
                    candidate(source.as_object().expect("definition"), &|_| true),
                    Ok(Ok(_))
                ),
                "{mutation}"
            );
        }
    }
}
