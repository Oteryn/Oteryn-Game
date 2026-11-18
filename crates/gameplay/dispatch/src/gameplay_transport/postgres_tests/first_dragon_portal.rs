//! Exercise the registered sapphire UID through the ordinary movement wire command.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

pub(super) const NAME: &str = "first-dragon-entry-35004";

#[test]
fn first_dragon_descendant_step_requires_progress_and_persists_cooldown_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, Position { x: 32075, y: 32402, floor: -8 }, false)
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    origin: MovementLocalPosition,
) -> TestResult {
    use super::quest_world_callbacks::{local, wire};

    let step = encode_step_intent(StepDirection::East);
    wire(owner, admitted, 1, 1, step.clone()).await?;
    let actor = admitted.runtime_actor.ok_or("portal actor")?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), origin);
    let requirements: Vec<_> = callback.storage.iter()
        .map(|state| (state.track.clone(), state.minimum))
        .collect();
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h, owner, admitted, &requirements, 105,
    ).await?;
    owner.admit_quest_session(admitted).await;
    wire(owner, admitted, 2, 1, step).await?;
    let encounter = callback.effects.encounter.as_ref().ok_or("descendant encounter")?;
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime.read_actor_position(actor).map_err(debug)?.position(),
        local(encounter.step_destination.ok_or("portal destination")?)
    );
    let Some(monster) = encounter.monsters.first() else { return Err("descendant boss".into()); };
    assert!(runtime.positioned_melee_census().map_err(debug)?.iter().any(|(_, position, session)| {
        session.is_none() && position.position() == local(monster.position)
    }));
    drop(runtime);
    let clock: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2")
        .bind(&callback.quest).bind(&callback.state_operations[0].track)
        .fetch_one(&h.pool).await?;
    assert!(clock > 0);
    Ok(())
}

pub(super) const BOWL: &str = "first-dragon-flower-bowl";
pub(super) const FLOWER: &str = "first-dragon-heaven-blossom";

#[test]
fn first_dragon_native_monster_tiles_transform_source_creatures_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(FLOWER, || {
        let target = Position { x: 33625, y: 31021, floor: -14 };
        super::quest_world_callbacks::scenario(FLOWER, target, false)?;
        super::quest_world_callbacks::scenario(BOWL, target, false)
    })
}

pub(super) async fn exercise_monster_tile(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: Position,
    consume: bool,
) -> TestResult {
    use crate::map::view::MapFacts;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

    let (source, replacement) = if consume {
        ("oteryn:creature.spirit_of_fertility", "oteryn:creature.angry_plant")
    } else {
        ("oteryn:creature.unbeatable_dragon", "oteryn:creature.somewhat_beatable")
    };
    let tile = super::quest_world_callbacks::local(target);
    let from = MovementLocalPosition { y: tile.y + 1, ..tile };
    let map = owner.bundle_map().ok_or("blossom bundle")?;
    let mut runtime = owner.runtime.lock().await;
    let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native policies")?;
    native.install_companion_policies(&mut runtime).map_err(debug)?;
    let health = runtime.companion_policy(source).map_err(debug)?.maximum_health;
    let spirits = runtime.commit_quest_creature_spawn(&[(source, health, from)]).map_err(debug)?;
    let spirit = *spirits.first().ok_or("spawned spirit")?;
    runtime.commit_quest_platform_displacement(from, tile, |_, _, _| true, || {
        map.facts.record_monster_step(crate::map::facts::MonsterStep { actor: spirit, from, to: tile })
    }).map_err(debug)?;
    let states = owner.states.spell_states.lock().await;
    crate::gameplay_transport::paced_movement::monster_trap::apply_monster_trap_steps(
        &mut runtime, &states, map, 1_000_000,
    );
    assert!(!runtime.contains_live_creature(spirit));
    assert!(runtime.positioned_melee_census().map_err(debug)?.iter().any(|(actor, position, session)| {
        session.is_none() && position.position() == tile
            && runtime.companion_snapshot(*actor).is_ok_and(|snapshot| snapshot.state.policy.definition_key == replacement)
    }));
    let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, 1).ok_or("blossom placement")?;
    assert_eq!(map.facts.base_hidden(key), consume);
    if consume {
        let plant = runtime.visible_entities().creatures.into_iter().find(|creature| {
            runtime.companion_snapshot(creature.actor).is_ok_and(|snapshot| {
                snapshot.state.policy.definition_key == "oteryn:creature.angry_plant"
            })
        }).ok_or("transformed plant")?.actor;
        let actor = admitted.runtime_actor.ok_or("plant attacker")?;
        let session = admitted.game_session_id;
        let lease = runtime.bound_attacker_lease(actor, session).map_err(debug)?;
        let now = owner.owner_now();
        let health = runtime.companion_snapshot(plant).map_err(debug)?.health;
        runtime.borrow_exact_actor_commit_at(now.get() / 1000).commit_damage_for_bound_attacker(
            plant, actor, crate::foundation::CommandRef::new(session, crate::foundation::CommandId::new(39).map_err(debug)?),
            0, crate::foundation::OwnerDamageCommand {
                target: b"oteryn:creature.angry_plant",
                occurrence: session.as_bytes(),
                binding: b"first-dragon-angry-plant-death",
                damage: health,
            },
        ).map_err(debug)?;
        let queue = owner.states.attack.lock().await.kills().clone();
        crate::gameplay_transport::kill_reward::record_creature_kill(
            &mut runtime,
            crate::gameplay_transport::kill_reward::KillSink {
                prey: Some(&states), activity: &[], queue: &queue,
                rewards: native.reward_table(), charms: None,
            },
            plant,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(), lease_generation: lease.generation(),
                session, actor,
            },
            now.get() / 1000,
            Some(now),
        );
        drop(states);
        drop(runtime);
        assert_eq!(owner.drain_kill_rewards(session).await, crate::gameplay_transport::kill_reward::DrainOutcome::Drained);
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.bound_corpse(plant.corpse_identity()).ok_or("bound plant corpse")?.item.production_key, "oteryn:item.tibia.i2983");
        let dragon = "oteryn:creature.unbeatable_dragon";
        let health = runtime.companion_policy(dragon).map_err(debug)?.maximum_health;
        let dragon = runtime.commit_quest_creature_spawn(&[(dragon, health, from)]).map_err(debug)?[0];
        runtime.commit_quest_platform_displacement(from, tile, |_, _, _| true, || {
            map.facts.record_monster_step(crate::map::facts::MonsterStep { actor: dragon, from, to: tile })
        }).map_err(debug)?;
        let states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::paced_movement::monster_trap::apply_monster_trap_steps(
            &mut runtime, &states, map, 1_001_000,
        );
        assert!(!runtime.contains_live_creature(dragon));
        assert!(runtime.visible_entities().creatures.iter().any(|creature| {
            runtime.companion_snapshot(creature.actor).is_ok_and(|snapshot| {
                snapshot.state.policy.definition_key == "oteryn:creature.somewhat_beatable"
            })
        }));
    }
    Ok(())
}
