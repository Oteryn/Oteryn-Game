//! Crystal 201189f8, dangerous_depth/movements_boss_entrance.lua:10-47.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness, fixture::debug};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use serde_json::json;

pub(super) const CASE: &str = "dangerous-depths-warzone-four-portal";
const PORTAL: Position = Position { x:33460, y:32267, floor:-15 };
const ARRIVAL: Position = Position { x:33650, y:32310, floor:-15 };
const BOSS: &str = "oteryn:creature.the_baron_from_below";
const QUEST: &str = "oteryn:quest.dangerous_depths_quest";
const TRACK: &str = "oteryn:quest-progress/dangerous-depths/warzone-iv/admitted-at";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dangerous_depth/movements_boss_entrance.lua:10-47";

#[test]
fn warzone_step_stamps_individual_cooldown_without_summoning_another_boss() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CASE, || {
        super::quest_world_callbacks::scenario(CASE, PORTAL, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key":"oteryn:quest-callback/dangerous-depths/warzone-iv-entry",
        "quest":QUEST,"trigger":"step_in",
        "selector":{"position":PORTAL,"action":57243,"appearance":1949},
        "reversible":false,"passages":[],"storage":[],"offerings":[],
        "requires_world_admission":true,"return_on_failure":true,
        "teleport":ARRIVAL,
        "state_operations":[{"track":TRACK,"from":{"kind":"any"},"effect":{"kind":"set_now"}}],
        "effects":{"boss_admission_cooldown":{"creature":BOSS,"seconds":36000}},
        "sources":[SOURCE]
    }))?)
}

pub(super) fn catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    use sha2::{Digest, Sha256};
    let digest: String = Sha256::digest(SOURCE).iter().map(|byte| format!("{byte:02x}")).collect();
    let fixture = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"sources":[SOURCE],
        "tracks":[{"key":TRACK,"initial":-1,"minimum":-1,"maximum":i64::MAX}],
        "stages":[{"key":"warzone-iv-entry","completes":false,
            "trigger":{"kind":"step_in","callback":callback}}]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&fixture)?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let origin = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let entered = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(entered.position(), super::quest_world_callbacks::local(ARRIVAL));
        assert!(runtime.current_live_creature_census().map_err(debug)?.is_empty());
        let map = owner.bundle_map().ok_or("map")?;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let return_to_approach = crate::map::floor::prepare_quest_callback_teleport(
            &runtime, owner.movement_cells(), actor, admitted.game_session_id,
            entered, origin, false, &closed,
        ).map_err(debug)?;
        runtime.commit_source_step(return_to_approach).map_err(debug)?;
    }
    super::questpc7_ferumbras::step(owner, admitted, StepDirection::East).await?;
    assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), origin);
    let cooldown: i64 = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM now()) * 1000)::bigint FROM game_character_boss_cooldowns WHERE boss_key=$1",
    ).bind(BOSS).fetch_one(&h.pool).await?;
    assert!((35_990_000..=36_000_000).contains(&cooldown));
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1",
    ).bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(receipts, 1);
    Ok(())
}
