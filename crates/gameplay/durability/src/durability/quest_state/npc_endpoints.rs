//! Finite final-outfit branches reuse the admitted Quest owner's evaluator.
use super::DurabilityError;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrackChange};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Bindings {
    rows: Vec<Binding>,
}
#[derive(Deserialize)]
struct Binding {
    source_key: String,
    matches: Vec<TrackBinding>,
}
#[derive(Deserialize)]
struct TrackBinding {
    binding: TrackIdentity,
    existing_transitions: Vec<TransitionIdentity>,
}
#[derive(Deserialize)]
struct TrackIdentity {
    key: String,
    quest: String,
}
#[derive(Deserialize)]
struct TransitionIdentity {
    key: String,
}

pub(in crate::durability) struct PreparedEndpoint {
    pub transition_key: String,
    pub quest: String,
    pub definition_hash: [u8; 32],
    pub completes: bool,
    pub changes: Vec<QuestTrackChange>,
    /// Explicit absent-Lua versus declared-Quest initial-value projections.
    pub absent_initial_projections: Vec<(String, i64)>,
}
static BINDINGS: std::sync::OnceLock<Result<Bindings, serde_json::Error>> =
    std::sync::OnceLock::new();
#[derive(Clone, Copy)]
pub(in crate::durability) struct QualifiedBindings(&'static Bindings);
/// Parse immutable content before opening the Root transaction.
pub(in crate::durability) fn preload() -> Result<QualifiedBindings, DurabilityError> {
    BINDINGS.get_or_init(||serde_json::from_str(include_str!("../../../../../../tools/content-schema/native-gameplay/npc-quest-endpoint-bindings.json")))
        .as_ref().map(QualifiedBindings).map_err(|_|DurabilityError::InvalidStoredState)
}

/// `source_changes` is the exact selected Source NPC plan, not client input.
/// Unbound endpoints stay refused until an explicit admitted Quest addition exists.
pub(in crate::durability) fn evaluate_final_endpoints(
    source_key: &str,
    source_changes: &[oteryn_gameplay_quests::quest::npc_cosmetics::TrackChange],
    stored: &BTreeMap<String, i64>,
    now: i64,
    catalogue: &QuestStateCatalogue,
    native_keys: &std::collections::BTreeSet<String>,
    qualified: &QualifiedBindings,
) -> Result<Vec<PreparedEndpoint>, DurabilityError> {
    let bindings = qualified.0;
    let binding = bindings
        .rows
        .iter()
        .find(|row| row.source_key == source_key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if source_changes.len() != binding.matches.len()
        || source_changes.iter().any(|change| {
            !binding
                .matches
                .iter()
                .any(|row| row.binding.key == change.key && row.binding.quest == change.quest)
        })
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mut expected_keys = std::collections::BTreeSet::new();
    for row in &binding.matches {
        if native_keys.contains(&row.binding.key) {
            continue;
        }
        if !expected_keys.insert((row.binding.quest.as_str(), row.binding.key.as_str())) {
            return Err(DurabilityError::InvalidStoredState);
        }
        if source_changes
            .iter()
            .filter(|change| change.quest == row.binding.quest && change.key == row.binding.key)
            .count()
            != 1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let mut covered = std::collections::BTreeSet::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut output = Vec::new();
    for track_binding in &binding.matches {
        if native_keys.contains(&track_binding.binding.key) {
            continue;
        }
        let expected = source_changes
            .iter()
            .find(|change| {
                change.key == track_binding.binding.key
                    && change.quest == track_binding.binding.quest
            })
            .ok_or(DurabilityError::InvalidStoredState)?;
        // Source helpers such as Dedoras max(Questlog,1) can be satisfied by
        // the unchanged, actually stored mature-owner value. Re-prove that
        // owner/value instead of manufacturing a redundant Quest transition.
        if expected.before == expected.after
            && stored.get(&expected.key) == Some(&expected.before)
            && catalogue
                .track(&expected.key)
                .is_some_and(|track| track.quest == expected.quest)
        {
            if !covered.insert((expected.quest.as_str(), expected.key.as_str())) {
                return Err(DurabilityError::InvalidStoredState);
            }
            continue;
        }
        if track_binding.existing_transitions.len() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let key = &track_binding.existing_transitions[0].key;
        if !seen.insert(key.clone()) {
            continue;
        }
        let transition = catalogue
            .transition(key)
            .filter(|transition| {
                transition.quest == expected.quest && transition.experience.is_none()
            })
            .ok_or(DurabilityError::InvalidStoredState)?;
        let changes = catalogue
            .evaluate(transition, stored, now)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let mut absent_initial_projections = Vec::new();
        for change in &changes {
            let expected = source_changes
                .iter()
                .find(|expected| expected.key == change.track && expected.quest == transition.quest)
                .ok_or(DurabilityError::InvalidStoredState)?;
            if !expected_keys.contains(&(expected.quest.as_str(), expected.key.as_str()))
                || expected.after != change.after
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            match stored.get(&change.track) {
                Some(actual) if *actual == expected.before && *actual == change.before => {}
                None if expected.before == -1 => {
                    let track = catalogue
                        .track(&change.track)
                        .filter(|track| {
                            track.quest == expected.quest && track.initial == change.before
                        })
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    absent_initial_projections.push((track.key.clone(), track.initial));
                }
                _ => return Err(DurabilityError::InvalidStoredState),
            }
            if !covered.insert((expected.quest.as_str(), expected.key.as_str())) {
                // Two admitted transitions cannot claim the same source write.
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        output.push(PreparedEndpoint {
            transition_key: key.clone(),
            quest: transition.quest.clone(),
            definition_hash: catalogue
                .definition_hash(&transition.quest)
                .ok_or(DurabilityError::InvalidStoredState)?,
            completes: transition.completes,
            changes,
            absent_initial_projections,
        });
    }
    if covered != expected_keys {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(output)
}

/// A source NPC parent owns one Character revision. Its Quest child is one
/// immutable owner write, even when several admitted source transitions supply
/// that owner's effects. Ordinary Quest receipt uniqueness stays unchanged.
pub(in crate::durability) struct PreparedQuestChild {
    pub quest: String,
    pub definition_hash: [u8; 32],
    pub transition_keys: Vec<String>,
    pub completes: bool,
    pub changes: Vec<QuestTrackChange>,
    pub absent_initial_projections: Vec<(String, i64)>,
}
pub(in crate::durability) fn group_final_endpoints(
    endpoints: Vec<PreparedEndpoint>,
) -> Result<Vec<PreparedQuestChild>, DurabilityError> {
    let mut grouped: BTreeMap<String, PreparedQuestChild> = BTreeMap::new();
    for endpoint in endpoints {
        let child = grouped
            .entry(endpoint.quest.clone())
            .or_insert_with(|| PreparedQuestChild {
                quest: endpoint.quest.clone(),
                definition_hash: endpoint.definition_hash,
                transition_keys: Vec::new(),
                completes: false,
                changes: Vec::new(),
                absent_initial_projections: Vec::new(),
            });
        if child.definition_hash != endpoint.definition_hash
            || child.transition_keys.contains(&endpoint.transition_key)
            || endpoint.changes.iter().any(|change| {
                child
                    .changes
                    .iter()
                    .any(|existing| existing.track == change.track)
            })
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        child.transition_keys.push(endpoint.transition_key);
        child.completes |= endpoint.completes;
        child.changes.extend(endpoint.changes);
        child
            .absent_initial_projections
            .extend(endpoint.absent_initial_projections);
        if child.changes.len() > 8 || child.transition_keys.len() > 8 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(grouped.into_values().collect())
}

/// Decide whether the mature exact transition is executable for this locked
/// before-state. Supplemental handling still requires an explicit native source
/// operation; a refusal never turns into a general fallback track writer.
pub(in crate::durability) fn requires_native(
    source_key: &str,
    key: &str,
    catalogue: &QuestStateCatalogue,
    stored: &BTreeMap<String, i64>,
    now: i64,
    qualified: &QualifiedBindings,
) -> Result<bool, DurabilityError> {
    let bindings = qualified.0;
    let row = bindings
        .rows
        .iter()
        .find(|row| row.source_key == source_key)
        .and_then(|row| row.matches.iter().find(|row| row.binding.key == key))
        .ok_or(DurabilityError::InvalidStoredState)?;
    if row.existing_transitions.len() != 1 {
        return Ok(true);
    }
    let transition = catalogue
        .transition(&row.existing_transitions[0].key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(catalogue.evaluate(transition, stored, now).is_err())
}
