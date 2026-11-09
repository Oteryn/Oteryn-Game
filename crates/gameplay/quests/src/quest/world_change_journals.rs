//! Personal native world-change programmes, independent of global calendars.
use super::{QuestStateCatalogue, QuestTrack, native_kills::Counter};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::OnceLock};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "'de: 'static"))]
struct Document {
    schema: u32,
    programs: Vec<Program>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "'de: 'static"))]
struct Program {
    key: String,
    quest: String,
    mode: String,
    repeat: String,
    runtime_enabled: bool,
    requires_premium: bool,
    minimum_level: u32,
    journal: Journal,
    phase_track: String,
    tracks: Vec<Track>,
    world_callbacks: Vec<serde_json::Value>,
    npc_callbacks: Vec<serde_json::Value>,
    counters: Vec<AreaCounter>,
    stage_bindings: std::collections::BTreeMap<String, Vec<String>>,
    completion: Completion,
    achievement: Option<String>,
    taming: Option<serde_json::Value>,
    sources: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    name: String,
    objectives: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    key: String,
    quest: String,
    initial: i64,
    min: i64,
    max: i64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Completion {
    callbacks: Vec<String>,
    counter_track: Option<String>,
    counter_target: Option<i64>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AreaCounter {
    transition: &'static str,
    creature: &'static str,
    quest: &'static str,
    track: &'static str,
    gate: &'static str,
    maximum: i64,
    area: [u16; 6],
    #[serde(skip)]
    compiled: Option<Counter>,
}

fn document() -> Result<&'static Document, String> {
    static DOCUMENT: OnceLock<Result<Document, String>> = OnceLock::new();
    DOCUMENT
        .get_or_init(|| {
            parse(include_str!(
                "../../../../../content/quests/world-change-journals.json"
            ))
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn parse(source: &'static str) -> Result<Document, String> {
    let mut document: Document = serde_json::from_str(source).map_err(|e| e.to_string())?;
    let mut keys = BTreeSet::new();
    if document.schema != 1 || document.programs.len() > 16 {
        return Err("world-change journal bounds".into());
    }
    for program in &mut document.programs {
        let prefix = format!(
            "oteryn:quest-progress/world-change-journal/{}/",
            program.key
        );
        if !keys.insert(program.quest.clone())
            || program.mode != "personal_on_demand"
            || program.repeat != "once"
            || program.sources.is_empty()
            || program.journal.name.is_empty()
            || program.journal.name.len() > 128
            || program.journal.objectives.is_empty()
            || program.journal.objectives.join(" ").len() > 800
            || !super::valid_quest_key(&program.quest)
            || program.phase_track != format!("{prefix}phase")
            || program.tracks.is_empty()
            || program.tracks.len() > 8
            || program.stage_bindings.is_empty()
            || program.stage_bindings.values().any(Vec::is_empty)
            || program
                .sources
                .iter()
                .any(|url| !url.starts_with("https://"))
            || program.tracks.iter().any(|track| {
                track.quest != program.quest
                    || !track.key.starts_with(&prefix)
                    || track.min > track.initial
                    || track.initial > track.max
                    || (track.max > 10_000
                        && !(track.key == format!("{prefix}waves") && track.max == i64::MAX))
            })
        {
            return Err(format!("invalid personal journal {}", program.key));
        }
        // Incomplete source programmes stay outside every executable registry.
        if !program.runtime_enabled {
            continue;
        }
        if program.taming.is_some()
            || program.achievement.as_deref().is_some_and(|key| {
                !key.starts_with("oteryn:achievement/") || !super::valid_quest_key(key)
            })
        {
            return Err("unsupported enabled journal reward policy".into());
        }
        let callbacks = super::world_callbacks::parse(
            &serde_json::json!({
                "schema": 1, "callbacks": program.world_callbacks,
            })
            .to_string(),
        )?;
        let mut events = BTreeSet::new();
        for callback in callbacks {
            if callback.quest != program.quest
                || !callback.requires_world_admission
                || callback.world_change.is_some()
                || callback.minimum_level.unwrap_or(0) != program.minimum_level
                || !callback
                    .key
                    .starts_with("oteryn:quest-callback/worldchg-m31/")
                || callback
                    .storage
                    .iter()
                    .any(|s| !program.tracks.iter().any(|t| t.key == s.track))
            {
                return Err("personal journal callback ownership".into());
            }
            events.insert(callback.key);
        }
        let npc_callbacks = lower_npc_callbacks(program)?;
        for callback in &npc_callbacks {
            events.insert(callback.key.clone());
        }
        program.npc_callbacks = npc_callbacks
            .into_iter()
            .map(|callback| serde_json::to_value(callback).map_err(|error| error.to_string()))
            .collect::<Result<_, _>>()?;
        for counter in &mut program.counters {
            let [x0, x1, y0, y1, z0, z1] = counter.area;
            if counter.quest != program.quest
                || counter.maximum < 1
                || counter.maximum > 10_000
                || x0 > x1
                || y0 > y1
                || z0 > z1
                || z1 > 15
                || !counter
                    .transition
                    .starts_with("oteryn:quest-transition/native-kills/m31/")
                || !super::valid_quest_key(counter.creature)
                || !program
                    .tracks
                    .iter()
                    .any(|t| t.key == counter.track && t.max == counter.maximum)
                || !program
                    .tracks
                    .iter()
                    .any(|t| t.key == counter.gate && t.max == 1)
            {
                return Err("personal journal counter ownership".into());
            }
            counter.compiled = Some(Counter {
                transition: counter.transition,
                creature: counter.creature,
                quest: counter.quest,
                track: counter.track,
                gate: counter.gate,
                maximum: counter.maximum,
                world_change: None,
            });
            events.insert(counter.transition.into());
        }
        if program
            .stage_bindings
            .values()
            .flatten()
            .any(|event| !events.contains(event))
            || (program.completion.callbacks.is_empty()
                && program.completion.counter_track.is_none())
            || program
                .completion
                .callbacks
                .iter()
                .any(|event| !events.contains(event))
        {
            return Err("unbound personal journal stage".into());
        }
        match (
            &program.completion.counter_track,
            program.completion.counter_target,
        ) {
            (None, None) => {}
            (Some(track), Some(target))
                if program.completion.callbacks.is_empty()
                    && target > 0
                    && program
                        .counters
                        .iter()
                        .any(|counter| counter.track == track)
                    && program
                        .counters
                        .iter()
                        .filter(|counter| counter.track == track)
                        .all(|counter| counter.maximum == target) => {}
            _ => return Err("unbound personal journal counter goal".into()),
        }
    }
    Ok(document)
}

fn lower_npc_callbacks(
    program: &Program,
) -> Result<Vec<super::reward_callbacks::RewardCallback>, String> {
    if program.npc_callbacks.len() > 16 {
        return Err("personal journal NPC callback bound".into());
    }
    let mut callbacks = Vec::new();
    for value in &program.npc_callbacks {
        let mut value = value.clone();
        let fields = value.as_object_mut().ok_or("NPC callback object")?;
        // These are programme requirements, not optional per-dialogue overrides.
        fields.insert("requires_premium".into(), program.requires_premium.into());
        fields.insert(
            "minimum_level".into(),
            (program.minimum_level > 0)
                .then_some(program.minimum_level)
                .into(),
        );
        let callback: super::reward_callbacks::RewardCallback =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        if callback.quest != program.quest
            || !callback
                .key
                .starts_with("oteryn:quest-reward/worldchg-m31/")
            || callback.carried_item.is_some()
            || callback.completes != program.completion.callbacks.contains(&callback.key)
            || callback.achievement.as_ref().is_some_and(|achievement| {
                !callback.completes || program.achievement.as_ref() != Some(achievement)
            })
            || callback
                .storage
                .iter()
                .map(|storage| &storage.track)
                .chain(callback.state_ops.iter().map(|operation| &operation.track))
                .any(|key| !program.tracks.iter().any(|track| &track.key == key))
        {
            return Err("personal journal NPC callback ownership".into());
        }
        callbacks.push(callback);
    }
    if program.achievement.is_some()
        && program.completion.counter_track.is_none()
        && !callbacks
            .iter()
            .any(|callback| callback.achievement.is_some())
    {
        return Err("personal journal achievement has no final NPC receipt".into());
    }
    super::reward_callbacks::validate_callbacks(&callbacks)?;
    Ok(callbacks)
}

pub(super) fn npc_callbacks() -> Result<Vec<super::reward_callbacks::RewardCallback>, String> {
    document()?
        .programs
        .iter()
        .filter(|program| program.runtime_enabled)
        .flat_map(|program| &program.npc_callbacks)
        .cloned()
        .map(|value| serde_json::from_value(value).map_err(|error| error.to_string()))
        .collect()
}

pub(super) fn callbacks() -> Result<Vec<super::world_callbacks::Callback>, String> {
    let values: Vec<_> = document()?
        .programs
        .iter()
        .filter(|p| p.runtime_enabled)
        .flat_map(|p| p.world_callbacks.iter().cloned())
        .collect();
    super::world_callbacks::parse(&serde_json::json!({"schema":1,"callbacks":values}).to_string())
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.into_values().collect();
    for program in document()?.programs.iter().filter(|p| p.runtime_enabled) {
        for track in &program.tracks {
            tracks.push(QuestTrack {
                key: track.key.clone(),
                quest: track.quest.clone(),
                initial: track.initial,
                min: track.min,
                max: track.max,
            });
        }
    }
    QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks,
        catalogue.transitions.into_values().collect(),
    )
    .map_err(|e| format!("{e:?}"))
}

pub(super) fn counters() -> Result<impl Iterator<Item = &'static Counter>, String> {
    Ok(document()?
        .programs
        .iter()
        .filter(|p| p.runtime_enabled)
        .flat_map(|p| p.counters.iter().filter_map(|c| c.compiled.as_ref())))
}

/// Position is captured from the actual dead actor; a player's later position is irrelevant.
#[must_use]
pub fn admits_position(transition: &str, position: Option<(u16, u16, i8)>) -> bool {
    if !transition.starts_with("oteryn:quest-transition/native-kills/m31/") {
        return true;
    }
    let Ok(document) = document() else {
        return false;
    };
    let Some(counter) = document
        .programs
        .iter()
        .filter(|p| p.runtime_enabled)
        .flat_map(|p| &p.counters)
        .find(|c| c.transition == transition)
    else {
        return false;
    };
    let Some((x, y, floor)) = position else {
        return false;
    };
    let [x0, x1, y0, y1, z0, z1] = counter.area;
    (x0..=x1).contains(&x)
        && (y0..=y1).contains(&y)
        && (i32::from(z0)..=i32::from(z1)).contains(&-i32::from(floor))
}

pub(super) fn completes(callback: &str) -> bool {
    document().is_ok_and(|d| {
        d.programs
            .iter()
            .filter(|p| p.runtime_enabled)
            .any(|p| completion_matches(p, callback))
    })
}

fn completion_matches(program: &Program, event: &str) -> bool {
    program.completion.callbacks.iter().any(|key| key == event)
        || program.counters.iter().any(|counter| {
            counter.transition == event
                && program.completion.counter_track.as_deref() == Some(counter.track)
                && program.completion.counter_target == Some(counter.maximum)
        })
}

fn counter_achievement<'a>(program: &'a Program, event: &str) -> Option<&'a str> {
    if program.runtime_enabled
        && program
            .counters
            .iter()
            .any(|counter| counter.transition == event)
        && completion_matches(program, event)
    {
        program.achievement.as_deref()
    } else {
        None
    }
}

/// Immutable reward policy for a native counter's final, goal-reaching receipt.
/// The durable caller must also verify the committed goal and native death proof.
pub fn counter_completion_achievement(event: &str) -> Result<Option<&'static str>, String> {
    Ok(document()?
        .programs
        .iter()
        .filter(|program| program.runtime_enabled)
        .find_map(|program| counter_achievement(program, event)))
}

fn journal_end(p: &Program) -> Result<(String, i64), String> {
    if let (Some(track), Some(target)) = (&p.completion.counter_track, p.completion.counter_target)
    {
        return Ok((track.clone(), target));
    }
    let end = p
        .world_callbacks
        .iter()
        .chain(&p.npc_callbacks)
        .filter(|c| {
            p.completion
                .callbacks
                .iter()
                .any(|key| c["key"].as_str() == Some(key))
        })
        .filter_map(|c| {
            c["storage"]
                .as_array()?
                .iter()
                .find(|s| s["track"].as_str() == Some(&p.phase_track))?["set"]
                .as_i64()
        })
        .max()
        .ok_or("journal completion phase")?;
    Ok((p.phase_track.clone(), end))
}
/// Requirement terms are included in the callback binding and read in its fenced transaction.
#[must_use]
pub fn requires_premium(callback: &str) -> bool {
    document().is_ok_and(|d| {
        d.programs
            .iter()
            .filter(|p| p.runtime_enabled && p.requires_premium)
            .any(|p| {
                p.world_callbacks
                    .iter()
                    .any(|c| c["key"].as_str() == Some(callback))
            })
    })
}

/// Bind executable journal rules, including death areas, to every activation receipt.
pub(super) fn policy_binding(callback: &str) -> Result<Option<serde_json::Value>, String> {
    let program = document()?
        .programs
        .iter()
        .filter(|p| p.runtime_enabled)
        .find(|p| {
            p.world_callbacks
                .iter()
                .any(|c| c["key"].as_str() == Some(callback))
        });
    program.map(executable_policy).transpose()
}

fn executable_policy(p: &Program) -> Result<serde_json::Value, String> {
    serde_json::to_value((
        &p.key,
        &p.quest,
        p.requires_premium,
        p.minimum_level,
        &p.counters,
        &p.completion,
        &p.achievement,
        &p.taming,
    ))
    .map_err(|error| error.to_string())
}

pub(super) fn append_journals(entries: &mut Vec<super::log::QuestLogQuest>) -> Result<(), String> {
    use super::log::{
        JournalPiece, JournalText, QuestJournal, QuestLogKind, QuestLogMission, QuestLogQuest,
    };
    for p in document()?.programs.iter().filter(|p| p.runtime_enabled) {
        let (mission_track, end) = journal_end(p)?;
        let mut pieces = vec![JournalPiece::Text(p.journal.objectives.join(" "))];
        let mut seen = BTreeSet::new();
        for counter in &p.counters {
            if seen.insert(counter.track) {
                pieces.extend([
                    JournalPiece::Text(" Progress: ".into()),
                    JournalPiece::Track(counter.track.into()),
                    JournalPiece::Text(format!("/{}.", counter.maximum)),
                ]);
            }
        }
        entries.retain(|entry| entry.key != p.quest);
        entries.push(QuestLogQuest {
            key: p.quest.clone(),
            kind: QuestLogKind::Storyline,
            shown_in_quest_log: true,
            start: Some((p.phase_track.clone(), 1)),
            hide_when_completed: false,
            missions: vec![QuestLogMission {
                name: p.journal.name.clone(),
                track: mission_track,
                start_value: i64::from(p.completion.counter_track.is_none()),
                end_value: end,
                journal: QuestJournal::Fixed(JournalText::Template(pieces)),
            }],
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_counter_achievement_programme_requires_a_bound_goal() -> Result<(), String> {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/world-change-journals.json"
        ))
        .map_err(|error| error.to_string())?;
        for programme in value["programs"].as_array_mut().ok_or("programmes")? {
            programme["runtime_enabled"] = (programme["key"] == "fire_from_the_earth").into();
        }
        let mut document = super::parse(Box::leak(value.to_string().into_boxed_str()))?;
        let programme = document
            .programs
            .iter_mut()
            .find(|p| p.runtime_enabled)
            .ok_or("programme")?;
        assert_eq!(
            programme.achievement.as_deref(),
            Some("oteryn:achievement/fire_from_the_earth")
        );
        assert_eq!(programme.completion.counter_target, Some(50));
        for counter in &programme.counters {
            assert_eq!(
                super::counter_achievement(programme, counter.transition),
                programme.achievement.as_deref()
            );
        }
        for callback in &programme.world_callbacks {
            assert_eq!(
                super::counter_achievement(
                    programme,
                    callback["key"].as_str().ok_or("callback key")?
                ),
                None
            );
        }
        assert_eq!(
            super::counter_achievement(
                programme,
                "oteryn:quest-transition/native-kills/m31/foreign/dragon"
            ),
            None
        );
        assert_eq!(
            super::counter_completion_achievement(
                "oteryn:quest-transition/native-kills/m31/foreign/dragon"
            )?,
            None,
            "foreign counter keys cannot grant achievements"
        );
        programme.runtime_enabled = false;
        assert_eq!(
            super::counter_achievement(programme, programme.counters[0].transition),
            None,
            "disabled source programmes cannot grant achievements"
        );
        for invalid in 0..5 {
            let mut bad = value.clone();
            let programme = bad["programs"]
                .as_array_mut()
                .ok_or("programmes")?
                .iter_mut()
                .find(|p| p["runtime_enabled"] == true)
                .ok_or("programme")?;
            match invalid {
                0 => programme["completion"]["counter_target"] = 49.into(),
                1 => {
                    programme["completion"]["counter_track"] =
                        "oteryn:quest-progress/foreign".into()
                }
                2 => programme["completion"]["counter_target"] = serde_json::Value::Null,
                3 => programme["achievement"] = "oteryn:item.tibia.i2576".into(),
                _ => {
                    programme["completion"] = serde_json::json!({"callbacks":[],"counter_track":null,"counter_target":null})
                }
            }
            assert!(
                super::parse(Box::leak(bad.to_string().into_boxed_str())).is_err(),
                "unbound achievement case {invalid}"
            );
        }
        Ok(())
    }

    #[test]
    fn personal_room_entry_rejects_unfenced_shared_or_remote_staging() -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/world-change-journals.json"
        ))
        .map_err(|error| error.to_string())?;
        let programme = value["programs"]
            .as_array()
            .ok_or("programmes")?
            .iter()
            .find(|p| p["key"] == "insectoid_invasion")
            .ok_or("invasion")?;
        let callback: super::super::world_callbacks::Callback =
            serde_json::from_value(programme["world_callbacks"][0].clone())
                .map_err(|error| error.to_string())?;
        assert_eq!(callback.personal_room_entry(), callback.selector.position);
        super::super::world_callbacks::validate_callbacks(std::slice::from_ref(&callback))?;
        for case in 0..5 {
            let mut bad = callback.clone();
            match case {
                0 => bad.requires_world_admission = false,
                1 => bad.storage.clear(),
                2 => bad.trigger = super::super::world_callbacks::Trigger::Use,
                3 => bad.selector.position.as_mut().ok_or("selector")?.x += 1,
                _ => {
                    let encounter = bad.effects.encounter.as_mut().ok_or("encounter")?;
                    encounter.players.push(encounter.players[0].clone());
                }
            }
            assert_eq!(bad.personal_room_entry(), None, "staging case {case}");
            assert!(super::super::world_callbacks::validate_callbacks(&[bad]).is_err());
        }
        Ok(())
    }

    #[test]
    fn personal_counter_goal_programmes_lower_without_premature_completion() -> Result<(), String> {
        for selected in ["insectoid_invasion", "jungle_camp"] {
            let mut value: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../content/quests/world-change-journals.json"
            ))
            .map_err(|error| error.to_string())?;
            for program in value["programs"].as_array_mut().ok_or("programmes")? {
                program["runtime_enabled"] = (program["key"] == selected).into();
            }
            let source = Box::leak(value.to_string().into_boxed_str());
            let document = super::parse(source)?;
            let programme = document
                .programs
                .iter()
                .find(|p| p.runtime_enabled)
                .ok_or("programme")?;
            assert!(programme.completion.callbacks.is_empty());
            assert!(programme.completion.counter_track.is_some());
            let (track, target) = super::journal_end(programme)?;
            assert_eq!(
                Some(track.as_str()),
                programme.completion.counter_track.as_deref()
            );
            assert_eq!(Some(target), programme.completion.counter_target);
            for counter in &programme.counters {
                assert_eq!(
                    super::completion_matches(programme, counter.transition),
                    counter.track == track
                );
            }
            assert!(!super::completion_matches(
                programme,
                "oteryn:quest-transition/native-kills/m31/foreign/scout"
            ));
            assert!(
                programme
                    .world_callbacks
                    .iter()
                    .all(|callback| !super::completion_matches(
                        programme,
                        callback["key"].as_str().unwrap_or_default()
                    ))
            );
            for invalid in [
                serde_json::json!({"callbacks": [], "counter_track": track, "counter_target": 0}),
                serde_json::json!({"callbacks": [], "counter_track": track, "counter_target": target + 1}),
                serde_json::json!({"callbacks": [], "counter_track": "oteryn:quest-progress/foreign", "counter_target": target}),
                serde_json::json!({"callbacks": [], "counter_track": track, "counter_target": null}),
                serde_json::json!({"callbacks": [], "counter_track": null, "counter_target": target}),
                serde_json::json!({"callbacks": [programme.world_callbacks[0]["key"]], "counter_track": track, "counter_target": target}),
            ] {
                let mut bad = value.clone();
                let selected = bad["programs"]
                    .as_array_mut()
                    .ok_or("programmes")?
                    .iter_mut()
                    .find(|p| p["runtime_enabled"] == true)
                    .ok_or("programme")?;
                selected["completion"] = invalid;
                assert!(super::parse(Box::leak(bad.to_string().into_boxed_str())).is_err());
            }
        }
        Ok(())
    }

    #[test]
    fn native_counter_completion_requires_the_actual_goal_change() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("m31-goal-test")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        let key = "oteryn:quest-transition/native-kills/m31/grimvale/werewolf";
        let counter = super::super::native_kills::counter(key).ok_or("counter")?;
        let mut transition = catalogue.transition(key).ok_or("transition")?.clone();
        transition.completes = true;
        for after in 1..=counter.maximum {
            let change = super::super::QuestTrackChange {
                track: counter.track.into(),
                before: after - 1,
                after,
            };
            assert_eq!(
                catalogue.completes_after(&transition, &[change]),
                after == counter.maximum
            );
        }
        assert!(!catalogue.completes_after(&transition, &[]));
        assert!(!catalogue.completes_after(
            &transition,
            &[super::super::QuestTrackChange {
                track: counter.gate.into(),
                before: 0,
                after: counter.maximum,
            }]
        ));
        Ok(())
    }

    fn npc_programme_fixture() -> Result<serde_json::Value, String> {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/world-change-journals.json"
        ))
        .map_err(|error| error.to_string())?;
        for program in value["programs"].as_array_mut().ok_or("programmes")? {
            let selected = program["key"].as_str() == Some("lumberjack");
            program["runtime_enabled"] = selected.into();
            if selected {
                program["minimum_level"] = 17.into();
                program["requires_premium"] = true.into();
                for callback in program["world_callbacks"]
                    .as_array_mut()
                    .ok_or("world callbacks")?
                {
                    callback["minimum_level"] = 17.into();
                }
            }
        }
        Ok(value)
    }

    #[test]
    fn native_npc_journals_inherit_recipe_gates_and_complete_with_the_achievement()
    -> Result<(), String> {
        let value = npc_programme_fixture()?;
        let source: &'static str = Box::leak(value.to_string().into_boxed_str());
        let document = super::parse(source)?;
        let program = document
            .programs
            .iter()
            .find(|p| p.runtime_enabled)
            .ok_or("NPC programme")?;
        let callbacks: Vec<super::super::reward_callbacks::RewardCallback> = program
            .npc_callbacks
            .iter()
            .cloned()
            .map(serde_json::from_value)
            .collect::<Result<_, _>>()
            .map_err(|error| error.to_string())?;
        assert_eq!(callbacks.len(), 2);
        assert!(
            callbacks
                .iter()
                .all(|callback| callback.requires_premium && callback.minimum_level == Some(17))
        );
        let completion = callbacks
            .iter()
            .find(|callback| callback.completes)
            .ok_or("completion")?;
        assert_eq!(
            completion.achievement.as_deref(),
            Some("oteryn:achievement/whistle_blower")
        );
        assert!(program.completion.callbacks.contains(&completion.key));
        Ok(())
    }
    #[test]
    fn npc_journals_reject_foreign_storage_and_unbound_achievements() -> Result<(), String> {
        for case in 0..4 {
            let mut value = npc_programme_fixture()?;
            let program = value["programs"]
                .as_array_mut()
                .ok_or("programmes")?
                .iter_mut()
                .find(|program| program["runtime_enabled"] == true)
                .ok_or("NPC programme")?;
            match case {
                0 => {
                    program["npc_callbacks"][0]["storage"][0]["track"] =
                        "oteryn:quest-progress/world-change-journal/grimvale/phase".into()
                }
                1 => {
                    program["npc_callbacks"][1]["achievement"] =
                        "oteryn:achievement/loyal_subject".into()
                }
                2 => {
                    program["npc_callbacks"][0]["achievement"] =
                        "oteryn:achievement/whistle_blower".into()
                }
                _ => program["npc_callbacks"][1]["completes"] = false.into(),
            }
            let source: &'static str = Box::leak(value.to_string().into_boxed_str());
            assert!(
                super::parse(source).is_err(),
                "unbound NPC policy case {case}"
            );
        }
        Ok(())
    }

    #[test]
    fn npc_bindings_cover_ungated_achievement_and_completion_terms() -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/world-change-journals.json"
        ))
        .map_err(|error| error.to_string())?;
        let program = value["programs"]
            .as_array()
            .ok_or("programmes")?
            .iter()
            .find(|program| program["key"] == "kingsday")
            .ok_or("Kingsday")?;
        let mut callback: super::super::reward_callbacks::RewardCallback =
            serde_json::from_value(program["npc_callbacks"][0].clone())
                .map_err(|error| error.to_string())?;
        assert!(callback.minimum_level.is_none());
        let before =
            super::super::reward_callbacks::gated_binding(&callback)?.ok_or("NPC binding")?;
        callback.reply.push("Display-only revision".into());
        assert_eq!(
            Some(before),
            super::super::reward_callbacks::gated_binding(&callback)?
        );
        callback.achievement = Some("oteryn:achievement/whistle_blower".into());
        assert_ne!(
            Some(before),
            super::super::reward_callbacks::gated_binding(&callback)?
        );
        callback = serde_json::from_value(program["npc_callbacks"][0].clone())
            .map_err(|error| error.to_string())?;
        callback.completes = false;
        assert_ne!(
            Some(before),
            super::super::reward_callbacks::gated_binding(&callback)?
        );
        Ok(())
    }

    #[test]
    fn activation_policy_binds_death_area_and_goal_but_not_display_text() -> Result<(), String> {
        let mut document = super::parse(include_str!(
            "../../../../../content/quests/world-change-journals.json"
        ))?;
        let p = document
            .programs
            .iter_mut()
            .find(|p| p.runtime_enabled && !p.counters.is_empty())
            .ok_or("counter program")?;
        let before = super::executable_policy(p)?;
        p.journal.name.push_str(" revised display");
        assert_eq!(before, super::executable_policy(p)?);
        p.counters[0].area[1] += 1;
        assert_ne!(before, super::executable_policy(p)?);
        p.counters[0].area[1] -= 1;
        p.counters[0].maximum += 1;
        assert_ne!(before, super::executable_policy(p)?);
        Ok(())
    }
    #[test]
    fn personal_counter_uses_its_registered_native_owner() -> Result<(), String> {
        let counter = super::super::native_kills::counter(
            "oteryn:quest-transition/native-kills/m31/grimvale/werewolf",
        )
        .ok_or("personal counter")?;
        super::super::admission_binding::validate(&counter.transition(), false)?;
        Ok(())
    }
    #[test]
    fn personal_encounters_require_fenced_single_player_admission() -> Result<(), String> {
        let callback = super::callbacks()?
            .into_iter()
            .find(|c| c.key.ends_with("grimvale/island"))
            .ok_or("island encounter")?;
        assert!(callback.effects.encounter.is_some());
        let mut value = serde_json::to_value(&callback).map_err(|e| e.to_string())?;
        value["requires_world_admission"] = false.into();
        assert!(
            super::super::world_callbacks::parse(
                &serde_json::json!({"schema":1,"callbacks":[value]}).to_string()
            )
            .is_err()
        );
        let mut bad = callback;
        let extra = bad.effects.encounter.as_ref().ok_or("encounter")?.players[0].clone();
        bad.effects
            .encounter
            .as_mut()
            .ok_or("encounter")?
            .players
            .push(extra);
        assert!(
            super::super::world_callbacks::parse(
                &serde_json::json!({"schema":1,"callbacks":[bad]}).to_string()
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn personal_counters_require_the_frozen_native_scene() {
        let key = "oteryn:quest-transition/native-kills/m31/grimvale/werewolf";
        assert!(super::admits_position(key, Some((33312, 31709, -7))));
        for at in [
            None,
            Some((10, 10, -7)),
            Some((33312, 31709, 7)),
            Some((33312, 31709, -16)),
        ] {
            assert!(!super::admits_position(key, at));
        }
        assert!(!super::admits_position(
            "oteryn:quest-transition/native-kills/m31/unknown",
            Some((33312, 31709, -7))
        ));
        assert!(super::admits_position(
            "oteryn:quest-transition/native-kills/dragon-slayer-dragon",
            None
        ));
    }
    #[test]
    fn m31_selected_native_journals_are_lowered_without_calendar_gates() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("m31-test")
            .map_err(|e| format!("{e:?}"))?;
        for key in ["fury_gates", "grimvale", "hive_outpost"] {
            let phase = format!("oteryn:quest-progress/world-change-journal/{key}/phase");
            assert!(
                lowered.catalogue().tracks.contains_key(&phase),
                "missing personal {key} journal"
            );
        }
        let journal = super::super::log::loader::load_embedded_quest_log(&lowered)?;
        for key in [
            "oteryn:quest.fury_gates_mini_world_change",
            "oteryn:quest.grimvale_mini_world_change",
            "oteryn:quest.authored.hive_outpost_mini_world_change",
        ] {
            assert!(
                journal
                    .content
                    .active()
                    .quests()
                    .iter()
                    .any(|q| q.key == key && q.shown_in_quest_log && !q.missions.is_empty())
            );
        }
        Ok(())
    }
}
