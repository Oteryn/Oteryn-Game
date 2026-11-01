//! Q16: real House movement feeds the existing ordered LastPosition projection.
//! A retained actor preserves its command namespace. A lost process cannot
//! recreate a started namespace; terminal release precedes a fresh signed entry.
use super::{Harness, TestResult, debug};
use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
use crate::foundation::{ExactActorRef, GameSessionState};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, ConnectionEnd, ConnectionIdentifiers,
    ControlLossResult, FirstEntryOutcome, GraceExpiryResult, IDLE_LIVENESS, serve_admitted};
use oteryn_protocol_oteryn::CommandStatus;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[path = "position/admission_views.rs"]
mod admission_views;
#[path = "position/transfer_views.rs"]
mod transfer_views;

pub(super) async fn capture(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    actor: ExactActorRef,
) -> TestResult {
    let initial_vitals = wound(owner, admitted, actor, 1).await?;
    assert_eq!(owner.write_due_last_positions(true).await, 0);
    let first = snapshot(h, admitted).await?;
    assert_eq!(first.cell, (32310, 32804, 7));
    assert_eq!(first.vitals, Some(initial_vitals));
    let captured = owner.final_last_position(admitted.game_session_id, actor).await
        .ok_or("tracked House final capture")?;
    assert_eq!(captured.record.cell.x, first.cell.0);
    assert_eq!(captured.record.cell.y, first.cell.1);
    assert_eq!(captured.record.cell.floor, first.cell.2);
    assert_eq!(captured.record.vitals.map(|v| (v.health(), v.mana())), first.vitals);

    let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
        oteryn_protocol_oteryn::world_spatial::StepDirection::East);
    let result = super::wire::exchange(owner, &mut admitted,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &step).await?;
    assert_eq!(result.0, CommandStatus::Accepted);
    let state = owner.runtime.lock().await.house_actor_state(actor,
        admitted.game_session_id).map_err(debug)?;
    assert_eq!((state.position.x, state.position.y, state.position.floor), (32311, 32804, 7));
    let latest_vitals = wound(owner, admitted, actor, 2).await?;
    assert_eq!(owner.write_due_last_positions(true).await, 0);
    let latest = snapshot(h, admitted).await?;
    assert_eq!(latest.cell, (32311, 32804, 7));
    assert_eq!(latest.vitals, Some(latest_vitals));
    assert_eq!(latest.lease, first.lease);
    assert!(latest.sequence > first.sequence,
        "movement and vitals advance the existing ordered projection together");
    assert_eq!(owner.write_due_last_positions(true).await, 0);
    assert_eq!(snapshot(h, admitted).await?, latest,
        "an unchanged forced capture does not create another durable write");
    let current = admitted.item_fence.ok_or("House fence")?;
    let saved = h.root.read_saved_house_position(owner.character, current.character_id)
        .await.map_err(debug)?.ok_or("scoped LastPosition")?;
    let house = saved.house.ok_or("named House metadata")?;
    assert_eq!(house.house.house_key(), super::HOUSE);
    assert_eq!(house.origin_channel, admitted.channel_id);
    assert_eq!((house.origin.x, house.origin.y, house.origin.floor), (32309, 32805, 7));
    let root = h.root.read_current_character(owner.character, current.character_id)
        .await.map_err(debug)?;
    let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
        character_id: current.character_id,
        game_session_id: current.game_session_id,
        connection_generation: current.connection_generation,
        character_lease_generation: current.character_lease_generation,
        runtime_scope: current.runtime_scope,
        scope_ownership_generation: current.scope_ownership_generation,
        expected_character_revision: root.revision,
    };
    // The same current House custody reads native PvP state without inventing
    // a Channel fence. Refused proofs leave the owning state unchanged.
    assert!(h.root.read_house_pvp_state(owner.character, owner.holder, fence, admitted.channel_id)
        .await.map_err(debug)?.is_none());
    assert!(matches!(h.root.read_pvp_state(owner.character, owner.holder, fence).await,
        Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)));
    let mut stale_pvp = fence;
    stale_pvp.character_lease_generation += 1;
    assert!(matches!(h.root.read_house_pvp_state(owner.character, owner.holder,
        stale_pvp, admitted.channel_id).await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
    let mut wrong_house_pvp = fence;
    wrong_house_pvp.runtime_scope = super::HouseId::new(admitted.world_id, super::OTHER_HOUSE)
        .ok_or("other House")?.runtime_scope().map_err(debug)?;
    assert!(matches!(h.root.read_house_pvp_state(owner.character, owner.holder,
        wrong_house_pvp, admitted.channel_id).await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
    let wrong_node = super::NodeIncarnationProof::new(super::NodeRegistrationFact::new(
        h.node.fact().node_id(), h.node.fact().registration_revision()+1),
        super::BootstrapSecret::from_bytes([0x91;32]));
    assert!(matches!(h.root.read_house_pvp_state(owner.character, &wrong_node,
        fence, admitted.channel_id).await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
    let old = crate::durability::character_last_position::ScopedPeriodicLastPosition {
        write: crate::durability::character_last_position::PeriodicLastPosition {
            fence, write_sequence: captured.write_sequence, record: captured.record,
        },
        house: Some(house),
    };
    let mut outcome = h.root.write_scoped_character_last_positions(owner.character, owner.holder,
        vec![old.clone()]).await.map_err(debug)?;
    assert_eq!(outcome.len(), 1);
    assert_eq!(outcome.remove(0).map_err(debug)?,
        crate::durability::character_last_position::LastPositionWrite::Superseded);
    assert_eq!(snapshot(h, admitted).await?, latest,
        "a delayed earlier capture cannot overwrite either the newer cell or its vitals");
    let mut stale = old;
    stale.write.fence.character_lease_generation += 1;
    let outcome = h.root.write_scoped_character_last_positions(owner.character, owner.holder,
        vec![stale]).await.map_err(debug)?;
    assert!(matches!(outcome.as_slice(), [Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)]));
    assert_eq!(snapshot(h, admitted).await?, latest);
    let mut resumed = same_actor_resume(h, owner, admitted, actor).await?;
    assert_training_login_clock(h, owner, resumed).await?;
    let transfer_views = transfer_views::before(owner, resumed, actor).await?;
    // A real exit is another internal session, not another login. Exercise
    // both native transfer edges and the unchanged generic Channel logout.
    let old_session = resumed.game_session_id;
    let mut exit = vec![8, 2, 18, super::HOUSE.len() as u8];
    exit.extend_from_slice(super::HOUSE.as_bytes());
    let (status, payload) = super::wire::exchange(owner, &mut resumed,
        oteryn_protocol_oteryn::house_transfer::COMMAND_TYPE_HOUSE_TRANSFER, &exit).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let ack = h.root.house_transfer_recovery(resumed.game_session_id).await.map_err(debug)?
        .ok_or("native committed exit receipt")?.ack;
    assert_eq!(ack.house, None);
    assert_eq!(payload, oteryn_protocol_oteryn::house_transfer::encode_house_transfer_ack(&ack)
        .map_err(debug)?);
    assert_ne!(resumed.game_session_id, old_session);
    assert!(matches!(resumed.item_fence.ok_or("actual exit fence")?.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Channel { .. }));
    transfer_views::after(owner, resumed, actor, transfer_views).await?;
    logout(h, owner, &mut resumed, actor).await?;
    Ok(())
}

async fn same_actor_resume(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, actor: ExactActorRef) -> TestResult<AdmittedSession> {
    // exchange observed the actual socket EOF after the last STEP. This is the
    // exact listener callback, followed by an authenticated framed ClientResume.
    let saved = snapshot(h, admitted).await?;
    let next_command = admitted.continuity.next_command_id;
    assert_eq!(owner.lose_control_for_cause(admitted, std::time::Duration::ZERO, None).await,
        ControlLossResult::Recorded);
    let lost = owner.runtime.lock().await.house_player_control_facts(actor,
        admitted.game_session_id).map_err(debug)?.control_loss.ok_or("native House loss mark")?;
    let store = FreshAdmissionStore::from_root(h.root.clone()).with_house_lifecycle_owner(owner.holder);
    let current = store.current_session(admitted.game_session_id).await.map_err(debug)?;
    assert_eq!(current.current_game_session_id(), admitted.game_session_id);
    assert_ne!(current.commit().game_session_id(), admitted.game_session_id,
        "the original authenticated receipt remains origin provenance");
    let epoch = current.current_control_loss_epoch().ok_or("native owning loss epoch")?;
    let (operation, _) = store.owning_loss(admitted.game_session_id, epoch).await.map_err(debug)?
        .ok_or("destination owning-loss receipt")?;
    assert!(matches!(operation.observation.history,
        crate::foundation::ControlLossHistoryV1::ScopeTransfer { .. }));
    assert_eq!(operation.observation.session.current_game_session_id(), admitted.game_session_id);
    assert_eq!(operation.observation.session.commit(), current.commit());
    let encoded = crate::durability::fresh_admission::encode_fresh_loss(&operation).map_err(debug)?;
    assert_eq!(crate::durability::fresh_admission::decode_fresh_loss(&encoded, current.commit())
        .map_err(debug)?, operation, "codec retains both real identities");
    assert!(store.owning_loss(current.commit().game_session_id(), epoch).await.map_err(debug)?.is_none(),
        "the destination loss must not be journaled under its retired origin");
    let crate::durability::fresh_admission::FreshLossReconciliation::Committed { current: reconciled, .. }
        = store.reconcile_fresh_loss(&operation).await.map_err(debug)? else {
            return Err("actual destination loss must reconcile".into());
        };
    assert_eq!(reconciled.current_session.current_game_session_id(), admitted.game_session_id);
    let mut resumed = resume(owner, admitted, admitted.game_session_id, 7, 10).await?;
    assert_eq!(resumed.game_session_id, admitted.game_session_id);
    assert_eq!(resumed.runtime_actor, Some(actor));
    assert_eq!(resumed.continuity.connection_generation, admitted.continuity.connection_generation + 1);
    assert_eq!(resumed.continuity.next_command_id, next_command, "same-session recovery cannot reset commands");
    assert_eq!(resumed.continuity.selected_capabilities, admitted.continuity.selected_capabilities);
    let state = owner.runtime.lock().await.house_actor_state(actor, resumed.game_session_id).map_err(debug)?;
    assert_eq!((state.position.x, state.position.y, state.position.floor),
        (saved.cell.0, saved.cell.1, saved.cell.2));
    assert_eq!(owner.runtime.lock().await.house_player_control_facts(actor,
        resumed.game_session_id).map_err(debug)?.control_loss, None);
    assert!(!owner.runtime.lock().await.house_player_writes_fenced(actor,
        resumed.game_session_id).map_err(debug)?);
    assert_eq!(snapshot(h, resumed).await?, saved);
    // A second real loss uses the committed recovery history, not the entry
    // marker and not another fresh protection entitlement.
    let history = store.resumed_history(resumed.game_session_id).await.map_err(debug)?
        .ok_or("retained actual destination recovery")?;
    let crate::foundation::ControlLossHistoryV1::Resumed { budget, protection, .. } = history else {
        return Err("same-session recovery must retain its actual epoch".into());
    };
    assert_eq!(budget.epoch(), epoch);
    assert_eq!(owner.lose_control_for_cause(resumed, std::time::Duration::ZERO, None).await,
        ControlLossResult::Recorded);
    let next_epoch = store.current_session(resumed.game_session_id).await.map_err(debug)?
        .current_control_loss_epoch().ok_or("next loss")?;
    assert_eq!(next_epoch.get(), epoch.get()+1);
    let (second_loss, _) = store.owning_loss(resumed.game_session_id, next_epoch).await.map_err(debug)?
        .ok_or("second destination loss")?;
    assert!(matches!(second_loss.observation.history, crate::foundation::ControlLossHistoryV1::Resumed { .. }));
    assert_eq!(second_loss.observation.protection, protection);
    resumed = resume(owner, resumed, admitted.game_session_id, 8, 11).await?;
    assert_eq!(resumed.continuity.connection_generation, admitted.continuity.connection_generation+2);
    assert_eq!(resumed.continuity.next_command_id, next_command);
    // The recovered actor is input-eligible through the actual House lane.
    let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
        oteryn_protocol_oteryn::world_spatial::StepDirection::West);
    let reply = super::wire::exchange(owner, &mut resumed,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, &step).await?;
    assert_eq!(reply.0, CommandStatus::Accepted);
    assert_eq!(resumed.continuity.next_command_id, next_command + 1);
    assert!(lost.epoch > 0);
    Ok(resumed)
}

pub(super) async fn logout_and_relog(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, mut admitted: AdmittedSession,
    actor: ExactActorRef) -> TestResult {
    let expected_vitals = wound(owner, admitted, actor, 1).await?;
    step(owner, &mut admitted, oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        CommandStatus::Accepted).await?;
    // No explicit periodic save here: the actual wire logout must capture the
    // final current cell and vitals in the native terminal release transaction.
    logout(h, owner, &mut admitted, actor).await?;
    let saved = snapshot(h, admitted).await?;
    assert_eq!(saved.cell, (32311, 32804, 7));
    assert_eq!(saved.vitals, Some(expected_vitals));
    let fresh = fresh(owner, admitted, 20, 12).await?;
    let source = crate::foundation::GameSessionId::decode(&super::id(20)).map_err(debug)?;
    admission_views::assert_initialized(h, owner, fresh, source).await?;
    assert!(owner.progression_binding(fresh.game_session_id).is_some());
    let fresh = admission_views::recover_lost_views(h, owner, fresh, source).await?;
    let replacement = fresh.runtime_actor.ok_or("fresh native actor")?;
    assert_ne!(fresh.game_session_id, admitted.game_session_id);
    assert_ne!(replacement, actor, "the retired slot must advance its native generation");
    assert!(fresh.item_fence.ok_or("fresh House fence")?.character_lease_generation
        > admitted.item_fence.ok_or("old House fence")?.character_lease_generation);
    assert_restored(owner, fresh, &saved).await?;
    assert_eq!(fresh.continuity.next_command_id, 1,
        "only a genuinely new GameSession starts a new command namespace");
    assert_eq!(snapshot(h, fresh).await?.cell, saved.cell);
    // Commit the genuine terminal owner, then model losing the acknowledgement
    // before process-local retirement. Reconciliation must remove the still
    // present House actor; a generic Channel lookup cannot declare it gone.
    let controller = fresh.controller.ok_or("fresh controller")?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(owner.final_position_store(&store, fresh.game_session_id, replacement).await
        .release_logout_session(fresh.game_session_id,
            &crate::gameplay_transport::canonical_uuid(&controller.account_id), controller.transport)
        .await.map_err(debug)?, ExpiredLossReleaseV1::Released { .. }));
    assert!(owner.runtime.lock().await.house_actor_state(replacement, fresh.game_session_id).is_ok());
    assert_eq!(owner.reconcile_terminal_session(fresh).await, GraceExpiryResult::Released);
    assert!(owner.runtime.lock().await.house_actor_state(replacement, fresh.game_session_id).is_err());
    assert_eq!(owner.reconcile_terminal_session(fresh).await, GraceExpiryResult::Released);
    Ok(())
}

pub(super) async fn before_process_loss(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, mut admitted: AdmittedSession,
    actor: ExactActorRef) -> TestResult<(AdmittedSession, Snapshot)> {
    wound(owner, admitted, actor, 2).await?;
    step(owner, &mut admitted, oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        CommandStatus::Accepted).await?;
    assert_eq!(owner.write_due_last_positions(true).await, 0);
    // A real loss/resume leaves the exact native epoch required by the existing
    // abandoned-controller release. No fixture SQL invents that lifecycle.
    let admitted = same_actor_resume(h, owner, admitted, actor).await?;
    assert_eq!(owner.write_due_last_positions(true).await, 0);
    let saved = snapshot(h, admitted).await?;
    assert!(admitted.continuity.next_command_id > 1);
    Ok((admitted, saved))
}

pub(super) async fn after_process_loss(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession,
    saved: Snapshot) -> TestResult {
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let (before, _) = store.current_session_at(admitted.game_session_id).await.map_err(debug)?;
    assert_eq!(before.session_state(), GameSessionState::Active);
    assert!(owner.runtime.lock().await.house_player_for_session(admitted.game_session_id)
        .map_err(debug)?.is_none());
    assert!(resume(owner, admitted, admitted.game_session_id, 13, 14).await.is_err(),
        "an entry receipt cannot reset the namespace after a real command was started");
    let (after, _) = store.current_session_at(admitted.game_session_id).await.map_err(debug)?;
    assert_eq!(after.session_state(), before.session_state());
    assert_eq!(after.current_transport(), before.current_transport());
    assert_eq!(after.current_character_lease(), before.current_character_lease());
    assert!(owner.runtime.lock().await.house_player_for_session(admitted.game_session_id)
        .map_err(debug)?.is_none(), "refusal cannot reconstruct an actor from stale entry state");
    assert_eq!(snapshot(h, admitted).await?, saved);
    let controller = admitted.controller.ok_or("exact ended controller")?;
    assert!(matches!(store.with_house_release_owner(&h.node).release_abandoned_session(
        admitted.game_session_id,
        &crate::gameplay_transport::canonical_uuid(&controller.account_id), controller.transport,
    ).await.map_err(debug)?, ExpiredLossReleaseV1::Released { .. }));
    assert_eq!(snapshot(h, admitted).await?, saved,
        "process loss keeps the last genuine capture; terminal release fabricates no newer cell");
    let fresh = fresh(owner, admitted, 21, 15).await?;
    let source = crate::foundation::GameSessionId::decode(&super::id(21)).map_err(debug)?;
    admission_views::assert_initialized(h, owner, fresh, source).await?;
    assert_ne!(fresh.game_session_id, admitted.game_session_id);
    assert_restored(owner, fresh, &saved).await?;
    Ok(())
}

pub(super) async fn revoked_access(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, mut admitted: AdmittedSession,
    actor: ExactActorRef) -> TestResult {
    consume_carrot(h, owner, &mut admitted).await?;
    let character = admitted.item_fence.ok_or("House character")?.character_id;
    let conditions = h.root.read_gourmet_conditions(owner.character, character)
        .await.map_err(debug)?;
    assert!(conditions.attributes.is_some(), "the actor has a real committed gourmet condition");
    wound(owner, admitted, actor, 1).await?;
    // A settled ownership change removes this current resident's access. Its
    // existing House custody remains real history and may still be retired.
    remove_owner_access(h).await?;
    assert!(!owner.validate_house_resume(actor, admitted.game_session_id).await);
    step(owner, &mut admitted, oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        CommandStatus::Rejected).await?;
    logout(h, owner, &mut admitted, actor).await?;
    let saved = snapshot(h, admitted).await?;
    assert_eq!(saved.cell, (32310, 32804, 7));
    let checkpoint = h.root.read_gourmet_conditions(owner.character, character)
        .await.map_err(debug)?;
    assert!(checkpoint.attributes.is_some(), "revocation does not strand a living actor's final condition save");
    assert_fresh_refused_and_released(h, owner, admitted, 22, 16).await?;
    assert_eq!(snapshot(h, admitted).await?, saved);
    // Grant current guest access and genuinely authenticate again. This proves
    // the refused fresh COMMIT released its claims and original reserved slot.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
        .bind(admitted.world_id.as_bytes().as_slice()).bind(super::HOUSE)
        .bind(character.as_bytes().as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_properties SET revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(admitted.world_id.as_bytes().as_slice()).bind(super::HOUSE).execute(&mut *tx).await?;
    tx.commit().await?;
    let fresh = fresh(owner, admitted, 23, 17).await?;
    assert_restored(owner, fresh, &saved).await?;
    Ok(())
}

pub(super) async fn occupied_saved_cell(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, mut admitted: AdmittedSession,
    actor: ExactActorRef, hireling: [u8; 16]) -> TestResult {
    assert!(super::active(h, hireling).await?);
    let state = owner.runtime.lock().await.house_actor_state(actor, admitted.game_session_id)
        .map_err(debug)?;
    let placed: (i32, i32, i16) = sqlx::query_as("SELECT x::int,y::int,floor::smallint FROM game_house_hirelings WHERE hireling_id=encode($1,'hex')::uuid")
        .bind(hireling.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((placed.0, placed.1, -placed.2),
        (state.position.x, state.position.y, state.position.floor));
    assert!(owner.validate_house_resume(actor, admitted.game_session_id).await,
        "the proved retained actor may share its original lamp placement cell");
    wound(owner, admitted, actor, 1).await?;
    logout(h, owner, &mut admitted, actor).await?;
    let saved = snapshot(h, admitted).await?;
    assert_fresh_refused_and_released(h, owner, admitted, 24, 18).await?;
    assert_eq!(snapshot(h, admitted).await?, saved,
        "a blocked new placement cannot overwrite the saved native cell or fall back to another tile");
    assert!(super::active(h, hireling).await?);
    assert_eq!(super::uses(h).await?, 1);
    assert_eq!(super::inbox_count(h).await?, 0);
    Ok(())
}

async fn assert_fresh_refused_and_released(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, prior: AdmittedSession,
    nonce: u8, transport: u8) -> TestResult {
    let refused = fresh(owner, prior, nonce, transport).await?;
    let source = crate::foundation::GameSessionId::decode(&super::id(nonce)).map_err(debug)?;
    assert_eq!(refused.game_session_id, source);
    assert_eq!(refused.first_entry, FirstEntryOutcome::RefusedUnavailable);
    let actor = refused.runtime_actor.ok_or("real refused source slot")?;
    assert!(matches!(refused.item_fence.ok_or("real source fence")?.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Channel { .. }));
    assert!(owner.runtime.lock().await.house_actor_state(actor, source).is_err());
    assert!(owner.runtime.lock().await.read_actor_position(actor).is_err(),
        "known House refusal cannot publish a fallback Channel position");
    // Exercise the ordinary refused-first-entry socket end and listener loss.
    // Keep the native sixty-second grace and durable clock; no test SQL edits
    // a deadline, session state, interpretation or claim.
    let (mut server, mut client) = tokio::io::duplex(4096);
    client.shutdown().await?;
    assert!(matches!(serve_admitted(&mut server, refused, owner, IDLE_LIVENESS).await,
        ConnectionEnd::AdmittedThenDisconnected(ended) if ended.game_session_id == source));
    assert_eq!(owner.lose_control_for_cause(refused, std::time::Duration::ZERO, None).await,
        ControlLossResult::Recorded);
    assert_eq!(tokio::time::timeout(std::time::Duration::from_secs(70),
        owner.expire_session_control_loss(refused)).await?, GraceExpiryResult::Released);
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(source).await.map_err(debug)?;
    assert_eq!(current.session_state(), GameSessionState::Terminal,
        "known House restoration refusal must retire the genuinely committed fresh source");
    assert!(owner.runtime.lock().await.house_player_for_session(source)
        .map_err(debug)?.is_none());
    Ok(())
}

async fn assert_restored(owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, saved: &Snapshot) -> TestResult {
    assert!(matches!(admitted.first_entry, FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled));
    let actor = admitted.runtime_actor.ok_or("fresh House actor")?;
    let state = owner.runtime.lock().await.house_actor_state(actor, admitted.game_session_id)
        .map_err(debug)?;
    assert_eq!(state.house, super::HOUSE);
    assert_eq!((state.position.x, state.position.y, state.position.floor), saved.cell);
    let states = owner.states.spell_states.lock().await;
    let restored = &states.actors.iter().find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
        .ok_or("fresh native vitals")?.2;
    assert_eq!(restored.saved_vitals(), saved.vitals,
        "fresh actor recovery restores the captured living vitals with its exact saved cell");
    Ok(())
}

async fn step(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    direction: oteryn_protocol_oteryn::world_spatial::StepDirection,
    expected: CommandStatus) -> TestResult {
    let frame = oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction);
    let reply = super::wire::exchange(owner, admitted,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, &frame).await?;
    assert_eq!(reply.0, expected);
    Ok(())
}

async fn assert_training_login_clock(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual login-clock fence")?;
    let original_revision = h.root.read_current_character(owner.character, fence.character_id)
        .await.map_err(debug)?.revision;
    let gameplay = crate::durability::character_progression::CurrentCharacterGameplayFence {
        character_id: fence.character_id, game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: original_revision,
    };
    let (login_clock, destination_clock): (i64, i64) = sqlx::query_as(
        "SELECT original.training_created_at_us,destination.training_created_at_us \
         FROM game_durability_reconnect_sessions destination \
         JOIN game_durability_fresh_admission_receipts r \
           ON r.replay_key=destination.initial_fresh_replay_key \
         JOIN game_durability_reconnect_sessions original ON original.game_session_id=r.game_session_id \
         JOIN game_character_training_markers m ON m.game_session_id=original.game_session_id \
           AND m.character_id=original.character_id AND m.cause='offline_settlement' \
         WHERE destination.game_session_id=encode($1,'hex')::uuid",
    ).bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert!(destination_clock > login_clock, "native handoff creates a later internal session clock");
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert_eq!(store.read_training_login_clock_for_test(gameplay).await.map_err(debug)?,
        Some(login_clock), "logout credits the authentic login, not an internal House or exit destination");
    let mut stale = gameplay; stale.character_lease_generation += 1;
    assert!(store.read_training_login_clock_for_test(stale).await.is_err(),
        "the historical clock cannot supply current authority to a stale fence");
    Ok(())
}

async fn logout(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession, actor: ExactActorRef) -> TestResult {
    let fence = admitted.item_fence.ok_or("actual logout fence")?;
    let build_before = h.root.read_character_build_state(owner.character, fence.character_id)
        .await.map_err(debug)?;
    assert_training_login_clock(h, owner, *admitted).await?;
    let reply = super::wire::exchange(owner, admitted,
        oteryn_protocol_oteryn::logout::COMMAND_TYPE_LOGOUT_INTENT, &[]).await?;
    assert_eq!(reply.0, CommandStatus::Accepted);
    assert_eq!(reply.1, oteryn_protocol_oteryn::logout::encode_logout_result(
        oteryn_protocol_oteryn::logout::LogoutResult::Accepted).map_err(debug)?);
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(admitted.game_session_id).await.map_err(debug)?;
    assert_eq!(current.session_state(), GameSessionState::Terminal);
    assert!(owner.runtime.lock().await.house_actor_state(actor, admitted.game_session_id).is_err());
    let (count, exact): (i64, bool) = sqlx::query_as(
        "SELECT count(*),coalesce(bool_and( \
           m.character_id=s.character_id AND m.lease_generation=s.character_lease_generation \
           AND b.character_id=m.character_id AND b.cause=m.cause \
           AND b.original_character_revision+1=b.committed_character_revision \
           AND b.committed_character_revision=r.character_revision \
           AND b.committed_at=m.committed_at_us/1000 \
           AND m.selected_skill=b.offline_skill_after AND m.selected_skill=-1 \
           AND b.stamina_minutes_before=b.stamina_minutes_after),false) \
         FROM game_character_training_markers m \
         JOIN game_character_build_receipts b USING(build_occurrence_id) \
         JOIN game_durability_reconnect_sessions s ON s.game_session_id=m.game_session_id \
         JOIN game_character_roots r ON r.character_id=m.character_id \
         WHERE m.game_session_id=encode($1,'hex')::uuid AND m.cause='logout'",
    ).bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((count, exact), (1, true),
        "wire logout retains exactly one native training marker and matching build receipt");
    assert_eq!(h.root.read_character_build_state(owner.character, fence.character_id)
        .await.map_err(debug)?, build_before, "logout cannot grant build progress");
    let controller = admitted.controller.ok_or("actual logout controller")?;
    assert_eq!(FreshAdmissionStore::from_root(h.root.clone()).release_logout_session(
        admitted.game_session_id, &crate::gameplay_transport::canonical_uuid(&controller.account_id),
        controller.transport,
    ).await.map_err(debug)?, ExpiredLossReleaseV1::Terminal);
    let replay_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_training_markers \
         WHERE game_session_id=encode($1,'hex')::uuid AND cause='logout'",
    ).bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(replay_count, 1, "terminal replay must not make a second training marker");
    Ok(())
}

async fn consume_carrot(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession) -> TestResult {
    super::super::item_look::seed_main_backpack(h, &[("oteryn:item.tibia.i29409", 1)]).await?;
    let definition = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(&runtime,
            owner.active_generation).ok_or("gourmet index")?;
        let key = "oteryn:item.tibia.i29409";
        index.revision(key).and_then(|revision| index.definition_ref("Item", key, revision))
            .ok_or("qualified carrot")?
    };
    use oteryn_protocol_oteryn::world_object as wire;
    let body = wire::encode_item_use_intent(&wire::UseIntent {
        target: wire::UseTarget::ItemByDefinition(definition), use_with: None,
        use_with_map_item: None, use_with_position: None,
    }).map_err(debug)?;
    let reply = super::wire::exchange(owner, admitted, wire::COMMAND_TYPE_USE_INTENT, &body).await?;
    assert_eq!(reply.0, CommandStatus::Accepted);
    assert_eq!(wire::decode_use_result(&reply.1).map_err(debug)?, wire::UseDisposition::Committed);
    Ok(())
}

async fn remove_owner_access(h: &Harness) -> TestResult {
    let landlord = super::id(121);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
    let copied: String = sqlx::query_scalar("SELECT (to_jsonb(r)||jsonb_build_object('character_id',encode($2,'hex')::uuid,'name','Position Fixture Landlord'))::text FROM game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid")
        .bind(super::id(41).as_slice()).bind(landlord.as_slice()).fetch_one(&mut *tx).await?;
    super::insert_character_root_copy(&mut tx, &copied).await?;
    sqlx::query("UPDATE game_house_properties SET owner_character_id=encode($1,'hex')::uuid,ownership_generation=ownership_generation+1,revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(landlord.as_slice()).bind(super::id(42).as_slice()).bind(super::HOUSE).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,ownership_revision=ownership_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(landlord.as_slice()).bind(super::id(42).as_slice()).bind(super::HOUSE).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn resume(owner: &ComposedFreshAdmission<'_, '_, '_>, prior: AdmittedSession,
    source: crate::foundation::GameSessionId, nonce: u8, transport: u8) -> TestResult<AdmittedSession> {
    let token = super::RecoverySource::token(nonce)?;
    let mut body = Vec::new();
    bytes(&mut body, 1, source.as_bytes());
    bytes(&mut body, 2, token.as_bytes());
    scalar(&mut body, 3, prior.continuity.server_sequence);
    scalar(&mut body, 4, 1);
    scalar(&mut body, 5, 1);
    scalar(&mut body, 6, 1);
    for capability in prior.continuity.selected_capabilities.as_slice() {
        scalar(&mut body, 7, u64::from(*capability));
    }
    bytes(&mut body, 8, b"hireling-house-position");
    let mut frame = vec![8, crate::foundation::MessageType::ClientResume as u8];
    bytes(&mut frame, 4, &body);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let admitted = crate::gameplay_transport::connection::admit_frame(&mut server, &frame,
        owner, &Identifiers { session: None, transport }).await
        .map_err(|error| format!("House recovery: {error:?}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let length = u32::from_be_bytes(output.get(..4).ok_or("resume reply")?.try_into()?) as usize;
    let reply = crate::foundation::decode_wire_envelope(output.get(4..4 + length)
        .ok_or("complete resume reply")?).map_err(debug)?;
    assert_eq!(reply.message_type(), crate::foundation::MessageType::ServerResumeAccepted);
    if admitted.continuity.server_sequence == prior.continuity.server_sequence {
        assert_eq!(output.len(), 4 + length,
            "admission-only recovery writes exactly ServerResumeAccepted, without a synthetic command result");
    }
    Ok(admitted)
}

pub(super) async fn fresh(owner: &ComposedFreshAdmission<'_, '_, '_>, prior: AdmittedSession,
    nonce: u8, transport: u8) -> TestResult<AdmittedSession> {
    let token = super::RecoverySource::fresh_token(nonce)?;
    let character = prior.item_fence.ok_or("fresh character identity")?.character_id;
    let frame = oteryn_protocol_oteryn::encode_client_bootstrap(
        &oteryn_protocol_oteryn::ClientBootstrapValue {
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(character.as_bytes()).map_err(debug)?,
            admission_material: token.as_bytes(),
            client_build_id: "hireling-house-position",
            supported_capabilities: prior.continuity.selected_capabilities.as_slice(),
        }).map_err(debug)?;
    // A new identifier is supplied at the real connection-ID boundary. Only the
    // authenticated native fresh COMMIT can create that session or its lease.
    let candidate = crate::foundation::GameSessionId::decode(&super::id(nonce)).map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let admitted = crate::gameplay_transport::connection::admit_frame(&mut server, &frame,
        owner, &Identifiers { session: Some(candidate), transport }).await
        .map_err(|error| format!("House fresh authentication: {error:?}"))?;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let length = u32::from_be_bytes(output.get(..4).ok_or("fresh reply")?.try_into()?) as usize;
    let reply = crate::foundation::decode_wire_envelope(output.get(4..4 + length)
        .ok_or("complete fresh reply")?).map_err(debug)?;
    assert_eq!(reply.message_type(), crate::foundation::MessageType::ServerAccepted);
    Ok(admitted)
}

struct Identifiers { session: Option<crate::foundation::GameSessionId>, transport: u8 }
impl ConnectionIdentifiers for Identifiers {
    fn game_session_id(&self) -> Option<crate::foundation::GameSessionId> { self.session }
    fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
        crate::foundation::AuthenticatedTransportRefV1::decode(&[self.transport; 16]).ok()
    }
}
fn scalar(out: &mut Vec<u8>, tag: u8, mut value: u64) {
    out.push(tag << 3);
    while value >= 128 { out.push(value as u8 | 128); value >>= 7; }
    out.push(value as u8);
}
fn bytes(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    out.push(tag << 3 | 2);
    let mut length = value.len();
    while length >= 128 { out.push(length as u8 | 128); length >>= 7; }
    out.push(length as u8);
    out.extend_from_slice(value);
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub cell: (i32, i32, i16),
    pub lease: u64,
    pub sequence: u64,
    pub vitals: Option<(u32, u32)>,
}

pub(super) async fn snapshot(h: &Harness, admitted: AdmittedSession) -> TestResult<Snapshot> {
    let character = admitted.item_fence.ok_or("current House fence")?.character_id;
    let row: (i32, i32, i16, String, String, Option<i32>, Option<i32>) = sqlx::query_as(
        "SELECT tile_x,tile_y,tile_floor,character_lease_generation::text,write_sequence::text,health,mana \
         FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let vitals = match (row.5, row.6) {
        (Some(health), Some(mana)) => Some((u32::try_from(health)?, u32::try_from(mana)?)),
        (None, None) => None,
        _ => return Err("LastPosition cannot separate health and mana".into()),
    };
    Ok(Snapshot { cell: (row.0, row.1, row.2), lease: row.3.parse()?,
        sequence: row.4.parse()?, vitals })
}

async fn wound(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession,
    actor: ExactActorRef, health_loss: u32) -> TestResult<(u32, u32)> {
    // The test stages a living wounded actor through its native vitals owner;
    // it never writes the durable projection or manufactures a recovery row.
    let mut states = owner.states.spell_states.lock().await;
    let state = &mut states.actors.iter_mut()
        .find(|(a, session, _)| *a == actor && *session == admitted.game_session_id)
        .ok_or("House spell/vitals owner")?.2;
    let maxima = state.vitals();
    assert!(maxima.max_health > health_loss);
    // Use living values below both the original test actor's and the genuine
    // fresh Character's maxima, so a later cap clamp cannot mask lost vitals.
    state.restore_saved_vitals(maxima.max_health.min(20) - health_loss, 0);
    state.saved_vitals().ok_or_else(|| "wounded House actor must have saved vitals".into())
}
