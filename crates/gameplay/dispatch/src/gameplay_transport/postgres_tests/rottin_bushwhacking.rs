//! Crystal 201189f8 moviment_trap.lua: a real creature push delivers merchant StepIn.
use super::autowalk::at;
use super::classic_quests::{install, talk};
use super::creature_push::{prepare, reference, scenario, spawn};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::creature_push::{CreaturePushDisposition, CreaturePushIntent};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::io::DuplexStream;

#[test]
fn rottin_merchant_step_in_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
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
const QUESTLINE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/questline";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const TURN_IN: &str = "oteryn:quest-reward/rottin-wood/bushwhacking";
const MERRY_ADVENTURES: &str = "oteryn:achievement/merry_adventures";
const LOOT: &str = "oteryn:quest-callback/rottin-wood/loot-merchant";

#[test]
fn rottin_merchant_step_loot_and_mission_turn_in_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_merchant_loot", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking USE PostgreSQL thread panicked")?
        .map_err(Into::into)
}

const NETS: &str = "oteryn:quest-progress/crystalserver/kv/rottinwood-trap-count";
const EARLY_NET: &str = "oteryn:quest-callback/rottin-wood/place-net";
const FINAL_NET: &str = "oteryn:quest-callback/rottin-wood/final-net";

#[test]
fn rottin_merchant_step_final_net_starts_scene_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_merchant_wave", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking scene PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_merchant_wave_skips_a_blocked_spawn_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_merchant_wave_blocked", false, Some(12187)))?
        .join()
        .map_err(|_| "Bushwhacking scene PostgreSQL thread panicked")?
        .map_err(Into::into)
}

pub(super) fn catalogue(
    wave: bool,
    blocked_merchant_spawn: bool,
) -> TestResult<QuestStateCatalogue> {
    let quest = "oteryn:quest.rottin_wood_and_the_married_men_quest";
    let source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/rottin_wood_and_married_men/actions_corpse.lua:3-36";
    let mut authored = json!({
        "schema": 1,
        "quests": [{
            "quest": quest,
            "recipe_sha256": Sha256::digest(source.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "tracks": [
                {"key": MISSION, "initial": 1, "minimum": 0, "maximum": 2},
                {"key": CAPTURES, "initial": if wave { 0 } else { 4 }, "minimum": 0, "maximum": 5}
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
    });
    if !wave {
        authored["quests"][0]["tracks"]
            .as_array_mut()
            .ok_or("fixture tracks")?
            .push(json!({"key": QUESTLINE, "initial": 5, "minimum": 0, "maximum": 6}));
        let donor =
            "201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rottin_wood.lua:117-127";
        authored["quests"][0]["stages"]
            .as_array_mut()
            .ok_or("fixture stages")?
            .push(json!({
                "key": "bushwhacking-turn-in", "completes": false,
                "trigger": {"kind": "npc_dialogue", "callback": {
                    "key": TURN_IN, "quest": quest, "npc": ROTTIN,
                    "keywords": ["mission"], "topic": 0, "next_topic": 0,
                    "storage": [
                        {"track": MISSION, "minimum": 1, "maximum": 1, "set": 2},
                        {"track": QUESTLINE, "minimum": 5, "maximum": 5, "set": 6}
                    ],
                    "requires": [{"track": CAPTURES, "minimum": 5, "maximum": 5}],
                    "take": [], "give": {"item": "oteryn:item.tibia.i3035", "count": 5},
                    "outfit": null, "achievement": MERRY_ADVENTURES, "experience": 1000,
                    "reply": ["Thanks for helping us."], "sources": [donor]
                }}
            }));
    }
    if wave {
        let donor = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/rottin_wood_and_married_men/actions_trap.lua:65-123";
        let track = json!({"key":NETS,"initial":3,"minimum":0,"maximum":5});
        authored["quests"][0]["tracks"]
            .as_array_mut()
            .ok_or("fixture tracks")?
            .push(track);
        let at = |x, y| json!({"x":x,"y":y,"floor":-7});
        let first_spawn = if blocked_merchant_spawn {
            (5, 2)
        } else {
            (2, 4)
        };
        let monsters: Vec<_> = [first_spawn, (3, 4), (4, 4), (5, 4), (3, 5), (4, 5), (5, 5)]
            .into_iter().map(|(x,y)| json!({"position":at(x,y),"creature":"oteryn:creature.travelling_merchant"})).collect();
        let stages = authored["quests"][0]["stages"]
            .as_array_mut()
            .ok_or("fixture stages")?;
        stages.push(json!({
            "key":"place-net", "completes":false,
            "trigger":{"kind":"map_use","callback":{
                "key": EARLY_NET, "quest": quest, "trigger": "use",
                "selector": {"appearance":10480},
                "reversible": false, "requires_world_admission": true,
                "passages": [], "offerings": [], "return_on_failure": false,
                "item_on_target": {"item":"oteryn:item.tibia.i12186","count":1,"selected_instance":true},
                "storage": [
                    {"track":MISSION,"minimum":1,"maximum":1,"set":1,"preserve":true},
                    {"track":NETS,"minimum":0,"maximum":3,"set":4,"add":1}
                ],
                "activation":{"track":NETS,"minimum":0,"maximum":3},
                "effects":{"place_item":12187},
                "sources": [donor]
            }}
        }));
        stages.push(json!({
            "key":"final-net", "completes":false,
            "trigger":{"kind":"map_use","callback":{
                "key": FINAL_NET, "quest": quest, "trigger": "use",
                "selector": {"position":at(3,2),"appearance":10480},
                "reversible": false, "requires_world_admission": true,
                "passages": [], "offerings": [], "return_on_failure": false,
                "teleport": at(2,3),
                "item_on_target": {"item":"oteryn:item.tibia.i12186","count":1,"selected_instance":true},
                "storage": [
                    {"track":MISSION,"minimum":1,"maximum":1,"set":1,"preserve":true},
                    {"track":NETS,"minimum":4,"maximum":4,"set":5,"add":1}
                ],
                "activation":{"track":NETS,"minimum":4,"maximum":4},
                "effects": {
                    "place_item":12187, "merchant_entrance":at(4,2),
                    "encounter":{
                        "minimum_level":0,"players":[],"monsters":monsters,
                        "room_min":at(2,2),"room_max":at(5,5),
                        "timing":{"spawn_after_ms":1,"reset_after_ms":300000}
                    }
                },
                "sources": [donor]
            }}
        }));
    }
    let bindings = Bindings::parse(&serde_json::to_vec(&authored)?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("empty catalogue")?)?)
}

pub(super) async fn exercise(
    client: Option<&mut DuplexStream>,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    loot: Option<&Harness>,
) -> TestResult {
    if let Some(client) = client {
        prepare(client, owner).await?;
    } else {
        let mut runtime = owner.runtime.lock().await;
        owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("active creature policies")?
            .install_companion_policies(&mut runtime)
            .map_err(super::fixture::debug)?;
    }
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
        super::autowalk::place(owner, admitted, at(3, 2)).await?;
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
            .use_craft(
                actor,
                command,
                UseIntent {
                    target: UseTarget::Item(std::num::NonZeroU64::MIN),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: None,
                },
                Some(ItemKey::MapAdded(id.get())),
                None,
            )
            .await
            .ok_or("captured merchant USE binding")?;
        assert_eq!(outcome.disposition, UseDisposition::Committed);
        assert!(map.facts.movement_items(tile).is_empty());
        let progress: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(CAPTURES)
                .fetch_one(&h.pool)
                .await?;
        assert_eq!(progress, 5);
        let payouts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1 AND mint_transaction_id IS NOT NULL"
        ).bind(LOOT).fetch_one(&h.pool).await?;
        assert_eq!(payouts, 1);
    }
    Ok(())
}

pub(super) async fn exercise_mission(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [npc] = install(owner, &admitted, [ROTTIN]).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    talk(owner, &admitted, npc, 1701, "hi").await?;
    let before_xp = super::rottin_rewards::experience(h).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("dialogue clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "quest fence")?
        .ok_or("quest fence")?;
    let refused = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("quest actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 1702,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "mission".into(),
            },
        )
        .await;
    assert_eq!(refused.disposition, NpcIntentDisposition::Rejected);
    assert_eq!(super::rottin_rewards::experience(h).await?, before_xp);

    exercise(None, owner, admitted, Some(h)).await?;

    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        talk(owner, &admitted, npc, 1703, "mission").await?,
        ["Thanks for helping us."]
    );
    assert_eq!(
        super::rottin_rewards::experience(h).await?,
        before_xp + 1000
    );
    let (stage, questline, platinum, achievement): (i64, i64, i64, bool) = sqlx::query_as(
        "SELECT
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$3),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances
          WHERE definition_production_key='oteryn:item.tibia.i3035' AND lifecycle=1),
         EXISTS(SELECT 1 FROM game_account_achievements WHERE achievement_key=$2)",
    )
    .bind(MISSION)
    .bind(MERRY_ADVENTURES)
    .bind(QUESTLINE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((stage, questline, platinum, achievement), (2, 6, 6, true));
    Ok(())
}

pub(super) async fn use_quest_tool(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    pos: TilePos,
    ordinal: u8,
    command_id: u64,
    source: u8,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_millis(250))
        .ok_or("world action clock")?;
    let actor = admitted.runtime_actor.ok_or("tool user")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = crate::map::view::placement_key(pos, ordinal).ok_or("tool target site")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let intent = UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    let outcome = owner
        .tame_mount(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            intent,
            Some(ItemKey::Instance(id(source))),
            None,
            false,
        )
        .await
        .map_err(|()| "quest tool USE route")?;
    Ok(outcome.map_or(UseDisposition::NothingToUse, |result| result.disposition))
}

pub(super) async fn exercise_wave(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
    expected_merchants: usize,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("tool user")?;
    let map = owner.bundle_map().ok_or("map")?;
    let early = TilePos {
        x: 3,
        y: 3,
        floor: -7,
    };
    let pos = TilePos {
        x: 3,
        y: 2,
        floor: -7,
    };
    let entrance = TilePos {
        x: 4,
        y: 2,
        floor: -7,
    };
    let ordinary_ground = TilePos {
        x: 2,
        y: 3,
        floor: -7,
    };
    assert!(
        !map.facts
            .quest_callback_blocked(map.overlay.base(), entrance, false)
    );
    assert_eq!(
        use_quest_tool(owner, admitted, ordinary_ground, 0, 69, 71).await?,
        UseDisposition::NothingToUse,
        "the selected net requires the authored trap ground"
    );
    assert!(map.facts.movement_items(ordinary_ground).is_empty());
    assert_eq!(
        use_quest_tool(owner, admitted, early, 0, 1, 70).await?,
        UseDisposition::NothingToUse,
        "the selected backpack cannot stand in for a carried net"
    );
    assert!(map.facts.movement_items(early).is_empty());
    assert_eq!(
        use_quest_tool(owner, admitted, early, 0, 2, 71).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        map.facts.movement_items(early)[0].facts.appearance_id,
        12187
    );
    assert!(
        owner
            .states
            .quest_callbacks
            .next_encounter_due_us()
            .is_none()
    );
    assert_eq!(
        use_quest_tool(owner, admitted, pos, 0, 3, 72).await?,
        UseDisposition::Committed
    );
    assert!(
        map.facts
            .movement_items(pos)
            .iter()
            .any(|entry| entry.facts.appearance_id == 12187)
    );
    assert!(
        map.facts
            .quest_callback_blocked(map.overlay.base(), entrance, false)
    );
    assert!(
        owner
            .states
            .quest_callbacks
            .next_encounter_due_us()
            .is_some()
    );
    let snapshot = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(super::fixture::debug)?;
    assert_eq!(snapshot.position(), at(2, 3));
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key IN ($1, $2)",
    )
    .bind(EARLY_NET)
    .bind(FINAL_NET)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(receipts, 2);
    let placed: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(NETS)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(placed, 5);
    let nets_left: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i12186' AND lifecycle=1",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(nets_left, 0);
    drain_next(owner).await?;
    let runtime = owner.runtime.lock().await;
    let live = runtime
        .current_live_creature_census()
        .map_err(super::fixture::debug)?;
    assert_eq!(
        live.iter()
            .filter(|actor| runtime
                .matches_live_creature_identity(**actor, b"oteryn:creature.travelling_merchant"))
            .count(),
        expected_merchants
    );
    drop(runtime);
    drain_next(owner).await?;
    let runtime = owner.runtime.lock().await;
    let live = runtime
        .current_live_creature_census()
        .map_err(super::fixture::debug)?;
    assert!(!live.iter().any(|actor| {
        runtime.matches_live_creature_identity(*actor, b"oteryn:creature.travelling_merchant")
    }));
    drop(runtime);
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.movement_items(entrance).is_empty());
    assert!(
        !map.facts
            .quest_callback_blocked(map.overlay.base(), entrance, false)
    );
    assert!(
        map.facts
            .movement_items(pos)
            .iter()
            .any(|entry| entry.facts.appearance_id == 12189)
    );
    drain_next(owner).await?;
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.movement_items(pos).is_empty());
    assert!(map.facts.movement_items(early).is_empty());
    Ok(())
}

async fn drain_next(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    let due = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("merchant scene deadline")?;
    owner.clock_origin = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_micros(due))
        .ok_or("scene clock")?;
    owner.drain_quest_encounter_deadlines().await;
    Ok(())
}
