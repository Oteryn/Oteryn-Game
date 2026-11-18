//! Authored qcredit bindings through the real lethal projector and quest-state writer.
use super::fixture::{debug, id};
use super::{ComposedFreshAdmission, Dependencies, TestResult};
use crate::foundation::{
    AttackerCommand, ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId,
    MovementLocalPosition, OwnerDamageCommand,
};
use crate::gameplay_transport::kill_reward::{DrainOutcome, KillSink, record_creature_kill};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Callback,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const RAT: &str = "oteryn:creature.rat";
const CAVE_RAT: &str = "oteryn:creature.cave_rat";
const TRACK: &str = "oteryn:quest-progress/qcredit-fixture/kills";
const PHASE: &str = "oteryn:quest-progress/qcredit-fixture/phase";
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const EXIT: &str = "oteryn:quest-progress/qcredit-fixture/exit";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    All,
    Top,
    Time,
    Window,
    WindowStateless,
    WindowVisual,
    WindowReset,
    WindowResetDeparted,
    Area,
    DefeatTime,
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

#[test]
fn boss_defeat_clock_is_stored_for_damagers_and_absent_for_bystander() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::DefeatTime)
}

#[test]
fn two_players_share_the_timed_boss_exit_and_return_on_expiry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Window)
}

#[test]
fn stateless_seal_exit_moves_both_players_without_a_quest_write() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::WindowStateless)
}

#[test]
fn seal_portal_changes_on_death_and_restores_after_both_players_use_it() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::WindowVisual)
}

#[test]
fn mortal_shell_crystals_reset_at_two_minutes_with_a_remaining_damager() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::WindowReset)
}

#[test]
fn mortal_shell_crystals_stay_active_if_all_direct_damagers_depart() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::WindowResetDeparted)
}

impl Case {
    fn is_window(self) -> bool {
        matches!(
            self,
            Self::Window
                | Self::WindowStateless
                | Self::WindowVisual
                | Self::WindowReset
                | Self::WindowResetDeparted
        )
    }
    fn is_stateless_window(self) -> bool {
        matches!(
            self,
            Self::WindowStateless
                | Self::WindowVisual
                | Self::WindowReset
                | Self::WindowResetDeparted
        )
    }
    fn has_reset(self) -> bool {
        matches!(self, Self::WindowReset | Self::WindowResetDeparted)
    }
}

fn window_callback(at: MovementLocalPosition, case: Case) -> TestResult<Callback> {
    let x = u16::try_from(at.x)?;
    let y = u16::try_from(at.y)?;
    let floor = i8::try_from(-at.floor)?;
    let mut callback = json!({
        "key":"oteryn:quest-callback/qcredit-fixture/timed-exit",
        "quest":QUEST,
        "trigger":"step_in",
        "selector":{"position":{"x":x+1,"y":y,"floor":floor},"appearance":1949},
        "reversible":false,
        "passages":[],
        "teleport":{"x":x+3,"y":y,"floor":floor},
        "effects":{"death_window":{
            "creatures":[RAT,CAVE_RAT],
            "area":{"minimum":[x+2,y,at.floor],"maximum":[x+2,y,at.floor]},
            "required_deaths":2,
            "duration_ms":600_000
        }},
        "storage":[{"track":EXIT, "minimum":0,"maximum":1,"set":1}],
        "offerings":[],
        "return_on_failure":true,
        "sources":["Crystal201189f creaturescripts_inquisition_boss.lua:19-29"]
    });
    if case.is_stateless_window() {
        callback["storage"] = json!([]);
        callback["sources"] =
            json!(["Crystal201189f ferumbras_ascension/creaturescripts_bosses_kill.lua:126-134"]);
        callback["effects"]["death_window"]["duration_ms"] = json!(60_000);
        callback["effects"]["death_window"]["required_deaths"] = json!(1);
    }
    if case == Case::WindowVisual {
        callback["failure_teleport"] = json!({"x":x+4,"y":y,"floor":floor});
        callback["effects"]["death_window"]["open_appearance"] = json!(22761);
        // Crystal creaturescripts_bosses_kill.lua:130 names CONST_ME_THUNDER.
        // content/presentations/bindings/client-ids.json maps it to client ID 73.
        callback["effects"]["death_window"]["open_effect"] = json!(73);
    }
    if case.has_reset() {
        callback["sources"] =
            json!(["Crystal 201189f8 creaturescripts_bosses_kill.lua:58-86,136-144"]);
        callback["effects"]["death_window"]["creatures"] = json!([RAT]);
        callback["effects"]["death_window"]["delayed_reset"] = json!({
            "delay_ms":120_000,"from":14961,"to":14955,
            "positions":[{"x":x+4,"y":y,"floor":floor},{"x":x+5,"y":y,"floor":floor}]
        });
    }
    Ok(serde_json::from_value(callback)?)
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
        Case::WindowStateless | Case::WindowVisual => {
            "Crystal201189f ferumbras_ascension/creaturescripts_bosses_kill.lua:126-134"
        }
        Case::WindowReset | Case::WindowResetDeparted => {
            "Crystal201189f ferumbras_ascension/creaturescripts_bosses_kill.lua:58-86,136-144"
        }
        Case::DefeatTime => {
            counter["record_defeat_time"] = json!(true);
            counter["maximum"] = json!(i64::MAX);
            "Crystal201189f ferumbras_ascension/creaturescripts_bosses_kill.lua:114-124"
        }
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
        Case::Window => {
            "Crystal201189f creaturescripts_inquisition_boss.lua:19-29 and movements_teleport_main.lua:191-201"
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
    if case.is_window() {
        stages.push(json!({
            "key":"timed-exit", "completes":false,
            "trigger":{"kind":"step_in","callback":window_callback(at, case)?}
        }));
    }
    let fixture_digest = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let definition = json!({"schema":1, "quests":[{
        "quest":QUEST,
        "recipe_sha256": fixture_digest,
        "sources":[source],
        "tracks":[
            {"key":PHASE, "initial":1, "minimum":0, "maximum":1},
            {"key":TRACK, "initial":0, "minimum":0, "maximum":if case == Case::DefeatTime { i64::MAX } else { 5 }},
            {"key":EXIT, "initial":0, "minimum":0, "maximum":1}
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
        let world = if case.is_window() {
            let callback = window_callback(at, case)?;
            let position = callback.selector.position.ok_or("exit tile")?;
            Some(if case == Case::WindowVisual {
                super::quest_world_callbacks::world_with_objects(
                    &callback,
                    position,
                    &[],
                    &[22761],
                )?
            } else if case.has_reset() {
                let reset = callback
                    .effects
                    .death_window
                    .as_ref()
                    .and_then(|window| window.delayed_reset.as_ref())
                    .ok_or("crystal reset")?;
                let objects = reset
                    .positions
                    .iter()
                    .map(|position| (*position, reset.from))
                    .collect::<Vec<_>>();
                super::quest_world_callbacks::world_with_objects(
                    &callback,
                    position,
                    &objects,
                    &[reset.to],
                )?
            } else {
                super::quest_world_callbacks::world(&callback, position)?
            })
        } else {
            None
        };
        let cells = world
            .as_ref()
            .map(|world| {
                world
                    .movement_cells(pin.qualified_room().movement_cells())
                    .map_err(debug)
            })
            .transpose()?;
        let channel_pin = if let Some(world) = &world {
            crate::foundation::ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                world.frame_binding_digest(deps.admitted.world_id),
                world.map_revision_digest(),
                (at.x, at.y, at.floor),
            )
        } else {
            pin.into_channel_pin()
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
            channel_pin,
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
        for index in 0..if case.is_window() { 2 } else { 3 } {
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
            let initial = if case.is_window() && index == 1 {
                MovementLocalPosition {
                    x: at.x + 1,
                    y: at.y - 1,
                    ..at
                }
            } else {
                at
            };
            runtime
                .initialize_pinned_test_position(actor, initial)
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
        if let Some(cells) = &cells {
            owner.movement_cells = cells.into();
        }
        for player in &players {
            owner.refresh_quest_session(player.session, true).await;
        }

        if cells.is_some() {
            exercise_window(&owner, &players, at, &h.pool, case).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
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
        let before = crate::gameplay_transport::npc_trade_command::unix_ms_now() / 1_000;
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
            Case::DefeatTime => {
                let actual = track_value(&h.pool, 60).await?;
                let after = crate::gameplay_transport::npc_trade_command::unix_ms_now() / 1_000;
                assert!((before..=after).contains(&actual));
                [actual, actual, 0]
            }
            Case::Window
            | Case::WindowStateless
            | Case::WindowVisual
            | Case::WindowReset
            | Case::WindowResetDeparted => {
                return Err("timed exit requires bundle cells".into());
            }
        };
        assert_tracks(&h.pool, expected).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn exercise_window(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    at: MovementLocalPosition,
    pool: &sqlx::PgPool,
    case: Case,
) -> TestResult {
    let portal = MovementLocalPosition { x: at.x + 1, ..at };
    let ward = MovementLocalPosition { x: at.x + 2, ..at };
    let landing = MovementLocalPosition { x: at.x + 3, ..at };
    // Crystal 201189f8 creaturescripts_bosses_kill.lua:134 restores a distinct exit.
    let closed_landing = MovementLocalPosition { x: at.x + 4, ..at };
    let original = [
        at,
        MovementLocalPosition {
            y: at.y - 1,
            ..portal
        },
    ];
    let set_clock = |ms: u64| {
        owner
            .states
            .test_owner_time
            .store(ms * 1_000 + 1, std::sync::atomic::Ordering::Relaxed);
    };
    for (player, origin) in players.iter().zip(original) {
        if case == Case::WindowVisual {
            assert_eq!(enter_exit(owner, *player, portal).await?, closed_landing);
            move_to(owner, *player, origin).await?;
        } else {
            assert_eq!(enter_exit(owner, *player, portal).await?, origin);
        }
    }
    if case == Case::WindowVisual {
        assert_eq!(portal_appearance(owner, portal)?, 1949);
    }
    set_clock(199_000);
    death(owner, players, RAT, landing, case, 99).await?;
    set_clock(200_000);
    death(owner, players, RAT, ward, Case::Window, 100).await?;
    if !case.is_stateless_window() {
        assert_eq!(enter_exit(owner, players[0], portal).await?, original[0]);
        set_clock(201_000);
        death(owner, players, CAVE_RAT, ward, case, 101).await?;
    }
    set_clock(202_000);
    death(owner, players, RAT, ward, case, 102).await?;
    if case == Case::WindowVisual {
        assert_eq!(portal_appearance(owner, portal)?, 22761);
    }
    assert_eq!(enter_exit(owner, players[0], portal).await?, landing);
    move_to(owner, players[0], original[0]).await?;
    // Crystal movements_teleport_main.lua:191-201 reads the shared boss flag
    // but writes personal progress, so the second player can take the same exit.
    assert_eq!(enter_exit(owner, players[1], portal).await?, landing);
    move_to(owner, players[1], original[1]).await?;
    let passed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1 AND value=1",
    )
    .bind(EXIT)
    .fetch_one(pool)
    .await?;
    assert_eq!(passed, if case.is_stateless_window() { 0 } else { 2 });
    set_clock(if case.is_stateless_window() {
        260_000
    } else {
        801_000
    });
    if case == Case::WindowVisual {
        // The same decay owner runs on the live source cycle.
        let map = owner.bundle_map().ok_or("seal bundle map")?;
        assert_eq!(
            map.facts
                .movement_effects_since(0)
                .into_iter()
                .filter(|(_, pos, effect)| {
                    i32::from(pos.x) == portal.x
                        && i32::from(pos.y) == portal.y
                        && -i32::from(pos.floor) == i32::from(portal.floor)
                        && *effect == 73
                })
                .count(),
            1
        );
        assert_eq!(portal_appearance(owner, portal)?, 22761);
        assert_eq!(map.facts.expire_movement_items(259_999_999), 0);
        assert_eq!(map.facts.expire_movement_items(260_000_000), 1);
        assert_eq!(portal_appearance(owner, portal)?, 1949);
    }
    for (player, origin) in players.iter().zip(original) {
        if case == Case::WindowVisual {
            assert_eq!(enter_exit(owner, *player, portal).await?, closed_landing);
            move_to(owner, *player, origin).await?;
        } else {
            assert_eq!(enter_exit(owner, *player, portal).await?, origin);
        }
    }
    if case.has_reset() {
        let crystals = [closed_landing, MovementLocalPosition { x: at.x + 5, ..at }];
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .remove_terminal_session(players[0].session, players[0].actor)
                .map_err(debug)?;
            if case == Case::WindowResetDeparted {
                runtime
                    .remove_terminal_session(players[1].session, players[1].actor)
                    .map_err(debug)?;
            }
            for (clock, expected) in [
                (319_999, 14961),
                (
                    320_000,
                    if case == Case::WindowResetDeparted {
                        14961
                    } else {
                        14955
                    },
                ),
            ] {
                set_clock(clock);
                owner.drain_quest_death_map_resets(&runtime);
                for crystal in crystals {
                    assert_eq!(portal_appearance(owner, crystal)?, expected);
                }
            }
        }
    }
    Ok(())
}

fn portal_appearance(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    portal: MovementLocalPosition,
) -> TestResult<u16> {
    use crate::map::{overlay::TilePos, view::MapFacts};
    let map = owner.bundle_map().ok_or("seal bundle map")?;
    let position = TilePos {
        x: u16::try_from(portal.x)?,
        y: u16::try_from(portal.y)?,
        floor: i8::try_from(-portal.floor)?,
    };
    let base = map
        .overlay
        .base()
        .tile(position.x, position.y, position.floor)
        .ok_or("seal tile")?;
    let id = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(1)
        .map(|(id, _)| *id)
        .ok_or("seal portal")?;
    Ok(map
        .facts
        .base_entry(position, 1, id)
        .ok_or("seal portal appearance")?
        .appearance_id)
}

async fn enter_exit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: Player,
    portal: MovementLocalPosition,
) -> TestResult<MovementLocalPosition> {
    let permit = owner.spell_lane_permit().await.ok_or("step lane")?;
    let (origin, entered) = move_with_revision(owner, player, portal).await?;
    Ok(owner
        .step_quest_callback(&permit, player.actor, player.session, origin, entered)
        .await
        .position())
}

async fn move_to(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: Player,
    position: MovementLocalPosition,
) -> TestResult {
    move_with_revision(owner, player, position).await?;
    Ok(())
}

async fn move_with_revision(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: Player,
    destination: MovementLocalPosition,
) -> TestResult<(
    crate::foundation::MovementPositionSnapshot,
    crate::foundation::MovementPositionSnapshot,
)> {
    let mut runtime = owner.runtime.lock().await;
    let origin = runtime.read_actor_position(player.actor).map_err(debug)?;
    let map = owner.bundle_map().ok_or("bundle map for timed exit")?;
    let doors = crate::map::floor::BundleDoors(&map.facts);
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells(),
        player.actor,
        player.session,
        origin,
        destination,
        false,
        &doors,
    )
    .map_err(debug)?;
    let entered = runtime.commit_source_step(proof).map_err(debug)?;
    Ok((origin, entered))
}

async fn track_value(pool: &sqlx::PgPool, character: u8) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)",
    )
    .bind(id(character).as_slice())
    .bind(TRACK)
    .fetch_one(pool)
    .await?)
}

async fn assert_tracks(pool: &sqlx::PgPool, expected: [i64; 3]) -> TestResult {
    for (index, value) in expected.into_iter().enumerate() {
        let stored = track_value(pool, 60 + u8::try_from(index)?).await?;
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
