//! Only the other quests' prerequisites are arranged; Tome counters and books are not.

use super::*;

use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};

use std::collections::BTreeMap;

pub(super) async fn install(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    stage_ready: bool,
) -> TestResult {
    let canonical = owner.quest_catalogue.clone().ok_or("catalogue")?;
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::offered_for(&canonical)?;
    let mut tracks = BTreeMap::new();
    let mut transitions = BTreeMap::new();
    for callback in callbacks {
        let mut keys = vec![callback.key.clone()];
        keys.extend(callback.participants.iter().enumerate().map(|(n, _)| {
            oteryn_gameplay_quests::quest::reward_callbacks::participant_key(&callback.key, n)
        }));
        for key in keys {
            let t = canonical
                .transition(&key)
                .cloned()
                .ok_or_else(|| format!("source reward transition {key}"))?;
            for key in t.effects.iter().map(|e| e.track.as_str()).chain(
                callback
                    .requires
                    .iter()
                    .chain(callback.peer_requires.iter())
                    .map(|r| r.track.as_str()),
            ) {
                tracks.insert(
                    key.to_owned(),
                    canonical.track(key).cloned().ok_or("source track")?,
                );
            }
            transitions.insert(t.key.clone(), t);
        }
    }
    for callback in oteryn_gameplay_quests::quest::world_callbacks::load()? {
        if let Some(t) = canonical.transition(&callback.key) {
            for e in &t.effects {
                tracks.insert(
                    e.track.clone(),
                    canonical.track(&e.track).cloned().ok_or("world track")?,
                );
            }
            transitions.insert(t.key.clone(), t.clone());
        }
    }
    if stage_ready {
        // The native Curos and Zalamon routes own these tracks. Start this Tome
        // fixture at their reward stages and drive each reward through its owner.
        for (key, value) in [
            ("oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/questline", 4),
            ("oteryn:quest-progress/quest/u8_54/children_of_the_revolution/questline", 17),
            ("oteryn:quest-progress/quest/u8_54/children_of_the_revolution/mission04", 5),
        ] {
            let mut track = canonical.track(key).cloned().ok_or("foreign prerequisite")?;
            track.initial = value;
            tracks.insert(key.into(), track);
        }
    }
    for (key, quest, values) in [
        (
            "frontier-ready",
            "the_new_frontier_quest",
            vec![(format!("{FRONTIER}mission08"), 2)],
        ),
        (
            "frontier-finished",
            "the_new_frontier_quest",
            vec![(format!("{FRONTIER}mission10_1"), 2)],
        ),
    ] {
        for (track, _) in &values {
            tracks.insert(
                track.clone(),
                canonical
                    .track(track)
                    .cloned()
                    .ok_or("foreign prerequisite")?,
            );
        }
        let t = QuestTransition {
            world_change: None,
            key: format!("oteryn:quest-transition/m39-tomes-fixture/{key}"),
            quest: format!("oteryn:quest.{quest}"),
            effects: values
                .into_iter()
                .map(|(track, value)| QuestEffect {
                    track,
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
            completes: false,
            experience: None,
        };
        transitions.insert(t.key.clone(), t);
    }
    owner.quest_catalogue = Some(std::sync::Arc::new(
        QuestStateCatalogue::new(
            canonical.content_revision(),
            tracks.into_values().collect(),
            transitions.into_values().collect(),
        )
        .map_err(debug)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

pub(super) async fn apply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
    n: u64,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: format!("oteryn:quest-transition/m39-tomes-fixture/{key}"),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(n).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
