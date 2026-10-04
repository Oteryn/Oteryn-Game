//! QUEST-CAT-BOOT-1 (ARCH-QUEST-WIRING-PACKETS-1 §1.1-§1.3): the node loads the embedded quest
//! state catalogue at boot for its served content revision, refuses boot on a malformed
//! document, and keeps `Computed` transitions refused `NOT_SUPPORTED`. Every expected count is
//! taken from the committed document and the loaded catalogue, never from constants.

use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;

use crate::durability::quest_state::quest::loader::{load_embedded_quest_state, parse_quest_state};
use crate::durability::quest_state::quest::{QuestRefusal, QuestStateCatalogue};
use crate::interaction_chest_use::entry_chest;
use crate::node::serve::{BootError, load_quest_catalogue};

type TestResult = Result<(), Box<dyn Error>>;

const DOCUMENT: &str = include_str!("../../../../content/quests/missions/quest-state.json");

/// The served content revision the boot loads the catalogue for (§1.2).
fn served_revision() -> &'static str {
    crate::content::accepted::REVISIONS[0]
}

/// The catalogue and event line exactly as `boot_and_serve` loads them.
fn boot_catalogue() -> Result<(Arc<QuestStateCatalogue>, String), Box<dyn Error>> {
    load_quest_catalogue(load_embedded_quest_state, served_revision())
        .map_err(|error| format!("{error:?}").into())
}

/// The committed document's transitions, read independently of the loader.
fn document_transitions() -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    let document: serde_json::Value = serde_json::from_str(DOCUMENT)?;
    let quests = document["quests"].as_array().ok_or("quests")?;
    let mut transitions = Vec::new();
    for quest in quests {
        transitions.extend(
            quest["transitions"]
                .as_array()
                .ok_or("transitions")?
                .clone(),
        );
    }
    Ok(transitions)
}

fn effects(transition: &serde_json::Value) -> Result<&Vec<serde_json::Value>, Box<dyn Error>> {
    Ok(transition["effects"].as_array().ok_or("effects")?)
}

fn explicit_computed(effect: &serde_json::Value) -> bool {
    effect["effect"]["op"] == "COMPUTED" || effect["effect"]["kind"] == "COMPUTED"
}

fn inexact(effect: &serde_json::Value) -> bool {
    effect["from_exact"] == false
}

/// The value of `field=` in the boot event line.
fn event_field(line: &str, field: &str) -> Result<usize, Box<dyn Error>> {
    let prefix = format!("{field}=");
    let value = line
        .split(' ')
        .find_map(|part| part.strip_prefix(prefix.as_str()))
        .ok_or_else(|| format!("{field} missing from {line}"))?;
    Ok(value.parse()?)
}

#[test]
fn a_malformed_quest_document_refuses_boot() {
    for text in [
        "",
        "{",
        "{}",
        r#"{"schema":"OTERYN_QUEST_STATE_LOWERING/v0"}"#,
    ] {
        let refused = load_quest_catalogue(
            |revision| parse_quest_state(text, revision),
            served_revision(),
        );
        assert!(
            matches!(
                refused,
                Err(BootError::ContentActivation("quest state catalogue"))
            ),
            "{text:?} must refuse boot"
        );
    }
}

#[test]
fn the_boot_catalogue_is_the_served_content_revision() -> TestResult {
    let (catalogue, line) = boot_catalogue()?;
    assert_eq!(catalogue.content_revision(), served_revision());
    assert_eq!(catalogue.content_revision(), entry_chest::CONTENT_REVISION);
    assert!(line.starts_with("event=quest_catalogue state=loaded "));
    assert!(line.contains(&format!("content_revision={} ", served_revision())));
    Ok(())
}

#[test]
fn computed_transitions_refuse_and_the_event_line_counts_them() -> TestResult {
    let (catalogue, line) = boot_catalogue()?;
    let transitions = document_transitions()?;
    let document: serde_json::Value = serde_json::from_str(DOCUMENT)?;
    let mut refused = 0;
    let mut explicit = 0;
    let mut inexact_effects = 0;
    for transition in &transitions {
        let key = transition["key"].as_str().ok_or("key")?;
        let loaded = catalogue.transition(key).ok_or(key.to_owned())?;
        let not_supported =
            catalogue.evaluate(loaded, &BTreeMap::new(), 0) == Err(QuestRefusal::NotSupported);
        let computed = effects(transition)?
            .iter()
            .any(|effect| explicit_computed(effect) || inexact(effect));
        assert_eq!(not_supported, computed, "{key}");
        refused += usize::from(not_supported);
        for effect in effects(transition)? {
            explicit += usize::from(explicit_computed(effect));
            inexact_effects += usize::from(inexact(effect));
        }
    }
    assert!(refused > 0 && explicit > 0 && inexact_effects > 0);
    assert_eq!(
        event_field(&line, "quests")?,
        document["quests"].as_array().ok_or("quests")?.len()
    );
    assert_eq!(event_field(&line, "transitions")?, transitions.len());
    assert_eq!(event_field(&line, "not_supported")?, refused);
    assert_eq!(event_field(&line, "not_supported_explicit")?, explicit);
    assert_eq!(
        event_field(&line, "not_supported_inexact")?,
        inexact_effects
    );
    Ok(())
}

#[test]
fn an_exact_transition_of_the_boot_catalogue_applies() -> TestResult {
    let (catalogue, _) = boot_catalogue()?;
    let mut applied = 0;
    for transition in document_transitions()? {
        let key = transition["key"].as_str().ok_or("key")?;
        let loaded = catalogue.transition(key).ok_or(key.to_owned())?;
        if let Ok(changes) = catalogue.evaluate(loaded, &BTreeMap::new(), 0) {
            assert_eq!(changes.len(), loaded.effects.len(), "{key}");
            applied += 1;
        }
    }
    assert!(
        applied > 0,
        "no exact transition applies from the initial values"
    );
    Ok(())
}
