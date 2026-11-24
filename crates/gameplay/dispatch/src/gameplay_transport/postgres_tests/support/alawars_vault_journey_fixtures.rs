//! The chosen Folda route, with the shipped Senja callbacks and once-claim owner.

use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks_fixtures::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply, SessionContinuity,
};
use crate::gameplay_transport::last_position::AdmissionPosition;
use crate::map::floor::{Ground, resolve_landing};
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use crate::movement::CardinalStep;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::quest_log::{
    COMMAND_TYPE_QUEST_LOG_QUERY, QuestLogQuery, QuestLogView, decode_quest_log,
    encode_quest_log_query,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    CAPABILITY_DIAGONAL_MOVEMENT_V1, StepDirection, StepDisposition,
};
use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU64;
use std::time::Duration;

pub(super) const NAME: &str = "alawars_vault_full_journey";
const QUEST: &str = "oteryn:quest.alawar_s_vault_quest";
const TRACKS: [&str; 6] = [
    "oteryn:quest-progress/alawars-vault/journey",
    "oteryn:quest-progress/alawars-vault/lever",
    "oteryn:quest-progress/chosen-source/alawar_s_vault_quest/s1",
    "oteryn:quest-progress/kv/alawars_vault1/claimed",
    "oteryn:quest-progress/kv/alawars_vault2/claimed",
    "oteryn:quest-progress/alawars-vault/return",
];

const fn position(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

pub(super) const START: Position = position(32031, 31687, -8);
const FOLDA_CACHE: Position = position(32030, 31688, -8);
const FOLDA_LOCK: Position = position(32035, 31642, -8);
const SHAFT: Position = position(32040, 31636, -8);
const LANDING: Position = position(32040, 31636, -10);
const KEY_CACHE: Position = position(32201, 31571, -10);
const VAULT_LOCK: Position = position(32107, 31568, -9);
const VAULT_ARRIVAL: Position = position(32107, 31567, -9);
const CASTLE: Position = position(32189, 31625, -4);
const LEVER: Position = position(32180, 31633, -8);
const SENJA_APPROACH: Position = position(32187, 31623, -8);
const PEARLS: Position = position(32105, 31567, -9);
const SWORD: Position = position(32109, 31567, -9);

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    // The Folda corridor crosses one region boundary. Keep the production collision
    // and floor changes from both regions and qualify this route against them.
    super::source_world_crop::compiled_regions(
        tile(position(31925, 31520, -10)),
        tile(position(32239, 31712, -4)),
        tile(START),
        native,
    )
}

fn tile(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Ledger {
    tracks: [i64; TRACKS.len()],
    completed: bool,
    inventory: BTreeMap<(String, i32), i64>,
}

impl Ledger {
    async fn read(owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness) -> TestResult<Self> {
        let character = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
        let state = owner
            .root
            .read_character_quest_state(owner.character, character)
            .await
            .map_err(debug)?;
        let rows: Vec<(String, i32, i64)> = sqlx::query_as(
            "SELECT i.definition_production_key, coalesce(k.key_number,0), sum(i.quantity)::bigint
             FROM game_quest_callback_carried(encode($1,'hex')::uuid, encode($2,'hex')::uuid) c
             JOIN game_item_instances i ON i.item_instance_id=c.item AND i.lifecycle=1
             LEFT JOIN game_item_keys k USING (item_instance_id)
             GROUP BY i.definition_production_key, k.key_number",
        )
        .bind(character.as_bytes().as_slice())
        .bind(owner.world_id.as_bytes().as_slice())
        .fetch_all(&h.pool)
        .await?;
        Ok(Self {
            tracks: TRACKS.map(|key| state.tracks().get(key).copied().unwrap_or_default()),
            completed: state.states().get(QUEST).is_some_and(|s| s.completed),
            inventory: rows
                .into_iter()
                .map(|(key, number, count)| ((key, number), count))
                .collect(),
        })
    }

    fn add(&mut self, item: u16, number: i32, count: i64) {
        *self
            .inventory
            .entry((format!("oteryn:item.tibia.i{item}"), number))
            .or_default() += count;
    }

    async fn assert(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        h: &Harness,
        step: &str,
    ) -> TestResult {
        assert_eq!(Self::read(owner, h).await?, *self, "{step}");
        Ok(())
    }
}

async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<MovementLocalPosition> {
    super::full_world_support::position(owner, admitted.runtime_actor.ok_or("actor")?).await
}

fn next_tick(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("clock")?;
    Ok(())
}

type Cell = (i32, i32, i32);

fn cell(at: MovementLocalPosition) -> Cell {
    (at.x, at.y, i32::from(at.floor))
}

const DIRECTIONS: [(CardinalStep, StepDirection); 8] = [
    (CardinalStep::North, StepDirection::North),
    (CardinalStep::East, StepDirection::East),
    (CardinalStep::South, StepDirection::South),
    (CardinalStep::West, StepDirection::West),
    (CardinalStep::NorthEast, StepDirection::NorthEast),
    (CardinalStep::SouthEast, StepDirection::SouthEast),
    (CardinalStep::SouthWest, StepDirection::SouthWest),
    (CardinalStep::NorthWest, StepDirection::NorthWest),
];

fn route(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    start: Cell,
    goal: Cell,
) -> TestResult<Vec<(StepDirection, Cell)>> {
    let cells = owner.movement_cells();
    let map = owner.bundle_map().ok_or("compiled map")?;
    let ground = |(x, y, z): Cell| {
        let (Ok(tx), Ok(ty), Ok(floor)) = (u16::try_from(x), u16::try_from(y), i8::try_from(-z))
        else {
            return Ground::Absent;
        };
        if map.facts.doors().blocks(TilePos {
            x: tx,
            y: ty,
            floor,
        }) {
            return Ground::Blocked;
        }
        match cells.index().lookup(cells.scope(), LogicalCell { x, y, z }) {
            Ok(CollisionClass::Walkable) => Ground::Walkable,
            Ok(CollisionClass::Blocked) => Ground::Blocked,
            Err(_) => Ground::Absent,
        }
    };
    let mut arrivals = BTreeMap::new();
    let mut pending = VecDeque::from([start]);
    while let Some(current) = pending.pop_front() {
        if current == goal {
            let mut path = Vec::new();
            let mut destination = goal;
            while destination != start {
                let (origin, direction) = arrivals[&destination];
                path.push((direction, destination));
                destination = origin;
            }
            path.reverse();
            return Ok(path);
        }
        for (direction, wire_direction) in DIRECTIONS {
            let Ok(next) = resolve_landing(&map.floors, ground, |_| true, current, direction)
            else {
                continue;
            };
            if next == start || arrivals.contains_key(&next) {
                continue;
            }
            if arrivals.len() >= 32_768 {
                return Err(format!("route bound from {start:?} to {goal:?}").into());
            }
            arrivals.insert(next, (current, wire_direction));
            pending.push_back(next);
        }
    }
    Err(format!("compiled World has no route from {start:?} to {goal:?}").into())
}

async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    next_tick(owner)?;
    let (result, _) = owner
        .paced_step(
            admitted.runtime_actor.ok_or("step actor")?,
            admitted.game_session_id,
            direction,
        )
        .await;
    assert_eq!(result.disposition, StepDisposition::Moved, "{direction:?}");
    Ok(())
}

async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    goal: Position,
) -> TestResult {
    let path = route(owner, cell(at(owner, admitted).await?), cell(local(goal)))?;
    for (direction, expected) in path {
        step(owner, admitted, direction).await?;
        assert_eq!(
            cell(at(owner, admitted).await?),
            expected,
            "source walk to {goal:?}"
        );
    }
    Ok(())
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    p: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = placement_key(tile(p), ordinal).ok_or("map placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    next_tick(owner)?;
    let target = target(owner, p, ordinal)?;
    let payload =
        oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let body = wire(owner, admitted, command, 2, payload).await?;
    oteryn_protocol_oteryn::world_object::decode_use_result(&body).map_err(debug)
}

async fn use_key(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    p: Position,
    number: i32,
) -> TestResult {
    let reference = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i2970", "definition-r1"))
        .ok_or("copper key definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(owner, p, 1)?)),
        use_with_position: None,
    })
    .map_err(debug)?;
    // Both numbered keys share the same appearance. Try their projected
    // handles, and require the door to reject the wrong number without a change.
    for offset in 0..2 {
        next_tick(owner)?;
        let (next, disposition, _) = super::item_craft_fixtures::exchange_selected(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(reference),
            offset,
        )
        .await?;
        *admitted = next;
        if disposition == UseDisposition::Committed {
            return Ok(());
        }
        assert_eq!(disposition, UseDisposition::KeyMismatch, "key {number}");
    }
    Err(format!("no carried copper key opened lock {number}").into())
}

async fn take_vault_key(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::{
        connection::UseCommand,
        item_move::{LoopMove, LoopMoveResult, LoopTarget},
        item_view::ItemKey,
    };

    let key: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(k.item_instance_id)
         FROM game_item_keys k JOIN game_item_container_entries c USING (item_instance_id)
         JOIN game_item_instances bag ON bag.item_instance_id=c.parent_item_instance_id
         WHERE k.key_number=4502 AND bag.definition_production_key='oteryn:item.tibia.i2853'",
    )
    .fetch_one(&h.pool)
    .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "vault key fence")?
        .ok_or("vault key fence")?;
    assert_eq!(
        owner
            .move_loop_item(
                admitted.runtime_actor.ok_or("vault key actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 90_002,
                    item_fence: Some(fence.item_fence()),
                },
                LoopMove {
                    item: ItemKey::Instance(key.try_into().map_err(|_| "vault key identity")?),
                    count: None,
                    target: LoopTarget::MainBackpack,
                    slot: None,
                },
            )
            .await,
        LoopMoveResult::Committed,
        "the key from Alawar's bag is reachable through normal inventory custody"
    );
    Ok(())
}

async fn use_pick(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let index = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("native Item index")?;
    let reference = index
        .definition_ref("Item", "oteryn:item.tibia.i3456", "definition-r1")
        .ok_or("pick")?;
    let shovel = index
        .definition_ref("Item", "oteryn:item.tibia.i3457", "definition-r1")
        .ok_or("shovel")?;
    let shaft = target(owner, SHAFT, 0)?;
    let before = at(owner, admitted).await?;
    let wrong_tool = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(shovel),
        use_with_map_item: Some(MapUseTarget::Object(shaft.clone())),
        use_with: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    next_tick(owner)?;
    let (next, outcome, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &wrong_tool,
        None,
    )
    .await?;
    *admitted = next;
    assert_eq!(outcome, UseDisposition::NothingToUse, "wrong Folda tool");
    assert_eq!(at(owner, admitted).await?, before);

    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with_map_item: Some(MapUseTarget::Object(shaft)),
        use_with: None,
        use_with_position: None,
    })
    .map_err(debug)?;
    next_tick(owner)?;
    let (next, outcome, _) = super::item_craft_fixtures::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *admitted = next;
    assert_eq!(outcome, UseDisposition::Committed, "Folda shaft");
    assert_eq!(at(owner, admitted).await?, local(LANDING));
    Ok(())
}

async fn reconnect(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let last = at(owner, admitted).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "logout fence")?
        .ok_or("logout fence")?;
    assert!(owner.track_restored(
        admitted.game_session_id,
        admitted.runtime_actor.ok_or("logout actor")?,
        &AdmissionPosition::FirstLogin,
        fence
    ));
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(120))
        .ok_or("clock")?;
    assert_eq!(
        owner.logout_session(*admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    let session = super::full_world_support::relogin(h).await?;
    let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
    let current = store.current_session(session).await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "reconnect fence")?
        .ok_or("reconnect fence")?;
    let saved = owner
        .read_admission_position(fence)
        .await
        .map_err(|()| "stored position")?;
    assert!(matches!(saved, AdmissionPosition::Stored(_)));
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        owner
            .initialize_admitted_position(&mut runtime, actor, session, &saved)
            .await
            .map_err(debug)?;
        actor
    };
    assert!(owner.track_restored(session, actor, &saved, fence));
    *admitted = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        item_fence: Some(fence.item_fence()),
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            transport: current.current_transport().ok_or("reconnect transport")?,
            account_id: id(40),
        }),
        continuity: SessionContinuity {
            selected_capabilities: admitted.continuity.selected_capabilities,
            connection_generation: current.current_connection_generation().get(),
            ..SessionContinuity::FRESH
        },
        ..*admitted
    };
    owner.admit_progression_session(admitted).await;
    owner.admit_quest_session(admitted).await;
    assert_eq!(at(owner, admitted).await?, last);
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let capabilities = [4, 6, 11, 13, 15, 16, 18, CAPABILITY_DIAGONAL_MOVEMENT_V1];
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &capabilities)
            .ok_or("journey capabilities")?;
    assert_eq!(
        admitted.continuity.selected_capabilities.as_slice(),
        capabilities
    );
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "starter fence")?
        .ok_or("starter fence")?;
    // A pick is equipment brought to Folda. All quest items are acquired below.
    super::quest_world_callbacks_fixtures::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: START,
            item: "oteryn:item.tibia.i3456".into(),
            count: 1,
        },
        4000,
        true,
    )
    .await?;
    let mut expected = Ledger::read(owner, h).await?;
    assert_eq!(expected.tracks, [0; TRACKS.len()]);
    assert!(!expected.completed);
    for item in [2853, 2970, 3026, 3031, 3147, 3298, 3301, 3384] {
        let key = format!("oteryn:item.tibia.i{item}");
        assert!(expected.inventory.keys().all(|(owned, _)| owned != &key));
    }

    let command = admitted.continuity.next_command_id;
    let first = wire(
        owner,
        admitted,
        command,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::West),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result_profile(&first, true, true)
            .map_err(debug)?
            .disposition,
        StepDisposition::Moved
    );
    expected.tracks[0] = 1;
    expected.assert(owner, h, "Folda approach").await?;

    assert_eq!(
        use_object(owner, admitted, FOLDA_CACHE, 2).await?,
        UseDisposition::Committed
    );
    expected.tracks[0] = 2;
    expected.add(2970, 4503, 1);
    expected.assert(owner, h, "Folda key cache").await?;

    walk(owner, admitted, position(32036, 31642, -8)).await?;
    use_key(owner, admitted, FOLDA_LOCK, 4503).await?;
    expected.tracks[0] = 3;
    expected.assert(owner, h, "Folda lock").await?;
    walk(owner, admitted, FOLDA_LOCK).await?;

    walk(owner, admitted, position(32039, 31636, -8)).await?;
    use_pick(owner, admitted).await?;
    expected.tracks[0] = 4;
    expected
        .assert(owner, h, "Folda shaft and reusable pick")
        .await?;

    walk(owner, admitted, position(32202, 31571, -10)).await?;
    assert_eq!(
        use_object(owner, admitted, KEY_CACHE, 2).await?,
        UseDisposition::Committed
    );
    expected.tracks[0] = 5;
    for (item, number, count) in [
        (2853, 0, 1),
        (2970, 4502, 1),
        (3031, 0, 33),
        (3147, 0, 1),
        (3298, 0, 4),
        (3384, 0, 1),
    ] {
        expected.add(item, number, count);
    }
    expected
        .assert(owner, h, "key 4502 bag and side rewards")
        .await?;
    take_vault_key(owner, admitted, h).await?;

    walk(owner, admitted, position(32106, 31569, -9)).await?;
    use_key(owner, admitted, VAULT_LOCK, 4502).await?;
    expected.tracks[0] = 6;
    expected.assert(owner, h, "vault lock").await?;
    walk(owner, admitted, VAULT_LOCK).await?;
    walk(owner, admitted, VAULT_ARRIVAL).await?;
    step(owner, admitted, StepDirection::North).await?;
    assert_eq!(at(owner, admitted).await?, local(CASTLE));
    expected.tracks[5] = 1;
    expected
        .assert(
            owner,
            h,
            "early return permits exit and leaves quest incomplete",
        )
        .await?;

    walk(owner, admitted, position(32187, 31627, -6)).await?;
    assert_eq!(
        use_object(owner, admitted, position(32188, 31628, -6), 1).await?,
        UseDisposition::Committed
    );
    walk(owner, admitted, position(32181, 31634, -8)).await?;
    assert_eq!(
        use_object(owner, admitted, LEVER, 1).await?,
        UseDisposition::Committed
    );
    expected.tracks[1] = 1;
    expected.assert(owner, h, "Senja magic-wall lever").await?;

    walk(owner, admitted, position(32187, 31629, -8)).await?;
    assert_eq!(
        use_object(owner, admitted, position(32187, 31628, -8), 1).await?,
        UseDisposition::Committed
    );
    walk(owner, admitted, position(32187, 31627, -8)).await?;
    walk(owner, admitted, position(32187, 31626, -8)).await?;
    assert_eq!(
        use_object(owner, admitted, position(32187, 31625, -8), 1).await?,
        UseDisposition::Committed
    );
    walk(owner, admitted, SENJA_APPROACH).await?;
    expected.tracks[2] = 1;
    expected
        .assert(owner, h, "witnessed vault approach")
        .await?;
    step(owner, admitted, StepDirection::North).await?;
    assert_eq!(at(owner, admitted).await?, local(VAULT_ARRIVAL));
    expected.assert(owner, h, "vault entrance teleport").await?;

    walk(owner, admitted, position(32106, 31567, -9)).await?;
    assert_eq!(
        use_object(owner, admitted, PEARLS, 1).await?,
        UseDisposition::Committed
    );
    expected.tracks[3] = 1;
    expected.add(3026, 0, 3);
    expected.assert(owner, h, "white pearl chest").await?;

    walk(owner, admitted, position(32108, 31567, -9)).await?;
    next_tick(owner)?;
    let sword = target(owner, SWORD, 1)?;
    let use_id = admitted.continuity.next_command_id;
    let results = super::quest_callbacks_fixtures::wire_use_targets_retained(
        owner,
        admitted,
        &[(use_id, &sword), (use_id, &sword)],
    )
    .await?;
    assert_eq!(
        results,
        [
            (
                oteryn_protocol_oteryn::CommandStatus::Accepted,
                UseDisposition::Committed
            ),
            (
                oteryn_protocol_oteryn::CommandStatus::DuplicateReplay,
                UseDisposition::Committed
            ),
        ]
    );
    expected.tracks[4] = 1;
    expected.add(3301, 0, 1);
    expected
        .assert(owner, h, "broadsword and exact command replay")
        .await?;

    reconnect(owner, admitted, h).await?;
    assert_eq!(
        use_object(owner, admitted, SWORD, 1).await?,
        UseDisposition::NothingToUse
    );
    expected
        .assert(owner, h, "fresh USE refused after reconnect")
        .await?;
    walk(owner, admitted, VAULT_ARRIVAL).await?;
    step(owner, admitted, StepDirection::North).await?;
    assert_eq!(at(owner, admitted).await?, local(CASTLE));
    expected.completed = true;
    expected
        .assert(owner, h, "return teleport completes the full journey")
        .await?;

    let command = admitted.continuity.next_command_id;
    let query = encode_quest_log_query(&QuestLogQuery::ListOnly).map_err(debug)?;
    let body = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_QUEST_LOG_QUERY,
        query,
    )
    .await?;
    let Some(QuestLogView::List(log)) = decode_quest_log(&body).map_err(debug)?.view else {
        return Err("quest log list response".into());
    };
    assert!(
        log.is_empty(),
        "the vault quest remains hidden from a fresh character's log"
    );
    Ok(())
}
