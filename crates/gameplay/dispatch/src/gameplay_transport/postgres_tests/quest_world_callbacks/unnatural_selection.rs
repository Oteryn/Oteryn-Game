//! Focused durable dance proof: final source STEP, achievement, exact replay and fences.
//!
//! This uses the existing synthetic callback test map only to exercise the owner seam.
//! It does not qualify production map collision. Initial DanceStatus comes from its
//! existing scalar NPC transition; all sixteen dance steps use the wire dispatcher.
//! No quest rows, equipment edges or achievements are seeded by SQL.
use super::*;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::foundation::{CommandId, CommandRef, ConnectionGeneration};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, unnatural_selection as dance};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use std::sync::Arc;

const ACHIEVEMENT: &str = "oteryn:achievement/talented_dancer";

#[test]
fn unnatural_dance_postgres_full_wire_route_achievement_replay_stale_and_native_gate() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("unnatural-dance", || {
        super::scenario(
            "unnatural-selection-dance-2",
            Position {
                x: 32991,
                y: 31496,
                floor: -1,
            },
            false,
        )
    })
}

fn request(
    session: crate::foundation::GameSessionId,
    key: &str,
    ordinal: u64,
) -> TestResult<QuestTransitionRequest> {
    Ok(QuestTransitionRequest {
        transition_key: key.into(),
        cause: QuestCause::Step(CommandRef::new(
            session,
            CommandId::new(ordinal).map_err(debug)?,
        )),
    })
}

async fn context(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<QuestWorldContext> {
    let runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("dance actor")?;
    Ok(QuestWorldContext {
        map_revision: crate::map::overlay::map_revision(
            owner.bundle_map().ok_or("map")?.overlay.base(),
        ),
        content_revision: format!(
            "sha256:{}",
            runtime
                .content_pin()
                .server_artifact_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
        reset_epoch: owner.reset_epoch,
        actor_placement: actor.placement_identity(),
        movement_revision: runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .revision(),
    })
}

/// One sibling hook in the existing callback fixture, after quest-session admission.
#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("dance actor")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Dancer", owner.owner_now());
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone();
    let original = catalogue
        .transition(&callback.key)
        .ok_or("dance transition")?;

    // RED target: a renamed Native transition must not enter through the generic
    // writer merely because its key is absent from world_callbacks::load().
    let mut renamed = original.clone();
    renamed.key = "oteryn:quest-transition/unnatural-dance-renamed-native-regression".into();
    let renamed_key = renamed.key.clone();
    let tracks = dance::TRACKS
        .into_iter()
        .map(|key| catalogue.track(key).cloned().ok_or("canonical dance track"))
        .collect::<Result<Vec<_>, _>>()?;
    let renamed_catalogue = Arc::new(
        QuestStateCatalogue::new(catalogue.content_revision(), tracks, vec![renamed])
            .map_err(debug)?,
    );
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                request(admitted.game_session_id, &renamed_key, 900)?,
                renamed_catalogue,
            )
            .await;
        assert!(
            matches!(result, Err(CharacterProgressionError::InvalidInput)),
            "renamed Native requires the world owner before evaluating any stage: {result:?}"
        );
        let result = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                request(admitted.game_session_id, &callback.key, 901)?,
                catalogue.clone(),
            )
            .await;
        assert!(
            matches!(result, Err(CharacterProgressionError::InvalidInput)),
            "known Native callback must also reject the generic caller: {result:?}"
        );
    }
    assert_eq!(counts(h).await?, (0, 0, 0));

    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let start = QuestTransitionRequest {
            transition_key:
                "oteryn:quest-transition/quest/u8_54/unnatural_selection/dance_status/npc_1".into(),
            cause: QuestCause::Command(CommandRef::new(
                admitted.game_session_id,
                CommandId::new(950).map_err(debug)?,
            )),
        };
        assert!(matches!(
            slot.commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                start,
                catalogue.clone()
            )
            .await
            .map_err(debug)?,
            QuestTransitionOutcome::Committed(_)
        ));
    }
    let initial_revision = context(owner, admitted).await?.movement_revision;
    let mut diagonal_steps = 0;
    for (index, tile) in dance::SEQUENCE.into_iter().take(15).enumerate() {
        let (x, y, floor) = dance::TILES[usize::from(tile) - 1];
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let direction = match (i32::from(x) - at.x, i32::from(y) - at.y) {
            (0, -1) => StepDirection::North,
            (1, 0) => StepDirection::East,
            (0, 1) => StepDirection::South,
            (-1, 0) => StepDirection::West,
            (1, -1) => StepDirection::NorthEast,
            (1, 1) => StepDirection::SouthEast,
            (-1, 1) => StepDirection::SouthWest,
            (-1, -1) => StepDirection::NorthWest,
            _ => return Err("nonadjacent dance step".into()),
        };
        if matches!(
            direction,
            StepDirection::NorthEast
                | StepDirection::SouthEast
                | StepDirection::SouthWest
                | StepDirection::NorthWest
        ) {
            diagonal_steps += 1;
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let ordinal = u64::try_from(index)? + 1;
        let reply = wire(
            owner,
            admitted,
            ordinal,
            1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved,
            "dance step {index}"
        );
        let position = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(position.position(), local(Position { x, y, floor }));
        assert_eq!(position.revision(), initial_revision + ordinal);
        let state = owner
            .root
            .read_character_quest_state(authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(
            state.tracks().get(dance::DANCE_STATUS_TRACK),
            Some(&(i64::try_from(index)? + 2))
        );
        assert_eq!(counts(h).await?, (i64::try_from(index)? + 1, 0, 0));
    }
    assert_eq!(diagonal_steps, 12);
    let setup_context = context(owner, admitted).await?;
    let state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(dance::DANCE_STATUS_TRACK), Some(&16));
    assert_eq!(state.tracks().get(dance::MISSION03_TRACK), Some(&0));
    assert_eq!(state.tracks().get(dance::QUESTLINE_TRACK), Some(&0));
    assert_eq!(counts(h).await?, (15, 0, 0));

    // Both stale lease and stale connection generation must fail while the final
    // stage is eligible, without a new receipt, achievement or partial track write.
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current fence")?
        .ok_or("current fence")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    for connection in [false, true] {
        let mut stale = current;
        if connection {
            stale.connection_generation = ConnectionGeneration::new(
                current
                    .connection_generation
                    .get()
                    .checked_add(1)
                    .ok_or("generation")?,
            )
            .map_err(debug)?;
        } else {
            stale.character_lease_generation += 1;
        }
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                stale,
                request(
                    admitted.game_session_id,
                    &callback.key,
                    2000 + u64::from(connection),
                )?,
                catalogue.clone(),
                &permit,
                setup_context.clone(),
                None,
            )
            .await;
        assert!(
            matches!(result, Err(CharacterProgressionError::AuthorityRejected)),
            "stale generation={connection}: {result:?}"
        );
    }
    drop(permit);
    assert_eq!(counts(h).await?, (15, 0, 0));
    let revision_before: String = sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;

    // Complete the source route from tile5 to tile2 through the same wire dispatcher.
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North);
    let reply = wire(owner, admitted, 16, 1, payload.clone()).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
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
        local(trigger)
    );
    assert_eq!(
        context(owner, admitted).await?.movement_revision,
        initial_revision + 16
    );
    let state = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    for (key, value) in [
        (dance::DANCE_STATUS_TRACK, 17),
        (dance::MISSION03_TRACK, 3),
        (dance::QUESTLINE_TRACK, 7),
    ] {
        assert_eq!(state.tracks().get(key), Some(&value));
    }
    assert_eq!(counts(h).await?, (16, 1, 1));
    let revision_after: String = sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        revision_after.parse::<u64>()?,
        revision_before.parse::<u64>()? + 1
    );
    assert_eq!(owner.states.chat.drain(admitted.game_session_id).iter().filter(|line|
        matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == "Krunus should be pleased.")).count(), 1);

    let (kind, ordinal): (String, String) = sqlx::query_as(
        "SELECT cause_kind,cause_ordinal::text FROM game_character_quest_receipts \
         WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2 \
         ORDER BY committed_character_revision DESC LIMIT 1",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&callback.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(kind, "step");
    let ordinal = ordinal.parse::<u64>()?;
    let original_receipt = final_receipt(h, &callback.key).await?;
    let mut event = fence.character_id.as_bytes().to_vec();
    event.extend_from_slice(admitted.game_session_id.as_bytes());
    event.extend_from_slice(&ordinal.to_be_bytes());
    let grant: (String, Vec<u8>) = sqlx::query_as(
        "SELECT source_kind,source_event_id FROM game_account_achievement_grant_requests WHERE achievement_key=$1",
    ).bind(ACHIEVEMENT).fetch_one(&h.pool).await?;
    assert_eq!(grant, ("oteryn:quest-unnatural-dance".into(), event));

    let replay_context = context(owner, admitted).await?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let replay = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                current,
                request(admitted.game_session_id, &callback.key, ordinal)?,
                catalogue,
                &permit,
                replay_context,
                None,
            )
            .await
            .map_err(debug)?;
        let QuestTransitionOutcome::AlreadyCommitted(receipt) = replay else {
            return Err(format!("exact final replay: {replay:?}").into());
        };
        assert!(
            receipt
                .changes
                .iter()
                .any(|change| change.track == dance::DANCE_STATUS_TRACK
                    && change.before == 16
                    && change.after == 17)
        );
    }
    drop(permit);
    assert_eq!(final_receipt(h, &callback.key).await?, original_receipt);
    assert_eq!(counts(h).await?, (16, 1, 1));
    // Transport replay also preserves the receipt and must not repeat finish text.
    let _ = wire(owner, admitted, 16, 1, payload).await?;
    assert_eq!(final_receipt(h, &callback.key).await?, original_receipt);
    assert_eq!(counts(h).await?, (16, 1, 1));
    assert!(!owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
        matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == "Krunus should be pleased.")));
    eprintln!(
        "UNNATURAL_DANCE POSTGRES_CASE_COMPLETED wire_steps=16 diagonal_steps=12 final16to17 achievement_atomic replay_exact stale_lease stale_connection renamed_native_generic_refused"
    );
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-callback/unnatural-selection-dance-%'), \
         (SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key=$1), \
         (SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1)",
    ).bind(ACHIEVEMENT).fetch_one(&h.pool).await?)
}

async fn final_receipt(h: &Harness, key: &str) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar(
        "SELECT to_jsonb(r) FROM game_character_quest_receipts r WHERE transition_key=$1 \
         ORDER BY committed_character_revision DESC LIMIT 1",
    )
    .bind(key)
    .fetch_one(&h.pool)
    .await?)
}
