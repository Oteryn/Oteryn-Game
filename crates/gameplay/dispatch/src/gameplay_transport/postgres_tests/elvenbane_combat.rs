//! Source-native wire combat continuation; no fixture damage, health or kill flags.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::attack::*;

#[test]
fn elvenbane_source_native_dragon_earned_via_wire() -> TestResult {
    super::annihilator_route::elvenbane_source_combat()
}

#[test]
fn elvenbane_source_all_six_defenders_earned_via_wire() -> TestResult {
    super::annihilator_route::elvenbane_source_defenders()
}

pub(super) async fn scenario(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    pool: &sqlx::PgPool,
    all_defenders: bool,
    complete: bool,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(pool)
        .await?;
    eprintln!("ELVENBANE_WIRE_COMBAT POSTGRES_CASE_OBSERVED server_version_num={version}");
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .len(),
        6,
        "the normal source activation owner must realize all six native defenders"
    );
    super::elvenbane_route::exploration(owner, member, pool).await?;
    let target = {
        let runtime = owner.runtime.lock().await;
        runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|row| {
                runtime
                    .creature_spawn_cell(row.actor)
                    .is_some_and(|at| (at.x, at.y, at.floor) == (32590, 31645, 5))
            })
            .ok_or("actual source Dragon")?
    };
    fight(owner, member, target.actor, target.generation).await?;
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("session")?;
    let tracks: Vec<(String,i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(vec!["oteryn:quest-progress/elvenbane/phase","oteryn:quest-progress/elvenbane/native-defenders/dragon"])
        .fetch_all(pool).await?;
    assert_eq!(
        tracks,
        vec![
            (
                "oteryn:quest-progress/elvenbane/native-defenders/dragon".into(),
                1
            ),
            ("oteryn:quest-progress/elvenbane/phase".into(), 3)
        ],
        "wire lethal damage must earn both exact-origin and recipe Dragon progress"
    );
    if all_defenders {
        upper_defenders(owner, member).await?;
        let goals = oteryn_gameplay_quests::quest::native_kill_goals::load().map_err(debug)?;
        let counters: Vec<_> = goals.iter().map(|goal| goal.counter.clone()).collect();
        let earned: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) AND value=1")
            .bind(fence.character_id.as_bytes().as_slice()).bind(counters).fetch_one(pool).await?;
        assert_eq!(
            earned, 6,
            "all six source-native defenders must be earned through actual wire combat"
        );
        let mut expected: Vec<(String, i64)> =
            oteryn_gameplay_quests::quest::kill_objectives::OBJECTIVES
                .iter()
                .filter_map(|rule| rule.counter.map(|(key, cap)| (key.to_owned(), cap)))
                .collect();
        expected.sort();
        let keys: Vec<_> = expected.iter().map(|(key, _)| key.clone()).collect();
        let recipe: Vec<(String, i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key")
            .bind(fence.character_id.as_bytes().as_slice()).bind(keys).fetch_all(pool).await?;
        assert_eq!(
            recipe, expected,
            "every actual upper source kill must also earn its recipe objective"
        );
    }
    if complete {
        super::elvenbane_treasure::complete(owner, member, pool).await?;
    }
    eprintln!(
        "ELVENBANE_WIRE_COMBAT POSTGRES_CASE_COMPLETED source_dragon=earned source_defenders=6 upper_defenders={all_defenders} elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

async fn fight(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    target: crate::foundation::ExactActorRef,
    generation: u64,
) -> TestResult {
    let player = member.runtime_actor.ok_or("player")?;
    let initial = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(target)
        .map_err(debug)?;
    assert_eq!(
        initial.health, initial.maximum_health,
        "full native policy health before attacks"
    );
    super::annihilator_combat::command(
        owner,
        member,
        COMMAND_TYPE_FIGHT_MODES_INTENT,
        encode_fight_modes_intent(&FightModes {
            fight_mode: FightMode::Offensive,
            chase: ChaseMode::Chase,
            ..FightModes::DEFAULT
        }),
    )
    .await?;
    let entity = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: target.placement_identity(),
        generation,
    };
    super::annihilator_combat::command(
        owner,
        member,
        COMMAND_TYPE_ATTACK_TARGET_INTENT,
        encode_attack_target_intent(Some(&entity)).map_err(debug)?,
    )
    .await?;
    let mut killed = false;
    let mut dealt_damage = false;
    for pass in 0..512 {
        // The connection scheduler normally drives chase separately from swings.
        let chase = owner.chase_step_now(player, member.game_session_id).await;
        owner
            .refresh_attack_weapon(player, member.game_session_id)
            .await;
        let before = owner
            .runtime
            .lock()
            .await
            .companion_snapshot(target)
            .map(|s| s.health)
            .unwrap_or(0);
        owner.drain_auto_attacks().await;
        let after = owner
            .runtime
            .lock()
            .await
            .companion_snapshot(target)
            .map(|s| s.health)
            .unwrap_or(0);
        dealt_damage |= after < before;
        owner.drain_monster_melee().await;
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert!(
            !states.is_dead(player),
            "native defenders killed the participant"
        );
        let state = states
            .get(&runtime, player, member.game_session_id)
            .ok_or("player state")?;
        let heal = state.character_facts().max_health - state.health >= 1200;
        drop(states);
        let remaining = runtime
            .companion_snapshot(target)
            .map(|s| s.health)
            .unwrap_or(0);
        drop(runtime);
        if remaining == 0 {
            assert!(
                owner
                    .states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .has_session(member.game_session_id),
                "native lethal damage must emit the real reward occurrence"
            );
            killed = true;
            break;
        }
        if heal {
            super::annihilator_combat::heal(owner, member).await?;
        }
        if pass % 32 == 0 {
            let (player_at, target_at, origin, maximum, forge) = {
                let runtime = owner.runtime.lock().await;
                let current = runtime.companion_snapshot(target).map_err(debug)?;
                (
                    runtime
                        .read_actor_position(player)
                        .map_err(debug)?
                        .position(),
                    current.position,
                    runtime.creature_spawn_cell(target),
                    current.maximum_health,
                    current.forge,
                )
            };
            eprintln!(
                "ELVENBANE_WIRE_POS pass={pass} player={player_at:?} target={target_at:?} origin={origin:?} maximum={maximum} forge={forge:?} chase={chase:?}"
            );
            eprintln!(
                "ELVENBANE_WIRE_ATTACK pass={pass} health={remaining} initial_max={} combat={:?}",
                initial.maximum_health,
                owner.states.attack.lock().await.combat_state(
                    player,
                    member.game_session_id,
                    owner.owner_now()
                )
            );
        }
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_millis(
                crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS,
            ))
            .ok_or("clock")?;
    }
    assert!(
        killed && dealt_damage,
        "actual wire attacks must kill the full-health source defender"
    );
    let outcome = owner.drain_kill_rewards(member.game_session_id).await;
    assert!(
        matches!(
            outcome,
            crate::gameplay_transport::kill_reward::DrainOutcome::Drained
        ),
        "ordinary native XP/corpse/loot settlement: {outcome:?}"
    );
    Ok(())
}

async fn upper_defenders(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
) -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::Position;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
    use oteryn_protocol_oteryn::world_spatial::StepDirection::North;
    super::desert_earned_route::walk_to(
        owner,
        member,
        Position {
            x: 32593,
            y: 31644,
            floor: -5,
        },
    )
    .await?;
    let ladder = {
        let map = owner.bundle_map().ok_or("source map")?;
        let key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: 32593,
                y: 31643,
                floor: -5,
            },
            1,
        )
        .ok_or("source upper ladder")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        }
    };
    assert_eq!(
        super::annihilator_route::use_target(owner, member, &ladder).await?,
        UseDisposition::Committed,
        "ascend the actual source ladder through ordinary USE"
    );
    for floor in [4, 3] {
        if floor == 3 {
            super::desert_earned_route::walk_to(
                owner,
                member,
                Position {
                    x: 32588,
                    y: 31644,
                    floor: -4,
                },
            )
            .await?;
            super::desert_earned_route::step(owner, member, North).await?;
        }
        let targets = {
            let runtime = owner.runtime.lock().await;
            let at = runtime
                .read_actor_position(member.runtime_actor.ok_or("player")?)
                .map_err(debug)?
                .position();
            assert_eq!(
                at.floor, floor,
                "normal source ladder/stairs must reach each defender floor"
            );
            let mut targets: Vec<_> = runtime
                .visible_entities()
                .creatures
                .into_iter()
                .filter(|row| {
                    runtime
                        .creature_spawn_cell(row.actor)
                        .is_some_and(|origin| origin.floor == floor)
                })
                .collect();
            targets.sort_by_key(|row| row.actor.placement_identity());
            assert_eq!(
                targets.len(),
                if floor == 4 { 2 } else { 3 },
                "exact original native source cohort"
            );
            targets
        };
        for target in targets {
            fight(owner, member, target.actor, target.generation).await?;
        }
    }
    Ok(())
}
