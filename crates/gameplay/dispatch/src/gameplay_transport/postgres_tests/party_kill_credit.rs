//! Authored qcredit bindings through the real lethal projector and quest-state writer.
use super::fixture::{debug, id};
use super::{ComposedFreshAdmission, Dependencies, TestResult};
use crate::foundation::{
    AttackerCommand, ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId,
    MovementLocalPosition, OwnerDamageCommand,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const RAT: &str = "oteryn:creature.rat";
const CAVE_RAT: &str = "oteryn:creature.cave_rat";
const TRACK: &str = "oteryn:quest-progress/qcredit-fixture/kills";
const PHASE: &str = "oteryn:quest-progress/qcredit-fixture/phase";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    All,
    Top,
    Time,
    Area,
    Summon,
    Ordered,
}

#[test]
fn all_direct_damagers_get_one_credit_without_party_membership() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::All)
}

#[test]
fn highest_damage_gets_credit_even_when_another_player_lands_the_lethal_hit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Top)
}

#[test]
fn damage_age_is_measured_at_death_and_includes_the_window_boundary() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Time)
}

#[test]
fn kill_in_overlapping_areas_counts_once_and_outside_kill_does_not_count() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Area)
}

#[test]
fn summon_owner_credit_requires_binding_opt_in() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Summon)
}

#[test]
fn higher_boss_rank_advances_once_and_later_lower_rank_cannot_regress_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Ordered)
}

fn catalogue(case: Case, at: MovementLocalPosition) -> TestResult<QuestStateCatalogue> {
    let mut counter = json!({
        "key": "oteryn:quest-transition/native-kills/qcredit-fixture-rat",
        "creature": RAT,
        "track": TRACK,
        "gate": PHASE,
        "gate_value": 1,
        "maximum": 5
    });
    counter["damage_credit"] = json!({
        "recipients": if case == Case::Top { "top_damage" } else { "all_damagers" },
        "summon_owner": case == Case::Summon
    });
    let source = match case {
        Case::All => {
            counter["gate"] = json!(TRACK);
            counter["gate_value"] = json!(0);
            counter["maximum"] = json!(1);
            counter["set_to_maximum"] = json!(true);
            "Crystal201189f the_new_frontier/creaturescripts_shard_of_corruption_kill.lua:5-10"
        }
        Case::Top => {
            counter["damage_credit"]["recent_damage_ms"] = json!(60_000);
            "Crystal201189f the_inquisition_quest/creaturescripts_inquisition_ungreez.lua:2-10"
        }
        Case::Time => {
            counter["damage_credit"]["recent_damage_ms"] = json!(60_000);
            "Crystal201189f creature.lua:219-240, config.lua.dist:40"
        }
        Case::Area => {
            counter["death_areas"] = json!([
                {"minimum":[at.x, at.y, at.floor], "maximum":[at.x+1, at.y, at.floor]},
                {"minimum":[at.x+1, at.y, at.floor], "maximum":[at.x+2, at.y, at.floor]}
            ]);
            "Crystal201189f the_great_dragon_hunt_quest/creaturescripts_the_great_dragon_hunt.lua:1-25"
        }
        Case::Summon => {
            "Crystal201189f killing_in_the_name_of/creaturescripts_monster_kill.lua:18-56"
        }
        Case::Ordered => {
            counter["set_to_maximum"] = json!(true);
            counter["maximum"] = json!(1);
            "Crystal201189f the_secret_library_quest/creaturescripts_kill.lua:10-39"
        }
    };
    let stage = |name: &str, counter: &Value| json!({"key":name, "completes":false, "trigger":{"kind":"kill","counter":counter}});
    let mut stages = vec![stage("rat", &counter)];
    if case == Case::Ordered {
        counter["key"] = json!("oteryn:quest-transition/native-kills/qcredit-fixture-cave-rat");
        counter["creature"] = json!(CAVE_RAT);
        counter["maximum"] = json!(3);
        stages.push(stage("cave-rat", &counter));
    }
    let fixture_digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&stages)?));
    let definition = json!({"schema":1, "quests":[{
        "quest":"oteryn:quest.authored.draccoon_herald_outfits_quest",
        "recipe_sha256": fixture_digest,
        "sources":[source],
        "tracks":[
            {"key":PHASE, "initial":1, "minimum":0, "maximum":1},
            {"key":TRACK, "initial":0, "minimum":0, "maximum":5}
        ],
        "stages":stages
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture content revision")?;
    Ok(Bindings::parse(&serde_json::to_vec(&definition)?)?.attach(&base)?)
}

#[derive(Clone, Copy)]
struct Player {
    actor: ExactActorRef,
    session: GameSessionId,
}

fn run(admin: String, case: Case) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run_async(async move |_| {
        let h =
            crate::bestiary_postgres_harness::Harness::create(admin, "party_kill_credit", false)
                .await?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation
            .native_gameplay()
            .ok_or("native combat content")?;
        let start = pin.entry_start();
        let at = MovementLocalPosition {
            x: start.x,
            y: start.y,
            floor: start.floor,
        };
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            pin.into_channel_pin(),
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let now: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&h.pool)
                .await?;
        let mut players = Vec::new();
        for index in 0..3 {
            let (current, _) =
                crate::world_party_runtime_postgres_cases::committed_fresh_player_with_operation(
                    &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                        root: h.root.clone(),
                        node: h.node.clone(),
                        recovery: &h.recovery,
                    },
                    60 + index,
                    80 + index,
                    70 + index,
                    now,
                )
                .await?;
            let session = current.current_game_session_id();
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .bind_attacker_lease(actor, session, current.current_character_lease())
                .map_err(debug)?;
            runtime
                .initialize_pinned_test_position(actor, at)
                .map_err(debug)?;
            players.push(Player { actor, session });
        }
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner_from(&h.root, &h.node, &authority);
        owner.active_generation = Some(generation);
        owner.quest_catalogue = Some(Arc::new(catalogue(case, at)?));
        for player in &players {
            owner.refresh_quest_session(player.session, true).await;
        }

        if case == Case::Area {
            death(
                &owner,
                &players,
                RAT,
                MovementLocalPosition { x: at.x + 3, ..at },
                case,
                100,
            )
            .await?;
            assert_tracks(&h.pool, [0, 0, 0]).await?;
        }
        if case == Case::Summon {
            // Negative control: attributed-only damage never leaks into direct-player bindings.
            owner.quest_catalogue = Some(Arc::new(catalogue(Case::All, at)?));
            for player in &players {
                owner.refresh_quest_session(player.session, false).await;
            }
            death(
                &owner,
                &players,
                RAT,
                MovementLocalPosition { x: at.x + 1, ..at },
                case,
                110,
            )
            .await?;
            assert_tracks(&h.pool, [0, 1, 0]).await?;
            owner.quest_catalogue = Some(Arc::new(catalogue(case, at)?));
            for player in &players {
                owner.refresh_quest_session(player.session, false).await;
            }
        }
        if case == Case::Ordered {
            death(
                &owner,
                &players,
                CAVE_RAT,
                MovementLocalPosition { x: at.x + 1, ..at },
                case,
                120,
            )
            .await?;
            assert_tracks(&h.pool, [3, 3, 0]).await?;
        }
        if case == Case::Time {
            death(
                &owner,
                &players,
                RAT,
                MovementLocalPosition { x: at.x + 1, ..at },
                case,
                130,
            )
            .await?;
            assert_tracks(&h.pool, [0, 1, 0]).await?;
        }
        if case == Case::Top {
            death(
                &owner,
                &players,
                RAT,
                MovementLocalPosition { x: at.x + 1, ..at },
                case,
                140,
            )
            .await?;
            assert_tracks(&h.pool, [0, 1, 0]).await?;
        }
        death(
            &owner,
            &players,
            RAT,
            MovementLocalPosition { x: at.x + 1, ..at },
            case,
            200,
        )
        .await?;
        let expected = match case {
            Case::All | Case::Area => [1, 1, 0],
            Case::Top => [1, 1, 0],
            Case::Time | Case::Summon => [1, 2, 0],
            Case::Ordered => [3, 3, 0],
        };
        assert_tracks(&h.pool, expected).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn assert_tracks(pool: &sqlx::PgPool, expected: [i64; 3]) -> TestResult {
    for (index, value) in expected.into_iter().enumerate() {
        let stored: i64 = sqlx::query_scalar(
            "SELECT COALESCE((SELECT value FROM game_character_quest_tracks
             WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)",
        )
        .bind(id(60 + u8::try_from(index)?).as_slice())
        .bind(TRACK)
        .fetch_one(pool)
        .await?;
        assert_eq!(stored, value, "quest credit for participant {index}");
    }
    Ok(())
}

async fn death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    creature: &str,
    at: MovementLocalPosition,
    case: Case,
    ordinal: u64,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let now_ms = 100_000 + ordinal * 1_000;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let policy = runtime.companion_policy(creature).map_err(debug)?;
        let hp = policy.maximum_health;
        let target = runtime
            .admit_encounter_actor(creature, hp, at)
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, creature)
            .map_err(debug)?;
        let shares = [hp - 1, 1];
        let age = if case == Case::Time || ordinal == 140 {
            60_000 + u64::from(matches!(ordinal, 130 | 140))
        } else {
            1
        };
        for (index, player) in players.iter().take(2).enumerate() {
            let command = CommandRef::new(player.session, CommandId::new(ordinal).map_err(debug)?);
            let damage = OwnerDamageCommand {
                target: creature.as_bytes(),
                occurrence: player.session.as_bytes(),
                binding: b"qcredit-native-damage",
                damage: shares[index],
            };
            let hit_at = if index == 0 { now_ms - age } else { now_ms };
            if case == Case::Summon && index == 0 {
                // Use the carrier seam to seed a master-attributed hit with no Player
                // origin. The corpse, lethal callback and SQL remain real.
                let lease = runtime
                    .bound_attacker_lease(player.actor, player.session)
                    .map_err(debug)?;
                runtime
                    .borrow_exact_actor_commit_at(hit_at)
                    .commit_damage_for_attacker(
                        target,
                        AttackerCommand::new(lease.character_id(), lease.generation(), command, 0),
                        damage,
                    )
                    .map_err(debug)?;
            } else {
                runtime
                    .borrow_exact_actor_commit_at(hit_at)
                    .commit_damage_for_bound_attacker(target, player.actor, command, 0, damage)
                    .map_err(debug)?;
            }
        }
        let last = players[1];
        let lease = runtime
            .bound_attacker_lease(last.actor, last.session)
            .map_err(debug)?;
        for _ in 0..2 {
            record_creature_kill(
                &mut runtime,
                KillSink {
                    prey: None,
                    queue: &queue,
                    rewards: native.reward_table(),
                    activity: &[],
                    charms: None,
                },
                target,
                crate::combat::CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session: last.session,
                    actor: last.actor,
                },
                now_ms,
                Some(oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now_ms * 1000)),
            );
        }
        target
    };
    for player in players.iter().take(2) {
        assert_eq!(
            owner.drain_kill_rewards(player.session).await,
            DrainOutcome::Drained
        );
    }
    owner
        .runtime
        .lock()
        .await
        .remove_arena_opponent(target)
        .map_err(debug)?;
    Ok(())
}
