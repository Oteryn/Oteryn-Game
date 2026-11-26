//! A busy Channel yields its real profile holder to the first admission.
use super::fixture::{Harness, debug, id};
use super::house_transfer_fixtures::RecoverySource;
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::loop_monster_fixtures::fixture::dependencies_with_capacity;
use super::{Dependencies, TestResult};
use crate::content::{ActiveGeneration, ProjectV2DefinitionRef, ProjectV2Family};
use crate::durability::{DurabilityRoot, ROOT_RECOVERY_WINDOW};
use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, ExactActorRef, GameSessionId, MovementLocalPosition,
};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::monster_profile_cycle::{
    PROFILE_CHECKOUT_BARRIER, PROFILES_PER_PASS,
};
use crate::gameplay_transport::{ComposedFreshAdmission, FreshAdmissionAttempt};
use oteryn_gameplay_durability::durability::native_world_read::qualified_combat_tile;
use oteryn_gameplay_entities::carrier::{NativeRaidSpawnSpec, RaidSpawnIdentity, RaidSpawnOutcome};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::future::{Future, poll_fn};
use std::sync::Arc;
use std::task::Poll;
use tokio::sync::Notify;

#[test]
fn saturated_monster_profiles_yield_to_fresh_admission_and_finish_later() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "monster_profile_admission", true, false)
                .await?;
        let result = exercise(&h).await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}

async fn exercise(h: &Harness) -> TestResult {
    let source = RecoverySource::start(h).await?;
    let now = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs(),
    )?;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityPublicationV1,
    };
    let mut guard_source = super::fresh_fixture::fresh::Source::new(now)?;
    guard_source
        .rows
        .retain(|row| matches!(row.key, AdmissionAuthorityGuardKeyV1::Runtime(_)));
    let publication =
        AdmissionAuthorityPublicationV1::prepare(&guard_source, now).map_err(debug)?;
    h.root
        .publish_runtime_readiness(&h.node, &publication)
        .await
        .map_err(debug)?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;

    let admission_deps = Dependencies::new()?;
    let mut admission = admission_deps.owner(h, &authority);
    admission.evidence = &source.evidence;
    let token = RecoverySource::fresh_token(91)?;
    let session = GameSessionId::decode(&id(91)).map_err(debug)?;
    let attempt = FreshAdmissionAttempt {
        character_id: CharacterId::decode(&id(41)).map_err(debug)?,
        admission_material: token.as_bytes(),
        game_session_id: session,
        transport: AuthenticatedTransportRefV1::decode(&[91; 16]).map_err(debug)?,
        supported_capabilities: &[],
    };

    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
    let active = controller.active().ok_or("active generation")?;
    let (profile_deps, _) = dependencies_with_capacity(active, &pin, PROFILES_PER_PASS * 3)?;
    let mut profile = profile_deps.owner(h, &authority);
    profile.active_generation = Some(active);
    profile.movement_cells = profile_deps.room.movement_cells().into();
    let actors = admit_due_hunters(&profile, &profile_deps, active).await?;
    let first = 10_000_000;
    let window = crate::ai_think::CREATUREAI0_THINK_WINDOW_MICROS;

    let barrier = Arc::new((Notify::new(), Notify::new()));
    let released = Notify::new();
    let running_profile = async {
        PROFILE_CHECKOUT_BARRIER
            .scope(
                Arc::clone(&barrier),
                DurabilityRoot::with_background_holder(
                    profile.drain_monster_melee_at(SemanticTimeMicros::from_micros(first)),
                ),
            )
            .await;
        released.notify_one();
    };
    let admitting = async {
        barrier.0.notified().await;
        assert!(
            !h.root.is_ready(),
            "the Monster profile holds the real checkout"
        );
        let mut entering = Box::pin(admission.admit(attempt));
        assert!(
            poll_fn(|cx| Poll::Ready(entering.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        barrier.1.notify_one();
        released.notified().await;

        let completed = completed_profiles(&profile, &actors).await;
        // The fixture also has one independently scheduled Rat.
        assert!((PROFILES_PER_PASS - 1..=PROFILES_PER_PASS).contains(&completed));
        assert!(
            completed < actors.len(),
            "one holder pass must leave an ordered due suffix"
        );
        DurabilityRoot::with_background_holder(
            profile.drain_monster_melee_at(SemanticTimeMicros::from_micros(first + window)),
        )
        .await;
        assert_eq!(
            completed_profiles(&profile, &actors).await,
            completed,
            "the next scheduled checkout must yield to the already waiting admission"
        );
        let admitted = entering.await.map_err(debug)?;
        assert_eq!(admitted.game_session_id, session);
        assert!(matches!(
            admitted.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ));
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(ROOT_RECOVERY_WINDOW, async {
        tokio::join!(Box::pin(running_profile), Box::pin(admitting)).1
    })
    .await??;

    for turn in 2..=actors.len().div_ceil(PROFILES_PER_PASS) + 2 {
        DurabilityRoot::with_background_holder(profile.drain_monster_melee_at(
            SemanticTimeMicros::from_micros(first + u64::try_from(turn)? * window),
        ))
        .await;
    }
    assert_eq!(completed_profiles(&profile, &actors).await, actors.len());
    let states = profile.states.spell_states.lock().await;
    for actor in actors {
        let report = states
            .monster_melee
            .creature_ai
            .last_report_for_test(actor)
            .ok_or("completed profile")?;
        assert_eq!(
            report.sequence, 1,
            "the due profile must be applied exactly once"
        );
    }
    eprintln!("PG_EXECUTED saturated_monster_profiles_yield_to_fresh_admission_and_finish_later");
    Ok(())
}

async fn completed_profiles(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actors: &[ExactActorRef],
) -> usize {
    let states = owner.states.spell_states.lock().await;
    actors
        .iter()
        .filter(|actor| {
            states
                .monster_melee
                .creature_ai
                .last_report_for_test(**actor)
                .is_some()
        })
        .count()
}

async fn admit_due_hunters(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    active: &ActiveGeneration,
) -> TestResult<Vec<ExactActorRef>> {
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let mut runtime = deps.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let objects = deps.door.lock().await;
    let table = &mut states.monster_melee.creature_ai;
    let policy = runtime
        .companion_policy("oteryn:creature.hunter")
        .map_err(debug)?;
    let creature = ProjectV2DefinitionRef {
        family: ProjectV2Family::Creature,
        key: policy.definition_key.clone(),
        revision: policy.definition_revision.clone(),
    };
    let spawn = NativeRaidSpawnSpec::qualified(
        &creature.key,
        &creature.revision,
        native.source_digest(),
        false,
    )
    .map_err(debug)?;
    let viewer = runtime
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    let mut actors = Vec::new();
    for at in (-8..=12).flat_map(|x| {
        (4..=16).map(move |y| MovementLocalPosition {
            x,
            y,
            floor: viewer.floor,
        })
    }) {
        let admits = |runtime: &crate::foundation::ChannelRuntimeV1, at| {
            Some(
                qualified_combat_tile(&deps.room, runtime, &*objects, at).is_ok_and(|tile| {
                    tile.ground_present()
                        && !tile.block_solid()
                        && !tile.floor_change()
                        && !tile.protection_zone()
                }),
            )
        };
        if !crate::ai_think::targeting::sees_native(at, viewer)
            || admits(&runtime, at) != Some(true)
            || runtime
                .position_occupied_by_other(deps.actor, at)
                .map_err(debug)?
            || at == viewer
        {
            continue;
        }
        let stamp = runtime.issue_owner_work().map_err(debug)?;
        let receipt = runtime
            .realize_raid_spawn(
                stamp,
                RaidSpawnIdentity {
                    firing_id: *owner.world_id.as_bytes(),
                    wave: 0,
                    entry: 0,
                    index: u16::try_from(actors.len())?,
                },
                &spawn,
                Some(at),
                admits,
            )
            .map_err(debug)?;
        let RaidSpawnOutcome::Placed(actor) = receipt.outcome else {
            return Err("qualified Hunter spawn refused".into());
        };
        table
            .admit_native_creature(&runtime, native, actor, &creature, 0)
            .map_err(debug)?;
        assert_eq!(table.due(actor), Some(0));
        actors.push(actor);
        if actors.len() == PROFILES_PER_PASS * 2 + 1 {
            return Ok(actors);
        }
    }
    Err("qualified World fixture lacks enough visible cells for a saturated profile pass".into())
}
