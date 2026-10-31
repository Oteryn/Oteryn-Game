//! W15c portals through command-1 movement, native policy admission and PostgreSQL.
//! The fixture supplies walkable source coordinates; room qualification is the real
//! pinned Crystal map evidence, never a test override of the portal's authority.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::character_last_position::LastPositionWrite;
use crate::durability::quest_state::grizzly::NoGrizzlyPremium;
use crate::durability::quest_state::grizzly_boss::GrizzlyBossEntryState;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome};
use crate::foundation::{ChannelRuntimeV1, CharacterLease, CommandId, CommandRef,
    ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use crate::gameplay_transport::last_position::AdmissionPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::grizzly_adams::{self as quest, GrizzlyAction as Action};
use std::sync::Arc;

#[path = "grizzly_boss/combat.rs"]
mod combat;
#[path = "grizzly_boss/world.rs"]
mod world;
#[path = "grizzly_boss/vacancy.rs"]
mod vacancy;

#[test]
fn grizzly_boss_portal_wire_occupied_spawn_expiry_and_relog_return() -> TestResult {
    postgres_case(1, false)
}

#[test]
fn grizzly_boss_portal_wire_unqualified_room_preserves_assignment() -> TestResult {
    postgres_case(2, false)
}

#[test]
fn grizzly_boss_committed_expiry_without_reply_retries_online_return() -> TestResult {
    postgres_case(1, true)
}

fn postgres_case(boss: u8, unknown_expiry: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("GRIZZLY_BOSS_POSTGRES_CASE_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(boss, unknown_expiry, None).map_err(|error| error.to_string()))?.join()
        .map_err(|_| "Grizzly boss portal PostgreSQL thread panicked")??;
    Ok(())
}

fn local(position: quest::GrizzlyPosition) -> MovementLocalPosition {
    MovementLocalPosition { x: i32::from(position.x), y: i32::from(position.y), floor: i16::from(position.z) }
}

fn scenario(boss_id: u8, unknown_expiry: bool, vacancy: Option<vacancy::Case>) -> TestResult {
    let boss = quest::boss(boss_id).ok_or("boss")?;
    let origin = MovementLocalPosition { x: i32::from(boss.portal.x) - 1, ..local(boss.portal) };
    let world = world::source_rooms(if vacancy.is_some() { vec![boss, quest::boss(3).ok_or("second room")?] } else { vec![boss] })?;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "grizzly_boss_wire", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id, "content-1")?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native content")?;
        let cells = world.movement_cells(activated.qualified_room().movement_cells()).map_err(debug)?;
        let pin = ChannelContentPin::from_activation(deps.admitted.world_id, 1,
            generation.identity().server_artifact_digest(), generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(), world.map_revision_digest(), (origin.x, origin.y, origin.floor));
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(deps.admitted.world_id,
            deps.admitted.channel_id, crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1, 1, 1, "runtime-scope-assignment:1", 8, pin).map_err(debug)?;
        native.install_companion_policies(&mut runtime).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_pinned_test_position(deps.actor, origin).map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?.catalogue().clone()));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner.current_quest_fence(admitted.game_session_id).await
            .map_err(|()| "fence")?.ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        deps.runtime.lock().await.bind_attacker_lease(deps.actor, admitted.game_session_id,
            CharacterLease::new(crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?, fence.character_lease_generation).map_err(debug)?).map_err(debug)?;
        world::activate(&h, generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(), activated.frame_binding().digest()).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root.initialize_admission_equipment(&authority, &h.node, &fence.item_fence(),
            generation.identity().server_artifact_digest(), fresh.operation()).await.map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        assert!(owner.track_restored(admitted.game_session_id, deps.actor, &AdmissionPosition::FirstLogin, fence));
        action(&owner, 100, Action::Join).await?;
        world::boss_points(&h).await?;
        action(&owner, 101, Action::SelectBoss(boss_id)).await?;
        owner.refresh_quest_session(admitted.game_session_id, false).await;
        assert_eq!(value(&owner, boss.storage).await?, 1);
        if boss_id == 2 {
            // Hide's source room lacks the required complete map bounds (Q2=B).
            east(&owner, &mut admitted, 1).await?;
            assert_position(&owner, deps.actor, origin).await?;
            assert_eq!(value(&owner, boss.storage).await?, 1);
            assert_eq!(visits(&h).await?, 0);
            assert!(monsters(&owner).await?.is_empty());
        } else {
            qualified(&owner, &mut admitted, &h, &deps, boss, origin, vacancy.is_none()).await?;
            if let Some(case) = vacancy {
                vacancy::qualify(&owner, &mut admitted, &h, &deps, boss, origin, case).await?;
                drop(owner);
                eprintln!("GRIZZLY_BOSS_VACANCY POSTGRES_CASE_COMPLETED {case:?}");
                drop(authority); drop(seal); return h.cleanup().await;
            }
            if unknown_expiry {
                unacknowledged_expiry(&owner, &h, deps.actor, admitted.game_session_id, boss, origin).await?;
                drop(owner);
                eprintln!("GRIZZLY_BOSS_UNKNOWN_EXPIRY POSTGRES_CASE_COMPLETED");
                drop(authority); drop(seal); return h.cleanup().await;
            }
            // Save the live inside-room position through the actual position owner,
            // then lose the physical actor. Expiry must retain an offline return.
            let captured = owner.final_last_position(admitted.game_session_id, deps.actor)
                .await.ok_or("inside position")?;
            let current = owner.current_quest_fence(admitted.game_session_id).await
                .map_err(|()| "fence")?.ok_or("fence")?;
            assert_eq!(h.root.write_character_last_position(&authority, &h.node, current,
                captured.write_sequence, captured.record).await.map_err(debug)?, LastPositionWrite::Written);
            deps.runtime.lock().await.remove_test_actor(deps.actor).map_err(debug)?;
            world::expire(&h).await?;
            assert!(owner.grizzly_boss_pass().await.map_err(debug)?);
            assert!(monsters(&owner).await?.is_empty());
            let pending = h.root.pending_grizzly_boss_entry(&authority, fence.character_id)
                .await.map_err(debug)?.ok_or("offline return")?;
            assert_eq!(pending.state, GrizzlyBossEntryState::ReturnPending);
            assert_eq!(pending.exit, origin);
            assert_eq!(value(&owner, boss.storage).await?, 2);
            let original_deadline = pending.deadline_unix_ms;
            // Rebuild the runtime actor and every local session projection. Admission
            // reads the saved room cell; reconciliation returns it before new input.
            drop(owner);
            let reservation = deps.runtime.lock().await.reserve_fresh_session(admitted.game_session_id).map_err(debug)?;
            deps.actor = deps.runtime.lock().await.commit_fresh_session(reservation).map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = &cells;
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?.catalogue().clone()));
            let current = owner.current_quest_fence(admitted.game_session_id).await
                .map_err(|()| "fence")?.ok_or("fence")?;
            let decided = owner.read_admission_position(current).await.map_err(|()| "admission position")?;
            assert!(matches!(decided, AdmissionPosition::Stored(_)));
            {
                let mut runtime = deps.runtime.lock().await;
                owner.initialize_admitted_position(&mut runtime, deps.actor, admitted.game_session_id, &decided)
                    .await.map_err(debug)?;
                runtime.bind_attacker_lease(deps.actor, admitted.game_session_id,
                    CharacterLease::new(crate::foundation::CharacterId::decode(current.character_id.as_bytes())
                        .map_err(debug)?, current.character_lease_generation).map_err(debug)?).map_err(debug)?;
            }
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            owner.admit_quest_session(&admitted).await;
            assert!(owner.track_restored(admitted.game_session_id, deps.actor, &decided, current));
            assert_position(&owner, deps.actor, local(boss.player_position)).await?;
            assert!(owner.reconcile_grizzly_boss(admitted.game_session_id).await);
            assert_position(&owner, deps.actor, origin).await?;
            assert!(h.root.pending_grizzly_boss_entry(&authority, current.character_id).await.map_err(debug)?.is_none());
            let stored = h.root.read_character_last_position(&authority, &h.node, current)
                .await.map_err(debug)?.ok_or("returned durable position")?;
            assert_eq!((stored.record.cell.x, stored.record.cell.y, stored.record.cell.floor),
                (origin.x, origin.y, origin.floor));
            assert_eq!(value(&owner, boss.storage).await?, 2);
            let deadline: i64 = sqlx::query_scalar("SELECT deadline_at FROM game_character_grizzly_boss_entries")
                .fetch_one(&h.pool).await?;
            assert_eq!(deadline, original_deadline, "re-admission must never restart the timer");
            assert!(owner.reconcile_grizzly_boss(admitted.game_session_id).await);
            assert_eq!(visits(&h).await?, 1);
            drop(owner);
            eprintln!("GRIZZLY_BOSS_PORTAL POSTGRES_CASE_COMPLETED boss={boss_id}");
            drop(authority); drop(seal); return h.cleanup().await;
        }
        drop(owner);
        eprintln!("GRIZZLY_BOSS_PORTAL POSTGRES_CASE_COMPLETED boss={boss_id}");
        drop(authority); drop(seal); h.cleanup().await
    })
}

async fn unacknowledged_expiry(owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness,
    actor: ExactActorRef, session: GameSessionId, boss: &quest::GrizzlyBoss,
    origin: MovementLocalPosition) -> TestResult {
    assert!(owner.reconcile_grizzly_boss(session).await);
    world::expire(h).await?;
    let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
    let entry = h.root.pending_grizzly_boss_entry(owner.character, fence.character_id)
        .await.map_err(debug)?.ok_or("expiring visit")?;
    // This is the actual pre-COMMIT recovery hook, with no fabricated authority.
    // Discarding the response below models an expiry that committed but whose
    // caller never observed it; subsequent ACTIVE scans cannot find this row.
    owner.remember_grizzly_return(actor, session, &entry);
    let binding = owner.runtime.lock().await.binding();
    assert!(h.root.expire_grizzly_boss_entry(owner.character, owner.holder, binding.world_id(),
        binding.channel_id(), binding.scope_generation(), &entry).await.map_err(debug)?);
    assert!(h.root.due_grizzly_boss_entries(owner.character, owner.holder, binding.world_id(),
        binding.channel_id(), binding.scope_generation()).await.map_err(debug)?.is_empty());
    assert_position(owner, actor, local(boss.player_position)).await?;
    assert!(owner.grizzly_boss_pass().await.map_err(debug)?);
    assert_position(owner, actor, origin).await?;
    assert!(h.root.pending_grizzly_boss_entry(owner.character, fence.character_id).await.map_err(debug)?.is_none());
    let stored = h.root.read_character_last_position(owner.character, owner.holder, fence)
        .await.map_err(debug)?.ok_or("durable returned position")?;
    assert_eq!((stored.record.cell.x, stored.record.cell.y, stored.record.cell.floor),
        (origin.x, origin.y, origin.floor));
    assert!(owner.grizzly_boss_pass().await.map_err(debug)?);
    assert_eq!(value(owner, boss.storage).await?, 2);
    assert_eq!(visits(h).await?, 1);
    Ok(())
}

async fn qualified(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    h: &Harness, deps: &Dependencies, boss: &quest::GrizzlyBoss, origin: MovementLocalPosition, combat: bool) -> TestResult {
    let actor = deps.actor;
    // A physical player obstructs the qualified room. Its presence is a refusal,
    // never a grant of quest or session authority to the entering Character.
    let obstacle = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(GameSessionId::decode(&id(51)).map_err(debug)?).map_err(debug)?;
        let obstacle = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.initialize_pinned_test_position(obstacle, local(boss.player_position)).map_err(debug)?;
        obstacle
    };
    east(owner, admitted, 1).await?;
    assert_position(owner, actor, origin).await?;
    assert_eq!(value(owner, boss.storage).await?, 1);
    assert_eq!(visits(h).await?, 0);
    owner.runtime.lock().await.remove_test_actor(obstacle).map_err(debug)?;
    east(owner, admitted, 2).await?;
    assert_position(owner, actor, local(boss.player_position)).await?;
    assert_eq!(value(owner, boss.storage).await?, 2);
    let entry = h.root.pending_grizzly_boss_entry(owner.character,
        crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?)
        .await.map_err(debug)?.ok_or("active room visit")?;
    assert_eq!(entry.state, GrizzlyBossEntryState::Active);
    assert_eq!(entry.exit, origin);
    assert_eq!(entry.deadline_unix_ms - entry.entered_at_unix_ms, 600_000);
    let spawned = monsters(owner).await?;
    assert_eq!(spawned.len(), 1);
    let runtime = owner.runtime.lock().await;
    let creature = runtime.companion_snapshot(spawned[0]).map_err(debug)?;
    assert_eq!(creature.state.policy.definition_key, boss.creature_key);
    assert_eq!(creature.health, creature.state.policy.maximum_health);
    assert_eq!(runtime.read_actor_position(spawned[0]).map_err(debug)?.position(), local(boss.boss_position));
    drop(runtime);
    // The wire owner's command replay cannot create another physical boss or receipt.
    east(owner, admitted, 2).await?;
    assert_eq!(monsters(owner).await?, spawned);
    assert_eq!(visits(h).await?, 1);
    if combat {
        combat::kill(owner, h, actor, admitted.game_session_id, spawned[0], boss).await?;
        combat::ordinary(owner, h, deps, actor, admitted.game_session_id).await?;
    }
    Ok(())
}

async fn action(owner: &ComposedFreshAdmission<'_, '_, '_>, n: u64, action: Action) -> TestResult {
    let session = GameSessionId::decode(&id(50)).map_err(debug)?;
    let fence = owner.current_quest_fence(session).await.map_err(|()| "fence")?.ok_or("fence")?;
    let outcome = owner.revision_sequencer.acquire(fence.character_id).await.commit_grizzly_transition(
        owner.root, owner.character, owner.holder, fence,
        QuestCause::Command(CommandRef::new(session, CommandId::new(n).map_err(debug)?)),
        action, Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?), &NoGrizzlyPremium)
        .await.map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)), "{outcome:?}");
    Ok(())
}
async fn value(owner: &ComposedFreshAdmission<'_, '_, '_>, storage: u32) -> TestResult<i64> {
    let copy = owner.root.read_character_quest_state(owner.character,
        crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?).await.map_err(debug)?;
    Ok(quest::storage_value(copy.tracks(), storage))
}
async fn visits(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_grizzly_boss_entries").fetch_one(&h.pool).await?)
}
async fn monsters(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<Vec<ExactActorRef>> {
    Ok(owner.runtime.lock().await.positioned_melee_census().map_err(debug)?
        .into_iter().filter(|(_, _, session)| session.is_none()).map(|(actor, _, _)| actor).collect())
}
async fn assert_position(owner: &ComposedFreshAdmission<'_, '_, '_>, actor: ExactActorRef,
    expected: MovementLocalPosition) -> TestResult {
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), expected);
    Ok(())
}
async fn east(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, n: u64) -> TestResult {
    super::quest_world_callbacks::wire(owner, admitted, n, 1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(
            oteryn_protocol_oteryn::world_spatial::StepDirection::East)).await?;
    Ok(())
}
