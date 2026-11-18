//! Crystal 201189f8 data-global/npc/barnabas_dee.lua:72-98.
use std::sync::Arc;

use super::TestResult;
use super::archivist_vote::{FixtureItem, with_npc_items};
use super::classic_quests::{talk, talk_with_disposition};
use super::fixture::{Harness, debug, id, run};
use super::scenarios::{mint_carried, quantity};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, authored_bindings::Bindings};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use serde_json::json;

const QUEST: &str = "oteryn:quest.dark_trails_quest";
const ORAMOND: &str = "oteryn:quest.oramond_quest";
const NPC: &str = "oteryn:npc.barnabas_dee";
const POWDER: &str =
    "oteryn:quest-progress/quest/u10_50/oramond_quest/the_powder_of_the_stars/mission";
const VOTES: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";
const SEANCE: &str = "oteryn:quest-progress/quest/u10_50/dark_trails/mission15";
const POLLEN: &str = "oteryn:item.tibia.i21089";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/barnabas_dee.lua:72-98";

#[test]
fn seance_consumes_fifteen_pollen_and_credits_both_quests_at_the_same_revision() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    pollen_case(admin, "seance_pollen_handoff", 1)
}

#[test]
fn barnabas_spends_self_harvested_pollen_and_awards_a_vote_outside_the_seance() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    pollen_case(admin, "barnabas_pollen_vote", -1)
}

fn pollen_case(admin: String, database: &'static str, initial_mission: i64) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    database,
                    false,
                    false,
                )
                .await?;
                let result = Box::pin(exercise(&h, initial_mission)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "seance pollen case panicked")??;
    Ok(())
}

async fn exercise(h: &Harness, initial_mission: i64) -> TestResult {
    let base = QuestStateCatalogue::new(
        "content-1",
        vec![
            QuestTrack {
                key: POWDER.into(),
                quest: ORAMOND.into(),
                initial: 1,
                min: -1,
                max: 1,
            },
            QuestTrack {
                key: VOTES.into(),
                quest: ORAMOND.into(),
                initial: -1,
                min: -1,
                max: i64::MAX,
            },
        ],
        vec![],
    )
    .map_err(debug)?;
    let seance = json!({
        "key":"oteryn:quest-reward/fixture/seance-pollen","quest":QUEST,"npc":NPC,
        "topic":1,"next_topic":2,"keywords":["yes"],
        "storage":[{"track":SEANCE,"minimum":1,"maximum":1,"set":2}],
        "participants":[{"quest":ORAMOND,"storage":[{"track":POWDER,"minimum":1,"maximum":1,"set":-1}]}],
        "inventory":[{"item":POLLEN,"minimum":15,"maximum":u32::MAX}],
        "take":[{"item":POLLEN,"count":15}],
        "reply":["Now we shall proceed with the seance."],"failure_reply":["I need 15 doses."],"failure_next_topic":0,"sources":[SOURCE]
    });
    // Crystal barnabas_dee.lua:84-94 preserves the séance mission and credits
    // Oramond instead. The source's unset vote counter starts at one.
    let mut vote = seance.clone();
    vote["key"] = json!("oteryn:quest-reward/fixture/barnabas-pollen-vote");
    vote["storage"] = json!([{"track":SEANCE,"minimum":-1,"maximum":0,"set":0,"preserve":true}]);
    vote["participants"][0]["state_ops"] = json!([{
        "track":VOTES,"from":{"kind":"any"},"effect":{"kind":"add_at_least","value":1,"minimum":1}
    }]);
    vote["reply"] = json!(["Here, take this vote for your effort."]);
    vote["next_topic"] = json!(0);
    let mut post_seance_vote = vote.clone();
    post_seance_vote["key"] = json!("oteryn:quest-reward/fixture/barnabas-post-seance-vote");
    post_seance_vote["storage"] =
        json!([{"track":SEANCE,"minimum":2,"maximum":3,"set":2,"preserve":true}]);
    let binding = Bindings::parse(json!({"schema":1,"quests":[{
        "quest":QUEST,
        "recipe_sha256":<sha2::Sha256 as sha2::Digest>::digest(SOURCE).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        "tracks":[{"key":SEANCE,"initial":initial_mission,"minimum":-1,"maximum":3}],
        "stages":[{"key":"s1","completes":false,"trigger":{
            "kind":"npc_dialogue","callback":seance,"alternatives":[vote,post_seance_vote]
        }}],"sources":[SOURCE]
    }]}).to_string().as_bytes())?;
    let pollen = FixtureItem {
        id: 21089,
        name: "blue pollen",
        weight: 50,
        stackable: true,
    };
    with_npc_items(h, Arc::new(binding.attach(&base)?), NPC, &[pollen], async |owner, admitted, npc| {
        owner.states.npc_conversations.fixture_reward_offers(&json!({"schema":1,"offers":[{
            "npc":NPC,"keywords":["seance"],"topic":0,"next_topic":1,"storage":[],
            "requires":[{"track":POWDER,"minimum":1,"maximum":1}],"reply":["Did you bring the pollen?"]
        }]}).to_string())?;
        mint_carried(h, owner, *admitted, POLLEN, 14, 90).await?;
        talk(owner, admitted, npc, 1, "hi").await?;
        talk(owner, admitted, npc, 2, "seance").await?;
        assert_eq!(talk_with_disposition(owner, admitted, npc, 3, "yes", NpcIntentDisposition::Rejected).await?, ["I need 15 doses."]);
        assert_eq!(owner.states.npc_conversations.fixture_quest_topic(admitted.game_session_id)?, 0);
        assert_eq!(quantity(h, POLLEN).await?, 14);
        let read = || async {
            sqlx::query_as::<_, (String, i64, String)>(
                "SELECT track_key,value,committed_character_revision::text FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key IN ($2,$3,$4) ORDER BY track_key"
            ).bind(id(41).as_slice()).bind(SEANCE).bind(POWDER).bind(VOTES).fetch_all(&h.pool).await
        };
        assert!(read().await?.is_empty());
        mint_carried(h, owner, *admitted, POLLEN, 2, 91).await?;
        talk(owner, admitted, npc, 4, "seance").await?;
        let (expected_reply, mission, topic) = if initial_mission == 1 {
            ("Now we shall proceed with the seance.", 2, 2)
        } else {
            ("Here, take this vote for your effort.", initial_mission, 0)
        };
        assert_eq!(talk(owner, admitted, npc, 5, "yes").await?, [expected_reply]);
        assert_eq!(owner.states.npc_conversations.fixture_quest_topic(admitted.game_session_id)?, topic);
        assert_eq!(quantity(h, POLLEN).await?, 1);
        let rows = read().await?;
        assert_eq!(rows.len(), if initial_mission == 1 { 2 } else { 3 });
        assert_eq!((&*rows[0].0, rows[0].1), (SEANCE, mission));
        assert_eq!((&*rows[1].0, rows[1].1), (POWDER, -1));
        assert_eq!(rows[0].2, rows[1].2);
        if initial_mission != 1 {
            assert_eq!((&*rows[2].0, rows[2].1), (VOTES, 1));
            assert_eq!(rows[0].2, rows[2].2);
        }
        Ok(())
    }).await
}
