//! Crystal 201189f feaster_of_souls/actions_the_unwelcome_lever.lua:1-44,
//! creaturescripts_brother_worm_death.lua:3-20 and config.lua.dist:658-659.
use super::fixture::{debug, id};
use super::quest_world_callbacks::{local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{
    CharacterId, CharacterLease, CommandId, CommandRef, GameSessionId, OwnerDamageCommand,
};
use crate::gameplay_transport::kill_reward::{KillSink, record_creature_kill};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use std::time::Duration;

pub(super) const NAME: &str = "zarganash-linked-boss";
pub(super) const DEPARTED_TOP: &str = "zarganash-linked-boss-departed-top";
pub(super) const OCCUPIED_ARRIVAL: &str = "zarganash-linked-boss-occupied-arrival";
const UNWELCOME: &str = "oteryn:creature.the_unwelcome";
const BROTHER: &str = "oteryn:creature.brother_worm";
const QUEST: &str = "oteryn:quest.feaster_of_souls_quest";
const CREDIT_TRACK: &str = "oteryn:quest-progress/quest/u12_30/feaster_of_souls/unwelcome_killed";
// Crystal Feaster actions_bosses_killed.lua:1-18 and storages.lua:2978.
const CREDIT_RULE: &str = r#"{
    "creature":"oteryn:creature.the_unwelcome",
    "transition":"oteryn:quest-transition/source-death/feaster-unwelcome-killed",
    "effects":[{"track":"oteryn:quest-progress/quest/u12_30/feaster_of_souls/unwelcome_killed",
                "default":-1,"from":{"kind":"lt","value":1},"set":1}],
    "define_transition":true
}"#;
const ARRIVAL: Position = at(33705, 31539);
const LEVER: Position = at(33735, 31537);
const ENTRY: Position = at(33707, 31545);
const EXIT: Position = Position {
    x: 33611,
    y: 31528,
    floor: -10,
};

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

pub(super) fn binding() -> TestResult<Callback> {
    let players: Vec<_> = (33736..=33740)
        .map(|x| serde_json::json!({"from":at(x,31537),"to":ENTRY}))
        .collect();
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/feaster-of-souls-zarganash-linked-boss",
        "quest":"oteryn:quest.feaster_of_souls_quest","trigger":"use",
        "selector":{"position":LEVER,"native":{"position":LEVER,"ordinal":2,"appearance":8911}},
        "reversible":false,"return_on_failure":false,"storage":[],"passages":[],"offerings":[],
        "effects":{"encounter":{"minimum_level":250,"players":players,
            "room_min":at(33699,31529),"room_max":at(33719,31546),
            "monsters":[{"position":at(33708,31539),"creature":UNWELCOME}],
            "linked_arrival":{"monster":{"position":ARRIVAL,"creature":BROTHER},
                "after_ms":20000,"radius":30,"arrival_effect":11,"death_effect":18,
                "exit":EXIT,"time_limit_ms":1200000}}},
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/feaster_of_souls/actions_the_unwelcome_lever.lua:1-44",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/feaster_of_souls/creaturescripts_brother_worm_death.lua:3-20",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:config.lua.dist:658-659"
        ]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(
        callback,
        LEVER,
        8911,
        2,
        &[],
        &[],
        None,
        &[ARRIVAL, at(33711, 31539), EXIT],
    )
}

#[test]
fn zarganash_two_players_enter_and_exit_as_brother_worm_finishes_only_its_linked_boss() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, LEVER, false))
}

#[test]
fn zarganash_unwelcome_credits_the_remaining_attacker_after_brother_worm_kills_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(DEPARTED_TOP, || {
        scenario(DEPARTED_TOP, LEVER, false)
    })
}

pub(super) fn with_credit_rule(
    catalogue: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    Ok(oteryn_gameplay_quests::quest::source_deaths::with_fixture_rule(
        catalogue, QUEST, CREDIT_RULE,
    )?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &super::fixture::Harness,
    name: &str,
) -> TestResult {
    let occupied_arrival = name == OCCUPIED_ARRIVAL;
    let departed_top = name == DEPARTED_TOP;
    let player = admitted.runtime_actor.ok_or("actor")?;
    let second_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let second = {
        let mut runtime = owner.runtime.lock().await;
        let map = owner.bundle_map().ok_or("map")?;
        let at = runtime.read_actor_position(player).map_err(debug)?;
        let move_to_staging = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            player,
            admitted.game_session_id,
            at,
            local(at_position(33736)),
            false,
            &crate::map::floor::BundleDoors(&map.facts),
        )
        .map_err(debug)?;
        runtime.commit_source_step(move_to_staging).map_err(debug)?;
        let lease = runtime
            .reserve_fresh_session(second_session)
            .map_err(debug)?;
        let second = runtime.commit_fresh_session(lease).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(second, local(at_position(33737)))
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                second,
                second_session,
                CharacterLease::new(CharacterId::decode(&id(101)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        let facts = states
            .get(&runtime, player, admitted.game_session_id)
            .ok_or("first player")?
            .character_facts();
        states
            .initialize(
                &runtime,
                second,
                second_session,
                facts,
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("second player")?;
        second
    };
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: LEVER.x,
            y: LEVER.y,
            floor: LEVER.floor,
        },
        2,
    )
    .ok_or("lever key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    assert_eq!(
        decode_use_result(&wire(owner, admitted, 1, 2, payload).await?).map_err(debug)?,
        UseDisposition::Committed
    );

    let live = super::engworldchg_journals::live_creatures;
    let [original] = live(owner, UNWELCOME)
        .await?
        .try_into()
        .map_err(|_| "original boss")?;
    assert!(live(owner, BROTHER).await?.is_empty());
    for member in [player, second] {
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(member)
                .map_err(debug)?
                .position(),
            local(ENTRY)
        );
    }
    if occupied_arrival {
        let mut runtime = owner.runtime.lock().await;
        let map = owner.bundle_map().ok_or("map")?;
        let expected = runtime.read_actor_position(second).map_err(debug)?;
        let move_to_arrival = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            second,
            second_session,
            expected,
            local(ARRIVAL),
            false,
            &crate::map::floor::BundleDoors(&map.facts),
        )
        .map_err(debug)?;
        runtime.commit_source_step(move_to_arrival).map_err(debug)?;
    }
    let unrelated = {
        let mut runtime = owner.runtime.lock().await;
        let hp = runtime
            .companion_policy(UNWELCOME)
            .map_err(debug)?
            .maximum_health;
        runtime
            .commit_quest_creature_spawn(&[(UNWELCOME, hp, local(at(33711, 31539)))])
            .map_err(debug)?[0]
    };
    owner.drain_quest_encounter_deadlines().await;
    assert!(live(owner, BROTHER).await?.is_empty());
    advance_to_deadline(owner)?;
    owner.drain_quest_encounter_deadlines().await;
    let [brother] = live(owner, BROTHER)
        .await?
        .try_into()
        .map_err(|_| "delayed boss")?;
    {
        let runtime = owner.runtime.lock().await;
        let spawned = runtime
            .read_actor_position(brother)
            .map_err(debug)?
            .position();
        let arrival = local(ARRIVAL);
        if occupied_arrival {
            assert_eq!(spawned.floor, arrival.floor);
            assert_eq!(
                spawned
                    .x
                    .abs_diff(arrival.x)
                    .max(spawned.y.abs_diff(arrival.y)),
                1
            );
            assert_eq!(
                runtime
                    .read_actor_position(second)
                    .map_err(debug)?
                    .position(),
                arrival
            );
        } else {
            assert_eq!(spawned, arrival);
        }
    }
    {
        let queue = owner.states.attack.lock().await.kills().clone();
        let now = owner.owner_now();
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        if departed_top {
            for (attacker, session, damage) in [
                (player, admitted.game_session_id, 1),
                (second, second_session, 2),
            ] {
                let hit = runtime.borrow_exact_actor_commit_at_us(now.get())
                    .commit_damage_for_bound_attacker(
                        original, attacker, CommandRef::new(session, CommandId::new(3199).map_err(debug)?),
                        0, OwnerDamageCommand {
                            target: UNWELCOME.as_bytes(),
                            occurrence: b"unwelcome-damage",
                            binding: b"unwelcome-damage-before-linked-lethal",
                            damage,
                        },
                    ).map_err(debug)?;
                assert_eq!(hit.health_before - hit.health_after, damage);
            }
            runtime.remove_test_actor(second).map_err(debug)?;
            assert!(runtime.borrow_combat_death().top_damage_contributor(original).map_err(debug)?.is_gone());
        }
        let lease = runtime
            .bound_attacker_lease(player, admitted.game_session_id)
            .map_err(debug)?;
        let health = runtime.companion_snapshot(brother).map_err(debug)?.health;
        let effect = runtime
            .borrow_exact_actor_commit_at_us(now.get())
            .commit_damage_for_bound_attacker(
                brother,
                player,
                CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(3200).map_err(debug)?,
                ),
                0,
                OwnerDamageCommand {
                    target: BROTHER.as_bytes(),
                    occurrence: b"linked-boss",
                    binding: b"linked-boss-native-lethal",
                    damage: health,
                },
            )
            .map_err(debug)?;
        assert_eq!(effect.health_after, 0);
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: Some(&states),
                activity: &[],
                queue: &queue,
                rewards: owner.reward_table(),
                charms: None,
            },
            brother,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session: admitted.game_session_id,
                actor: player,
            },
            now.get() / 1000,
            Some(now),
        );
        assert!(!runtime.contains_live_creature(original));
        assert!(runtime.contains_live_creature(unrelated));
    }
    if departed_top {
        use crate::gameplay_transport::kill_reward::DrainOutcome;
        assert_eq!(owner.drain_kill_rewards(admitted.game_session_id).await, DrainOutcome::Drained);
        let value: Option<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
        ).bind(QUEST).bind(CREDIT_TRACK).fetch_optional(&harness.pool).await?;
        assert_eq!(value, Some(1));
    }
    advance_to_deadline(owner)?;
    owner.drain_quest_encounter_deadlines().await;
    for member in [player, second] {
        if departed_top && member == second {
            continue;
        }
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(member)
                .map_err(debug)?
                .position(),
            local(EXIT)
        );
    }
    assert_eq!(live(owner, UNWELCOME).await?, vec![unrelated]);
    Ok(())
}

#[test]
fn zarganash_brother_worm_arrives_adjacent_to_the_occupied_spawn_and_ejects_both_players()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(OCCUPIED_ARRIVAL, || {
        scenario(OCCUPIED_ARRIVAL, LEVER, false)
    })
}

fn at_position(x: u16) -> Position {
    at(x, 31537)
}

fn advance_to_deadline(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    let due = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("encounter deadline")?;
    let elapsed = due.saturating_sub(owner.owner_now().get());
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_micros(elapsed))
        .ok_or("clock")?;
    Ok(())
}
