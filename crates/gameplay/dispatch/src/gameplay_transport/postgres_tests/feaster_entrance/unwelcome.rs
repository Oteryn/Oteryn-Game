//! The real native death publication closes only the boss in the Worm's phase.
use super::*;
use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
use crate::gameplay_transport::{
    connection::UseCommand,
    encounter_phases,
    kill_reward::{KillSink, record_creature_kill},
};
use crate::map::view::MapFacts;
use oteryn_gameplay_encounters::bosses::phase_controller::unwelcome::{BROTHER_WORM, UNWELCOME};
use oteryn_gameplay_quests::quest::world_callbacks::EncounterPhase;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

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
    h.seed_appearance_observer().await?;
    super::super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
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
        |key| {
            let mut item = boot::tests::items(key)?;
            if key == "item:box" {
                item.solid = Some(false);
                item.blocks_projectile = false;
            }
            Some(item)
        },
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
        Dependencies::with_content_and_capacity(pin, room.clone(), initial.spells.clone(), 5)?;
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
    let observer_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let observer = {
        let mut runtime = deps.runtime.lock().await;
        let at = |x| MovementLocalPosition { x, y: 0, floor: 7 };
        runtime
            .initialize_pinned_test_position(actor, at(6))
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, lease)
            .map_err(debug)?;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(observer_session)
            .map_err(debug)?;
        let observer = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(observer, at(7))
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                observer,
                observer_session,
                CharacterLease::new(CharacterId::decode(&id(101)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        for (actor, session) in [(actor, session), (observer, observer_session)] {
            assert!(
                states
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            level: 250,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }
        observer
    };
    let callback: Callback = serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/feaster-unwelcome-fixture",
        "quest":"oteryn:quest.feaster_of_souls_quest",
        "trigger":"use",
        "selector":{"position":{"x":7,"y":0,"floor":-7},"ordinal":1},
        "passages":[],"offerings":[],"storage":[],"reversible":false,
        "effects":{"encounter":{
            "minimum_level":250,
            "players":[
                {"from":{"x":6,"y":0,"floor":-7},"to":{"x":1,"y":0,"floor":-7}},
                {"from":{"x":7,"y":0,"floor":-7},"to":{"x":1,"y":0,"floor":-7}}
            ],
            "monsters":[
                {"position":{"x":2,"y":0,"floor":-7},"creature":UNWELCOME},
                {"position":{"x":3,"y":0,"floor":-7},"creature":BROTHER_WORM}
            ],
            "room_min":{"x":1,"y":0,"floor":-7},"room_max":{"x":3,"y":0,"floor":-7},
            "phase":EncounterPhase::Unwelcome
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/feaster_of_souls/actions_the_unwelcome_lever.lua:1-44"]
    }))?;
    assert!(
        owner
            .states
            .quest_callbacks
            .fixture
            .set(vec![callback])
            .is_ok()
    );
    let map = owner.bundle_map().ok_or("qualified map")?;
    let key = oteryn_world_bundle::bundle::placement_key(-7, 7, 0, 1).ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let before_use = owner.owner_now().get();
    let result = owner
        .use_world_object(
            actor,
            UseCommand {
                game_session_id: session,
                command_id: 1,
                item_fence: Some(fence.item_fence()),
            },
            WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await;
    assert_eq!(result.disposition, UseDisposition::Committed);
    let after_use = owner.owner_now().get();
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = deps.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    for participant in [actor, observer] {
        assert_eq!(
            runtime
                .read_actor_position(participant)
                .map_err(debug)?
                .position()
                .x,
            1
        );
    }
    let [boss] = runtime
        .current_live_creature_census()
        .map_err(debug)?
        .try_into()
        .map_err(|_| "immediate The Unwelcome")?;
    assert!(runtime.matches_live_creature_identity(boss, UNWELCOME.as_bytes()));
    let health = runtime
        .companion_policy(UNWELCOME)
        .map_err(debug)?
        .maximum_health;
    let [decoy] = runtime
        .commit_quest_creature_spawn(&[(
            UNWELCOME,
            health,
            MovementLocalPosition {
                x: 6,
                y: 0,
                floor: 7,
            },
        )])
        .map_err(debug)?
        .try_into()
        .map_err(|_| "independent The Unwelcome")?;
    encounter_phases::tick(&mut runtime, &states, before_use + 19_999_999);
    assert_eq!(
        runtime.current_live_creature_census().map_err(debug)?.len(),
        2
    );
    encounter_phases::tick(&mut runtime, &states, after_use + 20_000_000);
    let now = owner.owner_now();
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
    assert!(runtime.retains_corpse_projection(boss));
    assert!(runtime.contains_live_creature(decoy));
    drop(states);
    drop(runtime);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
