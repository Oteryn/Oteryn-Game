use super::fixture::Harness;

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};

use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

pub(super) static EXERCISED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    EXERCISED.store(true, std::sync::atomic::Ordering::SeqCst);
    eprintln!("POSTGRES_STARTED NPCFIX_M38 Minzy basic source contact");
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."));
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/npc-placements/index.json"),
    )?)?;
    let mut published = Vec::new();
    for shard in index["shards"].as_array().ok_or("placement shards")? {
        let data: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join(shard.as_str().ok_or("placement path")?),
        )?)?;
        for record in data["records"].as_array().ok_or("placement records")? {
            if record["declaration"]["npc"] == "oteryn:npc.minzy" {
                published.push(record["declaration"]["cell"].clone());
            }
        }
    }
    assert_eq!(
        published,
        [serde_json::json!({"x":32743,"y":31971,"floor":12})]
    );
    let [npc] = super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.minzy"]).await?;
    {
        let runtime = owner.runtime.lock().await;
        let actor = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|n| {
                n.actor.placement_identity() == npc.identity && n.generation == npc.generation
            })
            .ok_or("Minzy actor")?;
        assert_eq!(
            (actor.position.x, actor.position.y, actor.position.floor),
            (32743, 31971, 12)
        );
    }
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        super::classic_quests_fixtures::talk(owner, admitted, npc, 39700, "hi").await?,
        ["Welcome. Good you're showing up"]
    );
    assert_eq!(
        super::classic_quests_fixtures::talk(owner, admitted, npc, 39701, "name").await?,
        ["I am known as Minzy."]
    );
    assert_eq!(
        super::classic_quests_fixtures::talk(owner, admitted, npc, 39701, "name").await?,
        ["I am known as Minzy."]
    );
    let mut stale = npc;
    stale.generation = stale.generation.checked_add(1).ok_or("generation")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let result = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 39702,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: stale,
                text: "name".into(),
            },
        )
        .await;
    assert_ne!(result.disposition, NpcIntentDisposition::Ok);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        before, after,
        "basic dialogue must not charge a travel or exchange fee"
    );
    assert_eq!(owner.runtime.lock().await.visible_entities().npcs.len(), 1);
    Ok(())
}
