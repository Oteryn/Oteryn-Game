//! Source escape uid35012: an offering and exact conditional flag reset share one receipt.
//! The NPC-acquired flag is seeded through the existing quest writer solely for this
//! composition fixture. This file does not qualify the NPC exchange or physical map placement.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};
use std::sync::Arc;

const KEY: &str = "oteryn:quest-callback/draconia-escape-35012";
const TRACK: &str = "oteryn:quest-progress/dragonfetish";
const SEED: &str = "oteryn:quest-transition/fixture/draconia-flag";

#[test]
fn draconia_escape_missing_offering_recovers_without_progress_or_receipt() -> TestResult {
    scenario("draconia-escape-missing")
}
#[test]
fn draconia_escape_consumes_one_offering_and_retains_absent_flag() -> TestResult {
    scenario("draconia-escape-absent")
}
#[test]
fn draconia_escape_consumes_offering_and_resets_exact_flag_atomically() -> TestResult {
    scenario("draconia-escape-exact")
}
#[test]
fn draconia_escape_consumes_offering_without_rewinding_later_flag() -> TestResult {
    scenario("draconia-escape-later")
}

fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        // Fixture approach tile, not evidence of the donor map's uid placement.
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: 32818,
                y: 31598,
                floor: -9,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) fn authored_callback() -> TestResult<Callback> {
    let document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/quest-authoring/quests-2/world-callbacks/draconia_quest.json"
    )))?;
    let callback = document["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|c| c["key"].as_str() == Some(KEY))
        .ok_or("authored escape")?
        .clone();
    Ok(serde_json::from_value(callback)?)
}

pub(super) fn catalogue(callback: &Callback, name: &str) -> TestResult<Arc<QuestStateCatalogue>> {
    let seed = if name.ends_with("exact") { 1 } else { 2 };
    Ok(Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            vec![QuestTrack {
                key: TRACK.into(),
                quest: callback.quest.clone(),
                initial: -1,
                min: -1,
                max: i64::MAX,
            }],
            vec![
                oteryn_gameplay_quests::quest::world_callbacks::primary_transition(callback)?,
                QuestTransition {
                    world_change: None,
                    key: SEED.into(),
                    quest: callback.quest.clone(),
                    completes: false,
                    experience: None,
                    effects: vec![QuestEffect {
                        track: TRACK.into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(seed),
                    }],
                },
            ],
        )
        .map_err(debug)?,
    ))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    name: &str,
) -> TestResult {
    let initial = if name.ends_with("exact") {
        1
    } else if name.ends_with("later") {
        2
    } else {
        -1
    };
    if initial != -1 {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: SEED.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(900).map_err(debug)?,
                    )),
                },
                owner.quest_catalogue.clone().ok_or("catalogue")?,
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(result, QuestTransitionOutcome::Committed(_)),
            "{result:?}"
        );
    }
    if !name.ends_with("missing") {
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &callback.offerings[0],
            400,
            false,
        )
        .await?;
    }
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_CASE_STARTED {name} server_version={version}");
    let payload = encode_step_intent(StepDirection::East);
    let reply = wire(owner, admitted, 1, 1, payload.clone()).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    let after = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        after.position(),
        local(if name.ends_with("missing") {
            Position {
                x: 32818,
                y: 31599,
                floor: -9,
            }
        } else {
            Position {
                x: 32701,
                y: 31639,
                floor: -6,
            }
        })
    );
    assert_eq!(wire(owner, admitted, 1, 1, payload).await?, reply);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        after
    );
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    if name.ends_with("missing") {
        assert_eq!(state, before);
    } else {
        assert_eq!(
            state.tracks().get(TRACK),
            Some(&(if initial == 1 { 0 } else { initial }))
        );
    }
    let burned: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(burned, i64::from(!name.ends_with("missing")));
    let after_receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        after_receipts,
        receipts + i64::from(!name.ends_with("missing"))
    );
    Ok(())
}
