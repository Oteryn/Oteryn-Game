//! Targuna's closed, source-pinned compound transitions and event selection.
//! Writes remain with QuestState; item-reward transitions stay closed until their
//! compound grant is accepted by the existing durable owner in a separate change.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};

pub mod dialogue;
pub mod npc_rules;

pub const QUEST: &str = "oteryn:quest.targuna_quest";
pub const TRACK_PREFIX: &str = "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/";
pub const TRANSITION_PREFIX: &str = "oteryn:quest-transition/targuna/";
pub const SOURCE: &str = "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:";
const EMBEDDED: &str = include_str!("../../../../../content/quests/targuna.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reward {
    Morla,
    Turnips,
    Treasure,
}

impl Reward {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Morla => "morla",
            Self::Turnips => "turnips",
            Self::Treasure => "treasure",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardDefinition {
    pub key: String,
    pub source: String,
    pub transition: String,
    pub item: u32,
    pub contents: Vec<u32>,
    pub claim: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    quest: String,
    source: String,
    tracks: Vec<Track>,
    transitions: Vec<Transition>,
    arrival_tracks: Vec<String>,
    arrival_positions: Vec<Position>,
    sandcastle_positions: Vec<Position>,
    egg_positions: Vec<Position>,
    rewards: Vec<RewardDefinition>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    key: String,
    initial: i64,
    min: i64,
    max: i64,
    source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {
    key: String,
    effects: Vec<Effect>,
    experience: Option<i64>,
    source: Vec<String>,
    execution: Execution,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Execution {
    Ready,
    HeldSharedOwner,
    SourceOnly,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Effect {
    track: String,
    from: Operation,
    effect: Operation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Operation {
    op: String,
    value: Option<i64>,
}

fn data() -> Result<&'static Document, &'static str> {
    static DATA: OnceLock<Result<Document, String>> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: Document = serde_json::from_str(EMBEDDED).map_err(|error| error.to_string())?;
        if data.schema != "OTERYN_TARGUNA/v1"
            || data.quest != QUEST
            || !data.source.starts_with(SOURCE)
            || data.arrival_tracks.len() != 7
            || data.arrival_positions.len() != 9
            || data.sandcastle_positions.len() != 13
            || data.egg_positions.len() != 46
            || data.rewards.len() != 3
            || data
                .arrival_tracks
                .iter()
                .any(|key| !key.starts_with(TRACK_PREFIX))
            || data.tracks.iter().any(|track| {
                !track.key.starts_with(TRACK_PREFIX)
                    || !track.source.starts_with(SOURCE)
                    || track.min > track.initial
                    || track.initial > track.max
            })
            || data.transitions.iter().any(|step| {
                !step.key.starts_with(TRANSITION_PREFIX)
                    || step.source.is_empty()
                    || step.source.iter().any(|source| !source.starts_with(SOURCE))
                    || step
                        .effects
                        .iter()
                        .any(|effect| !effect.track.starts_with(TRACK_PREFIX))
            })
            || data.rewards.iter().any(|reward| {
                !reward.source.starts_with(SOURCE)
                    || !reward.transition.starts_with(TRANSITION_PREFIX)
                    || !reward.claim.starts_with("oteryn:reward-claim/targuna-")
            })
        {
            return Err("invalid Targuna source scope".into());
        }
        Ok(data)
    })
    .as_ref()
    .map_err(String::as_str)
}

fn comparison(operation: &Operation) -> Result<QuestComparison, String> {
    let value = || {
        operation
            .value
            .ok_or_else(|| "missing comparison value".to_owned())
    };
    Ok(match operation.op.as_str() {
        "any" if operation.value.is_none() => QuestComparison::Any,
        "eq" => QuestComparison::Eq(value()?),
        "ne" => QuestComparison::Ne(value()?),
        "lt" => QuestComparison::Lt(value()?),
        "ge" => QuestComparison::Ge(value()?),
        _ => return Err("unknown Targuna comparison".into()),
    })
}

fn source_transition(step: &Transition) -> Result<QuestTransition, String> {
    let effects = step
        .effects
        .iter()
        .map(|effect| {
            let value = effect
                .effect
                .value
                .ok_or_else(|| "missing effect value".to_owned())?;
            Ok(QuestEffect {
                track: effect.track.clone(),
                from: comparison(&effect.from)?,
                effect: match effect.effect.op.as_str() {
                    "set" => QuestEffectKind::Set(value),
                    "add" => QuestEffectKind::Add(value),
                    _ => return Err("unknown Targuna effect".into()),
                },
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(QuestTransition {
        key: step.key.clone(),
        quest: QUEST.into(),
        effects,
        completes: false,
        experience: step.experience,
    })
}

fn transition(step: &Transition) -> Result<QuestTransition, String> {
    let mut transition = source_transition(step)?;
    if matches!(step.execution, Execution::HeldSharedOwner) {
        // No reward flag/XP without the atomic existing owner. This batch must not
        // extend RewardClaimMint or NPC exchange contracts (coordinator ANSWER §5).
        // https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589524728
        for effect in &mut transition.effects {
            effect.effect = QuestEffectKind::Computed;
        }
    }
    Ok(transition)
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let data = data()?;
    // Source-only world/event models do not register an executable transition or
    // a new track. Their missing owner/binding is recorded in Left out, not bypassed.
    let tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for step in &data.transitions {
        if !matches!(step.execution, Execution::SourceOnly) {
            transitions.push(transition(step)?);
        }
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}

pub fn reward(reward: Reward) -> Option<&'static RewardDefinition> {
    data()
        .ok()?
        .rewards
        .iter()
        .find(|definition| definition.key == reward.key())
}

pub fn track_value(tracks: &BTreeMap<String, i64>, suffix: &str) -> i64 {
    tracks
        .get(&format!("{TRACK_PREFIX}{suffix}"))
        .copied()
        .unwrap_or(0)
}

pub fn npc_state(tracks: &BTreeMap<String, i64>) -> npc_rules::NpcState {
    use npc_rules::NpcState;
    let value = |suffix| track_value(tracks, suffix);
    NpcState {
        questline: value("questline"),
        ancient_mission: value("ancient_enemy/mission"),
        commander_killed: value("ancient_enemy/lizard_commander_killed"),
        camilla_reward: value("ancient_enemy/camilla_reward"),
        treasure_mission: value("long_lost_treasure/mission"),
        pirates: value("long_lost_treasure/pirate_kill_count"),
        eggs: value("long_lost_treasure/tortoise_egg_count"),
        morla_done: value("long_lost_treasure/morla_done"),
        treasure_marked: value("long_lost_treasure/treasure_marked"),
        burning_mission: value("burning_heart/mission"),
        herald_killed: value("burning_heart/herald_killed"),
        turnips_reward: value("burning_heart/turnips_reward"),
    }
}

pub fn mutation_transition(
    mutation: npc_rules::Mutation,
    state: npc_rules::NpcState,
) -> Option<String> {
    use npc_rules::Mutation as M;
    let suffix = match mutation {
        M::StartAncient => {
            if state.questline >= 1 {
                "start-ancient-existing"
            } else {
                "start-ancient-new"
            }
        }
        M::ReportCommander => "report-commander",
        M::CamillaReward => "camilla-reward",
        M::StartTreasure => {
            if state.questline >= 1 {
                "start-treasure-existing"
            } else {
                "start-treasure-new"
            }
        }
        M::MorlaReward => "morla-reward",
        M::MarkTreasure => "mark-treasure",
        M::StartBurning => {
            if state.questline >= 1 {
                "start-burning-existing"
            } else {
                "start-burning-new"
            }
        }
        M::Briefing => "crimson-briefing",
        M::TurnipsReward => "turnips-reward",
        M::ReplaceTurnips => return None,
    };
    Some(format!("{TRANSITION_PREFIX}{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{QuestRefusal, loader};

    fn catalogue() -> Result<QuestStateCatalogue, String> {
        loader::load_embedded_quest_state("targuna-test")
            .map(|loaded| loaded.catalogue().clone())
            .map_err(|error| format!("catalogue: {error:?}"))
    }
    fn stored(values: &[(&str, i64)]) -> BTreeMap<String, i64> {
        values
            .iter()
            .map(|(suffix, value)| (format!("{TRACK_PREFIX}{suffix}"), *value))
            .collect()
    }
    fn run(
        catalogue: &QuestStateCatalogue,
        key: &str,
        tracks: &BTreeMap<String, i64>,
    ) -> Result<Result<Vec<super::super::QuestTrackChange>, QuestRefusal>, String> {
        let transition = catalogue.transition(key).ok_or("transition")?;
        Ok(catalogue.evaluate(transition, tracks, 0))
    }

    #[test]
    fn targuna_npc_starts_are_atomic_and_do_not_reset_the_questline() -> Result<(), String> {
        use npc_rules::{Mutation, NpcState};
        let catalogue = catalogue()?;
        for mutation in [
            Mutation::StartAncient,
            Mutation::StartTreasure,
            Mutation::StartBurning,
        ] {
            for started in [0, 1] {
                let state = NpcState {
                    questline: started,
                    ..NpcState::default()
                };
                let key = mutation_transition(mutation, state).ok_or("NPC transition")?;
                let mut tracks = stored(&[("questline", started)]);
                let changes = run(&catalogue, &key, &tracks)?
                    .map_err(|error| format!("source start: {error:?}"))?;
                assert_eq!(changes.len(), 2);
                let expected = if mutation == Mutation::StartAncient {
                    2
                } else {
                    1
                };
                assert_eq!(changes[0].after, expected);
                assert_eq!(changes[1].after, 1);
                for change in changes {
                    tracks.insert(change.track, change.after);
                }
                assert_eq!(
                    run(&catalogue, &key, &tracks)?,
                    Err(QuestRefusal::StageMismatch)
                );
            }
        }
        let key = format!("{TRANSITION_PREFIX}camilla-reward");
        assert_eq!(
            catalogue.transition(&key).ok_or("Camilla")?.experience,
            Some(1500)
        );
        assert_eq!(
            run(&catalogue, &key, &stored(&[]))?,
            Err(QuestRefusal::StageMismatch)
        );
        let changes = run(
            &catalogue,
            &key,
            &stored(&[("ancient_enemy/lizard_commander_killed", 1)]),
        )?
        .map_err(|error| format!("Camilla source grant: {error:?}"))?;
        assert_eq!(changes[0].after, 1);
        assert_eq!(changes[2].after, 4);
        Ok(())
    }

    #[test]
    fn targuna_rewards_require_all_conditions_and_preserve_counts() -> Result<(), String> {
        let catalogue = catalogue()?;
        let morla = reward(Reward::Morla).ok_or("Morla")?;
        assert_eq!(
            run(&catalogue, &morla.transition, &stored(&[]))?,
            Err(QuestRefusal::NotSupported)
        );
        // Verify the retained source model independently; the active catalogue is closed.
        let step = data()?
            .transitions
            .iter()
            .find(|step| step.key == morla.transition)
            .ok_or("source step")?;
        let catalogue = QuestStateCatalogue::new(
            "source-model",
            catalogue.tracks.values().cloned().collect(),
            vec![source_transition(step)?],
        )
        .map_err(|error| format!("source model: {error:?}"))?;
        let mut values = stored(&[
            ("long_lost_treasure/mission", 2),
            ("long_lost_treasure/pirate_kill_count", 20),
            ("long_lost_treasure/tortoise_egg_count", 9),
        ]);
        assert_eq!(
            run(&catalogue, &morla.transition, &values)?,
            Err(QuestRefusal::StageMismatch)
        );
        values.insert(
            format!("{TRACK_PREFIX}long_lost_treasure/tortoise_egg_count"),
            10,
        );
        let changes = run(&catalogue, &morla.transition, &values)?
            .map_err(|error| format!("both thresholds: {error:?}"))?;
        assert_eq!(
            changes
                .iter()
                .find(|change| change.track.ends_with("pirate_kill_count"))
                .ok_or("pirates")?
                .after,
            20
        );
        for change in changes {
            values.insert(change.track, change.after);
        }
        assert_eq!(
            run(&catalogue, &morla.transition, &values)?,
            Err(QuestRefusal::StageMismatch)
        );
        assert_eq!(
            reward(Reward::Treasure).ok_or("treasure")?.contents,
            vec![3003]
        );
        let active = self::catalogue()?;
        assert_eq!(
            active
                .transition(&reward(Reward::Turnips).ok_or("turnips")?.transition)
                .ok_or("transition")?
                .experience,
            Some(5000)
        );
        assert_eq!(
            run(
                &active,
                &reward(Reward::Turnips).ok_or("turnips")?.transition,
                &stored(&[("burning_heart/herald_killed", 1)])
            )?,
            Err(QuestRefusal::NotSupported)
        );
        Ok(())
    }

    #[test]
    fn targuna_world_and_event_models_are_not_registered_without_a_caller() -> Result<(), String> {
        let catalogue = catalogue()?;
        let data = data()?;
        assert_eq!(data.transitions.len(), 164);
        assert_eq!(
            data.transitions
                .iter()
                .filter(|step| matches!(step.execution, Execution::Ready))
                .count(),
            10
        );
        assert_eq!(
            data.transitions
                .iter()
                .filter(|step| matches!(step.execution, Execution::HeldSharedOwner))
                .count(),
            3
        );
        assert_eq!(
            data.transitions
                .iter()
                .filter(|step| matches!(step.execution, Execution::SourceOnly))
                .count(),
            151
        );
        assert_eq!(
            catalogue
                .transitions
                .keys()
                .filter(|key| key.starts_with(TRANSITION_PREFIX))
                .count(),
            13
        );
        for step in &data.transitions {
            if matches!(step.execution, Execution::SourceOnly) {
                assert!(
                    catalogue.transition(&step.key).is_none(),
                    "source data must not create an unqualified writer"
                );
            }
        }
        assert!(
            !catalogue
                .tracks
                .contains_key(&format!("{TRACK_PREFIX}main_continent_hint"))
        );
        Ok(())
    }
}
