//! Real STEP transport over an authored stateless callback and the existing owners.
//! The source callback must not invoke the durable Quest writer. A control track is
//! first written through that writer, then compared after the real movement command.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CharacterId, CharacterLease, CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::world_callbacks::Trigger;
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    expected_destination: Position,
) -> TestResult {
    assert_eq!(callback.trigger, Trigger::StepIn);
    assert_eq!(
        callback
            .teleport
            .unwrap_or(callback.selector.position.ok_or("position")?),
        expected_destination
    );
    assert!(callback.storage.is_empty());
    assert!(callback.offerings.is_empty());
    assert!(callback.passages.is_empty());
    assert!(callback.lever.is_none());
    assert!(callback.item_on_target.is_none());
    // The scenario replaces its runtime; bind its current Character lease just as
    // the existing Rotten Blood real-dispatch fixture does before owner operations.
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;

    // Same baseline seed used by the existing liquid-black scenario. No quest
    // progress is inserted by SQL, and no new per-quest file is a helper dependency.
    let catalogue = owner.quest_catalogue.clone().ok_or("quest catalogue")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let seeded = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key:
                    "oteryn:quest-transition/quest/u9_4/liquid_black_quest/visitor/npc_1".into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(901).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    drop(slot);
    assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert!(
        !before.tracks().is_empty(),
        "non-empty durable control state"
    );
    let receipts_before = receipt_count(h, fence).await?;

    let reply = wire(
        owner,
        admitted,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(
            if callback.key == "oteryn:quest-callback/l6-native-step-fixture" {
                StepDirection::South
            } else {
                StepDirection::East
            },
        ),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(expected_destination),
        "authored stateless STEP destination for {}",
        callback.key
    );
    // Crystal201189f movements_waterfall.lua:16-29. Expectations are literal
    // donor values, independent of the authored callback's effect fields.
    let snapshot = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    let expected_facing = match callback.key.as_str() {
        "oteryn:quest-callback/waterfall-cave-entry" => {
            Some(crate::foundation::MovementFacing::South)
        }
        "oteryn:quest-callback/waterfall-cave-exit" => {
            Some(crate::foundation::MovementFacing::East)
        }
        _ => None,
    };
    if let Some(facing) = expected_facing {
        assert_eq!(snapshot.facing(), Some(facing), "source facing");
    }
    let expected_effect = match callback.key.as_str() {
        "oteryn:quest-callback/waterfall-cliff" => Some((32968, 32626, -5)),
        "oteryn:quest-callback/waterfall-splash" => Some((32968, 32630, -7)),
        _ => None,
    };
    if let Some((x, y, floor)) = expected_effect {
        let map = owner.bundle_map().ok_or("map")?;
        let effects = map.facts.movement_effects_since(0);
        assert_eq!(effects.len(), 1, "source watersplash occurrence");
        assert_eq!(
            (effects[0].1, effects[0].2),
            (crate::map::overlay::TilePos { x, y, floor }, 54)
        );
    }
    if callback.key.starts_with("oteryn:quest-callback/waterfall-") {
        let map = owner.bundle_map().ok_or("map")?;
        let effects = map.facts.movement_effects_since(0);
        let replay = wire(
            owner,
            admitted,
            1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(
                if callback.key == "oteryn:quest-callback/l6-native-step-fixture" {
                    StepDirection::South
                } else {
                    StepDirection::East
                },
            ),
        )
        .await?;
        assert_eq!(
            decode_step_result(&replay).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?,
            snapshot,
            "replayed STEP moved or turned twice"
        );
        assert_eq!(
            map.facts.movement_effects_since(0),
            effects,
            "replayed STEP repeated effects"
        );
    }
    let after = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        after.tracks(),
        before.tracks(),
        "stateless STEP changed quest tracks"
    );
    assert_eq!(receipt_count(h, fence).await?, receipts_before);
    assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
    eprintln!(
        "QUESTS3_STATELESS_POSTGRES_CASE_COMPLETED {} destination_verified tracks_unchanged receipts_unchanged",
        callback.key
    );
    Ok(())
}

async fn receipt_count(h: &Harness, fence: CurrentCharacterGameplayFence) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts \
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}
