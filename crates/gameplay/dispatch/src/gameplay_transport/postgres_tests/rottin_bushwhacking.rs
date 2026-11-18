//! Crystal 201189f8 moviment_trap.lua: a real creature push delivers merchant StepIn.
use super::autowalk::at;
use super::creature_push::{prepare, reference, scenario, spawn};
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::creature_push::{CreaturePushDisposition, CreaturePushIntent};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::io::DuplexStream;

#[test]
fn rottin_merchant_step_in_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_bushwhacking", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking PostgreSQL thread panicked")?
        .map_err(Into::into)
}

const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/mission03";
const CAPTURES: &str = "oteryn:quest-progress/crystalserver/kv/rottinwood-corpsecount";
const LOOT: &str = "oteryn:quest-callback/rottin-wood/loot-merchant";

#[test]
fn rottin_merchant_step_then_loot_use_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_merchant_loot", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking USE PostgreSQL thread panicked")?
        .map_err(Into::into)
}

pub(super) fn loot_catalogue() -> TestResult<QuestStateCatalogue> {
    let quest = "oteryn:quest.rottin_wood_and_the_married_men_quest";
    let source = "201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/rottin_wood_and_married_men/actions_corpse.lua:3-36";
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": quest,
            "recipe_sha256": format!("{:x}", Sha256::digest(source.as_bytes())),
            "tracks": [
                {"key": MISSION, "initial": 1, "minimum": 0, "maximum": 2},
                {"key": CAPTURES, "initial": 0, "minimum": 0, "maximum": 5}
            ],
            "sources": [source],
            "stages":[{
                "key":"loot-merchant", "completes":false,
                "trigger":{"kind":"map_use","callback":{
                    "key": LOOT, "quest": quest, "trigger": "use",
                    "selector": {"appearance": 12189},
                    "reversible": false, "requires_world_admission": true,
                    "passages": [], "offerings": [], "return_on_failure": false,
                    "storage": [
                        {"track": MISSION, "minimum":1, "maximum":1, "set":1, "preserve":true},
                        {"track": CAPTURES, "minimum":0, "maximum":4, "set":5, "add":1}
                    ],
                    "activation": {"track": CAPTURES, "minimum":0, "maximum":4},
                    "effects": {"loot_merchant": true, "item_reward":{"item":"oteryn:item.tibia.i3035","count":1}},
                    "sources": [source]
                }}
            }]
        }]
    }))?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("empty catalogue")?)?)
}

pub(super) async fn exercise(
    client: &mut DuplexStream,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    loot: Option<&Harness>,
) -> TestResult {
    prepare(client, owner).await?;
    let actor = admitted.runtime_actor.ok_or("pusher")?;
    let merchant = spawn(owner, "oteryn:creature.travelling_merchant", at(3, 2)).await?;
    let map = owner.bundle_map().ok_or("bundle")?;
    let tile = TilePos {
        x: 4,
        y: 2,
        floor: -7,
    };
    assert!(map.facts.create_movement_item(tile, 12187, 0, || true));
    let before = map.facts.movement_items(tile);
    assert_eq!(before.len(), 1);
    let intent = CreaturePushIntent {
        target: reference(owner, merchant).await?,
        destination: ActorPosition {
            x: 4,
            y: 2,
            floor: 7,
        },
    };
    assert_eq!(
        owner
            .push_creature_now(actor, admitted.game_session_id, intent)
            .await,
        CreaturePushDisposition::Moved,
    );
    assert!(!owner.runtime.lock().await.contains_live_creature(merchant));
    let after = map.facts.movement_items(tile);
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].source, before[0].source);
    assert_eq!(after[0].facts.appearance_id, 12189);
    if let Some(h) = loot {
        let crate::map::view::EntrySource::Added(id) = after[0].source else {
            return Err("captured merchant exact handle".into());
        };
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|_| "quest fence")?
            .ok_or("quest fence")?;
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 1,
            item_fence: Some(fence.item_fence()),
        };
        let outcome = owner
            .use_shared_callback_handle(actor, command, Some(ItemKey::MapAdded(id.get())), None)
            .await
            .ok_or("captured merchant USE binding")?;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        assert!(map.facts.movement_items(tile).is_empty());
        let progress: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(CAPTURES)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(progress, 1);
        let payouts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1 AND mint_transaction_id IS NOT NULL"
        ).bind(LOOT).fetch_one(&h.pool).await?;
        assert_eq!(payouts, 1);
    }
    Ok(())
}
