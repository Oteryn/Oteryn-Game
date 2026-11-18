//! Crystal 201189f8 arbaziloth_fight.lua:125-274 and arbaziloth_death.lua:18-55.
use super::*;
use crate::gameplay_transport::{FreshAdmissionAuthority, connection::UseCommand};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_protocol_oteryn::{
    world_object::{UseDisposition, UseIntent, UseTarget, WorldObjectTarget},
    world_spatial_entities::EntityRef,
};

pub(super) const FIRE: u16 = 49362;
const ROD: &str = "oteryn:item.tibia.i3483";
const SUPPORT: &str = "oteryn:creature.wolf";
const DEMON: &str = "oteryn:creature.overcharged_demon";

#[test]
fn two_players_break_four_shields_and_credit_the_enraged_death_and_exit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::ShieldHard)
}

#[test]
fn support_death_during_handoff_delay_keeps_easy_credit_and_denies_the_forge() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::ShieldEasy)
}

#[test]
fn overcharged_demon_refills_once_and_stays_owned_across_the_hard_handoff() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::ShieldReinforcements)
}

pub(super) fn lever(at: MovementLocalPosition, reinforcements: bool) -> TestResult<Callback> {
    let mut lever = serde_json::to_value(mode_lever(at, false)?)?;
    let tile = |x, y| json!({"x":at.x+x, "y":at.y+y, "floor":-at.floor});
    let room = &mut lever["effects"]["encounter"];
    room["minimum_level"] = json!(0);
    room["players"][1]["to"] = tile(3, 0);
    room["players"].as_array_mut().ok_or("staging tiles")?.push(json!({
        "from":tile(0,-1),"to":tile(3,0)
    }));
    room["monsters"].as_array_mut().ok_or("initial room roster")?.push(json!({
        "position":tile(4,1),"creature":SUPPORT
    }));
    if reinforcements {
        let roster = room["monsters"].as_array_mut().ok_or("reinforcement roster")?;
        roster.push(json!({"position":tile(6,1),"creature":DEMON}));
        roster.push(json!({"position":tile(7,1),"creature":DEMON}));
    }
    lever["reversible"] = json!(true);
    lever["effects"]["encounter_outcome"]["shield_handoff"] = json!({
        "support":SUPPORT,"terminal":CAVE_RAT,"thresholds_percent":[80,60,40,20],
        "fire_min":tile(4,0),"fire_max":tile(5,0),"fire_appearance":FIRE,
        "fire_delay_ms":1500,"tool_appearance":3483,"dodge_percent":0,
        "handoff_delay_ms":3000
    });
    if reinforcements {
        // Crystal 201189f8 arbaziloth_fight.lua:310-339.
        lever["effects"]["encounter_outcome"]["shield_handoff"]["reinforcements"] =
            json!({"creature":DEMON,"check_interval_ms":2000,"respawn_delay_ms":5000});
    }
    lever["sources"] = json!(["Crystal 201189f8 arbaziloth_fight.lua:125-274 and boss_lever.lua:59-74,171-187"]);
    Ok(serde_json::from_value(lever)?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    at: MovementLocalPosition,
    pool: &sqlx::PgPool,
    case: Case,
) -> TestResult {
    let hard = case != Case::ShieldEasy;
    let reinforcements = case == Case::ShieldReinforcements;
    let set_clock = |ms: u64| {
        owner.states.test_owner_time.store(ms * 1_000 + 1, std::sync::atomic::Ordering::Relaxed);
    };
    let player = players[0];
    let fence = owner.current_quest_fence(player.session).await.map_err(debug)?.ok_or("character fence")?.item_fence();
    let native = owner.active_generation.and_then(|generation| generation.native_gameplay()).ok_or("pinned items")?;
    let rod = native.item_index().and_then(|index| index.definition_ref("Item", ROD, "definition-r1")).ok_or("fishing rod")?;
    seed_rod(pool).await?;
    let map = owner.bundle_map().ok_or("shield map")?;
    let lever = lever(at, reinforcements)?.selector.position.ok_or("lever tile")?;
    let key = crate::map::view::placement_key(TilePos { x:lever.x, y:lever.y, floor:lever.floor }, 1).ok_or("lever key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let command = |id| UseCommand { game_session_id:player.session, command_id:id, item_fence:Some(fence) };
    set_clock(1_000);
    let result = owner.use_quest_callback(player.actor, command(501), map, &WorldObjectTarget {
        placement, expected_revision:map.facts.object_revision(key),
    }).await.ok_or("lever result")?;
    assert_eq!(result.disposition, UseDisposition::Committed);

    let room = MovementLocalPosition { x: at.x + 3, ..at };
    let portal = MovementLocalPosition { x: at.x + 6, ..at };
    let landing = MovementLocalPosition { x: at.x + 8, ..at };
    let (boss, support, target, max_hp, spawn_at) = {
        let runtime = owner.runtime.lock().await;
        for member in players {
            assert_eq!(runtime.read_actor_position(member.actor).map_err(debug)?.position(), room);
        }
        let actors = runtime.visible_entities().creatures;
        let boss = actors.iter().find(|c| runtime.matches_live_creature_identity(c.actor, RAT.as_bytes())).ok_or("first form")?;
        let support = actors.iter().find(|c| runtime.matches_live_creature_identity(c.actor, SUPPORT.as_bytes())).ok_or("support")?;
        (boss.actor, support.actor, EntityRef { identity:boss.actor.placement_identity(), generation:boss.generation },
            runtime.companion_policy(RAT).map_err(debug)?.maximum_health, boss.position)
    };
    let intent = UseIntent { target:UseTarget::ItemByDefinition(rod), use_with:Some(target), use_with_map_item:None, use_with_position:None };
    for member in players {
        assert_eq!(enter_exit(owner, *member, portal).await?, room);
    }

    let adds = if reinforcements {
        let first = MovementLocalPosition { x: at.x + 6, y: at.y + 1, ..at };
        let second = MovementLocalPosition { x: at.x + 7, ..first };
        Some(exercise_refill(owner, players, first, second).await?)
    } else {
        None
    };

    for (stage, percent) in [80, 60, 40, 20].into_iter().enumerate() {
        let number = u64::try_from(stage)?;
        let due = 10_000 + number * 2_000;
        let expected = u64::try_from(max_hp * percent / 100)?;
        {
            let mut runtime = owner.runtime.lock().await;
            for step in 0..2 {
                runtime.borrow_exact_actor_commit_at(due).commit_damage_for_bound_attacker(
                    boss, player.actor, CommandRef::new(player.session, CommandId::new(510 + number * 2 + step).map_err(debug)?),
                    0, OwnerDamageCommand {
                        target:RAT.as_bytes(), occurrence:player.session.as_bytes(),
                        binding:b"fixture-shield-threshold", damage:max_hp,
                    },
                ).map_err(debug)?;
                assert_eq!(runtime.live_creature_health(boss), Some(expected));
            }
            owner.tick_encounter_shields(&mut runtime, due);
            owner.tick_encounter_shields(&mut runtime, due + 1499);
            assert!(fire_tile(owner, at)?.is_none());
        }
        // A rod while shielded, but before the boss has touched the fire, is harmless.
        if stage == 0 {
            assert_eq!(FreshAdmissionAuthority::use_encounter_tool(owner, player.actor, command(530), &intent, None)
                .await.map_err(debug)?.ok_or("early rod")?.disposition, UseDisposition::Committed);
            assert!(owner.runtime.lock().await.encounter_shield_protected(boss));
        }
        {
            let mut runtime = owner.runtime.lock().await;
            owner.tick_encounter_shields(&mut runtime, due + 1500);
            let fire = fire_tile(owner, at)?.ok_or("projected fire")?;
            let before = runtime.read_actor_position(boss).map_err(debug)?;
            runtime.borrow_movement_position().commit_creature_relocation(boss, before, fire).map_err(debug)?;
            owner.tick_encounter_shields(&mut runtime, due + 1501);
            assert!(fire_tile(owner, at)?.is_none());
        }
        set_clock(due + 1501);
        assert_eq!(FreshAdmissionAuthority::use_encounter_tool(owner, player.actor, command(531 + number), &intent, None)
            .await.map_err(debug)?.ok_or("rod use")?.disposition, UseDisposition::Committed);
        let mut runtime = owner.runtime.lock().await;
        assert!(!runtime.encounter_shield_protected(boss));
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime.borrow_movement_position().commit_creature_relocation(boss, before, spawn_at).map_err(debug)?;
    }
    let after_rod = owner.owner_now().get() / 1_000;
    if !hard {
        settle_actor_death(owner, players, SUPPORT, support, Case::ShieldEasy, 540).await?;
    }
    let terminal = {
        let mut runtime = owner.runtime.lock().await;
        assert!(runtime.contains_live_creature(boss));
        owner.tick_encounter_shields(&mut runtime, after_rod + 3000);
        if let Some(adds) = adds {
            assert!(adds.into_iter().all(|add| runtime.contains_live_creature(add)));
        }
        if hard {
            assert!(!runtime.contains_live_creature(boss));
            assert!(!runtime.contains_live_creature(support));
            assert!(!runtime.retains_corpse_projection(boss));
            assert!(!runtime.retains_corpse_projection(support));
            let replacement = runtime.visible_entities().creatures.into_iter()
                .find(|c| runtime.matches_live_creature_identity(c.actor, CAVE_RAT.as_bytes())).ok_or("enraged form")?;
            assert_eq!(replacement.position, spawn_at);
            assert_eq!(runtime.live_creature_health(replacement.actor),
                Some(u64::try_from(runtime.companion_policy(CAVE_RAT).map_err(debug)?.maximum_health)?));
            replacement.actor
        } else {
            assert!(runtime.contains_live_creature(boss));
            boss
        }
    };
    for member in players {
        assert_eq!(enter_exit(owner, *member, portal).await?, room);
    }
    settle_actor_death(owner, players, if hard { CAVE_RAT } else { RAT }, terminal,
        if hard { Case::ShieldHard } else { Case::ShieldEasy }, 541).await?;
    if let Some(adds) = adds {
        let mut runtime = owner.runtime.lock().await;
        owner.tick_encounter_shields(&mut runtime, after_rod + 12_000);
        assert!(adds.into_iter().all(|add| !runtime.contains_live_creature(add)));
    }
    for (member, character) in players.iter().zip([60,61]) {
        assert_eq!(enter_exit(owner, *member, portal).await?, if hard { landing } else { room });
        assert_eq!(track_value(pool, character).await?, i64::from(!hard));
        let hard_wins:i64 = sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)")
            .bind(id(character).as_slice()).bind(HARD_TRACK).fetch_one(pool).await?;
        assert_eq!(hard_wins, i64::from(hard));
    }
    Ok(())
}

async fn exercise_refill(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    first: MovementLocalPosition,
    second: MovementLocalPosition,
) -> TestResult<[ExactActorRef; 2]> {
    fn at(runtime: &ChannelRuntimeV1, position: MovementLocalPosition) -> Option<ExactActorRef> {
        runtime.positioned_melee_census().ok()?.into_iter().find_map(|(actor, snapshot, _)| {
            (snapshot.position() == position
                && runtime.matches_live_creature_identity(actor, DEMON.as_bytes())).then_some(actor)
        })
    }
    let (fallen, survivor) = {
        let runtime = owner.runtime.lock().await;
        (at(&runtime, first).ok_or("first reinforcement")?,
         at(&runtime, second).ok_or("second reinforcement")?)
    };
    settle_actor_death(owner, players, DEMON, fallen, Case::ShieldReinforcements, 505).await?;
    let mut runtime = owner.runtime.lock().await;
    for now in [2_999, 3_000, 7_999] {
        owner.tick_encounter_shields(&mut runtime, now);
        assert_eq!(at(&runtime, first), None, "refill before the complete delay");
        assert_eq!(at(&runtime, second), Some(survivor));
    }
    owner.tick_encounter_shields(&mut runtime, 8_000);
    let replacement = at(&runtime, first).ok_or("delayed reinforcement")?;
    assert_ne!(replacement, fallen);
    assert!(runtime.retains_corpse_projection(fallen));
    for now in [8_000, 9_999] {
        owner.tick_encounter_shields(&mut runtime, now);
        assert_eq!(at(&runtime, first), Some(replacement));
        assert_eq!(at(&runtime, second), Some(survivor));
    }
    Ok([replacement, survivor])
}

fn fire_tile(owner: &ComposedFreshAdmission<'_, '_, '_>, at: MovementLocalPosition) -> TestResult<Option<MovementLocalPosition>> {
    let map = owner.bundle_map().ok_or("fire map")?;
    let facts = map.facts.base_only();
    for x in [at.x + 4, at.x + 5] {
        let tile = TilePos { x:u16::try_from(x)?, y:u16::try_from(at.y)?, floor:i8::try_from(-at.floor)? };
        if facts.movement_items(tile).iter().any(|entry| entry.facts.appearance_id == FIRE) {
            return Ok(Some(MovementLocalPosition { x, ..at }));
        }
    }
    Ok(None)
}

async fn seed_rod(pool: &sqlx::PgPool) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,game_character_uuid_v7())")
        .bind(id(214).as_slice()).bind(id(42).as_slice()).bind(ROD).execute(&mut *tx).await?;
    let bag: Option<String> = sqlx::query_scalar("SELECT item_instance_id::text FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(60).as_slice()).fetch_optional(&mut *tx).await?;
    let bag = if let Some(bag) = bag {
        bag
    } else {
        let bag: String = sqlx::query_scalar("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
            VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,game_character_uuid_v7()) RETURNING item_instance_id::text")
            .bind(id(42).as_slice()).fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,$2::uuid,encode($3,'hex')::uuid,game_character_uuid_v7())")
            .bind(id(60).as_slice()).bind(&bag).bind(id(42).as_slice()).execute(&mut *tx).await?;
        bag
    };
    sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
        SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::uuid,coalesce(max(placement_ordinal),0)+1,game_character_uuid_v7() FROM game_item_container_entries WHERE parent_item_instance_id=$4::uuid")
        .bind(id(214).as_slice()).bind(id(42).as_slice()).bind(id(60).as_slice()).bind(&bag).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
