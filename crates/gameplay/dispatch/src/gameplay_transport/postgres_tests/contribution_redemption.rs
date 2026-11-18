//! Crystal 201189f8 data-global/npc/sandomo.lua:53-59,97-165.
use std::sync::Arc;

use super::TestResult;
use super::archivist_vote::with_npc;
use super::classic_quests::talk;
use super::fixture::{Harness, debug, id, run};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, npc_rules::NpcRuleCatalog};
use serde_json::{Value, json};

const QUEST: &str = "oteryn:quest.roshamuul_quest";
const NPC: &str = "oteryn:npc.sandomo";
const MORTAR: &str = "oteryn:quest-progress/quest/u10_30/roshamuul_quest/rosk_mortar_thrown";
const FRAZZLEMAWS: &str =
    "oteryn:quest-progress/quest/u10_30/roshamuul_quest/rosk_killed_frazzlemaws";
const SILENCERS: &str = "oteryn:quest-progress/quest/u10_30/roshamuul_quest/rosk_killed_silencers";
const GOLD: &str = "oteryn:quest-progress/quest/u10_30/roshamuul_quest/rosk_gold_record";
const ITEM: &str = "oteryn:item.tibia.i20062";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sandomo.lua:97-165";

#[test]
fn contribution_books_reset_counts_and_two_quantity_choices_redeem_exact_gold() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |_| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "contribution_redemption",
                    false,
                    false,
                )
                .await?;
                let result = Box::pin(exercise(&h)).await;
                let cleanup = h.cleanup().await;
                result?;
                cleanup
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "contribution redemption case panicked")??;
    Ok(())
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let terms = json!([
        {"track":MORTAR,"weight":100},
        {"track":FRAZZLEMAWS,"weight":1},
        {"track":SILENCERS,"weight":1}
    ]);
    let topic = |value| json!({"kind":"set_topic","value":value});
    let quantity =
        |status| json!({"kind":"redemption_quantity","track":GOLD,"unit_cost":100,"status":status});
    let rule = |key, words: &[&str], from, guards: Vec<Value>, effects: Vec<Value>, reply| {
        json!({
            "key":format!("oteryn:quest-npc/roshamuul_quest/sandomo/{key}"),
            "quest":QUEST,"npc":NPC,"trigger":"say","topics":[from],"keywords":words,
            "any_message":words.is_empty(),"guards":guards,"effects":effects,"reply":[reply],"sources":[SOURCE]
        })
    };
    let rules = vec![
        rule(
            "gratitude",
            &["gratitude"],
            0,
            vec![],
            vec![topic(3)],
            "How did you contribute?",
        ),
        rule(
            "repairs",
            &["repairs"],
            3,
            vec![json!({"kind":"contribution_total","op":"gt","value":0,"terms":terms})],
            vec![topic(4)],
            "You have contributions to record.",
        ),
        rule(
            "books",
            &["books", "book"],
            4,
            vec![],
            vec![topic(5)],
            "Record the recent contributions?",
        ),
        rule(
            "confirm",
            &["yes"],
            5,
            vec![],
            vec![
                json!({"kind":"set_contribution_total","track":GOLD,"terms":terms}),
                json!({"kind":"set","track":MORTAR,"value":0}),
                json!({"kind":"set","track":FRAZZLEMAWS,"value":0}),
                json!({"kind":"set","track":SILENCERS,"value":0}),
                topic(0),
            ],
            "Contributions recorded.",
        ),
        rule(
            "trade",
            &["trade"],
            0,
            vec![json!({"kind":"storage","track":GOLD,"op":"ge","value":100})],
            vec![topic(6)],
            "How many clusters?",
        ),
        rule(
            "invalid",
            &[],
            6,
            vec![quantity("invalid")],
            vec![],
            "Give a whole positive quantity.",
        ),
        rule(
            "insufficient",
            &[],
            6,
            vec![quantity("insufficient")],
            vec![],
            "Your balance cannot cover that quantity.",
        ),
        rule(
            "redeem",
            &[],
            6,
            vec![quantity("affordable")],
            vec![
                json!({
                    "kind":"redeem_contribution","track":GOLD,"unit_cost":100,"item":ITEM
                }),
                topic(0),
            ],
            "Clusters delivered.",
        ),
    ];
    let tracks =
        [(MORTAR, 3), (FRAZZLEMAWS, 2), (SILENCERS, 1), (GOLD, 500)].map(|(key, initial)| {
            json!({
                "key":key,"quest":QUEST,"initial":initial,"min":-1,"max":i64::MAX,"sources":[SOURCE]
            })
        });
    let source = json!({"schema":1,"quest":QUEST,"tracks":tracks,"rules":rules}).to_string();
    let base = QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(debug)?;
    Ok(NpcRuleCatalog::from_json_documents(&[&source])?.with_fixture_npc_rules(base)?)
}

async fn exercise(h: &Harness) -> TestResult {
    with_npc(h, Arc::new(catalogue()?), NPC, async |owner, admitted, npc| {
        talk(owner, admitted, npc, 1, "hi").await?;
        for (index, word) in ["gratitude", "repairs", "books", "yes", "trade"].into_iter().enumerate() {
            talk(owner, admitted, npc, u64::try_from(index)? + 2, word).await?;
        }
        let amount = |track: &'static str| async move {
            sqlx::query_scalar::<_, i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                .bind(id(41).as_slice()).bind(track).fetch_one(&h.pool).await
        };
        assert_eq!(amount(GOLD).await?, 303);
        for track in [MORTAR, FRAZZLEMAWS, SILENCERS] {
            assert_eq!(amount(track).await?, 0);
        }
        assert_eq!(talk(owner, admitted, npc, 7, "1.5").await?, ["Give a whole positive quantity."]);
        assert_eq!(talk(owner, admitted, npc, 8, "4").await?, ["Your balance cannot cover that quantity."]);
        assert_eq!(amount(GOLD).await?, 303);
        assert_eq!(talk(owner, admitted, npc, 9, "2").await?, ["Clusters delivered."]);
        assert_eq!(amount(GOLD).await?, 103);
        talk(owner, admitted, npc, 10, "trade").await?;
        assert_eq!(talk(owner, admitted, npc, 11, "1").await?, ["Clusters delivered."]);
        assert_eq!(amount(GOLD).await?, 3);
        let (
            items, receipts
        ): (i64, i64) = sqlx::query_as("SELECT (SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1), (SELECT count(*) FROM game_quest_npc_item_sources WHERE transition_key='oteryn:quest-npc/roshamuul_quest/sandomo/redeem')")
            .bind(ITEM).fetch_one(&h.pool).await?;
        assert_eq!((items, receipts), (3, 2));
        Ok(())
    }).await
}
