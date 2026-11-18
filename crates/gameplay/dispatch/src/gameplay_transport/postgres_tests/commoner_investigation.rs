//! Crystal 201189f8 data-global/npc/sholley.lua:57-63.
use std::sync::Arc;

use super::TestResult;
use super::archivist_vote::with_npc;
use super::classic_quests::talk;
use super::fixture::{Harness, debug, id, run};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack, npc_rules::NpcRuleCatalog};

const QUEST: &str = "oteryn:quest.dark_trails_quest";
const NPC: &str = "oteryn:npc.sholley";
const VOTES: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";
const LEAD: &str = "oteryn:quest-progress/quest/u10_50/dark_trails/mission12";
const CREDIT: &str = "oteryn:quest-progress/quest/u10_50/dark_trails/mission13";

#[test]
fn sholley_credits_quandon_investigation_at_fifty_votes_without_debiting_votes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                for (tag, votes) in [
                    ("commoner_investigation_under", 49),
                    ("commoner_investigation_at", 50),
                ] {
                    let h = Harness::create_with_admission_seed(admin.clone(), tag, false, false)
                        .await?;
                    let result = Box::pin(exercise(&h, votes)).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "commoner investigation case panicked")??;
    Ok(())
}

fn catalogue(votes: i64) -> TestResult<QuestStateCatalogue> {
    let source =
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sholley.lua:57-63";
    let base = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: VOTES.into(),
            quest: "oteryn:quest.oramond_quest".into(),
            initial: votes,
            min: -1,
            max: i64::MAX,
        }],
        vec![],
    )
    .map_err(debug)?;
    let rules = serde_json::json!({
        "schema":1,"quest":QUEST,
        "tracks":[
            {"key":LEAD,"quest":QUEST,"initial":1,"min":-1,"max":1,"sources":[source]},
            {"key":CREDIT,"quest":QUEST,"initial":-1,"min":-1,"max":1,"sources":[source]}
        ],
        "rules":[{
            "key":"oteryn:quest-npc/dark_trails_quest/sholley/friend","quest":QUEST,"npc":NPC,
            "trigger":"say","keywords":["friend"],"guards":[
                {"kind":"storage","track":LEAD,"op":"eq","value":1},
                {"kind":"storage","track":CREDIT,"op":"lt","value":1},
                {"kind":"storage","track":VOTES,"op":"ge","value":50}
            ],
            "effects":[{"kind":"set","track":CREDIT,"value":1},{"kind":"set_topic","value":0}],
            "reply":["Investigate the lodging."],"sources":[source]
        }]
    })
    .to_string();
    Ok(NpcRuleCatalog::from_json_documents(&[&rules])?.with_fixture_npc_rules(base)?)
}

async fn exercise(h: &Harness, votes: i64) -> TestResult {
    with_npc(h, Arc::new(catalogue(votes)?), NPC, async |owner, admitted, npc| {
        talk(owner, admitted, npc, 1, "hi").await?;
        let reply = talk(owner, admitted, npc, 2, "friend").await?;
        assert_eq!(reply.iter().any(|line| line == "Investigate the lodging."), votes == 50);
        let read = |track: &'static str| async move {
            sqlx::query_scalar::<_, i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                .bind(id(41).as_slice()).bind(track).fetch_optional(&h.pool).await
        };
        assert_eq!(read(CREDIT).await?, (votes == 50).then_some(1));
        assert_eq!(read(VOTES).await?, None);
        Ok(())
    }).await
}
