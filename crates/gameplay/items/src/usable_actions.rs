//! SRVGAP-C4. Crystal 201189f: actions/items/{fireworks_rocket,party_trumpet,
//! party_hat,voodoo_doll,explosive_present,usable_outfit_items,usable_mount_items}.lua
//! and data-crystal/scripts/special/{addon_doll,mount_random}.lua.
//! The captured catalogues retain each rule's exact donor file:line.
use crate::reward_bag::reward_bag_bounded_draw;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, sync::OnceLock};

pub const PROVENANCE: &str = "crystal-item-cosmetics-201189f-v1";
pub const CAUSE: u32 = 970;

pub fn cosmetics() -> Option<&'static Value> {
    static SOURCE: OnceLock<Option<Value>> = OnceLock::new();
    SOURCE
        .get_or_init(|| {
            let value: Value = serde_json::from_str(include_str!(
                "../../../../tools/content-schema/native-gameplay/item-cosmetic-actions.json"
            ))
            .ok()?;
            (value["schema"] == "OTERYN_ITEM_COSMETICS/v1"
                && value["crystal_commit"] == "201189f871d23597b82caf88fd611454a2ddefb8")
                .then_some(value)
        })
        .as_ref()
}
fn novelty() -> Option<&'static Value> {
    static SOURCE: OnceLock<Option<Value>> = OnceLock::new();
    SOURCE
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../tools/content-schema/native-gameplay/novelty-item-actions.json"
            ))
            .ok()
        })
        .as_ref()
}
pub fn item_id(key: &str) -> Option<u32> {
    key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
}
pub fn cosmetic_rule(id: u32) -> Option<&'static Value> {
    cosmetics()?["rules"].as_array()?.iter().find(|r| {
        r["item_id"] == id
            && matches!(
                r["mode"].as_str(),
                Some("direct" | "random_outfit" | "random_mount")
            )
    })
}
pub fn supported(key: &str) -> bool {
    item_id(key).is_some_and(|id| {
        matches!(id, 906 | 3002 | 6572 | 6576 | 6578) || cosmetic_rule(id).is_some()
    })
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Action {
    pub item_id: u32,
    pub consume: bool,
    pub output: Option<u32>,
    pub grants: Vec<String>,
    pub effects: Vec<u16>,
    pub text: Option<String>,
    #[serde(default)]
    pub private_text: bool,
    pub damage: u32,
    pub achievements: Vec<String>,
    pub progress: Vec<(String, u32)>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Requirement,
    Duplicate,
    Invalid,
}
fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
fn progress(value: &Value) -> Option<(String, u32)> {
    Some((
        value["key"].as_str()?.to_owned(),
        u32::try_from(value["target"].as_u64()?).ok()?,
    ))
}

/// Donor duplicate draws are retained without consuming; no reroll of an owned choice.
/// The exact command receipt, rather than another draw, answers a lost acknowledgement.
#[allow(clippy::too_many_arguments)]
pub fn decide(
    key: &str,
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
    owned: &BTreeSet<String>,
    premium: bool,
    on_head: bool,
    player_target: bool,
) -> Result<Action, Refusal> {
    let id = item_id(key).ok_or(Refusal::Invalid)?;
    let draw = |upper| {
        reward_bag_bounded_draw(session, command, source, 58, upper).ok_or(Refusal::Invalid)
    };
    let mut action = Action {
        item_id: id,
        ..Action::default()
    };
    if let Some(rule) = cosmetic_rule(id) {
        if rule["premium"] == true && !premium {
            return Err(Refusal::Requirement);
        }
        let choices = rule["choices"].as_array().ok_or(Refusal::Invalid)?;
        let choice = choices
            .get(usize::try_from(draw(choices.len() as u64)?).map_err(|_| Refusal::Invalid)?)
            .ok_or(Refusal::Invalid)?;
        if strings(&choice["required"])
            .iter()
            .any(|g| !owned.contains(g))
        {
            return Err(Refusal::Requirement);
        }
        if choice["owned"].as_str().is_some_and(|g| owned.contains(g)) {
            return Err(Refusal::Duplicate);
        }
        action.consume = true;
        action.grants = strings(&choice["grants"])
            .into_iter()
            .filter(|g| !owned.contains(g))
            .collect();
        action.effects = choice["cues"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| c["effect_id"].as_u64().and_then(|v| v.try_into().ok()))
            .collect();
        action.text = choice["speech"]
            .as_str()
            .or(choice["feedback"].as_str())
            .map(str::to_owned);
        action.private_text = choice["speech"].is_null();
        action.achievements = strings(&choice["achievements"]);
        action.progress = choice["achievement_progress"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(progress)
            .collect();
        if let Some(achievement) = choice["achievement_when_full"].as_str()
            && strings(&choice["complete_grants"])
                .iter()
                .all(|g| owned.contains(g) || action.grants.contains(g))
        {
            action.achievements.push(achievement.to_owned());
        }
        return Ok(action);
    }
    let rule = novelty()
        .and_then(|c| c["rules"].as_array())
        .and_then(|rs| {
            rs.iter().find(|r| {
                r["item_ids"]
                    .as_array()
                    .is_some_and(|ids| ids.contains(&Value::from(id)))
            })
        })
        .ok_or(Refusal::Invalid)?;
    // Crystal party_hat.lua:4-6 and voodoo_doll.lua:4-5 refuse without spending.
    if (id == 6578 && !on_head) || (id == 3002 && !player_target) {
        return Err(Refusal::Requirement);
    }
    if !matches!(id, 906 | 3002 | 6572 | 6576 | 6578) {
        return Err(Refusal::Invalid);
    }
    action.private_text = id == 3002;
    let success = id != 3002 || draw(100)? < 5;
    action.consume = matches!(id, 906 | 6572 | 6576);
    action.output = rule["output"].as_u64().and_then(|v| v.try_into().ok());
    if success {
        action.effects = rule["effect_ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_u64().and_then(|v| v.try_into().ok()))
            .collect();
        if let Some(key) = rule["achievement"].as_str() {
            action.achievements.push(key.to_owned());
        }
    }
    action.text = if id == 3002 {
        rule[if success {
            "success_text"
        } else {
            "failure_text"
        }]
        .as_str()
    } else {
        rule["speech"].as_str()
    }
    .map(str::to_owned);
    action.damage = rule["damage"]
        .as_u64()
        .and_then(|v| v.try_into().ok())
        .unwrap_or(0);
    action
        .progress
        .extend(progress(&rule["achievement_progress"]));
    action
        .progress
        .extend(progress(&rule["secondary_achievement_progress"]));
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sourced_actions_refuse_duplicates_premium_missing_base_and_unequipped_hat()
    -> Result<(), Refusal> {
        let empty = BTreeSet::new();
        let decide = |key, owned: &BTreeSet<_>, premium, head, target| {
            super::decide(key, [1; 16], 1, [2; 16], owned, premium, head, target)
        };
        assert_eq!(
            decide("oteryn:item.tibia.i16252", &empty, false, false, false),
            Err(Refusal::Requirement)
        );
        assert_eq!(
            decide("oteryn:item.tibia.i16253", &empty, true, false, false),
            Err(Refusal::Requirement)
        );
        let base = decide("oteryn:item.tibia.i16252", &empty, true, false, false)?;
        let owned = base.grants.into_iter().collect();
        assert_eq!(
            decide("oteryn:item.tibia.i16252", &owned, true, false, false),
            Err(Refusal::Duplicate)
        );
        assert!(decide("oteryn:item.tibia.i16253", &owned, true, false, false)?.consume);
        assert_eq!(
            decide("oteryn:item.tibia.i6578", &empty, true, false, false),
            Err(Refusal::Requirement)
        );
        assert!(!decide("oteryn:item.tibia.i6578", &empty, true, true, false)?.consume);
        assert_eq!(
            decide("oteryn:item.tibia.i3002", &empty, true, false, false),
            Err(Refusal::Requirement)
        );
        assert_eq!(
            decide("oteryn:item.tibia.i6576", &empty, false, false, false)?.damage,
            10
        );
        assert_eq!(
            decide("oteryn:item.tibia.i6572", &empty, false, false, false)?.output,
            Some(6573)
        );
        Ok(())
    }
    #[test]
    fn random_outfit_and_mount_duplicates_never_consume_or_reroll() -> Result<(), Refusal> {
        let owned: BTreeSet<String> = cosmetics().ok_or(Refusal::Invalid)?["grants"]
            .as_array()
            .ok_or(Refusal::Invalid)?
            .iter()
            .filter_map(|g| g["key"].as_str().map(str::to_owned))
            .collect();
        for command in 1..=100 {
            for key in ["oteryn:item.tibia.i6579", "oteryn:item.tibia.i37746"] {
                assert_eq!(
                    decide(key, [1; 16], command, [2; 16], &owned, false, false, false),
                    Err(Refusal::Duplicate)
                );
                let action = decide(
                    key,
                    [1; 16],
                    command,
                    [2; 16],
                    &BTreeSet::new(),
                    false,
                    false,
                    false,
                )?;
                assert!(action.consume);
                assert_eq!(
                    action,
                    decide(
                        key,
                        [1; 16],
                        command,
                        [2; 16],
                        &BTreeSet::new(),
                        false,
                        false,
                        false
                    )?
                );
            }
        }
        Ok(())
    }
}
