use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use std::sync::Arc;

const NPC: &str = "oteryn:npc.the_orc_king";
const QUEST: &str = "oteryn:quest.orc_fortress_quest";
const TRACK: &str = "oteryn:quest-progress/orc-fortress/king-greeted";
const CLAIM: &str = "oteryn:quest-reward/orc-fortress/king-greeted";
const AMBUSH: &str = "Arrrrgh! A dirty paleskin! To me my children! Kill them my guards!";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/the_orc_king.lua:48-63";

#[test]
fn orc_king_first_greeting_claims_personal_flag_and_ambushes_only_once() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("orc-king-first-greeting", || {
        super::quest_world_callbacks_fixtures::npc_scenario(
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            |owner, admitted, harness| Box::pin(exercise(owner, admitted, harness)),
        )
    })
}

async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let bindings = Bindings::parse(&serde_json::to_vec(&serde_json::json!({
        "schema": 1, "quests": [{
            "quest": QUEST,
            "recipe_sha256": format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(SOURCE)),
            "sources": [SOURCE],
            "tracks": [{"key": TRACK, "initial": 0, "minimum": 0, "maximum": 1}],
            "stages": [{"key": "first-greeting", "completes": false, "trigger": {
                "kind": "npc_dialogue", "callback": {
                    "key": CLAIM, "quest": QUEST, "npc": NPC, "keywords": ["hi"], "topic": 0, "next_topic": 0,
                    "storage": [{"track": TRACK, "minimum": 0, "maximum": 0, "set": 1}],
                    "take": [], "reply": [AMBUSH], "sources": [SOURCE],
                    "greeting_encounter": [
                        "oteryn:creature.slime", "oteryn:creature.slime", "oteryn:creature.slime",
                        "oteryn:creature.orc_warlord", "oteryn:creature.orc_warlord",
                        "oteryn:creature.orc_leader", "oteryn:creature.orc_leader", "oteryn:creature.orc_leader"
                    ]
                }
            }}]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("base catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [king] = install(owner, admitted, [NPC]).await?;
    let before = owner
        .runtime
        .lock()
        .await
        .current_live_creature_census()
        .map_err(debug)?
        .len();

    assert_eq!(talk(owner, admitted, king, 6301, "hi").await?, [AMBUSH]);
    assert!(
        owner
            .states
            .chat
            .engaged(admitted.game_session_id)
            .is_none()
    );
    let value: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(value, 1);
    let spawned = owner
        .runtime
        .lock()
        .await
        .current_live_creature_census()
        .map_err(debug)?
        .len();
    assert_eq!(spawned, before + 8);

    assert_ne!(talk(owner, admitted, king, 6302, "hi").await?, [AMBUSH]);
    assert!(
        owner
            .states
            .chat
            .engaged(admitted.game_session_id)
            .is_some()
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .current_live_creature_census()
            .map_err(debug)?
            .len(),
        spawned
    );
    Ok(())
}
