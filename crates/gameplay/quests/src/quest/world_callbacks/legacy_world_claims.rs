//! The M39 source books retain their existing per-character RewardClaim identities.
//! This resolves immutable source pairs, never an alias supplied by a caller.
use super::{ActorKind, Callback, Trigger};
use crate::quest::reward_callbacks::{ItemReward, NativeClaimBinding};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::OnceLock};

const CALLBACKS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../content/quests/world-callbacks/m39-stories.json"
));
const CLAIMS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../content/interactions/reward_claims/reward-claims-00200-00230.json"
));
const PAIRS: &[(&str, &str)] = &[
    (
        "l2-tome-children-1",
        "children_of_the_revolution.chest_tome_of_knowledge1",
    ),
    (
        "l2-tome-children-2",
        "children_of_the_revolution.chest_tome_of_knowledge2",
    ),
    (
        "l2-tome-frontier-1",
        "the_new_frontier.chest_tome_of_knowledge1",
    ),
    (
        "l2-tome-frontier-2",
        "the_new_frontier.chest_tome_of_knowledge2",
    ),
    (
        "l2-tome-knowledge-1",
        "tomes_of_knowledge.chest_tome_of_knowledge1",
    ),
    (
        "l2-tome-knowledge-2",
        "tomes_of_knowledge.chest_tome_of_knowledge2",
    ),
    (
        "l2-tome-knowledge-3",
        "tomes_of_knowledge.chest_tome_of_knowledge3",
    ),
    (
        "l2-tome-knowledge-4",
        "tomes_of_knowledge.chest_tome_of_knowledge4",
    ),
    ("l2-tome-top-tower", "tomes_of_knowledge.top_tower"),
];

struct Binding {
    callback: Callback,
    claim: NativeClaimBinding,
    source_unique: u16,
}

fn only(value: &Value, fields: &[&str]) -> bool {
    value
        .as_object()
        .is_some_and(|object| object.keys().all(|key| fields.contains(&key.as_str())))
}

fn bind(callbacks: &str, claims: &str) -> Result<Vec<Binding>, String> {
    let callbacks = super::parse(callbacks)?;
    let claims: Value = serde_json::from_str(claims).map_err(|error| error.to_string())?;
    if claims["schema"] != "OTERYN_REWARD_CLAIM_SHARD/v1" || claims["family"] != "RewardClaim" {
        return Err("legacy World claim shard".into());
    }
    let records = claims["records"]
        .as_array()
        .ok_or("legacy World claim records")?;
    let mut bindings = Vec::new();
    let mut positions = BTreeSet::new();
    let reward: ItemReward = serde_json::from_value(json!({
        "item": "oteryn:item.tibia.i10217", "count": 1, "contents": [],
    }))
    .map_err(|error| error.to_string())?;
    for (callback_suffix, claim_suffix) in PAIRS {
        let key = format!("oteryn:quest-callback/{callback_suffix}");
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or("legacy World callback missing")?;
        let claim_key = format!("oteryn:reward-claim.quest.u8_54.{claim_suffix}");
        let matching: Vec<_> = records
            .iter()
            .map(|row| &row["definition"])
            .filter(|definition| definition["identity"]["key"] == claim_key)
            .collect();
        let [definition] = matching.as_slice() else {
            return Err("legacy World claim must be unique".into());
        };
        let position = callback
            .selector
            .position
            .ok_or("legacy World claim position")?;
        let appearance = callback
            .selector
            .appearance
            .ok_or("legacy World claim appearance")?;
        let placements = definition["placements"]
            .as_array()
            .ok_or("legacy World placements")?;
        let [placement] = placements.as_slice() else {
            return Err("legacy World claim must have one placement".into());
        };
        let source = &placement["source_binding"];
        let ids = source["legacy_unique_ids"]
            .as_array()
            .ok_or("legacy World source ids")?;
        let crystal: Vec<_> = ids
            .iter()
            .filter(|id| id["server"] == "crystalserver")
            .collect();
        let [crystal] = crystal.as_slice() else {
            return Err("legacy World Crystal source must be unique".into());
        };
        let uid = crystal["unique_id"]
            .as_u64()
            .filter(|uid| (1..=u64::from(u16::MAX)).contains(uid))
            .ok_or("legacy World source id")?;
        let source_quest = callback
            .quest
            .strip_prefix("oteryn:quest.")
            .ok_or("legacy World quest identity")?;
        if !positions.insert(position)
            || !only(
                definition,
                &[
                    "identity",
                    "claim",
                    "placements",
                    "quest",
                    "readiness",
                    "provenance",
                ],
            )
            || definition["identity"] != json!({"key": claim_key, "revision": "reward-claim-r1"})
            || definition["claim"] != json!({"per": "character", "repeat": {"kind": "once"}})
            || definition["readiness"] != "ready"
            || definition["quest"]["family"] != "Quest"
            || definition["quest"]["key"] != format!("canary:quest/{source_quest}")
            || !only(
                placement,
                &["appearance_tibia_id", "reward", "source_binding"],
            )
            || placement["appearance_tibia_id"] != appearance
            || placement["reward"]
                != json!({"items": [{"item": {
                "family": "Item", "key": "oteryn:item.tibia.i10217", "revision": "definition-r1"
            }, "count": 1}]})
            || !only(source, &["legacy_unique_ids", "project_position"])
            || source["project_position"]
                != json!({"x": position.x, "y": position.y, "z": -i16::from(position.floor)})
            || source["legacy_unique_ids"]
                != json!([
                    {"server": "canary", "unique_id": uid},
                    {"server": "crystalserver", "unique_id": uid},
                ])
            || callback.trigger != Trigger::Use
            || callback.actor_kind != ActorKind::Player
            || !matches!(appearance, 2472 | 2480)
            || callback.selector.unique.is_some()
            || callback.selector.action.is_some()
            || callback.selector.native.is_some()
            || callback.effects.item_reward.as_ref() != Some(&reward)
            || callback.effects.item_reward_cycle.is_some()
            || callback.effects.item_reward_per_use
            || callback.has_fenced_regrowth_reward()
        {
            return Err(format!("legacy World claim source mismatch: {key}"));
        }
        // These are the same semantic terms hashed by Content::bind_reward_claims.
        let terms = json!({
            "identity": definition.get("identity"),
            "claim": definition.get("claim"),
            "placements": definition.get("placements"),
        });
        let authority_sha256 =
            Sha256::digest(serde_json::to_vec(&terms).map_err(|error| error.to_string())?)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
        bindings.push(Binding {
            callback: callback.clone(),
            source_unique: u16::try_from(uid).map_err(|_| "legacy World source id")?,
            claim: NativeClaimBinding {
                key: claim_key,
                revision: "reward-claim-r1".into(),
                authority_sha256,
            },
        });
    }
    Ok(bindings)
}

/// Preserve an existing once claim only for its exact compiled source callback.
/// The World writer still owns physical admission and atomic progress plus MINT.
pub fn legacy_world_claim(
    callback: &Callback,
) -> Result<Option<&'static NativeClaimBinding>, String> {
    legacy_world_binding(callback).map(|binding| binding.map(|binding| &binding.claim))
}

/// Retained Crystal UID from the same fully checked immutable claim authority.
/// A changed callback cannot borrow this source or bypass its legacy owner.
pub fn legacy_world_source_unique(callback: &Callback) -> Result<Option<u16>, String> {
    legacy_world_binding(callback).map(|binding| binding.map(|binding| binding.source_unique))
}

fn legacy_world_binding(callback: &Callback) -> Result<Option<&'static Binding>, String> {
    static BINDINGS: OnceLock<Result<Vec<Binding>, String>> = OnceLock::new();
    let bindings = BINDINGS
        .get_or_init(|| bind(CALLBACKS, CLAIMS))
        .as_ref()
        .map_err(Clone::clone)?;
    let bound =
        bindings.iter().find(|binding| {
            callback.key == binding.callback.key
                || callback.selector.position == binding.callback.selector.position
                || callback.selector.native.as_ref().is_some_and(|native| {
                    Some(native.position) == binding.callback.selector.position
                })
        });
    match bound {
        Some(binding) if callback == &binding.callback => Ok(Some(binding)),
        Some(_) => Err("legacy World claim callback changed".into()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_source_books_retain_their_once_identity_and_the_tenth_stays_independent()
    -> Result<(), String> {
        let callbacks = super::super::load()?;
        let mut claimed = BTreeSet::new();
        for (callback_suffix, claim_suffix) in PAIRS {
            let key = format!("oteryn:quest-callback/{callback_suffix}");
            let callback = callbacks
                .iter()
                .find(|callback| callback.key == key)
                .ok_or(key)?;
            let proof = legacy_world_claim(callback)?.ok_or("legacy claim proof")?;
            assert_eq!(
                proof.key,
                format!("oteryn:reward-claim.quest.u8_54.{claim_suffix}")
            );
            assert_eq!(proof.revision, "reward-claim-r1");
            assert!(
                claimed.insert(&proof.key),
                "each source has its own once identity"
            );
        }
        assert_eq!(claimed.len(), 9);
        let tenth = callbacks
            .iter()
            .find(|callback| callback.key == "oteryn:quest-callback/l2-tome-container")
            .ok_or("independent tenth book")?;
        assert!(legacy_world_claim(tenth)?.is_none());
        assert!(legacy_world_source_unique(tenth)?.is_none());
        Ok(())
    }

    #[test]
    fn exact_world_sources_retain_the_canonical_crystal_uids() -> Result<(), String> {
        let callbacks = super::super::load()?;
        for (suffix, unique) in [
            ("children-1", 6289),
            ("children-2", 6291),
            ("frontier-1", 6287),
            ("frontier-2", 6288),
            ("knowledge-1", 6292),
            ("knowledge-2", 6293),
            ("knowledge-3", 6294),
            ("knowledge-4", 6295),
            ("top-tower", 6104),
        ] {
            let key = format!("oteryn:quest-callback/l2-tome-{suffix}");
            let callback = callbacks
                .iter()
                .find(|callback| callback.key == key)
                .ok_or(key)?;
            assert_eq!(legacy_world_source_unique(callback)?, Some(unique));
        }
        Ok(())
    }

    #[test]
    fn a_changed_callback_cannot_borrow_or_escape_the_legacy_identity() -> Result<(), String> {
        let bindings = bind(CALLBACKS, CLAIMS)?;
        for binding in bindings {
            for mutation in 0..9 {
                let mut changed = binding.callback.clone();
                match mutation {
                    0 => changed.key.push_str("-alias"),
                    1 => changed.selector.position.as_mut().ok_or("position")?.x += 1,
                    2 => {
                        changed.selector.appearance =
                            Some(if changed.selector.appearance == Some(2472) {
                                2480
                            } else {
                                2472
                            })
                    }
                    3 => changed.quest.push_str("_alias"),
                    4 => changed.storage.first_mut().ok_or("storage")?.set += 1,
                    5 => changed.effects.item_reward.as_mut().ok_or("reward")?.count += 1,
                    6 => changed.effects.item_reward_per_use = true,
                    7 => changed.sources.clear(),
                    _ => changed.effects.item_reward = None,
                }
                assert!(legacy_world_claim(&changed).is_err(), "mutation {mutation}");
                assert!(
                    legacy_world_source_unique(&changed).is_err(),
                    "source mutation {mutation}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn missing_duplicate_or_changed_legacy_authority_fails_closed() -> Result<(), String> {
        let original: Value = serde_json::from_str(CLAIMS).map_err(|error| error.to_string())?;
        for mutation in 0..10 {
            let mut changed = original.clone();
            let records = changed["records"].as_array_mut().ok_or("records")?;
            let index = records.iter().position(|row| row["definition"]["identity"]["key"]
                == "oteryn:reward-claim.quest.u8_54.the_new_frontier.chest_tome_of_knowledge1")
                .ok_or("legacy Frontier claim")?;
            let definition = &mut records[index]["definition"];
            match mutation {
                0 => definition["claim"]["repeat"]["kind"] = "daily".into(),
                1 => definition["claim"]["per"] = "account".into(),
                2 => definition["readiness"] = "waiting_item_semantics".into(),
                3 => {
                    definition["placements"][0]["source_binding"]["project_position"]["x"] =
                        1.into()
                }
                4 => definition["placements"][0]["appearance_tibia_id"] = 2480.into(),
                5 => definition["placements"][0]["reward"]["items"][0]["count"] = 2.into(),
                6 => definition["placements"][0]["quest_transition"] = "unexpected-owner".into(),
                7 => definition["identity"]["revision"] = "changed".into(),
                8 => {
                    records.remove(index);
                }
                _ => records.push(records[index].clone()),
            }
            assert!(
                bind(CALLBACKS, &changed.to_string()).is_err(),
                "mutation {mutation}"
            );
        }
        Ok(())
    }
}
