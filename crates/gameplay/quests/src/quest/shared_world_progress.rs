//! Explicit Oteryn native bindings for source-defined shared progress.
//!
//! Objective quantities and task identities: wiki56916@1184992 (War Against
//! the Hive), retained source_fact_supplements. These coordinates are authored
//! native choices around sourced Orockle33448,31309,7 and Navigator33639,31377,9;
//! they are not donor map coordinates. Task markers never award world progress:
//! only the actual Orockle completed-task receipt may do so.
use super::QuestTrackChange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedAction {
    HiveTask(u8),
    HeartDestroyed,
    SoulNetDestroyed,
    CoralPathCompleted,
    StairPathCompleted,
}
#[derive(Debug, Clone, Copy)]
pub struct NativeBinding {
    pub key: &'static str,
    pub action: SharedAction,
    pub position: (u16, u16, i8),
    pub appearance: u16,
    pub track: &'static str,
    pub required: i64,
}
/// Geometry/interaction choice only. Source mechanics retain their original units.
/// Hive Kollos progress MUST originate in credited Kollos deaths, not marker USE.
pub const BINDINGS: [NativeBinding; 10] = [
    NativeBinding {
        key: "oteryn:quest-transition/hive/antenna-turn-in",
        action: SharedAction::HiveTask(0),
        position: (33443, 31310, -7),
        appearance: 14076,
        track: "oteryn:quest-progress/hive/antennae",
        required: 8,
    },
    NativeBinding {
        key: "oteryn:quest-transition/hive/digestion-turn-in",
        action: SharedAction::HiveTask(1),
        position: (33446, 31309, -7),
        appearance: 13976,
        track: "oteryn:quest-progress/hive/blobs",
        required: 7,
    },
    NativeBinding {
        key: "oteryn:quest-transition/hive/chitin-turn-in",
        action: SharedAction::HiveTask(2),
        position: (33447, 31310, -7),
        appearance: 14053,
        track: "oteryn:quest-progress/hive/chitin",
        required: 8,
    },
    NativeBinding {
        key: "oteryn:quest-transition/hive/pores-turn-in",
        action: SharedAction::HiveTask(3),
        position: (33449, 31309, -7),
        appearance: 14064,
        track: "oteryn:quest-progress/hive/pores",
        required: 7,
    },
    NativeBinding {
        key: "oteryn:quest-transition/hive/kollos-turn-in",
        action: SharedAction::HiveTask(4),
        position: (33450, 31310, -7),
        appearance: 14077,
        track: "oteryn:quest-progress/hive/kollos",
        required: 9,
    },
    NativeBinding {
        key: "oteryn:quest-transition/hive/monolith-turn-in",
        action: SharedAction::HiveTask(5),
        position: (33451, 31310, -7),
        appearance: 14074,
        track: "oteryn:quest-progress/hive/monoliths",
        required: 5,
    },
    NativeBinding {
        key: "oteryn:quest-callback/deeplings-native-heart",
        action: SharedAction::HeartDestroyed,
        position: (33637, 31378, -9),
        appearance: 14018,
        track: "oteryn:quest-progress/deeplings-native/heart",
        required: 1,
    },
    NativeBinding {
        key: "oteryn:quest-callback/deeplings-native-soul-net",
        action: SharedAction::SoulNetDestroyed,
        position: (33638, 31378, -9),
        appearance: 14020,
        track: "oteryn:quest-progress/deeplings-native/soul-net",
        required: 1,
    },
    NativeBinding {
        key: "oteryn:quest-callback/deeplings-native-coral",
        action: SharedAction::CoralPathCompleted,
        position: (33640, 31379, -9),
        appearance: 14163,
        track: "oteryn:quest-progress/deeplings-native/coral",
        required: 1,
    },
    NativeBinding {
        key: "oteryn:quest-callback/deeplings-native-stair",
        action: SharedAction::StairPathCompleted,
        position: (33641, 31378, -9),
        appearance: 14164,
        track: "oteryn:quest-progress/deeplings-native/stair",
        required: 1,
    },
];

pub fn binding(key: &str) -> Option<&'static NativeBinding> {
    BINDINGS.iter().find(|binding| binding.key == key)
}
/// A completed Hive receipt resets its full objective count for a new personal
/// cycle. Destruction/path receipts retain a zero-valued personal observer;
/// the once-per-source/save World receipt owns their consumption.
/// No client-supplied quantity or unrelated quest change can generate credit.
pub const HATCHLINGS_TRACK: &str = "oteryn:quest-progress/hive/hatchlings";
pub const ACTIVE_TRACKS: [&str; 6] = [
    "oteryn:quest-progress/hive/antennae-active",
    "oteryn:quest-progress/hive/blobs-active",
    "oteryn:quest-progress/hive/chitin-active",
    "oteryn:quest-progress/hive/pores-active",
    "oteryn:quest-progress/hive/kollos-active",
    "oteryn:quest-progress/hive/monoliths-active",
];
pub fn accept_key(index: usize) -> String {
    format!("oteryn:quest-transition/hive-native-accept-{index}")
}
pub fn accepted_task(key: &str) -> Option<u8> {
    (0..6_u8).find(|index| key == accept_key(usize::from(*index)))
}
pub fn completed_action(key: &str, changes: &[QuestTrackChange]) -> Option<SharedAction> {
    let binding = binding(key)?;
    let expected = match binding.action {
        SharedAction::HiveTask(_) => (binding.required, 0),
        _ => (0, 0),
    };
    let main = changes
        .iter()
        .filter(|change| change.track == binding.track && (change.before, change.after) == expected)
        .count()
        == 1;
    let compound = binding.action == SharedAction::HiveTask(2);
    let hatchlings = changes
        .iter()
        .filter(|change| {
            change.track == HATCHLINGS_TRACK && (change.before, change.after) == (8, 0)
        })
        .count()
        == 1;
    let active = match binding.action {
        SharedAction::HiveTask(task) => {
            changes
                .iter()
                .filter(|change| {
                    change.track == ACTIVE_TRACKS[usize::from(task)]
                        && (change.before, change.after) == (1, 0)
                })
                .count()
                == 1
        }
        _ => true,
    };
    let length = match binding.action {
        SharedAction::HiveTask(2) => 4,
        SharedAction::HiveTask(_) => 3,
        _ => 1,
    };
    (main
        && active
        && changes.len() == length
        && (!compound || hatchlings)
        && match binding.action {
            SharedAction::HiveTask(task) => {
                changes
                    .iter()
                    .filter(|change| {
                        change.track == super::hive_access::FAVOUR
                            && change.after.checked_sub(change.before)
                                == Some(super::hive_access::favour_reward(task))
                    })
                    .count()
                    == 1
            }
            _ => true,
        })
    .then_some(binding.action)
}

/// Reserved transition namespace for an explicit, once-per-save reset profile.
pub const SAVE_EVENT_RESTART_PREFIX: &str = "oteryn:quest-transition/save-event-restart/";

/// Private daily quest scope. It never activates a physical World change.
pub const PERSONAL_SERVER_SAVE_SCOPE: &str = "server_save";

/// Stable reset declaration key for a canonical quest. The declaration names
/// exactly which owned tracks restart; other persistent tracks remain unchanged.
pub fn save_event_restart_key(quest: &str) -> String {
    let name = quest
        .strip_prefix("oteryn:quest.")
        .or_else(|| quest.strip_prefix("oteryn:quest/"))
        .unwrap_or(quest);
    format!("{SAVE_EVENT_RESTART_PREFIX}{name}")
}

pub const HIVE_QUEST: &str = "oteryn:quest.hive_native_shared";
pub const DEEPLING_QUEST: &str = "oteryn:quest.deeplings_native_shared";
pub const HIVE_OBJECTIVE_PREFIX: &str = "oteryn:quest-callback/hive-native-objective-";

/// Source mission counters cleared together at the first acceptance of a new cycle.
pub fn hive_mission_tracks() -> impl Iterator<Item = &'static str> {
    BINDINGS
        .iter()
        .filter(|binding| matches!(binding.action, SharedAction::HiveTask(_)))
        .map(|binding| binding.track)
        .chain(ACTIVE_TRACKS)
        .chain([HATCHLINGS_TRACK])
}

/// Build the physical receipt delta for first acceptance in a source cycle.
/// The durable owner must independently prove that this is the current cycle's
/// first acceptance. Private balances and timestamps are never part of this plan.
pub fn plan_hive_cycle_acceptance(
    catalogue: &super::QuestStateCatalogue,
    stored: &std::collections::BTreeMap<String, i64>,
    task: u8,
) -> Result<Vec<QuestTrackChange>, super::QuestRefusal> {
    let selected = ACTIVE_TRACKS
        .get(usize::from(task))
        .ok_or(super::QuestRefusal::StageMismatch)?;
    let mut changes = Vec::new();
    for key in hive_mission_tracks() {
        let track = catalogue
            .track(key)
            .ok_or(super::QuestRefusal::StageMismatch)?;
        let before = stored.get(key).copied().unwrap_or(track.initial);
        let after = i64::from(key == *selected);
        if track.quest != HIVE_QUEST
            || track.initial != 0
            || !(track.min..=track.max).contains(&before)
            || !(track.min..=track.max).contains(&after)
        {
            return Err(super::QuestRefusal::OutOfRange);
        }
        if before != after || key == *selected {
            changes.push(QuestTrackChange {
                track: key.into(),
                before,
                after,
            });
        }
    }
    Ok(changes)
}

/// Mission tracks scoped to the counter cycle. Private Favour and scent
/// timestamps remain persistent across cycles.
pub fn counter_progress_track(quest: &str, track: &str) -> bool {
    match quest {
        HIVE_QUEST => hive_mission_tracks().any(|key| key == track),
        DEEPLING_QUEST => BINDINGS.iter().any(|binding| {
            !matches!(binding.action, SharedAction::HiveTask(_)) && binding.track == track
        }),
        _ => false,
    }
}

/// Retain native object, kill, acceptance and completion progress receipts in
/// their counter cycle without classifying private-only purchases as missions.
pub fn records_counter_progress(transition: &super::QuestTransition) -> bool {
    transition
        .effects
        .iter()
        .any(|effect| counter_progress_track(&transition.quest, &effect.track))
}

/// Existing object USE owner supplies real handle, range, position and revision.
/// Tool appearances are source mission tools, retained across a successful action.
pub fn callbacks() -> Vec<super::world_callbacks::Callback> {
    use super::world_callbacks::{Callback, ItemOnTarget, Position, Selector, Storage, Trigger};
    let tools = [13974_u16, 13976, 14052, 14051, 0, 14055];
    let mut result:Vec<_>=BINDINGS.iter().enumerate().filter(|(index,_)|*index!=4).map(|(index,binding)| {
        let hive=index<6;
        let key=if hive {format!("{HIVE_OBJECTIVE_PREFIX}{index}")}else{binding.key.to_owned()};
        let tool=tools.get(index).copied().filter(|tool|*tool!=0);
        Callback { admission: None, requires_premium: false,actor_kind: Default::default(), opening_only: false, requires_world_admission: false, facing: None, state_ops: Vec::new(), world_requirements: Vec::new(),
            key,quest:if hive {HIVE_QUEST}else{DEEPLING_QUEST}.into(),trigger:Trigger::Use,
            selector:Selector{native: None,position:Some(Position{x:binding.position.0,y:binding.position.1,floor:binding.position.2}),unique:None,action:None,appearance:Some(binding.appearance)},
            lever:None,reversible:false,passages:vec![],teleport:None,
            storage:{let mut storage=vec![Storage{increment: false, at_least: false,track:binding.track.into(),minimum:0,maximum:if hive {binding.required-1}else{0},set:if hive {1}else{0},preserve:false}];if hive {storage.push(Storage{increment: false, at_least: false,track:ACTIVE_TRACKS[index].into(),minimum:1,maximum:1,set:1,preserve:true})}storage},
            minimum_level:None,experience: None, world_change: None,monk_reward:None,offerings:vec![],
            item_on_target:tool.map(|tool|ItemOnTarget{ target_remove: false,alternatives: Vec::new(), reusable: false,item:format!("oteryn:item.tibia.i{tool}"),count:1,replacement:Some(format!("oteryn:item.tibia.i{tool}")),target_replacement:None}),
            item_on_target:tool.map(|tool|ItemOnTarget{selected_instance:false,alternatives: Vec::new(), reusable: false,item:format!("oteryn:item.tibia.i{tool}"),count:1,replacement:Some(format!("oteryn:item.tibia.i{tool}")),target_replacement:None}),
            item_on_target:tool.map(|tool|ItemOnTarget{ ingredients: Vec::new(),alternatives: Vec::new(), reusable: false,item:format!("oteryn:item.tibia.i{tool}"),count:1,replacement:Some(format!("oteryn:item.tibia.i{tool}")),target_replacement:None}),
            minimum_level:None, character_requirements: None,experience: None, world_change: None,monk_reward:None,offerings:vec![],
            item_on_target:tool.map(|tool|ItemOnTarget{alternatives: Vec::new(), reusable: false,item:format!("oteryn:item.tibia.i{tool}"),count:1,replacement:Some(format!("oteryn:item.tibia.i{tool}")),target_replacement:None}),
            return_on_failure:false,failure_message:None,success_message:None,native_rule:None,effects:Default::default(),
            sources:vec!["crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/npc/orockle.lua:explicit-native-geometry-and-execution-adaptation; wiki56916@1184992; NOT_DONOR_POSITION".into()],
        }
    }).collect();
    if let Some(mut hatchling) = result
        .iter()
        .find(|callback| callback.key == format!("{HIVE_OBJECTIVE_PREFIX}2"))
        .cloned()
    {
        hatchling.key = "oteryn:quest-callback/hive-native-hatchlings".into();
        hatchling.selector.position = Some(Position {
            x: 33447,
            y: 31311,
            floor: -7,
        });
        hatchling.selector.appearance = Some(14255);
        hatchling.storage[0].track = HATCHLINGS_TRACK.into();
        hatchling.item_on_target = Some(ItemOnTarget {
            target_remove: false,
            selected_instance: false,
            ingredients: Vec::new(),
            alternatives: Vec::new(),
            reusable: false,
            item: "oteryn:item.tibia.i14255".into(),
            count: 1,
            replacement: Some("oteryn:item.tibia.i14255".into()),
            target_replacement: None,
        });
        result.push(hatchling);
    }
    if let Some(mut fluid) = result
        .iter()
        .find(|callback| callback.key == format!("{HIVE_OBJECTIVE_PREFIX}5"))
        .cloned()
    {
        fluid.key = "oteryn:quest-callback/hive-native-fluid-preparation".into();
        fluid.selector.position = Some(Position {
            x: 33451,
            y: 31311,
            floor: -7,
        });
        fluid.selector.appearance = Some(14055);
        fluid.storage[0] = Storage {
            increment: false,
            at_least: false,
            track: FLUID_TRACK.into(),
            minimum: 0,
            maximum: 0,
            set: 0,
            preserve: false,
        };
        fluid.item_on_target = Some(ItemOnTarget {
            target_remove: false,
            selected_instance: false,
            ingredients: Vec::new(),
            alternatives: Vec::new(),
            reusable: false,
            item: "oteryn:item.tibia.i14054".into(),
            count: 1,
            replacement: Some("oteryn:item.tibia.i14055".into()),
            target_replacement: None,
        });
        result.push(fluid);
    }
    result
}

pub const FLUID_TRACK: &str = "oteryn:quest-progress/hive/fluid-preparation";

/// Existing catalogue, quest receipt and revision owner; no second personal store.
/// Replaces only our own callback transitions after the normal callback extension.
pub fn extend(catalogue: super::QuestStateCatalogue) -> Result<super::QuestStateCatalogue, String> {
    use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition};
    let mut tracks = catalogue.tracks.clone();
    let mut transitions = catalogue.transitions.clone();
    for (index, binding) in BINDINGS.iter().enumerate() {
        let hive = index < 6;
        let quest = if hive { HIVE_QUEST } else { DEEPLING_QUEST };
        tracks.insert(
            binding.track.into(),
            QuestTrack {
                key: binding.track.into(),
                quest: quest.into(),
                initial: 0,
                min: 0,
                max: binding.required,
            },
        );
        if hive {
            let objective = if index == 4 {
                "oteryn:quest-transition/hive-native-kollos-kill".into()
            } else {
                format!("{HIVE_OBJECTIVE_PREFIX}{index}")
            };
            transitions.insert(
                objective.clone(),
                QuestTransition {
                    key: objective,
                    quest: quest.into(),
                    effects: vec![QuestEffect {
                        track: binding.track.into(),
                        from: QuestComparison::Between(0, binding.required - 1),
                        effect: QuestEffectKind::Add(1),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
            );
            transitions.insert(
                binding.key.into(),
                QuestTransition {
                    key: binding.key.into(),
                    quest: quest.into(),
                    effects: vec![QuestEffect {
                        track: binding.track.into(),
                        from: QuestComparison::Between(binding.required, binding.required),
                        effect: QuestEffectKind::Set(0),
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
            );
        }
    }
    for (index, active) in ACTIVE_TRACKS.iter().copied().enumerate() {
        tracks.insert(
            active.into(),
            QuestTrack {
                key: active.into(),
                quest: HIVE_QUEST.into(),
                initial: 0,
                min: 0,
                max: 1,
            },
        );
        let key = accept_key(index);
        transitions.insert(
            key.clone(),
            QuestTransition {
                key,
                quest: HIVE_QUEST.into(),
                effects: vec![QuestEffect {
                    track: active.into(),
                    from: QuestComparison::Between(0, 0),
                    effect: QuestEffectKind::Set(1),
                }],
                completes: false,
                experience: None,
                world_change: None,
            },
        );
        if let Some(turnin) = transitions.get_mut(BINDINGS[index].key) {
            turnin.effects.push(QuestEffect {
                track: active.into(),
                from: QuestComparison::Between(1, 1),
                effect: QuestEffectKind::Set(0),
            });
        }
        let objective = if index == 4 {
            "oteryn:quest-transition/hive-native-kollos-kill".into()
        } else {
            format!("{HIVE_OBJECTIVE_PREFIX}{index}")
        };
        if let Some(objective) = transitions.get_mut(&objective) {
            objective.effects.push(QuestEffect {
                track: active.into(),
                from: QuestComparison::Between(1, 1),
                effect: QuestEffectKind::Add(0),
            });
        }
    }
    tracks.insert(
        HATCHLINGS_TRACK.into(),
        QuestTrack {
            key: HATCHLINGS_TRACK.into(),
            quest: HIVE_QUEST.into(),
            initial: 0,
            min: 0,
            max: 8,
        },
    );
    let hatchling_key = "oteryn:quest-callback/hive-native-hatchlings";
    transitions.insert(
        hatchling_key.into(),
        QuestTransition {
            key: hatchling_key.into(),
            quest: HIVE_QUEST.into(),
            effects: vec![QuestEffect {
                track: HATCHLINGS_TRACK.into(),
                from: QuestComparison::Between(0, 7),
                effect: QuestEffectKind::Add(1),
            }],
            completes: false,
            experience: None,
            world_change: None,
        },
    );
    if let Some(hatchlings) = transitions.get_mut(hatchling_key) {
        hatchlings.effects.push(QuestEffect {
            track: ACTIVE_TRACKS[2].into(),
            from: QuestComparison::Between(1, 1),
            effect: QuestEffectKind::Add(0),
        });
    }
    if let Some(chitin) = transitions.get_mut(BINDINGS[2].key) {
        chitin.effects.push(QuestEffect {
            track: HATCHLINGS_TRACK.into(),
            from: QuestComparison::Between(8, 8),
            effect: QuestEffectKind::Set(0),
        });
    }
    tracks.insert(
        FLUID_TRACK.into(),
        QuestTrack {
            key: FLUID_TRACK.into(),
            quest: HIVE_QUEST.into(),
            initial: 0,
            min: 0,
            max: 0,
        },
    );
    for (key, maximum) in [
        (super::hive_access::FAVOUR, i64::MAX),
        (super::hive_access::PURCHASED, i64::MAX),
    ] {
        tracks.insert(
            key.into(),
            QuestTrack {
                key: key.into(),
                quest: HIVE_QUEST.into(),
                initial: 0,
                min: 0,
                max: maximum,
            },
        );
    }
    for (task, binding) in BINDINGS.iter().take(6).enumerate() {
        let transition = transitions.get_mut(binding.key).ok_or("Hive turn-in")?;
        transition.effects.push(QuestEffect {
            track: super::hive_access::FAVOUR.into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Add(super::hive_access::favour_reward(task as u8)),
        });
    }
    transitions.insert(
        super::hive_access::BUY.into(),
        QuestTransition {
            key: super::hive_access::BUY.into(),
            quest: HIVE_QUEST.into(),
            effects: vec![
                QuestEffect {
                    track: super::hive_access::FAVOUR.into(),
                    from: QuestComparison::Ge(50),
                    effect: QuestEffectKind::Add(-50),
                },
                QuestEffect {
                    track: super::hive_access::PURCHASED.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::SetNow,
                },
            ],
            completes: false,
            experience: None,
            world_change: None,
        },
    );
    super::QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|error| format!("{error:?}"))
}

/// Native accepted quest-kill producer must own current credited principal/death.
/// This returns a personal objective transition, never a World Task credit.
pub fn kill_objective(creature: &str) -> Option<&'static str> {
    (creature == "oteryn:creature.kollos")
        .then_some("oteryn:quest-transition/hive-native-kollos-kill")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn worldchg_shared_native_progress_requires_complete_objective_receipt() {
        for source in BINDINGS {
            let (before, after) = match source.action {
                SharedAction::HiveTask(_) => (source.required, 0),
                _ => (0, 0),
            };
            let mut changes = vec![QuestTrackChange {
                track: source.track.into(),
                before,
                after,
            }];
            if let SharedAction::HiveTask(task) = source.action {
                changes.push(QuestTrackChange {
                    track: ACTIVE_TRACKS[usize::from(task)].into(),
                    before: 1,
                    after: 0,
                });
            }
            if source.action == SharedAction::HiveTask(2) {
                changes.push(QuestTrackChange {
                    track: HATCHLINGS_TRACK.into(),
                    before: 8,
                    after: 0,
                });
            }
            if let SharedAction::HiveTask(task) = source.action {
                changes.push(QuestTrackChange {
                    track: super::super::hive_access::FAVOUR.into(),
                    before: 0,
                    after: super::super::hive_access::favour_reward(task),
                });
            }
            assert_eq!(completed_action(source.key, &changes), Some(source.action));
            changes[0].before = before - 1;
            assert_eq!(completed_action(source.key, &changes), None);
            changes[0].before = before;
            changes.push(changes[0].clone());
            assert_eq!(completed_action(source.key, &changes), None);
        }
        assert!(binding("oteryn:quest-callback/deeplings-boss-28574").is_none());
        assert!(binding("oteryn:quest-callback/deeplings-crystal").is_none());
    }
}

#[cfg(test)]
mod native_cycle_tests {
    use super::*;
    #[test]
    fn qm55_counter_history_includes_native_objectives_and_excludes_private_scent()
    -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        for binding in BINDINGS {
            assert!(records_counter_progress(
                catalogue.transition(binding.key).ok_or("native binding")?
            ));
        }
        for index in 0..6 {
            let objective = if index == 4 {
                "oteryn:quest-transition/hive-native-kollos-kill".into()
            } else {
                format!("{HIVE_OBJECTIVE_PREFIX}{index}")
            };
            for key in [accept_key(index), objective] {
                assert!(records_counter_progress(
                    catalogue.transition(&key).ok_or("native mission")?
                ));
            }
        }
        assert!(records_counter_progress(
            catalogue
                .transition("oteryn:quest-callback/hive-native-hatchlings")
                .ok_or("compound objective")?
        ));
        assert!(!records_counter_progress(
            catalogue
                .transition(super::super::hive_access::BUY)
                .ok_or("scent purchase")?
        ));
        Ok(())
    }
    #[test]
    fn worldchg_hive_native_objectives_require_full_count_then_npc_resets_cycle()
    -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = lowered.catalogue();
        let mut stored = std::collections::BTreeMap::new();
        for (index, source) in BINDINGS.iter().take(6).enumerate() {
            let turnin = catalogue
                .transition(source.key)
                .ok_or("native Orockle receipt missing")?;
            let objective = if index == 4 {
                "oteryn:quest-transition/hive-native-kollos-kill".into()
            } else {
                format!("{HIVE_OBJECTIVE_PREFIX}{index}")
            };
            let objective = catalogue
                .transition(&objective)
                .ok_or("native objective missing")?;
            assert!(catalogue.evaluate(turnin, &stored, 0).is_err());
            let accept = catalogue
                .transition(&accept_key(index))
                .ok_or("missionacceptance")?;
            let accepted = catalogue
                .evaluate(accept, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            stored.insert(accepted[0].track.clone(), accepted[0].after);
            for count in 1..=source.required {
                let result = catalogue
                    .evaluate(objective, &stored, 0)
                    .map_err(|e| format!("{e:?}"))?;
                assert_eq!(result[0].after, count);
                stored.insert(source.track.into(), count);
                if count < source.required {
                    assert!(catalogue.evaluate(turnin, &stored, 0).is_err())
                }
            }
            assert!(catalogue.evaluate(objective, &stored, 0).is_err());
            if index == 2 {
                assert!(catalogue.evaluate(turnin, &stored, 0).is_err());
                stored.insert(HATCHLINGS_TRACK.into(), 8);
            }
            let result = catalogue
                .evaluate(turnin, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(completed_action(source.key, &result), Some(source.action));
            for change in &result {
                stored.insert(change.track.clone(), change.after);
            }
            assert!(catalogue.evaluate(turnin, &stored, 0).is_err());
        }
        Ok(())
    }
}
