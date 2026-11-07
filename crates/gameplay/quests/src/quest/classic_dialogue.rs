//! Literal Crystal quest replies selected by dedicated, stateful NPC adapters.
//! No source expression, keyword tree or stored Lua is evaluated here.
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    npcs: Vec<Npc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Npc {
    npc: String,
    branches: Vec<Branch>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Branch {
    key: String,
    reply: Vec<String>,
}

fn parse(text: &str) -> Result<Document, String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if document.schema != 1 || document.npcs.len() > 12 {
        return Err("classic dialogue bounds".into());
    }
    let mut npcs = BTreeSet::new();
    for npc in &document.npcs {
        if !npc.npc.starts_with("oteryn:npc.") || !npcs.insert(&npc.npc) || npc.branches.len() > 256
        {
            return Err("classic dialogue NPC".into());
        }
        let mut branches = BTreeSet::new();
        for branch in &npc.branches {
            if !branch.key.starts_with("line-")
                || !branches.insert(&branch.key)
                || branch.reply.is_empty()
                || branch.reply.len() > 16
                || branch
                    .reply
                    .iter()
                    .any(|line| line.is_empty() || line.len() > 4096 || line.contains('\0'))
            {
                return Err("classic dialogue branch".into());
            }
        }
    }
    Ok(document)
}

/// The only selectable replies are literal source branches in the generated table.
#[must_use]
pub fn reply(npc: &str, key: &str) -> Option<&'static [String]> {
    static CONTENT: OnceLock<Option<Document>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            parse(include_str!(
                "../../../../../content/dialogues/classic-quest-branches.json"
            ))
            .ok()
        })
        .as_ref()?
        .npcs
        .iter()
        .find(|entry| entry.npc == npc)?
        .branches
        .iter()
        .find(|entry| entry.key == key)
        .map(|branch| branch.reply.as_slice())
}

/// Merge source-owned overlays using the existing QuestState reader and evaluator.
/// An existing track must match exactly; this never widens or renames another lane's track.
pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks.clone();
    let mut transitions = catalogue.transitions.clone();
    for source in [
        include_str!("../../../../../content/quests/dawnport-starts.json"),
        include_str!("../../../../../content/quests/troll-sabotage-starts.json"),
        include_str!("../../../../../content/quests/rottin-wood-starts.json"),
    ] {
        let overlay = super::loader::parse_quest_state(source, catalogue.content_revision())
            .map_err(|e| format!("{e:?}"))?;
        for (key, track) in &overlay.catalogue().tracks {
            if tracks.get(key).is_some_and(|existing| existing != track) {
                return Err(format!("classic quest track conflict: {key}"));
            }
            tracks.insert(key.clone(), track.clone());
        }
        for (key, transition) in &overlay.catalogue().transitions {
            if transitions
                .insert(key.clone(), transition.clone())
                .is_some()
            {
                return Err(format!("classic quest transition conflict: {key}"));
            }
        }
    }
    super::QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|e| format!("{e:?}"))
    .and_then(super::hero_of_bounac::extend)
    .and_then(super::planegazer_finale::extend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{QuestRefusal, loader};
    use std::collections::BTreeMap;

    #[test]
    fn rottin_start_resets_only_its_three_source_tracks_atomically() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("classic-test").map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        let transition = catalogue
            .transition("oteryn:quest-transition/classic/rottin-wood/start-disrepair")
            .ok_or("Rottin transition")?;
        let line = format!("{}questline", crate::quest::classic_talk::ROTTIN_STATE);
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::new(), 1),
            Err(QuestRefusal::StageMismatch)
        );
        let before = BTreeMap::from([(line.clone(), 2)]);
        let changes = catalogue
            .evaluate(transition, &before, 1)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(changes.len(), 3);
        assert_eq!(
            catalogue.evaluate(transition, &BTreeMap::from([(line, 3)]), 1),
            Err(QuestRefusal::StageMismatch)
        );
        Ok(())
    }

    #[test]
    fn literal_replies_preserve_all_parts_and_fail_closed_on_unknown_branch() -> Result<(), String>
    {
        let herbs = reply("oteryn:npc.mr_morris", "line-173").ok_or("herbs reply")?;
        assert_eq!(herbs.len(), 2);
        assert!(herbs[0].contains("Dawnfire buds"));
        assert!(herbs[1].contains("light grey-brown sand"));
        assert!(reply("oteryn:npc.jerom", "line-69").is_some_and(|lines| lines.len() == 3));
        assert!(reply("oteryn:npc.mr_morris", "line-99999").is_none());
        assert!(reply("oteryn:npc.other", "line-173").is_none());
        Ok(())
    }

    #[test]
    fn native_start_commits_both_herb_flags_and_refuses_stale_confirmation() -> Result<(), String> {
        // Donor qualification only: production excludes Dawnport (QUEST-FINISH).
        let loaded = loader::parse_quest_state(
            include_str!("../../../../../content/quests/dawnport-starts.json"),
            "classic-test",
        )
        .map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        let transition = catalogue
            .transition("oteryn:quest-transition/classic/dawnport/start-the-rare-herb")
            .ok_or("herb transition")?;
        let changes = catalogue
            .evaluate(transition, &BTreeMap::new(), 1)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(changes.len(), 2);
        let begun = BTreeMap::from([(
            "oteryn:quest-progress/quest/u10_55/dawnport/the_rare_herb".into(),
            2,
        )]);
        assert_eq!(
            catalogue.evaluate(transition, &begun, 1),
            Err(QuestRefusal::StageMismatch)
        );
        Ok(())
    }

    #[test]
    fn logbook_start_checks_torn_book_and_does_not_clear_its_track() -> Result<(), String> {
        let loaded = loader::parse_quest_state(
            include_str!("../../../../../content/quests/dawnport-starts.json"),
            "classic-test",
        )
        .map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        let transition = catalogue
            .transition("oteryn:quest-transition/classic/dawnport/start-logbook")
            .ok_or("logbook transition")?;
        let found = BTreeMap::from([(
            "oteryn:quest-progress/quest/u10_55/dawnport/torn_log_book".into(),
            1,
        )]);
        assert_eq!(
            catalogue.evaluate(transition, &found, 1),
            Err(QuestRefusal::StageMismatch)
        );
        assert!(catalogue.evaluate(transition, &BTreeMap::new(), 1).is_ok());
        Ok(())
    }

    #[test]
    fn jerom_start_cannot_reset_a_returned_necklace() -> Result<(), String> {
        let loaded =
            loader::load_embedded_quest_state("classic-test").map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        let transition = catalogue
            .transition("oteryn:quest-transition/classic/troll-sabotage/start")
            .ok_or("Jerom transition")?;
        for progress in 1..=3 {
            let stored = BTreeMap::from([(
                "oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/questline".into(),
                progress,
            )]);
            assert_eq!(
                catalogue.evaluate(transition, &stored, 1),
                Err(QuestRefusal::StageMismatch)
            );
        }
        assert!(catalogue.evaluate(transition, &BTreeMap::new(), 1).is_ok());
        Ok(())
    }
}
