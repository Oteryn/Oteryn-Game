//! Boreth's committed lethal hit drives the live kill settlement callback.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::combat::CapturedRewardPrincipal;
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition, NodeId, OwnerDamageCommand,
};
use crate::gameplay_transport::kill_reward::{KillSettlementQueue, KillSink, record_creature_kill};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

#[test]
fn boreth_lethal_callback_cleans_only_room_plaguethrowers_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("boreth_cleanup", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "boreth_cleanup", false, false).await?;
            let result = exercise(&h).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup?;
            Ok(())
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let deps = Dependencies::new()?;
    let session = deps.admitted.game_session_id;
    let world = deps.admitted.world_id;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        deps.admitted.channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        16,
        ChannelContentPin::from_activation(
            world,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            [4; 32],
            (32940, 31479, 1),
        ),
    )
    .map_err(debug)?;
    let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session)
        .await?;
    let lease = current.current_character_lease();
    runtime
        .bind_attacker_lease(actor, session, lease)
        .map_err(debug)?;
    runtime
        .initialize_first_entry_position(actor)
        .map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let mut spawn = |key: &str, x, y, floor| -> TestResult<_> {
        let health = runtime.companion_policy(key).map_err(debug)?.maximum_health;
        let target = runtime
            .admit_encounter_actor(key, health, MovementLocalPosition { x, y, floor })
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, key)
            .map_err(debug)?;
        Ok((target, health))
    };
    let boreth = "oteryn:creature.boreth";
    let add = "oteryn:creature.plaguethrower";
    let (boss, health) = spawn(boreth, 32940, 31478, 1)?;
    let (near, _) = spawn(add, 32936, 31474, 1)?;
    let (far_edge, _) = spawn(add, 32944, 31482, 1)?;
    let (outside, _) = spawn(add, 32945, 31482, 1)?;
    let (upstairs, _) = spawn(add, 32940, 31478, 2)?;
    let (rat, _) = spawn("oteryn:creature.rat", 32940, 31477, 1)?;
    let now = 1;
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            boss,
            actor,
            CommandRef::new(session, CommandId::new(10).map_err(debug)?),
            0,
            OwnerDamageCommand {
                target: boreth.as_bytes(),
                occurrence: session.as_bytes(),
                binding: b"boreth-cleanup",
                damage: health,
            },
        )
        .map_err(debug)?;
    let queue = KillSettlementQueue::default();
    record_creature_kill(
        &mut runtime,
        KillSink {
            prey: None,
            activity: &[],
            queue: &queue,
            rewards: native.reward_table(),
            charms: None,
        },
        boss,
        CapturedRewardPrincipal {
            character: lease.character_id(),
            lease_generation: lease.generation(),
            session,
            actor,
        },
        now,
        None,
    );
    assert!(!runtime.contains_live_creature(near));
    assert!(!runtime.contains_live_creature(far_edge));
    for survivor in [outside, upstairs, rat] {
        assert!(runtime.contains_live_creature(survivor));
    }
    Ok(())
}
