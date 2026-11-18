//! Crystal 201189f8, data-global/scripts/quests/the_first_dragon/
//! movements_entrance_teleport.lua:41-87 and data/libs/functions/functions.lua:226-239.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

pub(super) const CASE: &str = "first-dragon-tazhadur-step";
pub(super) const SWEEP_CASE: &str = "first-dragon-tazhadur-room-sweep";
// The portal is placed beside the source arena in this fixture. Its UID,
// admission thresholds, arena, monster and timers come from Crystal.
const PORTAL: Position = Position {
    x: 32014,
    y: 32466,
    floor: -8,
};
const ARRIVAL: Position = Position {
    x: 32015,
    y: 32466,
    floor: -8,
};
const KALYASSA: &str = "first-dragon-kalyassa-uid";
const ZORVORAX: &str = "first-dragon-zorvorax-uid";
const GELIDRAZAH: &str = "first-dragon-gelidrazah-uid";
const PEER: Position = Position {
    x: 32016,
    y: 32466,
    floor: -8,
};
const LATE_MONSTER: Position = Position {
    x: 32008,
    y: 32465,
    floor: -8,
};
const OUTSIDE_MONSTER: Position = Position {
    x: 32007,
    y: 32465,
    floor: -8,
};
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/questline";
const ENTERED: &str = "oteryn:quest-progress/first-dragon/descendant-visit";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/movements_entrance_teleport.lua:1-87";

struct Descendant {
    uid: u16,
    creature: &'static str,
    arrival: Position,
    boss_at: Position,
    track: &'static str,
    threshold: i64,
    fixture_value: i64,
}

fn descendant(name: &str) -> Option<Descendant> {
    // Crystal movements_entrance_teleport.lua:1-38. The chest task reaches 19 in
    // actions_treasure_chest.lua:100-109, so the Kalyassa case enters above its threshold.
    let (uid, creature, arrival, boss_at, track, threshold, fixture_value) = match name {
        CASE | SWEEP_CASE => (
            35001,
            "tazhadur",
            ARRIVAL,
            at(32018, 32465),
            "dragon_counter",
            200,
            200,
        ),
        KALYASSA => (
            35002,
            "kalyassa",
            at(32078, 32456),
            at(32079, 32459),
            "chest_counter",
            4,
            19,
        ),
        ZORVORAX => (
            35003,
            "zorvorax",
            at(32008, 32396),
            at(32015, 32396),
            "secrets_counter",
            3,
            3,
        ),
        GELIDRAZAH => (
            35004,
            "gelidrazah_the_frozen",
            at(32076, 32402),
            at(32078, 32400),
            "gelidrazah_access",
            1,
            1,
        ),
        _ => return None,
    };
    Some(Descendant {
        uid,
        creature,
        arrival,
        boss_at,
        track,
        threshold,
        fixture_value,
    })
}

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -8 }
}

pub(super) fn handles(name: &str) -> bool {
    descendant(name).is_some()
}

fn run(case: &'static str) -> TestResult {
    let spec = descendant(case).ok_or("Descendant")?;
    let fixture_portal = at(spec.arrival.x - 1, spec.arrival.y);
    super::kill_reward_e2e::scenario_thread::run(case, move || {
        super::quest_world_callbacks::scenario(case, fixture_portal, false)
    })
}

#[test]
fn tazhadur_step_summons_once_stamps_boss_cooldown_and_ejects_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CASE, || {
        super::quest_world_callbacks::scenario(CASE, PORTAL, false)
    })
}

#[test]
fn tazhadur_deadline_sweeps_late_monsters_but_only_ejects_the_admitted_player() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(SWEEP_CASE)
}

#[test]
fn kalyassa_uid_accepts_chest_progress_above_four_and_stamps_its_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(KALYASSA)
}

#[test]
fn zorvorax_uid_admits_the_three_secrets_route_and_clears_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(ZORVORAX)
}

#[test]
fn gelidrazah_uid_admits_the_access_flag_and_clears_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(GELIDRAZAH)
}

pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let spec = descendant(name).ok_or("Descendant")?;
    let mut row = serde_json::json!({
        "key": format!("oteryn:quest-callback/first-dragon/{}-entry", spec.creature),
        "quest": QUEST,
        "trigger": "step_in",
        "selector": {"unique": spec.uid},
        "reversible": false,
        "requires_world_admission": true,
        "return_on_failure": true,
        "offerings": [],
        "passages": [],
        "storage": [
            {"track":QUESTLINE, "minimum":1, "maximum":1, "set":1, "preserve":true},
            {"track":format!("oteryn:quest-progress/quest/u11_02/the_first_dragon/{}", spec.track),
             "minimum":spec.threshold, "maximum":spec.fixture_value, "set":spec.threshold, "preserve":true},
            {"track":ENTERED, "minimum":0, "maximum":1, "set":1}
        ],
        "effects": {"encounter": {
            "minimum_level": 0,
            "players": [],
            "monsters": [{"creature":format!("oteryn:creature.{}", spec.creature), "position":spec.boss_at}],
            "room_min": at(spec.boss_at.x - 10, spec.boss_at.y - 10),
            "room_max": at(spec.boss_at.x + 10, spec.boss_at.y + 10),
            "personal_cooldown_seconds": 7200,
            "personal_timeout_ms": 1800000,
            "personal_timeout_room_sweep": name != CASE
        }},
        "sources": [SOURCE, "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data/libs/functions/functions.lua:226-239"]
    });
    if name == CASE {
        row["selector"]["position"] = serde_json::to_value(PORTAL)?;
        row["selector"]["appearance"] = 1949.into();
        row["effects"]["encounter"]["players"] = serde_json::json!([{"from":PORTAL,"to":ARRIVAL}]);
    } else {
        row["effects"]["encounter"]["personal_entry_landing"] = serde_json::to_value(spec.arrival)?;
    }
    Ok(serde_json::from_value(row)?)
}

pub(super) fn catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    use sha2::{Digest, Sha256};
    let digest: String = Sha256::digest(SOURCE)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let fixture = serde_json::json!({
        "schema":1,
        "quests":[{
            "quest":QUEST,
            "recipe_sha256":digest,
            "sources":[SOURCE],
            "tracks":[
                {"key":QUESTLINE,"initial":1,"minimum":-1,"maximum":1},
                {"key":callback.storage[1].track,"initial":callback.storage[1].maximum,"minimum":-1,"maximum":callback.storage[1].maximum},
                {"key":ENTERED,"initial":0,"minimum":0,"maximum":1}
            ],
            "stages":[{"key":"tazhadur-entry","completes":false,
                "trigger":{"kind":"step_in","callback":callback}}]
        }]
    });
    let bindings = Bindings::parse(&serde_json::to_vec(&fixture)?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    let spec = descendant(name).ok_or("Descendant")?;
    let boss_key = format!("oteryn:creature.{}", spec.creature);
    let actor = admitted.runtime_actor.ok_or("player")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let origin = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position()
    };
    let step = encode_step_intent(StepDirection::East);
    super::quest_world_callbacks::wire(owner, admitted, 1, 1, step.clone()).await?;
    let boss = {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(spec.arrival)
        );
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|actor| runtime.matches_live_creature_identity(*actor, boss_key.as_bytes()))
            .ok_or("Descendant did not spawn")?
    };
    let cooldown: i64 = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM now()) * 1000)::bigint \
           FROM game_character_boss_cooldowns WHERE boss_key=$1",
    )
    .bind(&boss_key)
    .fetch_one(&h.pool)
    .await?;
    assert!((7_190_000..=7_200_000).contains(&cooldown));
    let sweep_witnesses = if name == SWEEP_CASE {
        let peer = super::ferumbras_crystal_reset::admit_peer(owner, h, PEER).await?;
        let mut runtime = owner.runtime.lock().await;
        let health = runtime
            .companion_policy(&boss_key)
            .map_err(debug)?
            .maximum_health;
        let [late, outside] = runtime
            .commit_quest_creature_spawn(&[LATE_MONSTER, OUTSIDE_MONSTER].map(|at| {
                (
                    boss_key.as_str(),
                    health,
                    super::quest_world_callbacks::local(at),
                )
            }))
            .map_err(debug)?
            .try_into()
            .map_err(|_| "two late monsters across the source room boundary")?;
        Some((peer, late, outside))
    } else {
        None
    };
    let due = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("encounter timeout")?;
    let clock = due.checked_add(1).ok_or("owner clock")?;
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_micros(clock))
        .ok_or("owner clock")?;
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            origin
        );
        assert!(!runtime.contains_live_creature(boss));
        if let Some((peer, late, outside)) = sweep_witnesses {
            assert_eq!(
                runtime.read_actor_position(peer).map_err(debug)?.position(),
                super::quest_world_callbacks::local(PEER)
            );
            assert!(!runtime.contains_live_creature(late));
            assert!(runtime.contains_live_creature(outside));
        }
    }
    assert!(
        owner
            .states
            .quest_callbacks
            .next_encounter_due_us()
            .is_none()
    );
    if name == SWEEP_CASE {
        return Ok(());
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    super::quest_world_callbacks::wire(owner, admitted, 2, 1, step).await?;
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );
    assert!(
        !runtime
            .current_live_creature_census()
            .map_err(debug)?
            .iter()
            .any(|actor| runtime.matches_live_creature_identity(*actor, boss_key.as_bytes()))
    );
    drop(runtime);
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(receipts, 1);
    Ok(())
}
