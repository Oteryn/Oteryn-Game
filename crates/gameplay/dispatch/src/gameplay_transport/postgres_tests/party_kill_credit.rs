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
const SHIELD_FIRST_WIN_TRACK: &str = "oteryn:quest-progress/qcredit-fixture/shield-first-win";
const PREOWNED_OUTFIT_TRACK: &str = "oteryn:quest-progress/qcredit-fixture/preowned-outfit";
// Crystal 201189f8 arbaziloth_death.lua:38-41.
const FIRST_DEFEAT_MESSAGE: &str =
    "You have defeated Arbaziloth for the first time. You have won the Fiend Slayer Outfit.";

// Crystal 201189f8 forge_teleport.lua:18-34.
const FORGE_COLD: &str = "The forge lies cold. There is no way through right now.";
const FORGE_RUNNING: &str = "The power surging to Arbaziloth prevents you from entering!";
const FORGE_EASY: &str =
    "The Forgemaster has been defeated, the lack of power prevents you from entering!";
const FORGE_OPEN: &str = "You enter the forge!";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    All,
    Top,
    Time,
    Window,
    WindowStateless,
    WindowVisual,
    WindowReset,
    WindowResetAmbiguous,
    WindowResetDeparted,
    Area,
    PartyArea,
    DefeatTime,
    FirstDefeatOutfit,
    Summon,
    Ordered,
    Outcome,
    ShieldEasy,
    ShieldHard,
    ShieldPulse,
    ShieldReinforcements,
}

#[path = "party_kill_credit/shield_handoff.rs"]
mod shield_handoff;

const OUTCOME_GROUP: &str = "oteryn:encounter.qcredit-mode";
const HARD_TRACK: &str = "oteryn:quest-progress/qcredit-fixture/hard-wins";

#[test]
fn two_players_receive_only_the_admitted_mode_and_use_the_hard_exit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::Outcome)
}

const CRYSTAL_RESET: &str = "oteryn:quest-transition/native-kills/qcredit-fixture-crystals-reset";
const CRYSTAL_TRACKS: [&str; 3] = [
    "oteryn:quest-progress/qcredit-fixture/crystals/one",
    "oteryn:quest-progress/qcredit-fixture/crystals/two",
    "oteryn:quest-progress/qcredit-fixture/crystals/all",
];

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
fn scoped_death_credits_the_top_damagers_party_without_crediting_an_outsider() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::PartyArea)
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
fn first_defeat_grants_the_base_outfit_once_to_each_damager() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::FirstDefeatOutfit)
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
fn mortal_shell_crystal_reset_waits_for_an_unambiguous_map_batch() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::WindowResetAmbiguous)
}

#[test]
fn mortal_shell_crystals_stay_active_if_all_direct_damagers_depart() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, Case::WindowResetDeparted)
}

impl Case {
    fn has_outcome(self) -> bool {
        self == Self::Outcome || self.has_shields()
    }

    fn has_shields(self) -> bool {
        matches!(self, Self::ShieldEasy | Self::ShieldHard | Self::ShieldPulse | Self::ShieldReinforcements)
    }

    fn is_window(self) -> bool {
        matches!(
            self,
            Self::Outcome | Self::ShieldEasy | Self::ShieldHard | Self::ShieldPulse | Self::ShieldReinforcements
                | Self::Window
                | Self::WindowStateless
                | Self::WindowVisual
                | Self::WindowReset
                | Self::WindowResetAmbiguous
                | Self::WindowResetDeparted
        )
    }
    fn is_stateless_window(self) -> bool {
        matches!(
            self,
            Self::WindowStateless
                | Self::WindowVisual
                | Self::WindowReset
                | Self::WindowResetAmbiguous
                | Self::WindowResetDeparted
        )
    }
    fn has_reset(self) -> bool {
        matches!(self, Self::WindowReset | Self::WindowResetAmbiguous | Self::WindowResetDeparted)
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
            "positions":[{"x":x+4,"y":y,"floor":floor},{"x":x+5,"y":y,"floor":floor}],
            "character_reset":{"transition":CRYSTAL_RESET,"tracks":CRYSTAL_TRACKS}
        });
    }
    Ok(serde_json::from_value(callback)?)
}

fn mode_lever(at: MovementLocalPosition, hard: bool) -> TestResult<Callback> {
    let x = u16::try_from(at.x)?;
    let y = u16::try_from(at.y)?;
    let floor = i8::try_from(-at.floor)?;
    let mode = if hard { "hard" } else { "easy" };
    let pos = |x, y| json!({"x":x,"y":y,"floor":floor});
    Ok(serde_json::from_value(json!({
        "key":format!("oteryn:quest-callback/qcredit-fixture/{mode}-entrance"),
        "quest":QUEST,"trigger":"use","selector":{"position":pos(x+1+u16::from(hard),y),"appearance":1945},
        "lever":[1945,1946],"reversible":false,"passages":[],"storage":[],"offerings":[],
        "effects":{"encounter_outcome":{"outcome":{"group":OUTCOME_GROUP,"mode":mode},"terminal":RAT},
            "encounter":{"minimum_level":1,"room_min":pos(x+3,y-1),"room_max":pos(x+7,y+1),
              "players":[{"from":pos(x,y),"to":pos(x+3,y)},{"from":pos(x+1,y-1),"to":pos(x+3,y-1)}],
              "monsters":[{"position":pos(x+5,y+1),"creature":RAT}]}},
        "return_on_failure":false,"sources":["Crystal 201189f8 arbaziloth_death.lua:18-27"]
    }))?)
}

fn mode_exit(at: MovementLocalPosition) -> TestResult<Callback> {
    let x = u16::try_from(at.x)?;
    let y = u16::try_from(at.y)?;
    let floor = i8::try_from(-at.floor)?;
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/qcredit-fixture/forge-exit","quest":QUEST,"trigger":"step_in",
        "selector":{"position":{"x":x+6,"y":y,"floor":floor},"appearance":37000},
        "teleport":{"x":x+8,"y":y,"floor":floor},"reversible":false,"passages":[],"storage":[],"offerings":[],
        "effects":{
            "require_encounter_outcome":{"group":OUTCOME_GROUP,"mode":"hard"},
            "encounter_outcome_feedback":{"running":FORGE_RUNNING,"wrong_outcome":FORGE_EASY}
        },
        "failure_message":FORGE_COLD,"success_message":FORGE_OPEN,
        "return_on_failure":true,"sources":["Crystal 201189f8 forge_teleport.lua:17-34"]
    }))?)
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
        Case::WindowReset | Case::WindowResetAmbiguous | Case::WindowResetDeparted => {
            "Crystal201189f ferumbras_ascension/creaturescripts_bosses_kill.lua:58-86,136-144"
        }
        Case::DefeatTime => {
            counter["record_defeat_time"] = json!(true);
            counter["maximum"] = json!(i64::MAX);
            "Crystal201189f ferumbras_ascension/creaturescripts_bosses_kill.lua:114-124"
        }
        Case::All | Case::FirstDefeatOutfit => {
            counter["gate"] = json!(TRACK);
            counter["gate_value"] = json!(0);
            counter["maximum"] = json!(1);
            counter["set_to_maximum"] = json!(true);
            if case == Case::FirstDefeatOutfit {
                counter["first_defeat_outfit"] = json!({"outfit":"oteryn:outfit.fiend_slayer","addons":0});
                counter["first_defeat_outfit_message"] = json!(FIRST_DEFEAT_MESSAGE);
                "Crystal 201189f8 arbaziloth_death.lua:29-41"
            } else {
                "Crystal201189f the_new_frontier/creaturescripts_shard_of_corruption_kill.lua:5-10"
            }
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
        Case::Area | Case::PartyArea => {
            if case == Case::PartyArea {
                counter.as_object_mut().ok_or("kill counter")?.remove("damage_credit");
            }
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
        Case::Outcome => "Crystal 201189f8 arbaziloth_death.lua:18-55 and forge_teleport.lua:17-34",
        Case::ShieldEasy | Case::ShieldHard | Case::ShieldPulse | Case::ShieldReinforcements => "Crystal 201189f8 arbaziloth_fight.lua:125-274 and arbaziloth_death.lua:18-55",
    };
    let stage = |name: &str, counter: &Value| json!({"key":name, "completes":false, "trigger":{"kind":"kill","counter":counter}});
    let mut stages = vec![stage("rat", &counter)];
    if matches!(case, Case::Ordered | Case::FirstDefeatOutfit) {
        counter["key"] = json!("oteryn:quest-transition/native-kills/qcredit-fixture-cave-rat");
        counter["creature"] = json!(CAVE_RAT);
        if case == Case::Ordered {
            counter["maximum"] = json!(3);
        } else {
            counter["track"] = json!(PREOWNED_OUTFIT_TRACK);
            counter["gate"] = json!(PREOWNED_OUTFIT_TRACK);
        }
        stages.push(stage("cave-rat", &counter));
    }
    if case.has_outcome() {
        if case.has_shields() {
            stages.push(json!({"key":"shield-entrance","completes":false,"trigger":{"kind":"map_use","callback":shield_handoff::lever(at, case)?}}));
            counter["creature"] = json!(CAVE_RAT);
        } else {
            for (mode, hard) in [("easy", false), ("hard", true)] {
                stages.push(json!({"key":mode,"completes":false,"trigger":{"kind":"map_use","callback":mode_lever(at, hard)?}}));
            }
        }
        stages.push(json!({"key":"forge-exit","completes":false,"trigger":{"kind":"step_in","callback":mode_exit(at)?}}));
        let ordinary = stages[0]["trigger"]["counter"]
            .as_object_mut()
            .ok_or("counter")?;
        ordinary.insert(
            "required_outcome".into(),
            json!({"group":OUTCOME_GROUP,"mode":"easy"}),
        );
        counter["required_outcome"] = json!({"group":OUTCOME_GROUP,"mode":"hard"});
        counter["key"] = json!("oteryn:quest-transition/native-kills/qcredit-fixture-hard");
        counter["track"] = json!(HARD_TRACK);
        stages.push(stage("hard-kill", &counter));
        if case.has_shields() {
            // Crystal 201189f8 enraged_arbaziloth.lua:1-8, arbaziloth_death.lua:29-41.
            // Both terminal definitions advance the same first-win track.
            for (mode, creature) in [("easy", RAT), ("hard", CAVE_RAT)] {
                let name = format!("first-win-{mode}");
                stages.push(stage(&name, &json!({
                    "key":format!("oteryn:quest-transition/native-kills/qcredit-fixture-{name}"),
                    "creature":creature,
                    "track":SHIELD_FIRST_WIN_TRACK,
                    "gate":SHIELD_FIRST_WIN_TRACK,
                    "gate_value":0,
                    "maximum":1,
                    "set_to_maximum":true,
                    "damage_credit":{"recipients":"all_damagers"},
                    "required_outcome":{"group":OUTCOME_GROUP,"mode":mode},
                    "first_defeat_outfit":{"outfit":"oteryn:outfit.fiend_slayer","addons":0},
                    "first_defeat_outfit_message":FIRST_DEFEAT_MESSAGE
                })));
            }
        }
    } else if case.is_window() {
        stages.push(json!({
            "key":"timed-exit", "completes":false,
            "trigger":{"kind":"step_in","callback":window_callback(at, case)?}
        }));
    }
    let fixture_digest = Sha256::digest(serde_json::to_vec(&stages)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut definition = json!({"schema":1, "quests":[{
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
    if case == Case::FirstDefeatOutfit {
        definition["quests"][0]["tracks"]
            .as_array_mut()
            .ok_or("outfit fixture tracks")?
            .push(json!({"key":PREOWNED_OUTFIT_TRACK,"initial":0,"minimum":0,"maximum":1}));
    }
    if case.has_shields() {
        definition["quests"][0]["tracks"]
            .as_array_mut()
            .ok_or("shield first-win track")?
            .push(json!({"key":SHIELD_FIRST_WIN_TRACK,"initial":0,"minimum":0,"maximum":1}));
    }
    if case.has_reset() {
        let tracks = definition["quests"][0]["tracks"]
            .as_array_mut()
            .ok_or("crystal fixture tracks")?;
        tracks.extend(CRYSTAL_TRACKS.into_iter().map(|track| {
            json!({
                "key":track,"initial":1,"minimum":0,"maximum":1
            })
        }));
    }
    if case.has_outcome() {
        definition["quests"][0]["tracks"]
            .as_array_mut()
            .ok_or("mode tracks")?
            .push(json!({
                "key":HARD_TRACK,"initial":0,"minimum":0,"maximum":5
            }));
    }
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
        let world = if case.has_shields() {
            let lever = shield_handoff::lever(at, case)?;
            let exit = mode_exit(at)?;
            Some(super::quest_world_callbacks::world_with_objects(
                &lever, lever.selector.position.ok_or("shield lever")?,
                &[(exit.selector.position.ok_or("exit")?, 37000)],
                &[shield_handoff::FIRE],
            )?)
        } else if case == Case::Outcome {
            let easy = mode_lever(at, false)?;
            let hard = mode_lever(at, true)?;
            let exit = mode_exit(at)?;
            Some(super::quest_world_callbacks::world_with_objects(
                &easy,
                easy.selector.position.ok_or("easy lever")?,
                &[
                    (hard.selector.position.ok_or("hard lever")?, 1945),
                    (exit.selector.position.ok_or("exit")?, 37000),
                ],
                &[1946],
            )?)
        } else if case.is_window() {
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
                let mut objects = reset
                    .positions
                    .iter()
                    .map(|position| (*position, reset.from))
                    .collect::<Vec<_>>();
                if case == Case::WindowResetAmbiguous {
                    objects.push((*reset.positions.get(1).ok_or("second crystal")?, reset.to));
                }
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
        let two_station_party = case.is_window() || case.has_outcome();
        for index in 0..if two_station_party { 2 } else { 3 } {
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
            let initial = if two_station_party && index == 1 {
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
            if case == Case::ShieldPulse {
                let runtime = owner.runtime.lock().await;
                owner.states.spell_states.lock().await.initialize(
                    &runtime, player.actor, player.session,
                    crate::spell::cast::CharacterCastFacts {
                        max_health: 4_000,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0), owner.owner_now(),
                ).ok_or("pulse player state")?;
            } else if case.has_outcome() {
                deps.initialize_player(&owner, player.actor, player.session).await;
            }
            owner.refresh_quest_session(player.session, true).await;
        }

        if cells.is_some() {
            if case.has_shields() {
                shield_handoff::exercise(&owner, &players, at, &h.pool, case).await?;
            } else if case == Case::Outcome {
                exercise_outcome(&owner, &players, at, &h.pool).await?;
            } else {
                exercise_window(&owner, &players, at, &h.pool, case).await?;
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }

        if case == Case::FirstDefeatOutfit {
            exercise_first_defeat_outfit(&owner, &players, at, &h.pool).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }

        if case == Case::PartyArea {
            exercise_positioned_party(&owner, &players, at, &h.pool).await?;
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
            Case::All | Case::Area | Case::PartyArea => [1, 1, 0],
            Case::Top => [1, 1, 0],
            Case::Time | Case::Summon => [1, 2, 0],
            Case::Ordered => [3, 3, 0],
            Case::DefeatTime => {
                let actual = track_value(&h.pool, 60).await?;
                let after = crate::gameplay_transport::npc_trade_command::unix_ms_now() / 1_000;
                assert!((before..=after).contains(&actual));
                [actual, actual, 0]
            }
            Case::FirstDefeatOutfit
            | Case::Window
            | Case::WindowStateless
            | Case::WindowVisual
            | Case::WindowReset
            | Case::WindowResetAmbiguous
            | Case::WindowResetDeparted
            | Case::Outcome | Case::ShieldEasy | Case::ShieldHard | Case::ShieldPulse | Case::ShieldReinforcements => {
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

/// Crystal 201189f8 creaturescripts_the_great_dragon_hunt.lua:12-25,
/// functions.lua:1064-1072 and data/libs/functions/party.lua Participants.
async fn exercise_positioned_party(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    at: MovementLocalPosition,
    pool: &sqlx::PgPool,
) -> TestResult {
    use crate::foundation::{PartyMemberSnapshot, PartyPresence, PartySnapshot};
    let world = owner.world_id.as_bytes();
    let channel = owner.channel_id.as_bytes();
    let party = id(92);
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq,shared_xp_enabled)
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,3,true)")
        .bind(party.as_slice()).bind(world.as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
    let mut members = Vec::new();
    for (character, order) in [(id(60), 1), (id(61), 2)] {
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_revision,presence_state,presence_channel_id)
                     VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,1,encode($4,'hex')::uuid)")
            .bind(character.as_slice()).bind(party.as_slice()).bind(i64::from(order)).bind(channel.as_slice()).execute(&mut *tx).await?;
        members.push(PartyMemberSnapshot {
            character,
            invitation_order: u64::try_from(order)?,
            presence_revision: 1,
            presence: PartyPresence::Channel,
            channel: Some(*channel),
        });
    }
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000000)::bigint")
        .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    owner.runtime.lock().await.project_world_party(players[0].actor, players[0].session, PartySnapshot {
        world: *world,
        character: id(60),
        party_id: Some(party),
        revision: Some(1),
        leader: Some(id(60)),
        shared_xp_enabled: true,
        members,
        observed_database_micros: now,
    }).map_err(debug)?;
    let attackers = [players[0], players[2]];
    for (offset, ordinal, expected) in [
        (3, 150, [0, 0, 0]), (1, 151, [1, 1, 0]),
    ] {
        death(owner, &attackers, RAT, MovementLocalPosition { x: at.x + offset, ..at }, Case::PartyArea, ordinal).await?;
        assert_eq!(owner.drain_kill_rewards(players[1].session).await, DrainOutcome::Drained);
        assert_tracks(pool, expected).await?;
    }
    Ok(())
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
        }
        if case == Case::WindowResetAmbiguous {
            change_second_crystal_entry(owner, crystals[1], 14961)?;
        }
        for (attempt, clock) in [319_999, 320_000, 320_000, 320_000].into_iter().enumerate() {
            if case == Case::WindowResetAmbiguous && attempt == 2 {
                change_second_crystal_entry(owner, crystals[1], 14955)?;
            }
            let reset = clock == 320_000
                && (case == Case::WindowReset || case == Case::WindowResetAmbiguous && attempt >= 2);
            set_clock(clock);
            {
                let runtime = owner.runtime.lock().await;
                owner.drain_quest_death_map_resets(&runtime);
            }
            owner.drain_quest_death_character_resets().await;
            for crystal in crystals {
                assert_eq!(
                    portal_appearance(owner, crystal)?,
                    if reset { 14955 } else { 14961 }
                );
            }
            for (character, expected) in [(60, 1), (61, i64::from(!reset))] {
                for track in CRYSTAL_TRACKS {
                    let value: i64 = sqlx::query_scalar(
                        "SELECT coalesce((SELECT value FROM game_character_quest_tracks
                         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2), 1)",
                    )
                    .bind(id(character).as_slice())
                    .bind(track)
                    .fetch_one(pool)
                    .await?;
                    assert_eq!(
                        value, expected,
                        "personal reset for {character}, track {track}"
                    );
                }
            }
            let receipts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
            )
            .bind(CRYSTAL_RESET)
            .fetch_one(pool)
            .await?;
            assert_eq!(receipts, i64::from(reset));
        }
    }
    Ok(())
}

fn change_second_crystal_entry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    position: MovementLocalPosition,
    appearance: u16,
) -> TestResult {
    use crate::map::{overlay::TilePos, view::{MapFacts, placement_key}};
    let map = owner.bundle_map().ok_or("crystal bundle map")?;
    let tile = TilePos {
        x: u16::try_from(position.x)?,
        y: u16::try_from(position.y)?,
        floor: i8::try_from(-position.floor)?,
    };
    let key = placement_key(tile, 2).ok_or("competing crystal entry")?;
    let replacement = map.facts.quest_callback_replacement(key, appearance)
        .ok_or("competing crystal appearance")?;
    assert!(map.facts.apply_quest_callback(
        key, map.facts.object_revision(key), replacement, &[], &[], false, None,
    ));
    Ok(())
}

async fn exercise_outcome(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    at: MovementLocalPosition,
    pool: &sqlx::PgPool,
) -> TestResult {
    use crate::map::{
        floor::CellBlocker,
        view::{MapFacts, placement_key},
    };
    let lobby = [
        at,
        MovementLocalPosition {
            x: at.x + 1,
            y: at.y - 1,
            ..at
        },
    ];
    let room = [
        MovementLocalPosition { x: at.x + 3, ..at },
        MovementLocalPosition {
            x: at.x + 3,
            y: at.y - 1,
            ..at
        },
    ];
    let portal = MovementLocalPosition { x: at.x + 6, ..at };
    let landing = MovementLocalPosition { x: at.x + 8, ..at };
    let map = owner.bundle_map().ok_or("encounter bundle map")?;
    let doors = crate::map::floor::BundleDoors(&map.facts);
    let blocker = crate::gameplay_transport::quest_callbacks::CallbackPortals {
        inner: &doors,
        map: Some(map),
        callbacks: owner.callback_content(),
    };
    assert!(
        blocker.suppresses_native_teleport(crate::content::LogicalCell {
            x: portal.x,
            y: portal.y,
            z: i32::from(portal.floor),
        }),
        "the authored gate must own the raw teleport before StepIn"
    );
    for (index, hard) in [false, true].into_iter().enumerate() {
        let callback = mode_lever(at, hard)?;
        let tile = callback.selector.position.ok_or("lever tile")?;
        let key = placement_key(
            crate::map::overlay::TilePos {
                x: tile.x,
                y: tile.y,
                floor: tile.floor,
            },
            1,
        )
        .ok_or("lever key")?;
        let terminal = {
            let mut runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let player = players[0];
            let command = CommandRef::new(
                player.session,
                CommandId::new(500 + u64::try_from(index)?).map_err(debug)?,
            );
            let after = map
                .facts
                .quest_callback_replacement(key, 1946)
                .ok_or("lever appearance")?;
            let revision = map.facts.object_revision(key);
            let origins = players.iter().map(|player| runtime.read_actor_position(player.actor))
                .collect::<Result<Vec<_>, _>>().map_err(debug)?;
            let creature_count = runtime.visible_entities().creatures.len();
            let use_lever = |runtime: &mut ChannelRuntimeV1,
                             states: &mut crate::gameplay_transport::actor_spell::ChannelSpellStates,
                             revision| {
                owner.execute_quest_encounter(
                    runtime,
                    states,
                    player.actor,
                    player.session,
                    map,
                    &callback,
                    key,
                    revision,
                    after,
                    false,
                    crate::durability::quest_state::QuestCause::Use(command),
                    None,
                )
            };
            assert_eq!(
                use_lever(&mut runtime, &mut states, revision.checked_add(1).ok_or("stale revision")?),
                oteryn_protocol_oteryn::world_object::UseDisposition::Rejected
            );
            assert_eq!(map.facts.object_revision(key), revision);
            assert_eq!(runtime.visible_entities().creatures.len(), creature_count);
            for (member, origin) in players.iter().zip(origins) {
                assert_eq!(runtime.read_actor_position(member.actor).map_err(debug)?, origin);
            }
            assert_eq!(
                use_lever(&mut runtime, &mut states, revision),
                oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            );
            for (player, expected) in players.iter().zip(room) {
                assert_eq!(
                    runtime
                        .read_actor_position(player.actor)
                        .map_err(debug)?
                        .position(),
                    expected
                );
            }
            runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|creature| {
                    runtime.matches_live_creature_identity(creature.actor, RAT.as_bytes())
                })
                .ok_or("admitted terminal")?
                .actor
        };
        for (player, origin) in players.iter().zip(room) {
            assert_eq!(enter_exit(owner, *player, portal).await?, origin);
        }
        if hard {
            // A second actor of the same species cannot supply this run's outcome.
            death(
                owner,
                players,
                RAT,
                MovementLocalPosition {
                    x: at.x + 4,
                    y: at.y + 1,
                    ..at
                },
                Case::Outcome,
                301,
            )
            .await?;
            for (player, origin) in players.iter().zip(room) {
                assert_eq!(enter_exit(owner, *player, portal).await?, origin);
            }
        }
        settle_actor_death(
            owner,
            players,
            RAT,
            terminal,
            Case::Outcome,
            if hard { 302 } else { 300 },
        )
        .await?;
        owner
            .runtime
            .lock()
            .await
            .remove_encounter_actor(terminal)
            .map_err(debug)?;
        for ((player, origin), lobby_cell) in players.iter().zip(room).zip(lobby) {
            assert_eq!(
                enter_exit(owner, *player, portal).await?,
                if hard { landing } else { origin }
            );
            if hard {
                move_to(owner, *player, lobby_cell).await?;
            }
        }
        for character in [60, 61] {
            assert_eq!(track_value(pool, character).await?, 1);
            let hard_wins:i64 = sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)")
                .bind(id(character).as_slice()).bind(HARD_TRACK).fetch_one(pool).await?;
            assert_eq!(hard_wins, i64::from(hard));
        }
        if !hard {
            for (player, origin) in players.iter().zip(lobby) {
                move_to(owner, *player, origin).await?;
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

async fn exercise_first_defeat_outfit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    at: MovementLocalPosition,
    pool: &sqlx::PgPool,
) -> TestResult {
    const GRANT: &str = "oteryn:outfit.fiend_slayer/quest-grant/0";
    for player in players {
        owner.states.chat.admit(player.session, "Adventurer", owner.owner_now());
    }
    // Interrupt the actual quest track write after the outfit insert. The
    // nontransactional sequence proves the writer reached that boundary.
    sqlx::raw_sql(
        "CREATE SEQUENCE qcredit_outfit_fault_hits;
         GRANT USAGE, SELECT ON SEQUENCE qcredit_outfit_fault_hits TO oteryn_game_runtime;
         CREATE FUNCTION qcredit_fail_first_defeat() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
           IF EXISTS(SELECT 1 FROM game_account_appearance_unlock_receipts
                     WHERE granting_character_id=NEW.character_id
                     AND unlock_key='oteryn:outfit.fiend_slayer/quest-grant/0') THEN
               PERFORM nextval('qcredit_outfit_fault_hits');
           END IF;
           RETURN NULL;
         END $$;
         CREATE TRIGGER qcredit_fail_first_defeat BEFORE INSERT ON game_character_quest_tracks
         FOR EACH ROW WHEN(NEW.track_key='oteryn:quest-progress/qcredit-fixture/kills')
         EXECUTE FUNCTION qcredit_fail_first_defeat();",
    )
    .execute(pool)
    .await?;
    let position = MovementLocalPosition { x: at.x + 1, ..at };
    let case = Case::FirstDefeatOutfit;
    let target = spawn_credit_creature(owner, RAT, position).await?;
    queue_actor_death(owner, players, RAT, target, case, 200).await?;
    assert_eq!(
        owner.drain_kill_rewards(players[0].session).await,
        DrainOutcome::Deferred
    );
    let reached: bool = sqlx::query_scalar("SELECT is_called FROM qcredit_outfit_fault_hits")
        .fetch_one(pool)
        .await?;
    assert!(reached, "the outfit was inserted before the injected track failure");
    assert_tracks(pool, [0, 0, 0]).await?;
    assert_first_defeat_messages(owner, players, [0, 0, 0]);
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key=$1",
    )
    .bind(GRANT)
    .fetch_one(pool)
    .await?;
    assert_eq!(grants, 0);
    // Let one recipient commit, then defer the second. Retrying the same
    // death must replay the first receipt without publishing its message.
    sqlx::raw_sql(
        "CREATE OR REPLACE FUNCTION qcredit_fail_first_defeat() RETURNS trigger
         LANGUAGE plpgsql AS $$
         BEGIN
           IF EXISTS(SELECT 1 FROM game_account_appearance_unlock_receipts
                     WHERE granting_character_id<>NEW.character_id
                     AND unlock_key='oteryn:outfit.fiend_slayer/quest-grant/0') THEN
               RETURN NULL;
           END IF;
           RETURN NEW;
         END $$;",
    )
    .execute(pool)
    .await?;
    assert_eq!(
        owner.drain_kill_rewards(players[0].session).await,
        DrainOutcome::Deferred
    );
    let first = [
        usize::try_from(track_value(pool, 60).await?)?,
        usize::try_from(track_value(pool, 61).await?)?,
        0,
    ];
    assert!(matches!(first, [1, 0, 0] | [0, 1, 0]));
    assert_first_defeat_messages(owner, players, first);
    sqlx::query("DROP TRIGGER qcredit_fail_first_defeat ON game_character_quest_tracks")
        .execute(pool)
        .await?;
    assert_eq!(
        owner.drain_kill_rewards(players[0].session).await,
        DrainOutcome::Drained
    );
    owner
        .runtime
        .lock()
        .await
        .remove_arena_opponent(target)
        .map_err(debug)?;
    assert_tracks(pool, [1, 1, 0]).await?;
    assert_first_defeat_outfits(pool).await?;
    assert_first_defeat_messages(owner, players, [1 - first[0], 1 - first[1], 0]);

    death(owner, players, RAT, position, case, 201).await?;
    assert_tracks(pool, [1, 1, 0]).await?;
    assert_first_defeat_messages(owner, players, [0, 0, 0]);

    // A separate first-defeat transition still credits existing outfit owners.
    death(owner, players, CAVE_RAT, position, case, 202).await?;
    let credited: Vec<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1 ORDER BY character_id",
    )
    .bind(PREOWNED_OUTFIT_TRACK)
    .fetch_all(pool)
    .await?;
    assert_eq!(credited, [1, 1]);
    assert_first_defeat_messages(owner, players, [0, 0, 0]);
    assert_first_defeat_outfits(pool).await
}

fn assert_first_defeat_messages<const N: usize>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    expected: [usize; N],
) {
    use oteryn_protocol_oteryn::chat::ChatLine;

    assert_eq!(players.len(), expected.len());
    for (player, count) in players.iter().zip(expected) {
        let lines = owner.states.chat.drain(player.session);
        assert_eq!(
            lines
                .iter()
                .filter(|line| {
                    matches!(line, ChatLine::Private { speaker_name, text }
                        if speaker_name == "Game" && text == FIRST_DEFEAT_MESSAGE)
                })
                .count(),
            count
        );
    }
}

async fn assert_first_defeat_outfits(pool: &sqlx::PgPool) -> TestResult {
    const GRANT: &str = "oteryn:outfit.fiend_slayer/quest-grant/0";
    let receipts: Vec<String> = sqlx::query_scalar(
        "SELECT granting_character_id::text FROM game_account_appearance_unlock_receipts
         WHERE unlock_key=$1 AND source_kind='quest' ORDER BY granting_character_id",
    )
    .bind(GRANT)
    .fetch_all(pool)
    .await?;
    let expected = [60, 61].map(|character| sqlx::types::Uuid::from_bytes(id(character)).to_string());
    assert_eq!(receipts, expected);
    let unlocks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_appearance_unlocks
         WHERE unlock_key LIKE 'oteryn:outfit.fiend_slayer/quest-grant/%'",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(unlocks, 2, "both accounts earn the base and no addon");
    Ok(())
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
    let target = spawn_credit_creature(owner, creature, at).await?;
    settle_actor_death(owner, players, creature, target, case, ordinal).await?;
    owner
        .runtime
        .lock()
        .await
        .remove_arena_opponent(target)
        .map_err(debug)?;
    Ok(())
}

async fn spawn_credit_creature(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    creature: &str,
    at: MovementLocalPosition,
) -> TestResult<ExactActorRef> {
    let mut runtime = owner.runtime.lock().await;
    let policy = runtime.companion_policy(creature).map_err(debug)?;
    let target = runtime
        .admit_encounter_actor(creature, policy.maximum_health, at)
        .map_err(debug)?;
    runtime
        .install_creature_policy(target, creature)
        .map_err(debug)?;
    Ok(target)
}

async fn settle_actor_death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    creature: &str,
    target: ExactActorRef,
    case: Case,
    ordinal: u64,
) -> TestResult {
    queue_actor_death(owner, players, creature, target, case, ordinal).await?;
    for player in players.iter().take(2) {
        assert_eq!(
            owner.drain_kill_rewards(player.session).await,
            DrainOutcome::Drained
        );
    }
    Ok(())
}

async fn queue_actor_death(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    players: &[Player],
    creature: &str,
    target: ExactActorRef,
    case: Case,
    ordinal: u64,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let queue = owner.states.attack.lock().await.kills().clone();
    let now_ms = 100_000 + ordinal * 1_000;
    {
        let mut runtime = owner.runtime.lock().await;
        let hp = runtime.companion_snapshot(target).map_err(debug)?.health;
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
    }
    Ok(())
}
