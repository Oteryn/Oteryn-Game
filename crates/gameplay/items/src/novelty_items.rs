//! Donor-authored novelty actions, selected before the shared consume/transform commit.
//! A receipt retains the complete outcome; replay never draws a second costume, die or doll.

use crate::reward_bag::reward_bag_bounded_draw;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

mod feroxa_trap;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NoveltyKind {
    Costume,
    Reward,
    Cosmetic,
    HouseTransform,
    Dice,
    Firework,
    Transform,
    EquippedEffect,
    ConsumeEffect,
    PlayerTargetEffect,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AchievementProgress {
    pub key: String,
    pub target: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Costume {
    pub monster: String,
    pub look_type: u32,
    pub donor: String,
}

#[derive(Debug, Deserialize)]
pub struct NoveltyRule {
    pub item_ids: Vec<u32>,
    pub donor: String,
    pub kind: NoveltyKind,
    pub cues: Vec<String>,
    pub effect_ids: Vec<u16>,
    pub cue_origin: Option<String>,
    /// min/max X followed by min/max Y, including the donor spectator defaults.
    pub dice_speech_range: Option<[u32; 4]>,
    #[serde(default)]
    pub ground_cues: Vec<String>,
    #[serde(default)]
    pub ground_effect_ids: Vec<u16>,
    #[serde(default)]
    pub costumes: Vec<Costume>,
    pub duration_ms: Option<u32>,
    pub output: Option<u32>,
    pub decay_to: Option<u32>,
    pub damage: Option<u32>,
    pub speech: Option<String>,
    pub success_percent: Option<u32>,
    pub success_text: Option<String>,
    pub failure_text: Option<String>,
    pub achievement: Option<String>,
    pub achievement_progress: Option<AchievementProgress>,
    pub secondary_achievement_progress: Option<AchievementProgress>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NoveltyCue {
    pub binding: String,
    pub effect_id: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CostumeEffect {
    pub monster: String,
    pub look_type: u32,
    pub duration_ms: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TimedOutput {
    pub duration_ms: u32,
    pub decay_to: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GroundEffect {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// Server-authored effects retained in the same fenced receipt as item consumption.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NoveltyEffects {
    pub kind: NoveltyKind,
    pub source_item_id: u32,
    pub donor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<GroundEffect>,
    pub cues: Vec<NoveltyCue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speech: Option<String>,
    pub private_speech: bool,
    pub require_head: bool,
    pub require_player_target: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dice_value: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub costume: Option<CostumeEffect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub achievement: Option<String>,
    pub achievement_progress: Vec<AchievementProgress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timed_output: Option<TimedOutput>,
}

#[derive(Debug)]
pub struct NoveltyPlan {
    pub recipe: String,
    pub outputs: Vec<(TypedDefinitionRef, u32)>,
    pub consume_source: bool,
    pub effects: serde_json::Value,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NoveltySelection {
    pub output: Option<u32>,
    pub consume_source: bool,
    pub effects: NoveltyEffects,
}

pub fn rules() -> Option<&'static [NoveltyRule]> {
    #[derive(Deserialize)]
    struct Catalogue {
        schema: String,
        crystal_commit: String,
        rules: Vec<NoveltyRule>,
    }
    static RULES: OnceLock<Option<Vec<NoveltyRule>>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let catalogue: Catalogue = serde_json::from_str(include_str!(
                "../../../../tools/content-schema/native-gameplay/novelty-item-actions.json"
            ))
            .ok()?;
            let mut ids = std::collections::BTreeSet::new();
            if catalogue.schema != "OTERYN_NOVELTY_ITEM_ACTIONS/v1"
                || catalogue.crystal_commit != "201189f871d23597b82caf88fd611454a2ddefb8"
                || catalogue.rules.is_empty()
                || catalogue
                    .rules
                    .iter()
                    .any(|rule| !valid(rule) || rule.item_ids.iter().any(|id| !ids.insert(*id)))
            {
                return None;
            }
            Some(catalogue.rules)
        })
        .as_deref()
}

fn valid(rule: &NoveltyRule) -> bool {
    let positive = |value: Option<u32>| value.is_some_and(|v| v > 0);
    !rule.item_ids.is_empty()
        && !rule.item_ids.contains(&0)
        && !rule.donor.is_empty()
        && rule.cues.len() == rule.effect_ids.len()
        && rule
            .cues
            .iter()
            .all(|cue| cue.starts_with("appearance:effect/"))
        && rule.effect_ids.iter().all(|id| *id > 0)
        && rule
            .achievement_progress
            .iter()
            .chain(rule.secondary_achievement_progress.iter())
            .all(|progress| !progress.key.is_empty() && progress.target > 0)
        && match rule.kind {
            NoveltyKind::Reward | NoveltyKind::Cosmetic | NoveltyKind::HouseTransform => false,
            NoveltyKind::Costume => {
                positive(rule.duration_ms)
                    && !rule.costumes.is_empty()
                    && rule
                        .costumes
                        .iter()
                        .all(|c| !c.monster.is_empty() && c.look_type > 0 && !c.donor.is_empty())
            }
            NoveltyKind::Dice => {
                rule.item_ids.len() == 6
                    && rule
                        .dice_speech_range
                        .is_some_and(|range| range.into_iter().all(|value| value > 0))
                    && rule
                        .item_ids
                        .windows(2)
                        .all(|ids| ids[0].checked_add(1) == Some(ids[1]))
            }
            NoveltyKind::Transform => {
                positive(rule.output) && positive(rule.duration_ms) && positive(rule.decay_to)
            }
            NoveltyKind::PlayerTargetEffect => {
                rule.success_percent.is_some_and(|p| p <= 100)
                    && rule.success_text.is_some()
                    && rule.failure_text.is_some()
            }
            NoveltyKind::Firework => positive(rule.damage),
            NoveltyKind::EquippedEffect | NoveltyKind::ConsumeEffect => true,
        }
}

pub fn rule_for_key(key: &str) -> Option<&'static NoveltyRule> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    rules()?.iter().find(|rule| rule.item_ids.contains(&id))
}

pub fn is_novelty(key: &str) -> bool {
    rule_for_key(key).is_some()
}

/// The draw callback is bounded and independently domain-separated from reward tables.
pub fn select(
    rule: &NoveltyRule,
    item_id: u32,
    mut draw: impl FnMut(u8, u64) -> Option<u64>,
) -> Option<NoveltySelection> {
    if !valid(rule) || !rule.item_ids.contains(&item_id) {
        return None;
    }
    let mut effects = NoveltyEffects {
        kind: rule.kind,
        source_item_id: item_id,
        donor: rule.donor.clone(),
        ground: None,
        cues: rule
            .cues
            .iter()
            .zip(&rule.effect_ids)
            .map(|(binding, effect_id)| NoveltyCue {
                binding: binding.clone(),
                effect_id: *effect_id,
            })
            .collect(),
        speech: rule.speech.clone(),
        private_speech: rule.kind == NoveltyKind::PlayerTargetEffect,
        require_head: rule.kind == NoveltyKind::EquippedEffect,
        require_player_target: rule.kind == NoveltyKind::PlayerTargetEffect,
        dice_value: None,
        costume: None,
        damage: rule.damage,
        achievement: rule.achievement.clone(),
        achievement_progress: rule
            .achievement_progress
            .iter()
            .chain(rule.secondary_achievement_progress.iter())
            .cloned()
            .collect(),
        timed_output: None,
    };
    let output = match rule.kind {
        NoveltyKind::Reward | NoveltyKind::Cosmetic | NoveltyKind::HouseTransform => return None,
        NoveltyKind::Costume => {
            let upper = u64::try_from(rule.costumes.len()).ok()?;
            let index = draw(192, upper)?;
            if index >= upper {
                return None;
            }
            let costume = &rule.costumes[usize::try_from(index).ok()?];
            effects.costume = Some(CostumeEffect {
                monster: costume.monster.clone(),
                look_type: costume.look_type,
                duration_ms: rule.duration_ms?,
            });
            None
        }
        NoveltyKind::Dice => {
            let upper = u64::try_from(rule.item_ids.len()).ok()?;
            let value = draw(193, upper)?;
            if value >= upper {
                return None;
            }
            effects.dice_value = Some(u32::try_from(value).ok()?.checked_add(1)?);
            Some(rule.item_ids[usize::try_from(value).ok()?])
        }
        NoveltyKind::Transform => {
            effects.timed_output = Some(TimedOutput {
                duration_ms: rule.duration_ms?,
                decay_to: rule.decay_to?,
            });
            rule.output
        }
        NoveltyKind::PlayerTargetEffect => {
            let roll = draw(194, 100)?;
            if roll >= 100 {
                return None;
            }
            let success = roll < u64::from(rule.success_percent?);
            effects.speech = if success {
                rule.success_text.clone()
            } else {
                rule.failure_text.clone()
            };
            if !success {
                effects.cues.clear();
                effects.achievement = None;
            }
            None
        }
        NoveltyKind::Firework | NoveltyKind::EquippedEffect | NoveltyKind::ConsumeEffect => None,
    };
    Some(NoveltySelection {
        output,
        consume_source: !matches!(
            rule.kind,
            NoveltyKind::EquippedEffect | NoveltyKind::PlayerTargetEffect
        ),
        effects,
    })
}

pub fn plan(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
) -> Option<NoveltyPlan> {
    let record = native
        .item_policy(&used.production_key, &used.revision_ref)?
        .record();
    if record.production_definition.family != used.family || !record.admission.materializable {
        return None;
    }
    let id = used
        .production_key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let rule = rule_for_key(&used.production_key)?;
    let mut selected = select(rule, id, |domain, upper| {
        reward_bag_bounded_draw(session, command, source, domain, upper)
    })?;
    if rule.cue_origin.as_deref() == Some("wire_source") {
        selected.effects.cues.clear();
    }
    let outputs = selected
        .output
        .into_iter()
        .map(|id| {
            let record = native.item_policy_for_source_id(id)?.record();
            if !record.admission.materializable {
                return None;
            }
            let definition = &record.production_definition;
            Some((
                TypedDefinitionRef {
                    family: definition.family.clone(),
                    production_key: definition.production_key.clone(),
                    revision_ref: definition.revision_ref.clone(),
                },
                1,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(NoveltyPlan {
        recipe: format!("oteryn:novelty-item.i{id}.v1"),
        outputs,
        consume_source: selected.consume_source,
        effects: serde_json::to_value(selected.effects).ok()?,
    })
}

#[derive(Debug, Deserialize)]
pub struct HouseTransformRule {
    pub item_id: u32,
    pub target: u32,
    pub donor: String,
}

pub fn house_transforms() -> Option<&'static [HouseTransformRule]> {
    #[derive(Deserialize)]
    struct Catalogue {
        schema: String,
        crystal_commit: String,
        house_transforms: Vec<HouseTransformRule>,
    }
    static RULES: OnceLock<Option<Vec<HouseTransformRule>>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let catalogue: Catalogue = serde_json::from_str(include_str!(
                "../../../../tools/content-schema/native-gameplay/novelty-item-actions.json"
            ))
            .ok()?;
            if catalogue.schema != "OTERYN_NOVELTY_ITEM_ACTIONS/v1"
                || catalogue.crystal_commit != "201189f871d23597b82caf88fd611454a2ddefb8"
                || catalogue.house_transforms.len() != 76
            {
                return None;
            }
            let unique = catalogue
                .house_transforms
                .iter()
                .map(|rule| rule.item_id)
                .collect::<std::collections::BTreeSet<_>>();
            if unique.len() != catalogue.house_transforms.len()
                || catalogue.house_transforms.iter().any(|rule| {
                    rule.item_id == 0
                        || rule.item_id == rule.target
                        || rule.donor.is_empty()
                        || !catalogue.house_transforms.iter().any(|reverse| {
                            reverse.item_id == rule.target && reverse.target == rule.item_id
                        })
                })
            {
                return None;
            }
            Some(catalogue.house_transforms)
        })
        .as_deref()
}

pub fn house_transform_for_key(key: &str) -> Option<&'static HouseTransformRule> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    house_transforms()?.iter().find(|rule| rule.item_id == id)
}

pub fn is_ground_action(key: &str) -> bool {
    if feroxa_trap::is_closed(key) {
        return true;
    }
    crate::event_items::is_event_reward(key)
        || house_transform_for_key(key).is_some()
        || rule_for_key(key).is_some_and(|rule| rule.kind != NoveltyKind::EquippedEffect)
}

pub fn select_ground(
    rule: &NoveltyRule,
    item_id: u32,
    ground: GroundEffect,
    mut draw: impl FnMut(u8, u64) -> Option<u64>,
) -> Option<NoveltySelection> {
    if rule.kind == NoveltyKind::EquippedEffect {
        return None;
    }
    let mut selected = select(rule, item_id, &mut draw)?;
    if matches!(
        rule.kind,
        NoveltyKind::Dice | NoveltyKind::Firework | NoveltyKind::Transform
    ) {
        selected.effects.ground = Some(ground);
    }
    if rule.kind == NoveltyKind::Firework {
        if rule.ground_cues.len() != 3 || rule.ground_effect_ids.len() != 3 {
            return None;
        }
        let roll = draw(195, 3)?;
        if roll >= 3 {
            return None;
        }
        let index = usize::try_from(roll).ok()?;
        selected.effects.cues = vec![NoveltyCue {
            binding: rule.ground_cues[index].clone(),
            effect_id: rule.ground_effect_ids[index],
        }];
        selected.effects.damage = None;
        selected.effects.speech = None;
        selected.effects.achievement_progress = rule.achievement_progress.iter().cloned().collect();
    }
    Some(selected)
}

/// Source ground actions whose admission also depends on the actual actor position.
#[allow(
    clippy::too_many_arguments,
    reason = "the existing ground action inputs plus its current native actor position"
)]
pub fn plan_ground_for_actor(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
    ground: GroundEffect,
    in_house: bool,
    actor: GroundEffect,
) -> Option<NoveltyPlan> {
    if feroxa_trap::is_closed(&used.production_key) {
        return feroxa_trap::plan(native, used, ground, actor);
    }
    plan_ground(native, used, session, command, source, ground, in_house)
}

/// Ground toys retain their source tile and the ground-only rocket branch. A
/// balloon mapping additionally requires current source-tile House membership.
#[allow(clippy::too_many_arguments)]
pub fn plan_ground(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
    ground: GroundEffect,
    in_house: bool,
) -> Option<NoveltyPlan> {
    let id = used
        .production_key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let policy = native.item_policy(&used.production_key, &used.revision_ref)?;
    if !policy.record().admission.materializable
        || policy.record().production_definition.family != used.family
    {
        return None;
    }
    let (selected, recipe) = if let Some(rule) = house_transform_for_key(&used.production_key) {
        if !in_house {
            return None;
        }
        (
            NoveltySelection {
                output: Some(rule.target),
                consume_source: true,
                effects: NoveltyEffects {
                    kind: NoveltyKind::HouseTransform,
                    source_item_id: id,
                    donor: rule.donor.clone(),
                    ground: Some(ground),
                    cues: Vec::new(),
                    speech: None,
                    private_speech: false,
                    require_head: false,
                    require_player_target: false,
                    dice_value: None,
                    costume: None,
                    damage: None,
                    achievement: None,
                    achievement_progress: Vec::new(),
                    timed_output: None,
                },
            },
            format!("oteryn:house-item.i{id}.v1"),
        )
    } else {
        (
            select_ground(
                rule_for_key(&used.production_key)?,
                id,
                ground,
                |domain, upper| reward_bag_bounded_draw(session, command, source, domain, upper),
            )?,
            format!("oteryn:novelty-ground.i{id}.v1"),
        )
    };
    let outputs = selected
        .output
        .into_iter()
        .map(|id| {
            let record = native.item_policy_for_source_id(id)?.record();
            if !record.admission.materializable {
                return None;
            }
            let definition = &record.production_definition;
            Some((
                TypedDefinitionRef {
                    family: definition.family.clone(),
                    production_key: definition.production_key.clone(),
                    revision_ref: definition.revision_ref.clone(),
                },
                1,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(NoveltyPlan {
        recipe,
        outputs,
        consume_source: selected.consume_source,
        effects: serde_json::to_value(selected.effects).ok()?,
    })
}

#[cfg(test)]
#[path = "novelty_items/tests.rs"]
mod tests;
