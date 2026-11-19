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
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use oteryn_protocol_oteryn::world_spatial::ActorPosition;
use tokio::io::DuplexStream;

const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/mission03";
const CAPTURES: &str = "oteryn:quest-progress/crystalserver/kv/rottinwood-corpsecount";
const QUESTLINE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/questline";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const MERRY_ADVENTURES: &str = "oteryn:achievement/merry_adventures";
const LOOT: &str = "oteryn:quest-callback/rottin-wood/loot-merchant";

const NETS: &str = "oteryn:quest-progress/crystalserver/kv/rottinwood-trap-count";
const EARLY_NET: &str = "oteryn:quest-callback/rottin-wood/place-net";
const FINAL_NET: &str = "oteryn:quest-callback/rottin-wood/final-net";

pub(super) const WAVE_ORIGIN: (u16, u16) = (32650, 32177);
pub(super) const WAVE_START: (u16, u16) = (32659, 32180);
pub(super) const WAVE_NETS: [(u16, u16); 2] = [(32660, 32180), (32660, 32181)];
pub(super) const BLOCKED_SPAWN: (u16, u16) = (32656, 32182);

pub(super) async fn seed_prerequisites(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    wave: bool,
) -> TestResult {
    let tracks = [
        (MISSION.into(), 1),
        (QUESTLINE.into(), 5),
        (CAPTURES.into(), if wave { 0 } else { 4 }),
        (NETS.into(), if wave { 3 } else { 5 }),
        ("oteryn:quest-progress/crystalserver/kv/nest-timer".into(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?.as_secs().try_into()?),
    ];
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(h, owner, admitted, &tracks, 170)
        .await
}

pub(super) async fn accept(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, admitted, &[(QUESTLINE.into(), 4)], 170,
    ).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    for (command, text) in [(1701, "hi"), (1702, "mission"), (1703, "yes"), (1704, "yes")] {
        talk(owner, admitted, npc, command, text).await?;
    }
    let (net_instances, net_quantity, stage, questline): (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
          (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
          (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
          (SELECT value FROM game_character_quest_tracks WHERE track_key=$2),
          (SELECT value FROM game_character_quest_tracks WHERE track_key=$3)",
    )
    .bind("oteryn:item.tibia.i12186")
    .bind(MISSION)
    .bind(QUESTLINE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((net_instances, net_quantity, stage, questline), (5, 5, 1, 5));
    Ok(())
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
        oteryn_gameplay_quests::quest::classic_dialogue::reply(ROTTIN, "line-119").ok_or("shipped Bushwhacking payout reply")?
    );
    assert_eq!(
        super::rottin_rewards_fixtures::experience(h).await?,
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
    let [early, pos] = WAVE_NETS.map(|(x, y)| TilePos { x, y, floor: -7 });
    let entrance = TilePos { x: 32662, y: 32190, floor: -7 };
    let ordinary_ground = TilePos { x: early.x - 1, y: early.y - 1, ..early };
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
    let map = owner.bundle_map().ok_or("map")?;
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
