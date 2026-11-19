use super::fixture::Harness;
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, classic_quests_fixtures as classic_quests,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Offering, Position},
};
use serde_json::json;

pub(super) const NAME: &str = "qcx62-sven-multiplied-compensation";
const QUEST: &str = "oteryn:quest.the_ice_islands_quest";
const BARBARIAN: &str = "oteryn:quest-progress/quest/u8_0/barbarian_test/questline";
const HUSKIES: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/husky_kill";
const STATUS: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/husky_kill_status";
const CALLBACK: &str = "oteryn:quest-reward/qcx62/sven-compensation";
const SOURCE: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sven.lua:117-127";

pub(super) fn catalogue() -> TestResult<QuestStateCatalogue> {
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema":1,
        "quests":[{
            "quest":QUEST,
            "recipe_sha256":"885076bfc128330b5ab9c41599bc6e604e710927b75367f9945d5cd3034bbe95",
            "tracks":[
                {"key":BARBARIAN,"initial":-1,"minimum":-1,"maximum":8},
                {"key":HUSKIES,"initial":0,"minimum":0,"maximum":i32::MAX},
                {"key":STATUS,"initial":0,"minimum":0,"maximum":1}
            ],
            "stages":[{
                "key":"sven-compensation",
                "completes":false,
                "trigger":{"kind":"npc_dialogue","callback":{
                    "key":CALLBACK,"quest":QUEST,"npc":"oteryn:npc.sven",
                    "keywords":["yes"],"topic":0,"next_topic":0,
                    "requires":[{"track":BARBARIAN,"minimum":8,"maximum":8}],
                    "storage":[
                        {"track":STATUS,"minimum":1,"maximum":1,"set":0},
                        {"track":HUSKIES,"minimum":1,"maximum":i32::MAX,"set":0}
                    ],
                    "fee":{"gold":1500,"include_bank":true,"multiplier_track":HUSKIES},
                    "take":[],"give":null,"outfit":null,"achievement":null,
                    "reply":["Alright, we are even!"],
                    "failure_reply":["You don't have enough. Bring me the money and I will forget about it."],
                    "sources":[SOURCE]
                }}
            }],"sources":[SOURCE]
        }]
    }))?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    at: Position,
) -> TestResult {
    let [sven] = classic_quests::install(owner, admitted, ["oteryn:npc.sven"]).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (track, value) in [(BARBARIAN, 8_i64), (HUSKIES, 2), (STATUS, 1)] {
        let inserted = sqlx::query(
            "INSERT INTO game_character_quest_tracks
             (character_id,track_key,quest_key,value,committed_character_revision)
             SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots
             WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(track)
        .bind(QUEST)
        .bind(value)
        .execute(&mut *tx)
        .await?;
        assert_eq!(inserted.rows_affected(), 1);
    }
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            item: "oteryn:item.tibia.i3035".into(),
            count: 20,
            position: at,
        },
        982,
        true,
    )
    .await?;
    classic_quests::talk(owner, admitted, sven, 19400, "hi").await?;
    super::qengine_rewards_fixtures::talk(owner, admitted, sven, 19401, "yes").await?;
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, HUSKIES).await?,
        2
    );
    assert!(payments(h).await?.is_empty());

    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            item: "oteryn:item.tibia.i3035".into(),
            count: 10,
            position: at,
        },
        983,
        true,
    )
    .await?;
    let reply = classic_quests::talk(owner, admitted, sven, 19402, "yes").await?;
    assert!(reply.iter().any(|line| line.contains("we are even")));
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, HUSKIES).await?,
        0
    );
    assert_eq!(
        classic_quests::progress(owner, admitted, authority, STATUS).await?,
        0
    );
    assert_eq!(payments(h).await?, [3000]);
    super::qengine_rewards_fixtures::talk(owner, admitted, sven, 19402, "yes").await?;
    assert_eq!(payments(h).await?, [3000]);
    Ok(())
}

async fn payments(h: &Harness) -> TestResult<Vec<i64>> {
    Ok(sqlx::query_scalar(
        "SELECT fee_gold_units FROM game_quest_reward_fees WHERE transition_key=$1",
    )
    .bind(CALLBACK)
    .fetch_all(&h.pool)
    .await?)
}
