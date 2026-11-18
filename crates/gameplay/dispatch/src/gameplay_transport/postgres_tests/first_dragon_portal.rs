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
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    let hash = catalogue.definition_hash(&callback.quest).ok_or("quest definition")?;
    let mut seed = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *seed).await?;
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) SELECT character_id,$1,'content-1',$2,character_revision,character_revision FROM game_character_roots")
        .bind(&callback.quest).bind(hash.as_slice()).execute(&mut *seed).await?;
    for state in &callback.storage {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$1,$2,$3,character_revision FROM game_character_roots")
            .bind(&state.track).bind(&callback.quest).bind(state.minimum).execute(&mut *seed).await?;
    }
    seed.commit().await?;
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

pub(super) const FLOWER: &str = "first-dragon-heaven-blossom";

#[test]
fn first_dragon_heaven_blossom_replaces_spirit_at_committed_step_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(FLOWER, || {
        super::quest_world_callbacks::scenario(FLOWER, Position { x: 33625, y: 31021, floor: -14 }, false)
    })
}

pub(super) async fn exercise_blossom(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    target: Position,
) -> TestResult {
    use crate::map::view::MapFacts;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

    const SPIRIT: &str = "oteryn:creature.spirit_of_fertility";
    const PLANT: &str = "oteryn:creature.angry_plant";
    let tile = super::quest_world_callbacks::local(target);
    let from = MovementLocalPosition { y: tile.y + 1, ..tile };
    let map = owner.bundle_map().ok_or("blossom bundle")?;
    let mut runtime = owner.runtime.lock().await;
    let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native policies")?;
    native.install_companion_policies(&mut runtime).map_err(debug)?;
    let health = runtime.companion_policy(SPIRIT).map_err(debug)?.maximum_health;
    let spirits = runtime.commit_quest_creature_spawn(&[(SPIRIT, health, from)]).map_err(debug)?;
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
            && runtime.companion_snapshot(*actor).is_ok_and(|snapshot| snapshot.state.policy.definition_key == PLANT)
    }));
    let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, 1).ok_or("blossom placement")?;
    assert!(map.facts.base_hidden(key));
    Ok(())
}
