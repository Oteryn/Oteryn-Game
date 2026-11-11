//! M33 drives both retained Feaster portal phases through real STEP dispatch.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

pub(super) const SOURCE: Position = Position {
    x: 32627,
    y: 32076,
    floor: -7,
};
const PROGRAMME: &str = "oteryn:quest-progress/questpc7/feaster/programme";

#[test]
fn m33_feaster_initial_phase_uses_first_portal_and_live_requirements() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-qworld-phase-0")
}
#[test]
fn m33_feaster_purification_phase_uses_second_portal_and_replays_once() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-qworld-phase-5")
}
fn callbacks() -> TestResult<Vec<Callback>> {
    Ok(oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|c| {
            matches!(
                c.key.as_str(),
                "oteryn:quest-callback/questpc7-feaster-entry"
                    | "oteryn:quest-callback/questpc7-feaster-purification-entry"
            )
        })
        .collect())
}
pub(super) fn geometry(mut callback: Callback, name: &str) -> TestResult<Callback> {
    let phase = if name.ends_with("-5") { 5 } else { 0 };
    let selected = callbacks()?
        .into_iter()
        .find(|c| {
            c.storage
                .iter()
                .any(|s| s.track == PROGRAMME && s.minimum == phase)
        })
        .ok_or("phase geometry")?;
    // Supply a walkable destination for this scenario, without a wall passage.
    // Dispatch still loads and selects both unmodified production callbacks.
    callback.selector = selected.selector;
    callback.trigger = selected.trigger;
    callback.teleport = selected.teleport;
    callback.passages.clear();
    callback.storage.clear();
    callback.state_ops.clear();
    callback.lever = None;
    callback.item_on_target = None;
    Ok(callback)
}
pub(super) fn catalogue(original: &QuestStateCatalogue) -> TestResult<QuestStateCatalogue> {
    let rows = callbacks()?;
    let mut tracks = std::collections::BTreeMap::new();
    let mut transitions = Vec::new();
    for callback in &rows {
        transitions.push(
            original
                .transition(&callback.key)
                .ok_or("phase transition")?
                .clone(),
        );
        for storage in &callback.storage {
            tracks.insert(
                storage.track.clone(),
                original.track(&storage.track).ok_or("phase track")?.clone(),
            );
        }
    }
    for phase in [0, 5] {
        transitions.push(QuestTransition {
            key: format!("oteryn:quest-transition/qworld-m33/phase-{phase}"),
            quest: rows[0].quest.clone(),
            effects: vec![QuestEffect {
                track: PROGRAMME.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(phase),
            }],
            completes: false,
            experience: None,
            world_change: None,
        });
    }
    QuestStateCatalogue::new(
        original.content_revision(),
        tracks.into_values().collect(),
        transitions,
    )
    .map_err(debug)
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    direction: StepDirection,
) -> TestResult<Vec<u8>> {
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        *command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    *command += 1;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(reply)
}
async fn arrange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    phase: i64,
    command: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: format!("oteryn:quest-transition/qworld-m33/phase-{phase}"),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(command).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}
async fn value(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    key: &str,
) -> TestResult<i64> {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    Ok(state.tracks().get(key).copied().unwrap_or(-1))
}
async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    expected: Position,
) -> TestResult {
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position(),
        super::quest_world_callbacks::local(expected)
    );
    Ok(())
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    _h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    name: &str,
) -> TestResult {
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_DATABASE_READY M33 {name}");
    let phase = if name.ends_with("-5") { 5 } else { 0 };
    let rows = callbacks()?;
    let callback = rows
        .iter()
        .find(|c| {
            c.storage
                .iter()
                .any(|s| s.track == PROGRAMME && s.minimum == phase)
        })
        .ok_or("selected phase")?;
    assert!(
        !callback.requires_premium,
        "both source phases use the structured admission path"
    );
    assert!(
        callback
            .admission
            .as_ref()
            .is_some_and(|guard| guard.premium)
    );
    assert!(callback.requires_world_admission);
    let flag = callback
        .storage
        .iter()
        .find(|s| !s.preserve)
        .ok_or("entry flag")?
        .track
        .as_str();
    let other_flag = rows
        .iter()
        .find(|c| c.key != callback.key)
        .and_then(|c| c.storage.iter().find(|s| !s.preserve))
        .ok_or("other entry flag")?
        .track
        .as_str();
    let mut command = 1;
    // Missing programme phase must not fall through to another portal.
    step(owner, admitted, &mut command, StepDirection::East).await?;
    at(owner, admitted, SOURCE).await?;
    step(owner, admitted, &mut command, StepDirection::West).await?;
    arrange(owner, admitted, phase, 900).await?;
    if phase == 0 {
        step(owner, admitted, &mut command, StepDirection::East).await?;
        at(owner, admitted, SOURCE).await?;
        assert_eq!(value(owner, admitted, flag).await?, -1);
        step(owner, admitted, &mut command, StepDirection::West).await?;
    }
    super::qengine_outfit_stories::earn_level(owner, admitted, authority, 250).await?;
    let premium = owner.premium_coordinator.take();
    step(owner, admitted, &mut command, StepDirection::East).await?;
    at(owner, admitted, SOURCE).await?;
    assert_eq!(value(owner, admitted, flag).await?, -1);
    step(owner, admitted, &mut command, StepDirection::West).await?;
    owner.premium_coordinator = premium;
    let committed_command = command;
    let committed = step(owner, admitted, &mut command, StepDirection::East).await?;
    at(
        owner,
        admitted,
        callback.teleport.ok_or("phase destination")?,
    )
    .await?;
    assert_eq!(value(owner, admitted, flag).await?, 1);
    assert_eq!(value(owner, admitted, other_flag).await?, -1);
    let replay = super::quest_world_callbacks::wire(
        owner,
        admitted,
        committed_command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(replay, committed);
    at(
        owner,
        admitted,
        callback.teleport.ok_or("phase destination")?,
    )
    .await?;
    assert_eq!(value(owner, admitted, other_flag).await?, -1);
    eprintln!(
        "M33_PHASE_PG_EXECUTED phase={phase} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
