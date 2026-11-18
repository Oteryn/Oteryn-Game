//! The real native death publication closes only the boss in the Worm's phase.
use super::*;
use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
use crate::gameplay_transport::{
    encounter_phases,
    kill_reward::{KillSink, record_creature_kill},
};
use oteryn_gameplay_encounters::bosses::phase_controller::unwelcome::{BROTHER_WORM, UNWELCOME};
use oteryn_gameplay_quests::quest::world_callbacks::EncounterPhase;

#[test]
fn feaster_unwelcome_native_worm_death_closes_only_its_linked_boss() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::super::kill_reward_e2e::scenario_thread::run_async(exercise)
}

async fn exercise(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "feaster_unwelcome", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let initial = Dependencies::new()?;
    let world_id = initial.admitted.world_id;
    let (controller, activated) =
        super::super::death_ml_loss_composed::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native")?;
    let room = activated.qualified_room();
    let (bytes, pins) = boot::tests::bundle();
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let world = boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
        world_id,
        initial.admitted.channel_id,
        boot::tests::items,
    )
    .map_err(debug)?;
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        world.frame_binding_digest(world_id),
        world.map_revision_digest(),
        (1, 0, 7),
    );
    let deps =
        Dependencies::with_content_and_capacity(pin, room.clone(), initial.spells.clone(), 4)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    let session = deps.admitted.game_session_id;
    let actor = deps.actor;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "session fence")?
        .ok_or("current session fence")?;
    let lease = CharacterLease::new(
        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
        fence.character_lease_generation,
    )
    .map_err(debug)?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, lease)
            .map_err(debug)?;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
    }
    deps.initialize(&owner).await;
    let mut callback = portal(None)?;
    callback.effects.encounter = Some(serde_json::from_value(serde_json::json!({
        "minimum_level":250,"players":[{"from":{"x":1,"y":0,"floor":-7},"to":{"x":2,"y":0,"floor":-7}}],
        "monsters":[
            {"position":{"x":2,"y":0,"floor":-7},"creature":UNWELCOME},
            {"position":{"x":3,"y":0,"floor":-7},"creature":BROTHER_WORM}
        ],
        "room_min":{"x":2,"y":0,"floor":-7},"room_max":{"x":3,"y":0,"floor":-7},
        "phase":EncounterPhase::Unwelcome
    }))?);
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = deps.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let health = runtime
        .companion_policy(UNWELCOME)
        .map_err(debug)?
        .maximum_health;
    let at = |x| MovementLocalPosition { x, y: 0, floor: 7 };
    let foes = runtime
        .commit_quest_creature_spawn(&[(UNWELCOME, health, at(2)), (UNWELCOME, health, at(7))])
        .map_err(debug)?;
    let now = owner.owner_now();
    let key = oteryn_world_bundle::bundle::placement_key(-7, 1, 0, 0).ok_or("source placement")?;
    let phase = encounter_phases::bind(
        &mut runtime,
        &callback,
        key,
        &foes[..1],
        &[(actor, session)],
        Vec::new(),
        cells.clone(),
        now.get(),
    )
    .map_err(debug)?;
    states
        .encounter_phases
        .reserve(&mut runtime, key, now.get())
        .map_err(debug)?
        .push(phase);
    encounter_phases::tick(&mut runtime, &states, now.get() + 19_999_999);
    assert_eq!(
        runtime.current_live_creature_census().map_err(debug)?.len(),
        2
    );
    encounter_phases::tick(&mut runtime, &states, now.get() + 20_000_000);
    let worm = runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .find(|foe| runtime.matches_live_creature_identity(*foe, BROTHER_WORM.as_bytes()))
        .ok_or("scheduled Brother Worm")?;
    let health = runtime
        .live_creature_health(worm)
        .ok_or("Brother Worm health")?;
    runtime
        .borrow_exact_actor_commit_at(now.get() / 1000)
        .commit_damage_for_bound_attacker(
            worm,
            actor,
            CommandRef::new(session, CommandId::new(90).map_err(debug)?),
            0,
            OwnerDamageCommand {
                target: BROTHER_WORM.as_bytes(),
                occurrence: session.as_bytes(),
                binding: b"feaster-brother-worm",
                damage: i64::try_from(health)?,
            },
        )
        .map_err(debug)?;
    record_creature_kill(
        &mut runtime,
        KillSink {
            prey: Some(&states),
            activity: &[],
            queue: &queue,
            rewards: native.reward_table(),
            charms: None,
        },
        worm,
        crate::combat::CapturedRewardPrincipal {
            character: lease.character_id(),
            lease_generation: lease.generation(),
            session,
            actor,
        },
        now.get() / 1000,
        Some(now),
    );
    assert!(runtime.retains_corpse_projection(foes[0]));
    assert!(runtime.contains_live_creature(foes[1]));
    drop(states);
    drop(runtime);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
