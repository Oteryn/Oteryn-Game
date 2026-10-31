//! Real admitted Instance death, existing one-item custody, proven return and retirement.
//! Fixture coordinates/content are explicitly authored test data, never gameplay defaults.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult};
use crate::domain::CharacterRevision;
use crate::durability::activity_handoff::{
    ActivityEnterOutcome, ActivityHandoffOwner, ActivityRefusal, ActivityReturnOutcome,
    ActivityReturnReason, ActivityReturnRequest,
};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::character_last_position::{
    FinalLastPosition, LastPositionCell, LastPositionRecord,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::item_decay_retire::ActivityGroundRetirement;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass, ItemTransferDestination,
    ItemTransferError, ItemTransferRequest, ItemWeightLookup, ItemWeightRefused,
};
use crate::durability::player_death_items::{PlayerDeathDropCause, PlayerDeathItemRequest};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, AssignmentState, ControlActor,
    OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::durability::spell_owner_commit::SpellLane;
use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelContentPin, ChannelRuntimeV1, CommandId, CommandRef,
    GameSessionId, MovementLocalPosition, RuntimeScopeRefV1,
};
use crate::gameplay_transport::death_ml_loss_postgres::death as death_request;
use crate::gameplay_transport::item_fence_of;
use crate::spell::mana_training::CompiledTrainingFormula;

#[path = "activity_player_death/departed_process.rs"]
mod departed_process;

#[derive(Clone, Copy)]
enum Case {
    RedSkullDrop,
    Amulet,
    InterruptedDrop,
    DepartedProcessDrop,
}

#[test]
fn activity_player_death_postgres_death_receipt_loss_private_ground_isolation_return_and_retirement()
-> TestResult {
    run_case(Case::RedSkullDrop)
}

#[test]
fn activity_player_death_postgres_amulet_burn_replay_and_pending_obligation() -> TestResult {
    run_case(Case::Amulet)
}

#[test]
fn activity_player_death_postgres_partial_loss_resumes_only_after_proven_owner_loss() -> TestResult
{
    run_case(Case::InterruptedDrop)
}

#[test]
fn activity_player_death_postgres_partial_loss_recovers_through_empty_origin_lifecycle() -> TestResult {
    run_case(Case::DepartedProcessDrop)
}

fn run_case(case: Case) -> TestResult {
    let Some(admin) = crate::bestiary_postgres_harness::configured_admin() else {
        eprintln!("ACTIVITY_PLAYER_DEATH POSTGRES_CASE_SKIPPED: no configured admin");
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let runtime =
                crate::bestiary_postgres_harness::runtime().map_err(|error| error.to_string())?;
            let h = runtime
                .block_on(Harness::create_with_admission_seed(
                    admin,
                    "activity_player_death",
                    true,
                    false,
                ))
                .map_err(|error| error.to_string())?;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(scenario(&h, case))
            }));
            let cleanup = runtime
                .block_on(h.cleanup())
                .map_err(|error| error.to_string());
            match result {
                Ok(result) => {
                    result.map_err(|error| error.to_string())?;
                    cleanup
                }
                Err(panic) => std::panic::resume_unwind(panic),
            }
        })?
        .join()
        .map_err(|_| "Instance player-death test panicked")??;
    Ok(())
}

fn gameplay(
    item: CurrentCharacterItemFence,
    revision: CharacterRevision,
) -> CurrentCharacterGameplayFence {
    CurrentCharacterGameplayFence {
        character_id: item.character_id,
        game_session_id: item.game_session_id,
        connection_generation: item.connection_generation,
        character_lease_generation: item.character_lease_generation,
        runtime_scope: item.runtime_scope,
        scope_ownership_generation: item.scope_ownership_generation,
        expected_character_revision: revision,
    }
}

fn cell_bytes(cell: LastPositionCell) -> Vec<u8> {
    [
        cell.x.to_be_bytes().as_slice(),
        cell.y.to_be_bytes().as_slice(),
        cell.floor.to_be_bytes().as_slice(),
    ]
    .concat()
}

async fn scenario(h: &Harness, case: Case) -> TestResult {
    // Use the shared real admission/control fixture, choosing vocation while still
    // in its current Channel. No fabricated Instance session, assignment or receipt.
    let (_evidence, enter) = super::activity_runtime::prepare_activity_fixture(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let activity = ActivityHandoffOwner::new(h.root.clone());
    let formula = CompiledTrainingFormula::from_profile(
        include_bytes!(
            "../../../../../../tools/content-schema/native-gameplay/build-training.json"
        ),
        "content-1",
    )
    .map_err(debug)?;
    let before = DurableBuildState::default();
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            gameplay(
                enter.participants[0].source,
                CharacterRevision::new(1).map_err(debug)?,
            ),
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(100)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                after: convert_vocation(&formula, &before, "druid").map_err(debug)?,
                before,
                pruned_stance: None,
            },
            &formula,
        )
        .await
        .map_err(debug)?;
    seed_equipment(h, case).await?;
    let entered = match activity.enter(&authority, &h.node, enter.clone()).await? {
        ActivityEnterOutcome::Entered(receipt) => receipt,
        outcome => return Err(format!("real Instance admission refused: {outcome:?}").into()),
    };
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let current = item_fence_of(
        store
            .current_session_at(enter.participants[0].destination_game_session_id)
            .await?
            .0,
    )
    .ok_or("current Instance fence")?;
    assert_eq!(current.runtime_scope, enter.destination_scope);
    let RuntimeScopeRefV1::Instance { instance_id, .. } = current.runtime_scope else {
        return Err("actual Instance required".into());
    };
    let fence = gameplay(current, CharacterRevision::new(2).map_err(debug)?);
    let mut request = death_request(101)?;
    request.death_cell.runtime_scope = current.runtime_scope;
    request.death_cell.spatial_position = cell_bytes(enter.participants[0].destination_cell);
    request.death_cell.map_revision = enter
        .destination_content
        .map_revision_digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    // This fixture explicitly authors its accepted home temple at its origin start.
    // It differs from both the death tile and the normal lever exit.
    let temple = enter.participants[0].source_position.cell;
    request.respawn_position = cell_bytes(temple);
    let sequencer = CharacterRevisionSequencer::new();
    let death = h
        .root
        .settle_runtime_player_death_with_items(
            &sequencer,
            &authority,
            &h.node,
            fence,
            request.clone(),
            None,
            None,
            Some(enter.destination_content.server_artifact_digest),
        )
        .await
        .map_err(debug)?;
    let post_loss = gameplay(current, death.committed_character_revision);
    assert_eq!(death.death_cell.runtime_scope, current.runtime_scope);
    assert_eq!(
        death.death_cell.spatial_position,
        request.death_cell.spatial_position
    );
    let stored: (i16, Option<Vec<u8>>, Vec<u8>, String) = sqlx::query_as(
        "SELECT death_scope_kind,uuid_send(death_channel_id),uuid_send(death_instance_id),death_scope_generation::text FROM game_character_death_receipts WHERE death_occurrence_id=encode($1,'hex')::uuid")
        .bind(request.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        stored,
        (
            2,
            None,
            instance_id.to_vec(),
            current.scope_ownership_generation.get().to_string()
        )
    );
    let selection = death
        .items
        .as_ref()
        .ok_or("immutable death item selection")?;
    match case {
        Case::RedSkullDrop | Case::InterruptedDrop | Case::DepartedProcessDrop => {
            assert_eq!(selection.lost.len(), 2);
            assert!(selection.lost.contains(&id(70)));
            assert!(selection.lost.contains(&id(75)));
            assert_eq!(selection.amulet, None);
        }
        Case::Amulet => {
            assert!(selection.lost.is_empty());
            assert_eq!(selection.amulet, Some(id(75)));
        }
    }
    let pending = h
        .root
        .pending_respawn(&authority, &h.node, post_loss)
        .await
        .map_err(debug)?
        .ok_or("pending Instance respawn")?;
    assert_eq!(pending.respawn_position, request.respawn_position);
    assert!(
        h.root
            .consume_pending_respawn(&authority, &h.node, post_loss, Some(request.occurrence))
            .await
            .is_err(),
        "an Instance cannot consume its own pending home-temple respawn"
    );
    assert!(
        activity
            .read_death_return_placement(
                &authority,
                &h.node,
                current,
                *request.occurrence.as_bytes()
            )
            .await?
            .is_none(),
        "the selected item operations must finish before the death return qualifies"
    );
    let return_placement = activity
        .read_return_placement(&authority, &h.node, current)
        .await?
        .ok_or("retained origin metadata")?;
    let mut returning = ActivityReturnRequest {
        return_id: id(104),
        handoff_id: entered.handoff_id,
        command_id: None,
        source: current,
        destination_game_session_id: GameSessionId::decode(&id(105)).map_err(debug)?,
        destination_transport: Some(
            AuthenticatedTransportRefV1::decode(&[105; 16]).map_err(debug)?,
        ),
        destination_generation: return_placement.destination_generation,
        reason: ActivityReturnReason::Death {
            occurrence: *request.occurrence.as_bytes(),
        },
        current_vitals: None,
        current_rest: None,
    };
    assert!(matches!(
        activity
            .return_participant(&authority, &h.node, returning.clone())
            .await?,
        ActivityReturnOutcome::Refused(_)
    ));
    let mut normal_exit = returning.clone();
    normal_exit.reason = ActivityReturnReason::Exit;
    normal_exit.command_id = Some(12);
    assert_eq!(
        activity
            .return_participant(&authority, &h.node, normal_exit.clone())
            .await?,
        ActivityReturnOutcome::Refused(ActivityRefusal::AuthorityRejected),
        "a valid ordinary-exit command must not discard the pending death"
    );
    assert!(
        activity
            .retirement_ready(
                &authority,
                &h.node,
                current.runtime_scope,
                current.scope_ownership_generation
            )
            .await?
            .is_none()
    );
    let before_items: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_player_death_item_records")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(before_items, 0);
    let deps = Dependencies::new()?;
    let dispatch = deps.owner(h, &authority);
    let lane = SpellLane::for_scope(current.runtime_scope);
    let permit = lane.acquire().await.map_err(|_| "spell lane unavailable")?;
    if matches!(case, Case::DepartedProcessDrop) {
        // Commit the real first selected loss, then discard this process-local
        // owner. The new lifecycle must discover and finish the remaining loss.
        h.root
            .commit_player_death_drop(
                &permit,
                &authority,
                &h.node,
                current,
                PlayerDeathDropCause {
                    death_occurrence: *request.occurrence.as_bytes(),
                    item_instance: id(75),
                },
                PlayerDeathItemRequest::selected(&death, id(75), false).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        drop(permit);
        drop(dispatch);
        drop(deps);
        return departed_process::recover(
            h,
            &_evidence.evidence,
            current,
            request.occurrence,
            enter.participants[0].source.runtime_scope,
            temple,
            death.committed_character_revision,
        )
        .await;
    }
    let departed = if matches!(case, Case::InterruptedDrop) {
        // A real first selected one-item transaction commits; the process then
        // stops before the backpack operation. No incomplete transaction survives.
        h.root
            .commit_player_death_drop(
                &permit,
                &authority,
                &h.node,
                current,
                PlayerDeathDropCause {
                    death_occurrence: *request.occurrence.as_bytes(),
                    item_instance: id(75),
                },
                PlayerDeathItemRequest::selected(&death, id(75), false).map_err(debug)?,
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_player_death_item_records")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        assert!(
            activity
                .read_departed_death(&authority, &h.node, current)
                .await?
                .is_none(),
            "a historical death cannot impersonate a lost current owner"
        );
        let mut recovery_request = returning.clone();
        recovery_request.destination_transport = None;
        assert_eq!(
            activity
                .recover(&authority, &h.node, recovery_request.clone())
                .await?,
            ActivityReturnOutcome::Refused(ActivityRefusal::OwnerStillCurrent)
        );
        revoke_instance(h, current.runtime_scope).await?;
        assert!(
            dispatch
                .apply_player_death_items(&permit, post_loss, &death)
                .await
                .is_err(),
            "ordinary mutation authority disappears with the source assignment"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_player_death_item_records")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        assert_eq!(
            activity
                .recover(&authority, &h.node, recovery_request)
                .await?,
            ActivityReturnOutcome::Refused(ActivityRefusal::AuthorityRejected),
            "owner loss alone cannot bypass an unfinished selected loss"
        );
        let mut stale = current;
        stale.character_lease_generation += 1;
        assert!(
            activity
                .read_departed_death(&authority, &h.node, stale)
                .await?
                .is_none()
        );
        let proof = activity
            .read_departed_death(&authority, &h.node, current)
            .await?
            .ok_or("sealed departed death proof")?;
        assert_eq!(proof.source(), current);
        assert_eq!(proof.occurrence(), *request.occurrence.as_bytes());
        assert_eq!(
            proof.origin_scope(),
            enter.participants[0].source.runtime_scope
        );
        let wrong_lane = SpellLane::for_scope(proof.origin_scope());
        let wrong_permit = wrong_lane
            .acquire()
            .await
            .map_err(|_| "spell lane unavailable")?;
        assert!(
            h.root
                .resume_departed_activity_death_items(&wrong_permit, &authority, &h.node, &proof)
                .await
                .is_err()
        );
        drop(wrong_permit);
        h.root
            .resume_departed_activity_death_items(&permit, &authority, &h.node, &proof)
            .await
            .map_err(debug)?;
        Some(proof)
    } else {
        dispatch
            .apply_player_death_items(&permit, post_loss, &death)
            .await
            .map_err(debug)?;
        None
    };
    let first_counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_player_death_item_records),(SELECT count(*) FROM game_item_audit_outbox)")
        .fetch_one(&h.pool).await?;
    let replay = h
        .root
        .settle_runtime_player_death_with_items(
            &sequencer,
            &authority,
            &h.node,
            fence,
            request.clone(),
            None,
            None,
            Some(enter.destination_content.server_artifact_digest),
        )
        .await
        .map_err(debug)?;
    assert_eq!(death, replay);
    if let Some(proof) = &departed {
        h.root
            .resume_departed_activity_death_items(&permit, &authority, &h.node, proof)
            .await
            .map_err(debug)?;
    } else {
        dispatch
            .apply_player_death_items(&permit, post_loss, &replay)
            .await
            .map_err(debug)?;
    }
    assert_eq!(sqlx::query_as::<_, (i64, i64)>("SELECT (SELECT count(*) FROM game_player_death_item_records),(SELECT count(*) FROM game_item_audit_outbox)")
        .fetch_one(&h.pool).await?, first_counts, "replay emits neither a second loss nor a second audit");
    assert_eq!(
        first_counts.0,
        i64::try_from(selection.lost.len() + usize::from(selection.amulet.is_some()))?
    );
    let ground = h
        .root
        .read_item_loop_ground_for_scope(current.runtime_scope)
        .await?;
    assert_eq!(ground.len(), selection.lost.len());
    for row in &ground {
        assert_eq!(row.item.runtime_scope, current.runtime_scope);
        assert_eq!(
            row.item.ground.spatial_position,
            request.death_cell.spatial_position
        );
    }
    assert!(
        h.root
            .read_item_loop_ground_for_scope(enter.participants[0].source.runtime_scope)
            .await?
            .is_empty(),
        "the origin Channel has no private death-drop visibility"
    );
    let child_parent: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(parent_item_instance_id) FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        child_parent,
        id(70).to_vec(),
        "the existing death bag retains the actual child identity"
    );
    let amulet_state: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(75).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        amulet_state,
        if matches!(case, Case::Amulet) {
            (0, 2)
        } else {
            (1, 1)
        }
    );
    drop(permit);
    if departed.is_none() {
        let proven = activity
            .read_death_return_placement(
                &authority,
                &h.node,
                current,
                *request.occurrence.as_bytes(),
            )
            .await?
            .ok_or("complete death return proof")?;
        assert_eq!(proven.exit_position.cell, temple);
        assert_ne!(
            proven.exit_position.cell,
            enter.participants[0].exit_position.cell
        );
        assert_eq!(
            activity
                .return_participant(&authority, &h.node, normal_exit)
                .await?,
            ActivityReturnOutcome::Refused(ActivityRefusal::AuthorityRejected)
        );
    }
    let outcome = if departed.is_some() {
        returning.destination_transport = None;
        let permit = lane.acquire().await.map_err(|_| "spell lane unavailable")?;
        activity
            .recover_departed_death(&permit, &authority, &h.node, returning.clone())
            .await?
    } else {
        activity
            .return_participant(&authority, &h.node, returning.clone())
            .await?
    };
    let returned = match outcome {
        ActivityReturnOutcome::Returned(receipt) => receipt,
        outcome => return Err(format!("proven death return refused: {outcome:?}").into()),
    };
    assert_eq!(returned.exit_cell, temple);
    let return_replay = if departed.is_some() {
        activity
            .recover(&authority, &h.node, returning.clone())
            .await?
    } else {
        activity
            .return_participant(&authority, &h.node, returning.clone())
            .await?
    };
    assert_eq!(return_replay, ActivityReturnOutcome::Replayed(returned));
    let back = item_fence_of(
        store
            .current_session_at(returning.destination_game_session_id)
            .await?
            .0,
    )
    .ok_or("fresh returned Channel fence")?;
    assert_eq!(
        back.runtime_scope,
        enter.participants[0].source.runtime_scope
    );
    assert_eq!(
        back.character_lease_generation,
        current.character_lease_generation + 1
    );
    let back_gameplay = gameplay(back, death.committed_character_revision);
    assert_eq!(
        h.root
            .pending_respawn(&authority, &h.node, back_gameplay)
            .await
            .map_err(debug)?
            .ok_or("handoff must retain pending")?
            .occurrence,
        request.occurrence
    );
    // Authoritative durable placement invokes the actual Channel native owner while
    // the new session/lease/assignment fences remain held, then consumes the row.
    let assignment = h
        .root
        .read_runtime_scope_assignment(back.runtime_scope)
        .await
        .map_err(debug)?
        .ok_or("returned assignment")?;
    assert_eq!(assignment.holder, Some(h.node.fact()));
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = back.runtime_scope
    else {
        return Err("Channel return required".into());
    };
    let origin_pin = enter.participants[0].source_position;
    let content = ChannelContentPin::from_activation(
        world_id,
        1,
        origin_pin.server_artifact_digest,
        [0x44; 32],
        origin_pin.coordinate_frame_digest,
        origin_pin.map_revision_digest,
        (temple.x, temple.y, temple.floor),
    );
    let mut native = ChannelRuntimeV1::from_committed_assignment(
        world_id,
        channel_id,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        assignment.ownership_generation,
        assignment.source_revision,
        &assignment.decision_identity,
        2,
        content,
    )
    .map_err(debug)?;
    let reservation = native
        .reserve_fresh_session(back.game_session_id)
        .map_err(debug)?;
    let actor = native.commit_fresh_session(reservation).map_err(debug)?;
    let exit = enter.participants[0].exit_position.cell;
    native
        .initialize_pinned_test_position(
            actor,
            MovementLocalPosition {
                x: exit.x,
                y: exit.y,
                floor: exit.floor,
            },
        )
        .map_err(debug)?;
    let temple_native = MovementLocalPosition {
        x: temple.x,
        y: temple.y,
        floor: temple.floor,
    };
    let placed = FinalLastPosition {
        write_sequence: 1,
        record: LastPositionRecord {
            cell: temple,
            vitals: None,
            rest: None,
            ..origin_pin
        },
    };
    let accepted = h
        .root
        .place_and_consume_pending_respawn_placed(
            &authority,
            &h.node,
            back_gameplay,
            request.occurrence,
            placed,
            |pending| {
                let exact = pending.is_some_and(|pending| {
                    pending.occurrence == request.occurrence
                        && pending.respawn_position == request.respawn_position
                });
                std::future::ready(
                    exact
                        && native
                            .place_admitted_respawn(
                                actor,
                                back.game_session_id,
                                Some(temple_native),
                                |cell| cell == temple_native,
                            )
                            .is_ok_and(|position| position.position() == temple_native),
                )
            },
        )
        .await
        .map_err(debug)?;
    assert!(accepted);
    assert!(
        h.root
            .pending_respawn(&authority, &h.node, back_gameplay)
            .await
            .map_err(debug)?
            .is_none()
    );
    let saved: (i32, i32, i16, Option<i32>, Option<i32>) = sqlx::query_as("SELECT tile_x,tile_y,tile_floor,health,mana FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(back.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(saved, (temple.x, temple.y, temple.floor, None, None));
    if matches!(case, Case::RedSkullDrop | Case::InterruptedDrop | Case::DepartedProcessDrop) {
        // This is a current Channel session, not the stale pre-entry source fence.
        // Scope refusal therefore proves actual private Ground isolation.
        let take = ItemTransferRequest {
            weight_generation_digest: origin_pin.server_artifact_digest,
            command: CommandRef::new(back.game_session_id, CommandId::new(11).map_err(debug)?),
            source_item_instance_id: id(70),
            destination: ItemTransferDestination::ContainerSlot,
            item: backpack_facts(),
            backpack: None,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "simulation-1".into(),
        };
        assert!(
            matches!(
                h.root
                    .freeze_item_transfer(
                        &authority,
                        &h.node,
                        back,
                        take,
                        &Weights(origin_pin.server_artifact_digest)
                    )
                    .await,
                Err(ItemTransferError::AuthorityRejected)
            ),
            "current Channel take must refuse the Instance source before container planning"
        );
        assert_eq!(
            h.root
                .read_item_loop_ground_for_scope(current.runtime_scope)
                .await?
                .len(),
            2
        );
    }
    let proof = activity
        .retirement_ready(
            &authority,
            &h.node,
            current.runtime_scope,
            current.scope_ownership_generation,
        )
        .await?
        .ok_or("all participants durably returned")?;
    let channel_lane = SpellLane::for_scope(back.runtime_scope);
    let channel_permit = channel_lane
        .acquire()
        .await
        .map_err(|_| "spell lane unavailable")?;
    assert!(
        h.root
            .retire_activity_ground_step(&channel_permit, &authority, &h.node, &proof)
            .await
            .is_err(),
        "a Channel permit never authorizes private Instance retirement"
    );
    drop(channel_permit);
    let permit = lane.acquire().await.map_err(|_| "spell lane unavailable")?;
    let mut retired = 0;
    while let ActivityGroundRetirement::Progress(_) = h
        .root
        .retire_activity_ground_step(&permit, &authority, &h.node, &proof)
        .await
        .map_err(debug)?
    {
        retired += 1;
        assert!(retired <= 4, "bounded fixture retirement must advance");
    }
    assert!(
        h.root
            .read_item_loop_ground_for_scope(current.runtime_scope)
            .await?
            .is_empty()
    );
    assert!(
        h.root
            .read_item_loop_ground_for_scope(back.runtime_scope)
            .await?
            .is_empty()
    );
    match case {
        Case::RedSkullDrop | Case::InterruptedDrop | Case::DepartedProcessDrop => {
            assert!(retired > 0);
            let live: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid) AND lifecycle=1")
                .bind(id(70).as_slice()).bind(id(71).as_slice()).bind(id(75).as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                live, 0,
                "the accepted existing retirement owner resolves the death bag and its child"
            );
        }
        Case::Amulet => assert_eq!(
            retired, 0,
            "a protected carried backpack is never private Ground retirement input"
        ),
    }
    assert!(matches!(
        h.root
            .retire_activity_ground_step(&permit, &authority, &h.node, &proof)
            .await
            .map_err(debug)?,
        ActivityGroundRetirement::Complete
    ));
    eprintln!("ACTIVITY_PLAYER_DEATH POSTGRES_CASE_COMPLETED");
    Ok(())
}

async fn seed_equipment(h: &Harness, case: Case) -> TestResult {
    super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i3031", 5)]).await?;
    // Existing fixture construction only. Every death, loss, return and retirement
    // transition below uses the genuine runtime owner with all triggers enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3057','definition-r1',1,1,encode($3,'hex')::uuid)")
        .bind(id(75).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,7,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(75).as_slice()).bind(id(42).as_slice()).bind(id(76).as_slice()).execute(&mut *tx).await?;
    if matches!(case, Case::RedSkullDrop | Case::InterruptedDrop | Case::DepartedProcessDrop) {
        sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id,skull,skull_until) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,clock_timestamp()+interval '1 day')")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

struct Weights([u8; 32]);
impl ItemWeightLookup for Weights {
    fn generation_digest(&self) -> [u8; 32] {
        self.0
    }
    fn weight_of(&self, definition: &TypedDefinitionRef) -> Result<u32, ItemWeightRefused> {
        if definition.family == "Item"
            && definition.revision_ref == "definition-r1"
            && matches!(
                definition.production_key.as_str(),
                "oteryn:item.tibia.i2854" | "oteryn:item.tibia.i3031" | "oteryn:item.tibia.i3057"
            )
        {
            Ok(1)
        } else {
            Err(ItemWeightRefused::Unknown)
        }
    }
}

fn backpack_facts() -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        weight: Some(1),
        stack: ItemStackClass::NonStackable,
        container_capacity: Some(20),
        container_slot_equip_pattern: true,
    }
}

async fn revoke_instance(h: &Harness, scope: RuntimeScopeRefV1) -> TestResult {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = scope
    else {
        return Err("revoke exact Instance only".into());
    };
    sqlx::query("INSERT INTO game_control_activity_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,3)")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).execute(&h.pool).await?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("control predecessor")?;
    let writer = RuntimeScopeAssignmentWriter::open(h.root.clone(), "sequencer-writer")
        .await
        .map_err(debug)?;
    let result = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([121; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Revoke { scope, predecessor },
        })
        .await
        .map_err(debug)?;
    let AssignmentOutcome::Committed(receipt) = result else {
        return Err(format!("control revoke refused: {result:?}").into());
    };
    assert_eq!(receipt.assignment.scope, scope);
    assert_eq!(receipt.assignment.state, AssignmentState::Revoked);
    Ok(())
}
