//! Crystal 201189f8 mission05_web_terror.lua:88-111,147-161.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

pub(super) const NAME: &str = "rookie-guard-web-grease";
const STONE: Position = Position {
    x: 32014,
    y: 32096,
    floor: -11,
};
pub(super) const WEB: Position = Position {
    x: 32014,
    y: 32097,
    floor: -11,
};
const QUEST: &str = "oteryn:quest.the_rookie_guard_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission05";
const GREASE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission05_web_terror.lua:88-111";
const SAMPLE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission05_web_terror.lua:147-161";

#[test]
fn greasy_stone_use_restarts_captured_attempt_and_web_sample_advances_mission() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, STONE, false)
    })
}

fn document() -> serde_json::Value {
    use serde_json::json;
    json!({"schema":1,"callbacks":[
        {"key":"oteryn:quest-callback/rookie-guard-grease-retry", "quest":QUEST, "trigger":"use",
         "selector":{"position":STONE,"appearance":12727},"requires_world_admission":true,
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":false,
         "storage":[{"track":TRACK,"minimum":4,"maximum":4,"set":2}],
         "effects":{"invisibility_ms":120000},"sources":[GREASE]},
        {"key":"oteryn:quest-callback/rookie-guard-web-sample", "quest":QUEST, "trigger":"use",
         "selector":{"position":WEB,"appearance":100},"requires_world_admission":true,
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":false,
         "storage":[{"track":TRACK,"minimum":2,"maximum":2,"set":3}],
         "sources":[SAMPLE]}
    ]})
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(document()["callbacks"][0].clone())?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let document = document();
    let fixture = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 4,
            min: -1,
            max: 5,
        }],
        vec![],
    )
    .map_err(debug)?;
    owner.quest_catalogue = Some(std::sync::Arc::new(world_callbacks::fixture_catalogue(
        fixture,
        &document.to_string(),
    )?));
    owner
        .states
        .quest_callbacks
        .fixture
        .set(serde_json::from_value(document["callbacks"].clone())?)
        .map_err(|_| "callback fixture already installed")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        use_object(owner, admitted, STONE).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 2);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let player = states.get(&runtime, actor, session).ok_or("player")?;
        let now = owner.owner_now().get();
        assert!(player.owned_invisible_at(now).map_err(debug)?);
        assert!(
            !player
                .owned_invisible_at(now + 120_000_000)
                .map_err(debug)?
        );
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    assert_eq!(
        use_object(owner, admitted, WEB).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 3);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    assert_ne!(
        use_object(owner, admitted, STONE).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 3);
    Ok(())
}

async fn use_object(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
) -> TestResult<UseDisposition> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, 1).ok_or("key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
    Ok(decode_use_result(&reply).map_err(debug)?)
}

async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?,
    )
}
