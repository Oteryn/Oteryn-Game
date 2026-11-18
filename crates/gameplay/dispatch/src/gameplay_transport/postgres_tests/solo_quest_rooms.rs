//! Crystal 201189f8 Wanted actions_portal_minis_kilmaresh.lua:2-15,59-106;
//! Path of Defiances movements_teleportTo.lua:1-7,75-90,95-128.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{local, wire};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{CharacterId, CharacterLease, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::{Arc, atomic::Ordering};

const QUEST: &str = "oteryn:quest.fixture_solo_room";
const TRACK: &str = "oteryn:quest-progress/fixture-solo-room/entered";
const CLOCK: &str = "oteryn:quest-progress/fixture-solo-room/cooldown";
const KEY: &str = "oteryn:quest-callback/fixture-solo-room/entry";

struct Room {
    entry: Position,
    landing: Position,
    boss_position: Position,
    min: Position,
    max: Position,
    exit: Position,
    boss: &'static str,
    source: &'static str,
    timeout_ms: u32,
    wanted: bool,
}

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

const WANTED: Room = Room {
    entry: at(33886, 31477, -6),
    landing: at(33871, 31547, -8),
    boss_position: at(33871, 31552, -8),
    min: at(33866, 31545, -8),
    max: at(33876, 31555, -8),
    exit: at(33886, 31478, -6),
    boss: "oteryn:creature.neferi_the_spy",
    source: "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/kilmaresh/actions_portal_minis_kilmaresh.lua:2-15,59-106",
    timeout_ms: 600_000,
    wanted: true,
};
const DEFIANCES: Room = Room {
    entry: at(32963, 32312, -8),
    landing: at(32958, 32309, -8),
    boss_position: at(32951, 32309, -8),
    min: at(32943, 32303, -8),
    max: at(32960, 32315, -8),
    exit: at(32963, 32307, -8),
    boss: "oteryn:creature.furious_scorpion",
    source: "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_secret_library_quest/the_parth_of_defiances_desert/movements_teleportTo.lua:1-7,75-90,95-128",
    timeout_ms: 300_000,
    wanted: false,
};

#[test]
fn wanted_step_sets_boss_cooldown_and_ejects_both_current_occupants_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| exercise(admin, WANTED).await)
}

#[test]
fn defiances_step_ejects_only_the_challenger_after_five_minutes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run_async(async |admin| {
        exercise(admin, DEFIANCES).await
    })
}

fn catalogue(room: &Room) -> TestResult<QuestStateCatalogue> {
    let data = json!({"schema": 1, "quests": [{
        "quest": QUEST, "recipe_sha256": Sha256::digest(room.source).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "sources": [room.source],
        "tracks": [
            {"key": TRACK, "initial": -1, "minimum": -1, "maximum": 1},
            {"key": CLOCK, "initial": -1, "minimum": -1, "maximum": i64::MAX}
        ],
        "stages": [{
            "key": "oteryn:quest-stage/fixture-solo-room/entry",
            "completes": false, "trigger": {"kind": "step_in", "callback": {
                "key": KEY, "quest": QUEST, "trigger": "step_in",
                "selector": {"position": room.entry, "ordinal": 0},
                "storage": [{"track": TRACK, "minimum": -1, "maximum": 1, "set": 1}],
                "state_operations": if room.wanted { json!([]) } else { json!([{
                    "track": CLOCK, "from": {"kind": "elapsed_at_least", "seconds": 72000},
                    "effect": {"kind": "set_now"}
                }]) },
                "effects": {"encounter": {
                    "players": [{"from": room.entry, "to": room.landing}],
                    "monsters": [{"position": room.boss_position, "creature": room.boss}],
                    "room_min": room.min, "room_max": room.max, "minimum_level": 1,
                    "solo_timeout": {"after_ms": room.timeout_ms, "exit": room.exit,
                        "eject_room": room.wanted,
                        "boss_cooldown_seconds": room.wanted.then_some(36_000)}
                }},
                "reversible": false, "passages": [], "offerings": [], "return_on_failure": true,
                "requires_world_admission": true, "sources": [room.source]
            }}
        }]
    }]});
    let base = QuestStateCatalogue::empty("content-1").ok_or("fixture catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&data)?)?.attach(&base)?)
}

async fn exercise(admin: String, room: Room) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "solo_quest_room", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("content generation")?;
    let tiles: Vec<_> = [
        room.entry,
        room.landing,
        room.boss_position,
        room.exit,
        room.min,
        room.max,
    ]
    .into_iter()
    .map(|p| (p.x, p.y, p.floor, 0, 1948))
    .collect();
    let world = super::quest_callbacks::source_world_spatial(
        (room.entry.x, room.entry.y, room.entry.floor),
        &tiles,
    )?;
    let start = MovementLocalPosition {
        x: i32::from(room.entry.x - 1),
        ..local(room.entry)
    };
    super::arena_match::fixture::runtime_at(&mut deps, generation, &pin, &world, start)?;
    let cells = world
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(catalogue(&room)?));
    deps.initialize(&owner).await;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest session")?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(world.frame_binding_digest(admitted.world_id).as_slice()).execute(&h.pool).await?;
    owner.admit_quest_session(&admitted).await;
    let set_time = |time| {
        owner
            .states
            .test_owner_time
            .store(time + 1u64, Ordering::Relaxed)
    };
    let entered_at = 1_000_000u64;
    set_time(entered_at);
    let result = wire(
        &owner,
        &mut admitted,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved
    );
    let boss = {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            local(room.landing)
        );
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|actor| runtime.matches_live_creature_identity(*actor, room.boss.as_bytes()))
            .ok_or("entered room's boss")?
    };
    if room.wanted {
        let boss_clock: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_boss_cooldowns WHERE boss_key=$1",
        )
        .bind(room.boss)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(boss_clock, 1);
    } else {
        let state = h
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert!(state.tracks().get(CLOCK).is_some_and(|time| *time > 0));
    }
    let peer_position = Position {
        x: room.landing.x + 1,
        ..room.landing
    };
    let peer_session = GameSessionId::decode(&id(51)).map_err(debug)?;
    // The entry portal admits one challenger. Populate the already active room
    // with a later occupant to exercise the source's deadline selection policy.
    let peer = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(peer_session).map_err(debug)?;
        let peer = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                peer,
                peer_session,
                CharacterLease::new(CharacterId::decode(&id(52)).map_err(debug)?, 1)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(peer, local(peer_position))
            .map_err(debug)?;
        peer
    };
    deps.initialize_player(&owner, peer, peer_session).await;
    let due = entered_at + u64::from(room.timeout_ms) * 1_000;
    set_time(due - 1);
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(runtime.contains_live_creature(boss));
        assert_eq!(
            runtime.read_actor_position(peer).map_err(debug)?.position(),
            local(peer_position)
        );
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            local(room.landing)
        );
    }
    set_time(due);
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(!runtime.contains_live_creature(boss));
        assert_eq!(
            runtime.read_actor_position(peer).map_err(debug)?.position(),
            local(if room.wanted {
                room.exit
            } else {
                peer_position
            })
        );
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            local(room.exit)
        );
    }
    if room.wanted {
        // Both players share the forced landing. Re-enter from the clear west
        // tile so the refused STEP has an available return position.
        for (command, direction) in [
            (2, StepDirection::West),
            (3, StepDirection::North),
            (4, StepDirection::East),
        ] {
            set_time(due + command * 2_000_000);
            let result = wire(
                &owner,
                &mut admitted,
                command,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                encode_step_intent(direction),
            )
            .await?;
            assert_eq!(
                decode_step_result(&result).map_err(debug)?,
                StepDisposition::Moved
            );
        }
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            start
        );
    }
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 1);
    drop(owner);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
