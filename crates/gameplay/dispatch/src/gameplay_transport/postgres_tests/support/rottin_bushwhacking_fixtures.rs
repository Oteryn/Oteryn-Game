//! Crystal 201189f8 moviment_trap.lua: a real creature push delivers merchant StepIn.
use super::autowalk_fixtures::at;
use super::classic_quests_fixtures::{install, talk};
use super::creature_push_fixtures::{prepare, reference, spawn};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::creature_push::{CreaturePushDisposition, CreaturePushIntent};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use tokio::io::DuplexStream;

const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/mission03";
const CAPTURES: &str = "oteryn:quest-progress/crystalserver/kv/rottinwood-corpsecount";
const QUESTLINE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/questline";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const START: &str = "oteryn:quest-reward/rottin-wood/start-bushwhacking";
const TURN_IN: &str = "oteryn:quest-reward/rottin-wood/bushwhacking";
const MERRY_ADVENTURES: &str = "oteryn:achievement/merry_adventures";
const LOOT: &str = "oteryn:quest-callback/rottin-wood/loot-merchant";

const NETS: &str = "oteryn:quest-progress/crystalserver/kv/rottinwood-trap-count";
const EARLY_NET: &str = "oteryn:quest-callback/rottin-wood/place-net";
const FINAL_NET: &str = "oteryn:quest-callback/rottin-wood/final-net";

pub(super) async fn accept(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    npc: EntityRef,
) -> TestResult {
    use super::rottin_rewards_fixtures::source_reply;

    let speaker = owner
        .states
        .chat
        .speaker_name(admitted.game_session_id)
        .ok_or("Bushwhacking customer name")?;
    let offer: Vec<_> = source_reply("line-111")?
        .iter()
        .map(|line| line.replace("|PLAYERNAME|", &speaker))
        .collect();
    assert_eq!(
        talk(owner, admitted, npc, 1609, "mission").await?,
        offer
    );
    assert_eq!(
        talk(owner, admitted, npc, 1610, "yes").await?,
        source_reply("line-197")?
    );
    assert_eq!(
        talk(owner, admitted, npc, 1611, "yes").await?,
        source_reply("line-205")?
    );
    let state: (i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$2),
         (SELECT COALESCE(sum(item.quantity),0)::bigint FROM game_quest_reward_sources AS reward
          JOIN game_item_instances AS item ON item.minted_transaction_id=reward.mint_transaction_id
          WHERE reward.transition_key=$3 AND item.definition_production_key='oteryn:item.tibia.i12186'
            AND item.lifecycle=1)",
    )
    .bind(MISSION)
    .bind(QUESTLINE)
    .bind(START)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(state, (1, 5, 5));
    Ok(())
}

pub(super) async fn seed(h: &Harness, wave: bool) -> TestResult {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs();
    super::rottin_rewards_fixtures::seed_tracks(h, &[
        (MISSION, 1), (QUESTLINE, 5), (CAPTURES, if wave { 0 } else { 4 }),
        (NETS, if wave { 3 } else { 0 }),
        ("oteryn:quest-progress/crystalserver/kv/nest-timer", i64::try_from(now)?),
    ]).await
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
    let gameplay = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay());
    let trap = crate::map::boot::native_item_definition(gameplay, "oteryn:item.tibia.i12187")
        .ok_or("shipped merchant trap")?;
    let captured = crate::map::boot::native_item_definition(gameplay, "oteryn:item.tibia.i12189")
        .ok_or("shipped captured merchant")?;
    assert_eq!(before[0].facts.definition, MapDefinition::Item(trap.reference));
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
    assert_eq!(after[0].facts.definition, MapDefinition::Item(captured.reference));
    if let Some(h) = loot {
        super::autowalk_fixtures::place(owner, admitted, at(3, 2)).await?;
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
    let before_xp = super::rottin_rewards_fixtures::experience(h).await?;
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
    assert_eq!(
        super::rottin_rewards_fixtures::experience(h).await?,
        before_xp
    );

    exercise(None, owner, admitted, Some(h)).await?;

    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        talk(owner, &admitted, npc, 1703, "mission").await?,
        super::rottin_rewards_fixtures::source_reply("line-119")?
    );
    assert_eq!(
        super::rottin_rewards_fixtures::experience(h).await?,
        before_xp + 1000
    );
    let (stage, questline, platinum, achievement, payouts): (i64, i64, i64, bool, i64) = sqlx::query_as(
        "SELECT
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$3),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances
          WHERE definition_production_key='oteryn:item.tibia.i3035' AND lifecycle=1),
         EXISTS(SELECT 1 FROM game_account_achievements WHERE achievement_key=$2),
         (SELECT count(*) FROM game_quest_reward_sources
          WHERE transition_key=$4 AND mint_transaction_id IS NOT NULL)",
    )
    .bind(MISSION)
    .bind(MERRY_ADVENTURES)
    .bind(QUESTLINE)
    .bind(TURN_IN)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((stage, questline, platinum, achievement, payouts), (2, 6, 6, true, 1));
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
    let gameplay = owner.active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("active creature policies")?;
    gameplay
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(super::fixture::debug)?;
    let trap = crate::map::boot::native_item_definition(Some(gameplay), "oteryn:item.tibia.i12187")
        .ok_or("shipped merchant trap")?;
    let captured =
        crate::map::boot::native_item_definition(Some(gameplay), "oteryn:item.tibia.i12189")
            .ok_or("shipped captured merchant")?;
    let map = owner.bundle_map().ok_or("map")?;
    let early = TilePos {
        x: 32659,
        y: 32182,
        floor: -7,
    };
    let pos = TilePos {
        x: 32660,
        y: 32182,
        floor: -7,
    };
    let entrance = TilePos {
        x: 32662,
        y: 32190,
        floor: -7,
    };
    let ordinary_ground = TilePos {
        x: 32658,
        y: 32181,
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
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.movement_items(ordinary_ground).is_empty());
    assert_eq!(
        use_quest_tool(owner, admitted, early, 0, 1, 70).await?,
        UseDisposition::NothingToUse,
        "the selected backpack cannot stand in for a carried net"
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.movement_items(early).is_empty());
    assert_eq!(
        use_quest_tool(owner, admitted, early, 0, 2, 71).await?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("map")?;
    let early_traps = map.facts.movement_items(early);
    assert_eq!(early_traps.len(), 1);
    assert_eq!(early_traps[0].facts.appearance_id, 12187);
    assert_eq!(early_traps[0].facts.definition, MapDefinition::Item(trap.reference));
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
    let map = owner.bundle_map().ok_or("map")?;
    assert!(
        map.facts
            .movement_items(pos)
            .iter()
            .any(|entry| entry.facts.appearance_id == 12187
                && entry.facts.definition == MapDefinition::Item(trap.reference))
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
    assert_eq!(snapshot.position(), at(32660, 32193));
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
            .any(|entry| entry.facts.appearance_id == 12189
                && entry.facts.definition == MapDefinition::Item(captured.reference))
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
