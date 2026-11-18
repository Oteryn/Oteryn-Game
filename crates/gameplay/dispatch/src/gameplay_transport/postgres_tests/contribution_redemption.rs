//! Crystal 201189f8 data-global/npc/sandomo.lua:53-59,97-165.
use std::sync::Arc;

use super::TestResult;
use super::archivist_vote::{FixtureItem, with_npc_items};
use super::classic_quests::talk;
use super::fixture::{Harness, debug, id, run};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, npc_rules::NpcRuleCatalog};
use serde_json::{Value, json};

const QUEST: &str = "oteryn:quest.roshamuul_quest";
pub(super) const NPC: &str = "oteryn:npc.sandomo";
pub(super) const MORTAR: &str = "oteryn:quest-progress/quest/u10_30/roshamuul_quest/rosk_mortar_thrown";
pub(super) const FRAZZLEMAWS: &str =
    "oteryn:quest-progress/quest/u10_30/roshamuul_quest/rosk_killed_frazzlemaws";
pub(super) const SILENCERS: &str = "oteryn:quest-progress/quest/u10_30/roshamuul_quest/rosk_killed_silencers";
pub(super) const GOLD: &str = "oteryn:quest-progress/quest/u10_30/roshamuul_quest/rosk_gold_record";
pub(super) const ITEM: &str = "oteryn:item.tibia.i20062";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sandomo.lua:97-165";

#[test]
fn contribution_books_reset_counts_and_two_quantity_choices_redeem_exact_gold() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    redemption_case(admin, "contribution_redemption", 500, exercise)
}

#[test]
fn contribution_redemption_mints_two_stacks_and_replay_does_not_charge_again() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    redemption_case(admin, "contribution_bulk_redemption", 10_103, exercise_bulk)
}

fn redemption_case(
    admin: String,
    database: &'static str,
    recorded_gold: i64,
    exercise: impl AsyncFnOnce(&Harness, i64) -> TestResult + Send + 'static,
) -> TestResult {
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
                let result = Box::pin(exercise(&h, recorded_gold)).await;
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

fn catalogue(recorded_gold: i64) -> TestResult<QuestStateCatalogue> {
    sandomo_catalogue(recorded_gold, [3, 2, 1])
}

pub(super) fn sandomo_catalogue(
    recorded_gold: i64,
    [mortar, frazzlemaws, silencers]: [i64; 3],
) -> TestResult<QuestStateCatalogue> {
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
        [(MORTAR, mortar), (FRAZZLEMAWS, frazzlemaws), (SILENCERS, silencers), (GOLD, recorded_gold)].map(|(key, initial)| {
            json!({
                "key":key,"quest":QUEST,"initial":initial,"min":-1,"max":i64::MAX,"sources":[SOURCE]
            })
        });
    let source = json!({"schema":1,"quest":QUEST,"tracks":tracks,"rules":rules}).to_string();
    let base = QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(debug)?;
    Ok(NpcRuleCatalog::from_json_documents(&[&source])?.with_fixture_npc_rules(base)?)
}

pub(super) const CLUSTER: FixtureItem = FixtureItem {
    id: 20062,
    name: "cluster of solace",
    weight: 280,
    stackable: true,
};

async fn exercise(h: &Harness, recorded_gold: i64) -> TestResult {
    with_npc_items(h, Arc::new(catalogue(recorded_gold)?), NPC, &[CLUSTER], async |owner, admitted, npc| {
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
        assert_eq!(talk(owner, admitted, npc, 9, "2.0").await?, ["Clusters delivered."]);
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


async fn exercise_bulk(h: &Harness, recorded_gold: i64) -> TestResult {
    with_npc_items(h, Arc::new(catalogue(recorded_gold)?), NPC, &[CLUSTER], async |owner, admitted, npc| {
        let read = || async {
            sqlx::query_as::<_, (i64, i64, i64, i64)>(
                "SELECT
                    COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),$3),
                    (SELECT count(*) FROM game_quest_npc_item_sources WHERE transition_key='oteryn:quest-npc/roshamuul_quest/sandomo/redeem'),
                    (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$4 AND lifecycle=1),
                    (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$4 AND lifecycle=1)"
            ).bind(id(41).as_slice()).bind(GOLD).bind(recorded_gold).bind(ITEM).fetch_one(&h.pool).await
        };
        let mut admitted = *admitted;
        super::qd_world::select_talk_capabilities(owner, &mut admitted, &[3, 6])?;
        let (mut server, peer) = tokio::io::duplex(1 << 20);
        use super::hireling_lifecycle::cook::{Client, drive};
        let (_, ()) = drive(owner, admitted, &mut server, async {
            let mut client = Client::new(peer, admitted);
            client.talk(npc, "hi").await?;
            client.talk(npc, "trade").await?;
            assert_eq!(client.talk(npc, "102").await?.lines, ["Your balance cannot cover that quantity."]);
            assert_eq!(read().await?, (10_103, 0, 0, 0));

            // Crystal npc/sandomo.lua:151-165 charges for all requested clusters
            // together. 101 crosses the current native stack size of 100.
            let command = admitted.continuity.next_command_id + 3;
            let committed = client.talk(npc, "101").await?;
            assert_eq!(committed.lines, ["Clusters delivered."]);
            assert_eq!(read().await?, (3, 1, 2, 101));
            let stack_sizes: Vec<i64> = sqlx::query_scalar(
                "SELECT quantity FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1 ORDER BY quantity"
            ).bind(ITEM).fetch_all(&h.pool).await?;
            assert_eq!(stack_sizes, [1, 100]);
            assert_eq!(client.talk_command(npc, "101", command).await?.lines, committed.lines);
            assert_eq!(read().await?, (3, 1, 2, 101));
            client.close().await
        }).await?;
        Ok(())
    }).await
}
