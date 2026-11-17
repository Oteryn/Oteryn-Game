//! M01a imported chosen-recipe admission and real NPC command execution.
//! A binding fixture, not an end-to-end qualification of the Yalahar journey.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.in_service_of_yalahar_quest";
const TRACK: &str = "oteryn:quest-progress/m01a/yalahar-talk";

#[test]
fn m01a_imported_recipe_native_talk_dispatch_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-authored-imported-talk")
}

#[test]
fn m01a_multi_npc_talk_stage_is_atomic_and_replay_bound_postgres() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-authored-imported-talk-group")
}

pub(super) async fn exercise_group(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    exercise_inner(owner, admitted, h, true).await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    exercise_inner(owner, admitted, h, false).await
}

async fn exercise_inner(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    group: bool,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M01A_IMPORTED_TALK_POSTGRES {version}");
    let shard: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/definitions/quests-00100-00199.json"
    ))?;
    let definition = shard["records"]
        .as_array()
        .ok_or("Quest records")?
        .iter()
        .map(|row| &row["definition"])
        .find(|row| row["identity"]["key"] == QUEST)
        .ok_or("imported Quest")?;
    let recipe = &definition["oteryn_recipe"]["payload"]["recipe"];
    let digest: String = Sha256::digest(serde_json::to_vec(recipe)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut callback: RewardCallback = serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-reward/m01a-imported-talk", "quest":QUEST,
        "npc":"oteryn:npc.palimuth", "keywords":["importedbinding"],
        "topic":0,"next_topic":0,
        "storage":[{"track":TRACK,"minimum":0,"maximum":0,"set":1}],
        "take":[],"give":null,"outfit":null,"achievement":null,
        "reply":["Imported TALK binding witness."],
        "sources":["M01a engine fixture: canonical imported recipe; no whole-quest claim"]
    }))?;
    let first = format!("{TRACK}/first");
    let second = format!("{TRACK}/second");
    let alternative = if group {
        callback.storage = serde_json::from_value(serde_json::json!([
            {"track":first,"minimum":0,"maximum":0,"set":1}
        ]))?;
        callback.state_ops = serde_json::from_value(serde_json::json!([{
            "track":TRACK,"from":{"kind":"between","minimum":0,"maximum":1},
            "effect":{"kind":"add","value":1}
        }]))?;
        let mut other = callback.clone();
        other.key.push_str("-west");
        other.npc = "oteryn:npc.mr_west".into();
        other.storage[0].track = second.clone();
        Some(other)
    } else {
        None
    };
    let mut document = serde_json::json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,
        "sources":["M01a canonical imported recipe admission fixture"],
        "tracks":[{"key":TRACK,"initial":0,"minimum":0,"maximum":1}],
        "stages":[{"key":"talk","completes":false,
            "trigger":{"kind":"npc_dialogue","callback":callback}}]
    }]});
    if let Some(other) = &alternative {
        document["quests"][0]["tracks"][0]["maximum"] = 2.into();
        let tracks = document["quests"][0]["tracks"]
            .as_array_mut()
            .ok_or("tracks")?;
        for track in [&first, &second] {
            tracks.push(serde_json::json!({"key":track,"initial":0,"minimum":0,"maximum":1}));
        }
        document["quests"][0]["stages"][0]["trigger"]["alternatives"] = serde_json::json!([other]);
    }
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    bindings.verify_embedded_recipe_evidence()?;
    let mut stale = bindings.clone();
    stale.quests[0].recipe_sha256 = "f".repeat(64);
    assert!(stale.verify_embedded_recipe_evidence().is_err());
    let revision = owner
        .quest_catalogue
        .as_ref()
        .ok_or("catalogue")?
        .content_revision()
        .to_owned();
    let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    let actors = if group {
        let [first, second] = super::classic_quests::install(
            owner,
            admitted,
            ["oteryn:npc.palimuth", "oteryn:npc.mr_west"],
        )
        .await?;
        vec![first, second]
    } else {
        let [first] =
            super::classic_quests::install(owner, admitted, ["oteryn:npc.palimuth"]).await?;
        vec![first]
    };
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let before_items: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    let (result, _, _) =
        super::qengine_rewards::wire_reward(owner, admitted, actors[0], &callback, false).await?;
    assert_eq!(result, NpcIntentDisposition::Ok);
    let value: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(QUEST)
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(value, 1);
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1")
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        receipts, 1,
        "literal command replay cannot duplicate TALK progress"
    );
    if let Some(other) = &alternative {
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let (result, _, _) =
            super::qengine_rewards::wire_reward(owner, admitted, actors[1], other, false).await?;
        assert_eq!(result, NpcIntentDisposition::Ok);
        let progress: i64 = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
        )
        .bind(QUEST)
        .bind(TRACK)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(progress, 2, "both exact NPCs contribute to the same stage");
        let flags: i64 = sqlx::query_scalar("SELECT sum(value)::bigint FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=ANY($2::text[])")
            .bind(QUEST).bind(vec![first,second]).fetch_one(&h.pool).await?;
        assert_eq!(flags, 2);
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1",
        )
        .bind(QUEST)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            receipts, 2,
            "literal replay cannot add a third contribution"
        );
    }
    let after_items: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(after_items, before_items);
    let completions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        completions, 0,
        "one TALK stage does not complete the journey"
    );
    Ok(())
}
