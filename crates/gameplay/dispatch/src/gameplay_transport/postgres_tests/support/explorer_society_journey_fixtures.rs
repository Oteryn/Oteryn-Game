//! The selected Explorer Society course through the native quest and custody owners.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks_fixtures::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::num::NonZeroU64;
use std::path::Path;
use std::sync::Arc;

pub(super) const NAME: &str = "authored-explorer-society";
pub(super) const START: Position = position(32671, 32729, -7);
const QUEST: &str = "oteryn:quest.the_explorer_society_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u7_6/explorer_society/";
const ANGUS: Position = position(32670, 32730, -7);
const UZGOD: Position = position(32664, 31894, -9);
// The ferry returns one cell south of Captain Max's source position.
const MAX: Position = position(32298, 32895, -6);
const CALASSA: Position = position(31911, 32710, -6);
const BROOCH: Position = position(32598, 31934, -15);
const ICE: Position = position(32133, 31688, -9);
const SECOND_ICE: Position = position(34040, 31756, -10);
const BUTTERFLY: Position = position(32580, 32731, -7);
const BELLS: Position = position(32548, 32836, -7);
const URN: Position = position(32957, 32834, -8);
const POETRY: Position = position(32704, 31605, -14);
const RACE: &str = "oteryn:creature.purple_butterfly";
const ITEMS: [u16; 10] = [4834, 4837, 4844, 4845, 4847, 4863, 4864, 4867, 4868, 4872];

const fn position(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let patches = [
        ANGUS, UZGOD, MAX, CALASSA, BROOCH, ICE, SECOND_ICE, BUTTERFLY, BELLS, URN, POETRY,
    ]
    .map(|p| (p.x - 3, p.y - 3, p.x + 3, p.y + 3, p.floor));
    super::questpc8_native_slice::world(
        native,
        &patches,
        &[4992, 4378],
        TilePos {
            x: START.x,
            y: START.y,
            floor: START.floor,
        },
    )
}

fn placements(content: &Path) -> TestResult<NpcPlacementTable> {
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        content.join("world/npc-placements/index.json"),
    )?)?;
    let expected = [
        ("oteryn:npc.angus", ANGUS),
        ("oteryn:npc.uzgod", UZGOD),
        ("oteryn:npc.captain_max", MAX),
    ];
    let mut npcs = Vec::new();
    for path in index["shards"].as_array().ok_or("NPC shards")? {
        let shard = path
            .as_str()
            .ok_or("NPC shard path")?
            .strip_prefix("content/")
            .ok_or("content NPC shard")?;
        let data: serde_json::Value = serde_json::from_slice(&std::fs::read(content.join(shard))?)?;
        for row in data["records"].as_array().ok_or("NPC records")? {
            let declaration = &row["declaration"];
            let Some(&(key, at)) = expected.iter().find(|(key, _)| declaration["npc"] == *key)
            else {
                continue;
            };
            let cell = &declaration["cell"];
            if cell["x"] == at.x && cell["y"] == at.y && cell["floor"] == -at.floor {
                assert_eq!(declaration["direction"], "north");
                npcs.push(NpcPlacements {
                    key: key.into(),
                    placements: vec![NpcPlacement {
                        x: at.x,
                        y: at.y,
                        floor: at.floor,
                        direction: MovementFacing::North,
                    }],
                });
            }
        }
    }
    assert_eq!(
        npcs.len(),
        expected.len(),
        "the field course needs real NPC placements"
    );
    Ok(NpcPlacementTable { npcs })
}

async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: &AdmittedSession,
    at: Position,
) -> TestResult {
    // Between distant map crops, arrange the approach only. All local movement,
    // interactions and persistent quest changes use their gameplay callers.
    let _lane = owner.spell_lane_permit().await.ok_or("movement lane")?;
    let map = owner.bundle_map().ok_or("bundle")?;
    let mut runtime = owner.runtime.lock().await;
    let actor = session.runtime_actor.ok_or("player")?;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        session.game_session_id,
        before,
        local(at),
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn npc(owner: &ComposedFreshAdmission<'_, '_, '_>, key: &str) -> TestResult<EntityRef> {
    let runtime = owner.runtime.lock().await;
    let npc = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|npc| runtime.npc_key(npc.actor) == Some(key))
        .ok_or("live source NPC")?;
    Ok(EntityRef {
        identity: npc.actor.placement_identity(),
        generation: npc.generation,
    })
}

fn talk_payload(npc_actor: EntityRef, text: &str) -> TestResult<Vec<u8>> {
    encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor,
        text: text.into(),
    })
    .map_err(debug)
}

fn allow_next_action(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    Ok(())
}

async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    npc: EntityRef,
    words: &str,
) -> TestResult<NpcIntentDisposition> {
    conversation(owner, session, npc, &["hi", words]).await
}

async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    npc: EntityRef,
    words: &[&str],
) -> TestResult<NpcIntentDisposition> {
    allow_next_action(owner)?;
    let first = session.continuity.next_command_id;
    let commands = words
        .iter()
        .enumerate()
        .map(|(offset, text)| Ok((first + u64::try_from(offset)?, 7, talk_payload(npc, text)?)))
        .collect::<TestResult<_>>()?;
    let mut replies = super::questl2_fixture::wire_sequence_observed(
        owner,
        session,
        commands,
        async |index, reply: &[u8]| {
            if index + 1 < words.len() {
                assert_eq!(
                    decode_npc_intent_result(reply).map_err(debug)?,
                    NpcIntentDisposition::Ok,
                    "Explorer dialogue {:?}",
                    words[index]
                );
            }
            Ok(())
        },
    )
    .await?;
    decode_npc_intent_result(&replies.pop().ok_or("Explorer dialogue result")?).map_err(debug)
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    dir: StepDirection,
    at: Position,
) -> TestResult {
    allow_next_action(owner)?;
    wire(
        owner,
        session,
        session.continuity.next_command_id,
        1,
        encode_step_intent(dir),
    )
    .await?;
    assert_position(owner, session, at).await
}

async fn assert_position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: &AdmittedSession,
    at: Position,
) -> TestResult {
    let actor = session.runtime_actor.ok_or("player")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(at)
    );
    Ok(())
}

fn map_object(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    at: Position,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("bundle")?;
    let key = crate::map::view::placement_key(
        TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        },
        if at == SECOND_ICE { 3 } else { 1 },
    )
    .ok_or("source object ordinal")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn map_use(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    at: Position,
) -> TestResult<UseDisposition> {
    allow_next_action(owner)?;
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&map_object(owner, at)?)
        .map_err(debug)?;
    let result = wire(
        owner,
        session,
        session.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    decode_use_result(&result).map_err(debug)
}

async fn tool_use(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    item: u16,
    map_item: Option<Position>,
) -> TestResult {
    allow_next_action(owner)?;
    let index = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .ok_or("native Item index")?;
    let source = index
        .definition_ref(
            "Item",
            &format!("oteryn:item.tibia.i{item}"),
            "definition-r1",
        )
        .ok_or("source tool")?;
    let handle = NonZeroU64::MIN;
    let (target, corpse) = if let Some(at) = map_item {
        (MapUseTarget::Object(map_object(owner, at)?), None)
    } else {
        (
            MapUseTarget::Item(handle),
            Some(
                index
                    .definition_ref("Item", "oteryn:item.tibia.i4992", "definition-r1")
                    .ok_or("butterfly corpse")?,
            ),
        )
    };
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(handle),
        use_with: None,
        use_with_map_item: Some(target),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, result, _) = super::rust_lottery_fixtures::exchange(
        owner,
        *session,
        session.continuity.next_command_id,
        &payload,
        Some(source),
        corpse,
    )
    .await?;
    *session = next;
    assert_eq!(result, UseDisposition::Committed, "selected tool {item}");
    Ok(())
}

async fn move_ice(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: &AdmittedSession,
    h: &Harness,
    on_ground: bool,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{ItemMoveStep, item_move};
    use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView};
    use oteryn_protocol_oteryn::item_view::{
        ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection, WorldTilePosition,
        encode_item_move_intent_for,
    };
    let actor = session.runtime_actor.ok_or("player")?;
    let item: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i4837'"
    ).fetch_one(&h.pool).await?;
    let key = ItemKey::Instance(item.try_into().map_err(|_| "icicle identity")?);
    let mut view = SessionItemView::resume(ItemViewContinuity::default())
        .with_map_view()
        .with_equip_drop();
    let snapshot = owner
        .observe_character_inventory(actor, session.game_session_id)
        .await
        .ok_or("field course inventory")?;
    view.snapshot(snapshot, None).map_err(debug)?;
    let (handle, destination) = if on_ground {
        let mut map = crate::gameplay_transport::world_map::SessionMapView::default();
        owner
            .observe_world_map(actor, &mut map, &mut view)
            .await
            .ok_or("field course map")?
            .map_err(debug)?;
        (
            view.table().handle(&key).ok_or("dropped icicle")?,
            ItemMoveDestination::MainBackpack,
        )
    } else {
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        (
            view.table().handle(&key).ok_or("carried icicle")?,
            ItemMoveDestination::Ground(WorldTilePosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            }),
        )
    };
    let selection = ItemMoveSelection {
        equip_drop: true,
        world_interaction: true,
        ..Default::default()
    };
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: handle,
            destination,
        },
        selection,
    )
    .map_err(debug)?;
    let fence = owner
        .current_quest_fence(session.game_session_id)
        .await
        .map_err(|()| "item fence")?
        .ok_or("item fence")?;
    let command = UseCommand {
        game_session_id: session.game_session_id,
        command_id: if on_ground { 90004 } else { 90003 },
        item_fence: Some(fence.item_fence()),
    };
    assert!(matches!(
        item_move(owner, actor, command, &payload, selection, false, &mut view).await,
        ItemMoveStep::Result(ItemMoveOutcome::Moved, _)
    ));
    Ok(())
}

async fn state(
    h: &Harness,
    phase: i64,
    mission: Option<(&str, i64)>,
    carried: &[u16],
) -> TestResult {
    let tracks: Vec<(String, i64)> = sqlx::query_as(
        "SELECT track_key, value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2"
    ).bind(id(41).as_slice()).bind(QUEST).fetch_all(&h.pool).await?;
    let value = |name| {
        tracks
            .iter()
            .find(|(key, _)| key == &format!("{TRACK}{name}"))
            .map_or(0, |(_, v)| *v)
    };
    assert_eq!(value("field_course"), phase);
    if let Some((name, expected)) = mission {
        assert_eq!(value(name), expected, "{name} at course phase {phase}");
    }
    assert_eq!(value("calassa_quest"), i64::from(phase == 20));
    let items: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,sum(i.quantity)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 GROUP BY 1"
    ).bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_all(&h.pool).await?;
    for item in ITEMS.into_iter().chain([9605]) {
        let key = format!("oteryn:item.tibia.i{item}");
        let count = items.iter().find(|(k, _)| *k == key).map_or(0, |(_, n)| *n);
        assert_eq!(
            count,
            i64::from(carried.contains(&item)),
            "{key} at course phase {phase}"
        );
    }
    Ok(())
}

async fn door(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    at: Position,
) -> TestResult {
    owner
        .refresh_quest_session(session.game_session_id, false)
        .await;
    assert_eq!(
        map_use(owner, session, at).await?,
        UseDisposition::Committed
    );
    // The source quest door moves onto the doorway as part of USE.
    assert_position(owner, session, at).await
}

async fn assert_journal(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::quest_log::{QuestLogLines, QuestLogObservation};
    use oteryn_protocol_oteryn::quest_log::{QuestLogQuery, encode_quest_log_query};
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let loaded = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let index = u32::try_from(
        loaded
            .content
            .active()
            .quests()
            .iter()
            .position(|q| q.key == QUEST)
            .ok_or("Explorer journal index")?,
    )?;
    owner.quest_log_content = Some(QuestLogLines::new(loaded.content).ok_or("journal limits")?);
    owner
        .refresh_quest_session(session.game_session_id, false)
        .await;
    let payload =
        encode_quest_log_query(&QuestLogQuery::Quest { quest_index: index }).map_err(debug)?;
    wire(
        owner,
        session,
        session.continuity.next_command_id,
        22,
        payload,
    )
    .await?;
    let QuestLogObservation::Changed(journal) = owner
        .quest_log_observation(
            session.runtime_actor.ok_or("player")?,
            session.game_session_id,
            None,
        )
        .await
    else {
        return Err("Explorer journal unavailable".into());
    };
    assert!(journal.copy.quest_state(QUEST).is_some_and(|q| q.completed));
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    session: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let content = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content"));
    let model = crate::content::NpcDataCatalogue::load(
        content,
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?
    .service_model_for_project()
    .map_err(debug)?;
    let native_npcs = oteryn_gameplay_quests::quest::authored_bindings::native_dialogue_npcs()?;
    owner.states.chat.install_npc_talk(Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model_with_native_quests(
            &model,
            &native_npcs.iter().map(String::as_str).collect::<Vec<_>>(),
        ),
    ));
    owner
        .states
        .chat
        .admit(session.game_session_id, "Explorer", owner.owner_now());
    owner.states.chat.set_npc_service(session.game_session_id);
    owner
        .runtime
        .lock()
        .await
        .place_npcs(&placements(content)?, |_| true, true)
        .map_err(debug)?;
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    session.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            &offered,
            &[3, 4, 6, 11, 12, 13, 15, 16, 17, 18],
        )
        .ok_or("Explorer route capabilities")?;

    let angus_side = position(32670, 32729, -7);
    let uzgod_side = position(32664, 31893, -9);
    let max_side = position(32297, 32895, -6);
    let angus = npc(owner, "oteryn:npc.angus").await?;
    let uzgod = npc(owner, "oteryn:npc.uzgod").await?;
    state(h, 0, None, &[]).await?;
    step(owner, session, StepDirection::West, angus_side).await?;
    let premium = owner
        .premium_coordinator
        .take()
        .ok_or("Premium coordinator")?;
    let result = talk(owner, session, angus, "join").await;
    owner.premium_coordinator = Some(premium);
    assert_eq!(result?, NpcIntentDisposition::Rejected);
    state(h, 0, None, &[]).await?;

    let joining = "joining_the_explorers";
    let ice = "the_ice_delivery";
    let butterfly = "the_butterfly_hunt";
    let plant = "the_plant_collection";
    let urn = "the_lizard_urn";
    let poetry = "the_elven_poetry";

    assert_eq!(
        talk(owner, session, angus, "join").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 1, Some((joining, 1)), &[]).await?;
    relocate(owner, session, uzgod_side).await?;
    assert_eq!(
        talk(owner, session, uzgod, "pickaxe").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 2, Some((joining, 2)), &[]).await?;
    relocate(owner, session, position(32598, 31932, -15)).await?;
    door(owner, session, position(32598, 31933, -15)).await?;
    assert_eq!(
        map_use(owner, session, BROOCH).await?,
        UseDisposition::Committed
    );
    state(h, 3, Some((joining, 3)), &[4834]).await?;
    relocate(owner, session, uzgod_side).await?;
    assert_eq!(
        talk(owner, session, uzgod, "brooch").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 4, Some((joining, 4)), &[4845]).await?;
    relocate(owner, session, angus_side).await?;
    assert_eq!(
        talk(owner, session, angus, "pickaxe").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 5, Some((joining, 5)), &[]).await?;
    assert_eq!(
        talk(owner, session, angus, "ice delivery").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 6, Some((ice, 6)), &[4872]).await?;
    // The icicle and the western and southern cells block movement. The donor
    // ice border directly north is walkable and in tool range.
    relocate(owner, session, position(ICE.x, ICE.y - 1, ICE.floor)).await?;
    tool_use(owner, session, 4872, Some(ICE)).await?;
    state(h, 7, Some((ice, 7)), &[4837, 4872]).await?;
    move_ice(owner, session, h, false).await?;
    state(h, 7, Some((ice, 7)), &[4872]).await?;
    move_ice(owner, session, h, true).await?;
    state(h, 7, Some((ice, 7)), &[4837, 4872]).await?;
    let timed: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states t JOIN game_item_instances i USING (item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i4837'").fetch_one(&h.pool).await?;
    assert_eq!(timed, 0, "selected course icicle must have no decay clock");
    let lifetime: (i64, i64) = sqlx::query_as(
        "SELECT p.duration_millis, p.expires_at_unix_ms-r.occurred_at
         FROM game_spell_item_temporal_schedules p
         JOIN game_reward_claim_mint_receipts r ON r.transaction_id=p.source_transaction_id
         WHERE p.definition_production_key='oteryn:item.tibia.i4837'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(lifetime, (600_000, 600_000));
    let original_deadline: i64 = sqlx::query_scalar(
        "SELECT expires_at_unix_ms FROM game_spell_item_temporal_schedules
         WHERE definition_production_key='oteryn:item.tibia.i4837'",
    ).fetch_one(&h.pool).await?;
    let mut predecessor = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *predecessor).await?;
    let removed = sqlx::query("DELETE FROM game_spell_item_temporal_schedules
                                WHERE definition_production_key='oteryn:item.tibia.i4837'")
        .execute(&mut *predecessor).await?;
    assert_eq!(removed.rows_affected(), 1, "the predecessor only retained the original mint");
    predecessor.commit().await?;
    let migration = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../../apps/game-server/migrations/3131_explorer_ice_sample_deadline.sql"));
    let backfill = migration.split_once("-- Carry forward live samples").ok_or("legacy backfill")?.1
        .split_once("-- Retire due icicles").ok_or("backfill boundary")?.0;
    let start = backfill.find("INSERT INTO game_spell_item_temporal_schedules")
        .ok_or("backfill statement")?;
    assert_eq!(sqlx::raw_sql(&backfill[start..])
        .execute(&h.pool).await?.rows_affected(), 1);
    let restored: i64 = sqlx::query_scalar(
        "SELECT expires_at_unix_ms FROM game_spell_item_temporal_schedules
         WHERE definition_production_key='oteryn:item.tibia.i4837'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(restored, original_deadline, "the upgrade keeps the original mint clock");

    // Advance the test sample beyond its absolute deadline, keeping it carried.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let due = sqlx::query(
        "UPDATE game_spell_item_temporal_schedules
         SET expires_at_unix_ms=floor(extract(epoch FROM statement_timestamp())*1000)::bigint-1
         WHERE definition_production_key='oteryn:item.tibia.i4837'",
    )
    .execute(&mut *tx)
    .await?;
    assert_eq!(due.rows_affected(), 1);
    tx.commit().await?;
    relocate(owner, session, angus_side).await?;
    assert_eq!(
        talk(owner, session, angus, "ice delivery").await?,
        NpcIntentDisposition::Rejected
    );
    owner.states.spell_states.lock().await.next_item_deadline_pass_us = 0;
    owner.drain_source_item_deadlines().await;
    state(h, 7, Some((ice, 7)), &[4872]).await?;
    assert_eq!(
        talk(owner, session, angus, "ice sample melted").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 6, Some((ice, 6)), &[4872]).await?;
    relocate(owner, session, position(SECOND_ICE.x - 1, SECOND_ICE.y, SECOND_ICE.floor)).await?;
    tool_use(owner, session, 4872, Some(SECOND_ICE)).await?;
    state(h, 7, Some((ice, 7)), &[4837, 4872]).await?;
    relocate(owner, session, angus_side).await?;
    assert_eq!(
        talk(owner, session, angus, "ice delivery").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 8, Some((ice, 8)), &[4872]).await?;
    assert_eq!(
        talk(owner, session, angus, "butterfly hunt").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 9, Some((butterfly, 9)), &[4863, 4872]).await?;
    relocate(
        owner,
        session,
        position(BUTTERFLY.x - 1, BUTTERFLY.y, BUTTERFLY.floor),
    )
    .await?;
    super::questpc7_native_deaths_fixtures::prepare_native_hunter(owner, session).await?;
    super::questpc7_goblin_fixtures::stalker_death(owner, session, RACE, BUTTERFLY, 90001).await?;
    state(h, 9, Some((butterfly, 9)), &[4863, 4872]).await?;
    tool_use(owner, session, 4863, None).await?;
    state(h, 10, Some((butterfly, 10)), &[4864, 4872]).await?;
    let samples: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_quest_corpse_samples WHERE appearance=4992")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(samples, 1);
    let consumed_corpse: (i16, i64, bool) = sqlx::query_as(
        "SELECT i.lifecycle,i.quantity::bigint,
                EXISTS (SELECT 1 FROM game_item_ground_locations g WHERE g.item_instance_id=i.item_instance_id)
         FROM game_quest_corpse_samples s JOIN game_item_instances i ON i.item_instance_id=s.corpse_item
         WHERE s.transition_key='oteryn:quest-reward/explorer-society/prepare-butterfly'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(consumed_corpse, (2, 0, false), "another field-course player cannot reuse that corpse");
    relocate(owner, session, angus_side).await?;
    assert_eq!(
        talk(owner, session, angus, "butterfly hunt").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 11, Some((butterfly, 17)), &[4872]).await?;
    assert_eq!(
        talk(owner, session, angus, "plant collection").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 12, Some((plant, 18)), &[4867, 4872]).await?;
    relocate(owner, session, position(BELLS.x - 1, BELLS.y, BELLS.floor)).await?;
    tool_use(owner, session, 4867, Some(BELLS)).await?;
    state(h, 13, Some((plant, 19)), &[4868, 4872]).await?;
    relocate(owner, session, angus_side).await?;
    assert_eq!(
        talk(owner, session, angus, "plant collection").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 14, Some((plant, 26)), &[4872]).await?;
    assert_eq!(
        talk(owner, session, angus, "lizard urn").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 15, Some((urn, 27)), &[4872]).await?;
    relocate(owner, session, position(32957, 32836, -8)).await?;
    door(owner, session, position(32957, 32835, -8)).await?;
    assert_eq!(
        map_use(owner, session, URN).await?,
        UseDisposition::Committed
    );
    state(h, 16, Some((urn, 28)), &[4847, 4872]).await?;
    relocate(owner, session, angus_side).await?;
    assert_eq!(
        talk(owner, session, angus, "lizard urn").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 17, Some((urn, 29)), &[4872]).await?;
    assert_eq!(
        talk(owner, session, angus, "elven poetry").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 18, Some((poetry, 36)), &[4872]).await?;
    relocate(owner, session, position(32702, 31605, -14)).await?;
    door(owner, session, position(32703, 31605, -14)).await?;
    assert_eq!(
        map_use(owner, session, POETRY).await?,
        UseDisposition::Committed
    );
    state(h, 19, Some((poetry, 37)), &[4844, 4872]).await?;

    relocate(owner, session, angus_side).await?;
    let terminal = session.continuity.next_command_id + 1;
    assert_eq!(
        talk(owner, session, angus, "elven book").await?,
        NpcIntentDisposition::Ok
    );
    state(h, 20, Some((poetry, 38)), &[4872, 9605]).await?;
    // Each wire helper closes its socket after reading the result. A newly
    // served connection replays the exact command before a new submission.
    let replay = wire(
        owner,
        session,
        terminal,
        7,
        talk_payload(angus, "elven book")?,
    )
    .await?;
    assert_eq!(
        decode_npc_intent_result(&replay).map_err(debug)?,
        NpcIntentDisposition::Ok
    );
    assert_eq!(
        talk(owner, session, angus, "elven book").await?,
        NpcIntentDisposition::Rejected
    );
    state(h, 20, Some((poetry, 38)), &[4872, 9605]).await?;
    let completions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(completions, 1);
    assert_journal(owner, session).await?;

    // Fund the fare independently of the quest. Only the terminal course
    // receipt supplies the route's admission fact.
    let mut funding = 90002;
    super::qengine_voucher_stories_fixtures::mint(owner, session, 3035, 2, &mut funding).await?;
    relocate(owner, session, max_side).await?;
    let captain = npc(owner, "oteryn:npc.captain_max").await?;
    assert_eq!(
        conversation(owner, session, captain, &["hi", "calassa", "yes"]).await?,
        NpcIntentDisposition::Ok
    );
    let arrived: MovementLocalPosition = owner
        .runtime
        .lock()
        .await
        .read_actor_position(session.runtime_actor.ok_or("player")?)
        .map_err(debug)?
        .position();
    assert_eq!(arrived, local(CALASSA));
    state(h, 20, Some((poetry, 38)), &[4872, 9605]).await
}
