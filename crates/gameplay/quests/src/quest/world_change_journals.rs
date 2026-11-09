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
        if program.achievement.is_some()
            || program.taming.is_some()
            || program.completion.counter_track.is_some()
            || program.completion.counter_target.is_some()
        {
            return Err("unsupported enabled journal reward policy".into());
        }
        let callbacks = super::world_callbacks::parse(
            &serde_json::json!({
                "schema": 1, "callbacks": program.world_callbacks,
            })
            .to_string(),
        )?;
        if !program.npc_callbacks.is_empty() {
            return Err("unsupported enabled journal NPC policy".into());
        }
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
            || program.completion.callbacks.is_empty()
            || program
                .completion
                .callbacks
                .iter()
                .any(|event| !events.contains(event))
        {
            return Err("unbound personal journal stage".into());
        }
    }
    Ok(document)
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
            .any(|p| p.completion.callbacks.iter().any(|key| key == callback))
    })
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
        let end = p
            .world_callbacks
            .iter()
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
                track: p.phase_track.clone(),
                start_value: 1,
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
