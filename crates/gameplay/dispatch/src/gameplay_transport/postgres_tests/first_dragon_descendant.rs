//! Crystal 201189f8, data-global/scripts/quests/the_first_dragon/
//! movements_entrance_teleport.lua:41-87 and data/libs/functions/functions.lua:226-239.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};

pub(super) const CASE: &str = "first-dragon-tazhadur-step";
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
const BOSS: &str = "oteryn:creature.tazhadur";
const QUEST: &str = "oteryn:quest.the_first_dragon_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/questline";
const DRAGONS: &str = "oteryn:quest-progress/quest/u11_02/the_first_dragon/dragon_counter";
const ENTERED: &str = "oteryn:quest-progress/first-dragon/tazhadur-visit";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/movements_entrance_teleport.lua:1-87";

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

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/first-dragon/tazhadur-entry",
        "quest": QUEST,
        "trigger": "step_in",
        "selector": {"position": PORTAL, "unique": 35001, "appearance": 1949},
        "reversible": false,
        "requires_world_admission": true,
        "return_on_failure": true,
        "offerings": [],
        "passages": [],
        "storage": [
            {"track":QUESTLINE, "minimum":1, "maximum":1, "set":1, "preserve":true},
            {"track":DRAGONS, "minimum":200, "maximum":200, "set":200, "preserve":true},
            {"track":ENTERED, "minimum":0, "maximum":1, "set":1}
        ],
        "effects": {"encounter": {
            "minimum_level": 0,
            "players": [{"from":PORTAL, "to":ARRIVAL}],
            "monsters": [{"creature":BOSS, "position":{"x":32018, "y":32465, "floor":-8}}],
            "room_min": {"x":32008,"y":32455,"floor":-8},
            "room_max": {"x":32028,"y":32475,"floor":-8},
            "personal_cooldown_seconds": 7200,
            "personal_timeout_ms": 1800000
        }},
        "sources": [SOURCE]
    }))?)
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
                {"key":DRAGONS,"initial":200,"minimum":-1,"maximum":200},
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
) -> TestResult {
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
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
            super::quest_world_callbacks::local(ARRIVAL)
        );
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .find(|actor| runtime.matches_live_creature_identity(*actor, BOSS.as_bytes()))
            .ok_or("Tazhadur did not spawn")?
    };
    let cooldown: i64 = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM now()) * 1000)::bigint \
           FROM game_character_boss_cooldowns WHERE boss_key=$1",
    )
    .bind(BOSS)
    .fetch_one(&h.pool)
    .await?;
    assert!((7_190_000..=7_200_000).contains(&cooldown));
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
    }
    assert!(
        owner
            .states
            .quest_callbacks
            .next_encounter_due_us()
            .is_none()
    );
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
            .any(|actor| runtime.matches_live_creature_identity(*actor, BOSS.as_bytes()))
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
