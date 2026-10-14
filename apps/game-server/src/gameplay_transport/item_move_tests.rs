//! ITEM-MOVE-1: the WIRE-0 §5 mapping, one test per row, and command 9 through the connection.

#![allow(clippy::expect_used)]

use super::super::capabilities::{OfferedCapability, SelectedCapabilities};
use super::super::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAttempt,
    IDLE_LIVENESS, SessionContinuity, StepOutcome, serve_admitted,
};
use super::super::item_view::{InventoryItems, ItemKey, ItemViewContinuity, ViewItem};
use super::super::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, encode_use_item_intent, encode_use_result,
};
use super::super::world_spatial::{
    ActorPosition, ENTITY_IDENTITY_BYTES, StepDirection, StepDisposition, WorldSpatialObservation,
};
use super::*;
use crate::combat_pickup::PickupContentError;
use crate::durability::DurabilityError;
use crate::durability::item_transfer::{CommittedItemTransfer, TransferShape};
use crate::foundation::{
    ChannelId, CommandStatus, GameSessionId, WorldId, encode_command_result, encode_state_delta,
};
use oteryn_protocol_oteryn::item_view::{
    CAPABILITY_ITEM_EQUIP_DROP_V1, CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES, COMMAND_TYPE_ITEM_MOVE_INTENT, EquipmentSlot,
    ItemHandle, ItemMoveIntent, decode_open_container, encode_item_move_intent,
    encode_item_move_intent_for, encode_item_move_result, encode_item_move_result_with_equip_drop,
};
use oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::error::Error;
use std::num::NonZeroU32;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

type TestResult = Result<(), Box<dyn Error>>;

fn fault(error: impl std::fmt::Debug) -> Box<dyn Error> {
    format!("{error:?}").into()
}

const HERE: ActorPosition = at(10, 10, 7);
const NEAR: ActorPosition = at(11, 10, 7);

const fn at(x: i32, y: i32, floor: i16) -> ActorPosition {
    ActorPosition { x, y, floor }
}

const fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70 | (tag & 0x0f);
    bytes[8] = 0x80 | (tag & 0x3f);
    bytes
}

fn instance(n: u16) -> ItemKey {
    let mut id = [0xA5; 16];
    id[..2].copy_from_slice(&n.to_be_bytes());
    ItemKey::Instance(id)
}

fn corpse(n: u8) -> ItemKey {
    ItemKey::Entity([n; ENTITY_IDENTITY_BYTES])
}

fn item(key: ItemKey) -> ViewItem {
    ViewItem {
        key,
        item_definition_ref: NonZeroU32::new(2854).expect("fixture"),
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}

fn inventory(entries: &[u16]) -> InventoryItems {
    InventoryItems {
        main_backpack: Some(item(instance(0))),
        entries: entries.iter().map(|n| item(instance(*n))).collect(),
        equipment: Vec::new(),
    }
}

/// Backpack entries `entries` and the occupied slots `worn`, in ascending slot order.
fn worn(entries: &[u16], worn: &[(EquipmentSlot, u16)]) -> InventoryItems {
    InventoryItems {
        equipment: worn
            .iter()
            .map(|(slot, n)| (*slot, item(instance(*n))))
            .collect(),
        ..inventory(entries)
    }
}

fn observe(
    actor: ActorPosition,
    position: ActorPosition,
    contents: &[u16],
) -> ItemTargetObservation {
    ItemTargetObservation {
        actor,
        target: UseItemTarget::Corpse {
            position,
            contents: contents.iter().map(|n| item(instance(*n))).collect(),
        },
    }
}

fn handle(n: u64) -> ItemHandle {
    ItemHandle::new(n).expect("fixture")
}

fn committed() -> CommittedItemTransfer {
    CommittedItemTransfer {
        transaction_id: [1; 16],
        event_id: [2; 16],
        occurred_at_unix_ms: 0,
        envelope_sha256: [3; 32],
        shape: TransferShape::NewEntry,
        source_item_instance_id: [0xA5; 16],
        source_quantity_before: 1,
        source_quantity_after: 0,
        receiver: None,
        destination: None,
    }
}

fn refused(refusal: ItemTransferRefusal) -> Result<ItemTransferOutcome, GroundPickupError> {
    Err(GroundPickupError::Transfer(ItemTransferError::Refused(
        refusal,
    )))
}

fn transfer(error: ItemTransferError) -> Result<ItemTransferOutcome, GroundPickupError> {
    Err(GroundPickupError::Transfer(error))
}

fn maps(results: Vec<Result<ItemTransferOutcome, GroundPickupError>>, outcome: ItemMoveOutcome) {
    for result in results {
        assert_eq!(outcome_of(&result), outcome, "{result:?}");
    }
}

// WIRE-0 §5, one test per row.

#[test]
fn moved_is_a_committed_or_already_committed_transfer() {
    maps(
        vec![
            Ok(ItemTransferOutcome::Committed(committed())),
            Ok(ItemTransferOutcome::AlreadyCommitted(committed())),
        ],
        ItemMoveOutcome::Moved,
    );
}

#[test]
fn stale_is_a_source_no_longer_on_the_corpse_or_another_source() {
    maps(
        vec![
            refused(ItemTransferRefusal::SourceNotOnGround),
            Err(GroundPickupError::SourceMismatch),
        ],
        ItemMoveOutcome::Stale,
    );
}

#[test]
fn no_backpack_is_no_main_backpack() {
    maps(
        vec![refused(ItemTransferRefusal::NoMainBackpack)],
        ItemMoveOutcome::NoBackpack,
    );
}

#[test]
fn no_room_is_a_full_main_backpack() {
    maps(
        vec![refused(ItemTransferRefusal::MainBackpackFull)],
        ItemMoveOutcome::NoRoom,
    );
}

#[test]
fn not_owner_is_the_corpse_exclusivity_window() {
    maps(
        vec![refused(ItemTransferRefusal::CorpseExclusiveWindow)],
        ItemMoveOutcome::NotOwner,
    );
}

#[test]
fn not_pickupable_is_a_corpse_source() {
    maps(
        vec![refused(ItemTransferRefusal::CorpseNotPickupable)],
        ItemMoveOutcome::NotPickupable,
    );
}

#[test]
fn not_supported_is_a_non_empty_container() {
    maps(
        vec![refused(ItemTransferRefusal::ContainerNotEmpty)],
        ItemMoveOutcome::NotSupported,
    );
}

#[test]
fn rejected_is_every_other_refusal_and_error() {
    maps(
        vec![
            refused(ItemTransferRefusal::DefinitionMismatch),
            refused(ItemTransferRefusal::UnknownStackClass),
            refused(ItemTransferRefusal::UnsupportedStackMaximum),
            refused(ItemTransferRefusal::QuantityAboveStackMaximum),
            refused(ItemTransferRefusal::UnsupportedContainerCapacity),
            refused(ItemTransferRefusal::NotContainerSlotEquippable),
            refused(ItemTransferRefusal::ContainerSlotOccupied),
            transfer(ItemTransferError::ConflictingCause),
            transfer(ItemTransferError::ConflictingCandidate),
            transfer(ItemTransferError::InvalidInput),
            transfer(ItemTransferError::CapacityExceeded),
            transfer(ItemTransferError::AuthorityRejected),
            transfer(ItemTransferError::Unavailable(DurabilityError::Unavailable)),
            Err(GroundPickupError::Content(
                PickupContentError::DefinitionNotFound,
            )),
            Err(GroundPickupError::Content(PickupContentError::NotAnItem)),
        ],
        ItemMoveOutcome::Rejected,
    );
}

// Command 9 against a Channel owner fixture.

#[derive(Debug, Clone, Copy)]
enum Replay {
    Nothing,
    Committed,
    Unavailable,
    Conflicting,
}

/// The occupied slots of one equipment read.
type Slots = Vec<(EquipmentSlot, ViewItem)>;

/// The Channel owner as command 9 sees it: the replay read, one TRANSFER result, the backpack and
/// one observation per item target read.
struct MoveAuthority {
    replay: Replay,
    take: RefCell<Option<Result<ItemTransferOutcome, GroundPickupError>>>,
    takes: RefCell<Vec<(u64, ItemKey, ItemKey)>>,
    replays: Cell<u32>,
    inventory: RefCell<Option<InventoryItems>>,
    after_take: Option<InventoryItems>,
    targets: RefCell<VecDeque<Option<ItemTargetObservation>>>,
    /// EQUIP-WIRE-1: the equipment replay read, one equipment move result and the inventory
    /// (with its slots) the owner shows after it commits.
    equip_replay: Option<EquipmentMoveReplay>,
    equip_result: Cell<Option<EquipmentMoveResult>>,
    equip_moves: RefCell<Vec<(u64, EquipmentMove)>>,
    equipment: RefCell<Option<Vec<(EquipmentSlot, ViewItem)>>>,
    after_equip: Option<InventoryItems>,
    /// Equipment reads answered in order before `equipment`, and the inventory reads counted.
    equipment_reads: RefCell<VecDeque<Option<Slots>>>,
    inventory_reads: Cell<u32>,
}

impl MoveAuthority {
    fn new(replay: Replay, targets: Vec<Option<ItemTargetObservation>>) -> Self {
        Self {
            replay,
            take: RefCell::new(None),
            takes: RefCell::new(Vec::new()),
            replays: Cell::new(0),
            inventory: RefCell::new(Some(inventory(&[1]))),
            after_take: None,
            targets: RefCell::new(targets.into()),
            equip_replay: Some(EquipmentMoveReplay::Absent),
            equip_result: Cell::new(None),
            equip_moves: RefCell::new(Vec::new()),
            equipment: RefCell::new(Some(Vec::new())),
            after_equip: None,
            equipment_reads: RefCell::new(VecDeque::new()),
            inventory_reads: Cell::new(0),
        }
    }

    /// The owner shows `shown` (backpack and slots) and answers the next move with `result`,
    /// showing `after` once it commits.
    fn equipping(
        mut self,
        shown: InventoryItems,
        result: EquipmentMoveResult,
        after: Option<InventoryItems>,
    ) -> Self {
        self.show(Some(shown));
        self.equip_result = Cell::new(Some(result));
        self.after_equip = after;
        self
    }

    fn show(&self, shown: Option<InventoryItems>) {
        *self.equipment.borrow_mut() = shown.as_ref().map(|shown| shown.equipment.clone());
        *self.inventory.borrow_mut() = shown.map(|shown| InventoryItems {
            equipment: Vec::new(),
            ..shown
        });
    }

    fn taking(
        mut self,
        result: Result<ItemTransferOutcome, GroundPickupError>,
        after: Option<InventoryItems>,
    ) -> Self {
        self.take = RefCell::new(Some(result));
        self.after_take = after;
        self
    }
}

impl FreshAdmissionAuthority for MoveAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: HERE,
        })
    }

    async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
        StepOutcome {
            disposition: StepDisposition::Blocked,
            moved_to: None,
        }
    }

    async fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<InventoryItems> {
        self.inventory_reads.set(self.inventory_reads.get() + 1);
        self.inventory.borrow().clone()
    }

    async fn observe_character_equipment(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<Vec<(EquipmentSlot, ViewItem)>> {
        if let Some(read) = self.equipment_reads.borrow_mut().pop_front() {
            return read;
        }
        self.equipment.borrow().clone()
    }

    async fn committed_equipment_move(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> Option<EquipmentMoveReplay> {
        self.equip_replay
    }

    async fn move_equipment(
        &self,
        _actor: ExactActorRef,
        command: UseCommand,
        request: EquipmentMove,
    ) -> EquipmentMoveResult {
        self.equip_moves
            .borrow_mut()
            .push((command.command_id, request));
        let result = self.equip_result.take().expect("one equipment move");
        if result == EquipmentMoveResult::Committed {
            self.show(self.after_equip.clone());
        }
        result
    }

    async fn observe_item_target(
        &self,
        _actor: ExactActorRef,
        _target: ItemKey,
    ) -> Option<ItemTargetObservation> {
        self.targets.borrow_mut().pop_front().flatten()
    }

    async fn committed_item_move(
        &self,
        _actor: ExactActorRef,
        _command: UseCommand,
    ) -> Result<Option<CommittedItemTransfer>, ItemTransferError> {
        self.replays.set(self.replays.get() + 1);
        match self.replay {
            Replay::Nothing => Ok(None),
            Replay::Committed => Ok(Some(committed())),
            Replay::Unavailable => {
                Err(ItemTransferError::Unavailable(DurabilityError::Unavailable))
            }
            Replay::Conflicting => Err(ItemTransferError::ConflictingCause),
        }
    }

    async fn take_corpse_entry(
        &self,
        _actor: ExactActorRef,
        command: UseCommand,
        corpse: ItemKey,
        entry: ItemKey,
    ) -> Result<ItemTransferOutcome, GroundPickupError> {
        self.takes
            .borrow_mut()
            .push((command.command_id, corpse, entry));
        let result = self.take.borrow_mut().take().expect("one TRANSFER");
        if result.is_ok()
            && let Some(after) = &self.after_take
        {
            *self.inventory.borrow_mut() = Some(after.clone());
        }
        result
    }
}

fn actor() -> ExactActorRef {
    let world_id = WorldId::decode(&uuid_v7(0x33)).expect("world");
    let channel_id = ChannelId::decode(&uuid_v7(0x44)).expect("channel");
    ExactActorRef::transport_fixture(world_id, channel_id)
}

fn use_command(command_id: u64) -> UseCommand {
    UseCommand {
        game_session_id: GameSessionId::decode(&uuid_v7(0x22)).expect("session"),
        command_id,
        item_fence: None,
    }
}

const CORPSE_ONLY: ItemMoveSelection = ItemMoveSelection {
    equip_drop: false,
    container_tree: false,
    world_interaction: false,
};

fn into_backpack(source: ItemHandle) -> Vec<u8> {
    encode_item_move_intent(&ItemMoveIntent {
        source,
        destination: ItemMoveDestination::MainBackpack,
    })
    .expect("intent")
}

/// The view after a snapshot that reopened corpse 7 (entries 5 and 6) in reach, backpack entry 1.
fn opened() -> SessionItemView {
    let mut view = SessionItemView::resume(ItemViewContinuity {
        open_corpse: Some(corpse(7)),
        ..ItemViewContinuity::default()
    });
    view.snapshot(inventory(&[1]), Some(observe(HERE, NEAR, &[5, 6])))
        .expect("snapshot");
    view
}

fn entry(view: &SessionItemView, n: u16) -> ItemHandle {
    view.table().handle(&instance(n)).expect("live")
}

fn run(test: impl std::future::Future<Output = TestResult>) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(test)
}

async fn step(
    authority: &MoveAuthority,
    view: &mut SessionItemView,
    payload: &[u8],
    selection: ItemMoveSelection,
) -> ItemMoveStep {
    item_move(authority, actor(), use_command(1), payload, selection, view).await
}

fn result(outcome: ItemMoveOutcome) -> ItemMoveStep {
    ItemMoveStep::Result(outcome, Vec::new())
}

#[test]
fn a_committed_take_answers_moved_then_the_corpse_and_backpack_deltas_keeping_the_handle()
-> TestResult {
    run(async {
        let mut view = opened();
        let source = entry(&view, 5);
        let authority = MoveAuthority::new(
            Replay::Nothing,
            vec![
                Some(observe(HERE, NEAR, &[5, 6])),
                Some(observe(HERE, NEAR, &[6])),
            ],
        )
        .taking(
            Ok(ItemTransferOutcome::Committed(committed())),
            Some(inventory(&[1, 5])),
        );
        let moved = step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await;
        let mut mirror = opened();
        let backpack_delta = mirror
            .inventory_committed(inventory(&[1, 5]))
            .map_err(fault)?
            .expect("domain 9");
        let corpse_delta = mirror
            .open_corpse_committed(Some(observe(HERE, NEAR, &[6])))
            .map_err(fault)?
            .expect("domain 11");
        assert_eq!(
            moved,
            ItemMoveStep::Result(ItemMoveOutcome::Moved, vec![corpse_delta, backpack_delta])
        );
        assert_eq!(*authority.takes.borrow(), [(1, corpse(7), instance(5))]);
        // The entry keeps its ItemKey from the corpse into the backpack, so it keeps its handle.
        assert_eq!(view.table().handle(&instance(5)), Some(source));
        assert_eq!(view.table().resolve(source), Some(instance(5)));
        assert_eq!(view.continuity(), mirror.continuity());
        Ok(())
    })
}

#[test]
fn replay_first_answers_moved_for_a_stale_handle_without_a_transfer() -> TestResult {
    run(async {
        let mut view = opened();
        let authority =
            MoveAuthority::new(Replay::Committed, vec![Some(observe(HERE, NEAR, &[5, 6]))]);
        let moved = step(
            &authority,
            &mut view,
            &into_backpack(handle(999)),
            CORPSE_ONLY,
        )
        .await;
        assert_eq!(moved, result(ItemMoveOutcome::Moved));
        assert!(authority.takes.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn an_unknown_replay_disconnects_and_a_conflicting_one_is_rejected() -> TestResult {
    run(async {
        let mut view = opened();
        let source = entry(&view, 5);
        let authority = MoveAuthority::new(Replay::Unavailable, Vec::new());
        assert_eq!(
            step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await,
            ItemMoveStep::Disconnect
        );
        let authority = MoveAuthority::new(Replay::Conflicting, Vec::new());
        assert_eq!(
            step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await,
            result(ItemMoveOutcome::Rejected)
        );
        assert!(authority.takes.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn a_malformed_intent_is_rejected_before_any_read() -> TestResult {
    run(async {
        let mut view = opened();
        let authority = MoveAuthority::new(Replay::Committed, Vec::new());
        assert_eq!(
            step(&authority, &mut view, &[0xff], CORPSE_ONLY).await,
            result(ItemMoveOutcome::Rejected)
        );
        assert_eq!(authority.replays.get(), 0);
        Ok(())
    })
}

#[test]
fn equipment_and_inventory_sources_are_not_supported() -> TestResult {
    run(async {
        let mut view = opened();
        let source = entry(&view, 5);
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new());
        let selection = ItemMoveSelection {
            equip_drop: true,
            container_tree: false,
            world_interaction: false,
        };
        let to_head = encode_item_move_intent_for(
            &ItemMoveIntent {
                source,
                destination: ItemMoveDestination::Equipment(EquipmentSlot::Head),
            },
            selection,
        )
        .map_err(fault)?;
        assert_eq!(
            step(&authority, &mut view, &to_head, selection).await,
            result(ItemMoveOutcome::NotSupported)
        );
        // A backpack entry is not an entry of the open corpse.
        let backpack_entry = entry(&view, 1);
        assert_eq!(
            step(
                &authority,
                &mut view,
                &into_backpack(backpack_entry),
                CORPSE_ONLY
            )
            .await,
            result(ItemMoveOutcome::NotSupported)
        );
        assert!(authority.takes.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn a_dead_handle_a_gone_corpse_or_a_taken_entry_is_stale() -> TestResult {
    run(async {
        let mut view = opened();
        let source = entry(&view, 5);
        let authority = MoveAuthority::new(
            Replay::Nothing,
            vec![
                None,
                Some(ItemTargetObservation {
                    actor: HERE,
                    target: UseItemTarget::Gone,
                }),
                Some(observe(HERE, NEAR, &[6])),
            ],
        );
        assert_eq!(
            step(
                &authority,
                &mut view,
                &into_backpack(handle(999)),
                CORPSE_ONLY
            )
            .await,
            result(ItemMoveOutcome::Stale)
        );
        for _ in 0..3 {
            assert_eq!(
                step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await,
                result(ItemMoveOutcome::Stale)
            );
        }
        assert!(authority.takes.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn reach_is_checked_before_the_transfer() -> TestResult {
    run(async {
        let mut view = opened();
        let source = entry(&view, 5);
        let authority = MoveAuthority::new(
            Replay::Nothing,
            vec![Some(observe(at(13, 10, 7), NEAR, &[5, 6]))],
        );
        assert_eq!(
            step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await,
            result(ItemMoveOutcome::TooFar)
        );
        assert!(authority.takes.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn a_refused_transfer_answers_its_outcome_without_deltas() -> TestResult {
    run(async {
        let mut view = opened();
        let source = entry(&view, 5);
        let authority =
            MoveAuthority::new(Replay::Nothing, vec![Some(observe(HERE, NEAR, &[5, 6]))])
                .taking(refused(ItemTransferRefusal::MainBackpackFull), None);
        assert_eq!(
            step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await,
            result(ItemMoveOutcome::NoRoom)
        );
        assert_eq!(authority.takes.borrow().len(), 1);
        Ok(())
    })
}

#[test]
fn an_unknown_transfer_or_an_unreadable_backpack_after_it_disconnects() -> TestResult {
    run(async {
        let mut view = opened();
        let source = entry(&view, 5);
        let authority =
            MoveAuthority::new(Replay::Nothing, vec![Some(observe(HERE, NEAR, &[5, 6]))]).taking(
                transfer(ItemTransferError::Unavailable(DurabilityError::Unavailable)),
                None,
            );
        assert_eq!(
            step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await,
            ItemMoveStep::Disconnect
        );
        let authority = MoveAuthority::new(
            Replay::Nothing,
            vec![
                Some(observe(HERE, NEAR, &[5, 6])),
                Some(observe(HERE, NEAR, &[6])),
            ],
        )
        .taking(Ok(ItemTransferOutcome::Committed(committed())), None);
        *authority.inventory.borrow_mut() = None;
        assert_eq!(
            step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await,
            ItemMoveStep::Disconnect
        );
        Ok(())
    })
}

// EQUIP-WIRE-1 (ITEM-MOVE-WIRE-1 §4): equip and unequip under capability 12.

const EQUIP: ItemMoveSelection = ItemMoveSelection {
    equip_drop: true,
    container_tree: false,
    world_interaction: false,
};

/// The view after a capability 12 snapshot showing `shown`.
fn wearing(shown: InventoryItems) -> SessionItemView {
    let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_equip_drop();
    view.snapshot(shown, None).expect("snapshot");
    view
}

fn into_slot(source: ItemHandle, slot: EquipmentSlot) -> Vec<u8> {
    encode_item_move_intent_for(
        &ItemMoveIntent {
            source,
            destination: ItemMoveDestination::Equipment(slot),
        },
        EQUIP,
    )
    .expect("intent")
}

#[test]
fn every_equipment_refusal_answers_its_section_4_outcome() {
    use EquipmentMoveRefusal as Refusal;
    for (refusal, outcome) in [
        (Refusal::Stale, ItemMoveOutcome::Stale),
        (Refusal::NoBackpack, ItemMoveOutcome::NoBackpack),
        (Refusal::NoRoom, ItemMoveOutcome::NoRoom),
        (Refusal::SlotMismatch, ItemMoveOutcome::SlotMismatch),
        (
            Refusal::RequirementNotMet,
            ItemMoveOutcome::RequirementNotMet,
        ),
        // Swap is the next ITEM-MOVE-2a slice.
        (Refusal::Occupied, ItemMoveOutcome::NotSupported),
        (Refusal::TimedNotHosted, ItemMoveOutcome::NotSupported),
        (Refusal::Rejected, ItemMoveOutcome::Rejected),
    ] {
        assert_eq!(equipment_outcome(refusal), outcome, "{refusal:?}");
    }
}

#[test]
fn an_equip_commits_moved_then_the_domain_9_delta_keeping_the_handle() -> TestResult {
    run(async {
        let shown = inventory(&[1, 2]);
        let after = worn(&[1], &[(EquipmentSlot::RightHand, 2)]);
        let mut view = wearing(shown.clone());
        let source = entry(&view, 2);
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
            shown.clone(),
            EquipmentMoveResult::Committed,
            Some(after.clone()),
        );
        let moved = step(
            &authority,
            &mut view,
            &into_slot(source, EquipmentSlot::RightHand),
            EQUIP,
        )
        .await;
        let mut mirror = wearing(shown);
        let delta = mirror
            .inventory_committed(after)
            .map_err(fault)?
            .expect("domain 9");
        assert_eq!(
            moved,
            ItemMoveStep::Result(ItemMoveOutcome::Moved, vec![delta])
        );
        assert_eq!(
            *authority.equip_moves.borrow(),
            [(
                1,
                EquipmentMove::Equip {
                    item: instance(2),
                    to: EquipmentSlot::RightHand
                }
            )]
        );
        assert_eq!(
            view.equipped_slot(&instance(2)),
            Some(EquipmentSlot::RightHand)
        );
        assert_eq!(view.table().handle(&instance(2)), Some(source));
        assert!(authority.takes.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn an_unequip_commits_moved_into_the_main_backpack() -> TestResult {
    run(async {
        let shown = worn(&[1], &[(EquipmentSlot::Armor, 3)]);
        let after = inventory(&[1, 3]);
        let mut view = wearing(shown.clone());
        let source = entry(&view, 3);
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
            shown,
            EquipmentMoveResult::Committed,
            Some(after),
        );
        let moved = step(&authority, &mut view, &into_backpack(source), EQUIP).await;
        assert!(matches!(
            moved,
            ItemMoveStep::Result(ItemMoveOutcome::Moved, ref deltas) if deltas.len() == 1
        ));
        assert_eq!(
            *authority.equip_moves.borrow(),
            [(
                1,
                EquipmentMove::Unequip {
                    item: instance(3),
                    from: EquipmentSlot::Armor
                }
            )]
        );
        assert_eq!(view.equipped_slot(&instance(3)), None);
        assert!(view.is_backpack_entry(&instance(3)));
        assert_eq!(view.table().handle(&instance(3)), Some(source));
        Ok(())
    })
}

#[test]
fn a_refused_equipment_move_answers_its_outcome_without_deltas() -> TestResult {
    run(async {
        for (refusal, outcome) in [
            (
                EquipmentMoveRefusal::SlotMismatch,
                ItemMoveOutcome::SlotMismatch,
            ),
            (
                EquipmentMoveRefusal::RequirementNotMet,
                ItemMoveOutcome::RequirementNotMet,
            ),
            (EquipmentMoveRefusal::NoRoom, ItemMoveOutcome::NoRoom),
            (EquipmentMoveRefusal::Stale, ItemMoveOutcome::Stale),
        ] {
            let shown = worn(&[1, 2], &[(EquipmentSlot::Head, 4)]);
            let mut view = wearing(shown.clone());
            let equip = into_slot(entry(&view, 2), EquipmentSlot::Feet);
            let unequip = into_backpack(entry(&view, 4));
            for payload in [equip, unequip] {
                let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
                    shown.clone(),
                    EquipmentMoveResult::Refused(refusal),
                    None,
                );
                assert_eq!(
                    step(&authority, &mut view, &payload, EQUIP).await,
                    result(outcome)
                );
                assert_eq!(authority.equip_moves.borrow().len(), 1);
            }
        }
        // Results 10 to 12 are never sent without capability 12.
        let shown = worn(&[1], &[(EquipmentSlot::Head, 4)]);
        let mut view = wearing(shown.clone());
        let source = entry(&view, 4);
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
            shown,
            EquipmentMoveResult::Refused(EquipmentMoveRefusal::SlotMismatch),
            None,
        );
        assert_eq!(
            step(&authority, &mut view, &into_backpack(source), CORPSE_ONLY).await,
            result(ItemMoveOutcome::Rejected)
        );
        Ok(())
    })
}

#[test]
fn an_occupied_slot_or_the_main_backpack_never_reaches_the_owner() -> TestResult {
    run(async {
        let shown = worn(&[1, 2], &[(EquipmentSlot::RightHand, 4)]);
        let mut view = wearing(shown.clone());
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
            shown,
            EquipmentMoveResult::Committed,
            None,
        );
        // Swap is not in this slice.
        let into_occupied = into_slot(entry(&view, 2), EquipmentSlot::RightHand);
        assert_eq!(
            step(&authority, &mut view, &into_occupied, EQUIP).await,
            result(ItemMoveOutcome::NotSupported)
        );
        // The main backpack never enters a slot of its own.
        let backpack = into_slot(entry(&view, 0), EquipmentSlot::Armor);
        assert_eq!(
            step(&authority, &mut view, &backpack, EQUIP).await,
            result(ItemMoveOutcome::SlotMismatch)
        );
        // Slot to slot is not a move of this slice.
        let slot_to_slot = into_slot(entry(&view, 4), EquipmentSlot::LeftHand);
        assert_eq!(
            step(&authority, &mut view, &slot_to_slot, EQUIP).await,
            result(ItemMoveOutcome::NotSupported)
        );
        assert!(authority.equip_moves.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn ground_stays_not_supported_under_capability_12() -> TestResult {
    run(async {
        let shown = worn(&[1, 2], &[(EquipmentSlot::Head, 4)]);
        let mut view = wearing(shown.clone());
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
            shown,
            EquipmentMoveResult::Committed,
            None,
        );
        for n in [2, 4] {
            let payload = encode_item_move_intent_for(
                &ItemMoveIntent {
                    source: entry(&view, n),
                    destination: ItemMoveDestination::Ground(
                        oteryn_protocol_oteryn::item_view::WorldTilePosition {
                            x: 10,
                            y: 10,
                            floor: 7,
                        },
                    ),
                },
                EQUIP,
            )
            .map_err(fault)?;
            assert_eq!(
                step(&authority, &mut view, &payload, EQUIP).await,
                result(ItemMoveOutcome::NotSupported)
            );
        }
        assert!(authority.equip_moves.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn the_equipment_replay_answers_moved_for_a_stale_handle_without_a_move() -> TestResult {
    run(async {
        let shown = worn(&[1], &[(EquipmentSlot::RightHand, 2)]);
        let mut view = wearing(shown.clone());
        let mut authority = MoveAuthority::new(Replay::Nothing, Vec::new());
        authority.show(Some(shown));
        authority.equip_replay = Some(EquipmentMoveReplay::Moved);
        assert_eq!(
            step(
                &authority,
                &mut view,
                &into_slot(handle(999), EquipmentSlot::Head),
                EQUIP
            )
            .await,
            result(ItemMoveOutcome::Moved)
        );
        let worn_two = into_backpack(entry(&view, 2));
        authority.equip_replay = Some(EquipmentMoveReplay::Conflicting);
        assert_eq!(
            step(&authority, &mut view, &worn_two, EQUIP).await,
            result(ItemMoveOutcome::Rejected)
        );
        authority.equip_replay = None;
        assert_eq!(
            step(&authority, &mut view, &worn_two, EQUIP).await,
            ItemMoveStep::Disconnect
        );
        assert!(authority.equip_moves.borrow().is_empty());
        Ok(())
    })
}

#[test]
fn an_unknown_equipment_move_or_unreadable_slots_after_it_disconnect() -> TestResult {
    run(async {
        let shown = inventory(&[1, 2]);
        let mut view = wearing(shown.clone());
        let source = entry(&view, 2);
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
            shown.clone(),
            EquipmentMoveResult::Unknown,
            None,
        );
        assert_eq!(
            step(
                &authority,
                &mut view,
                &into_slot(source, EquipmentSlot::Ring),
                EQUIP
            )
            .await,
            ItemMoveStep::Disconnect
        );
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
            shown,
            EquipmentMoveResult::Committed,
            None,
        );
        assert_eq!(
            step(
                &authority,
                &mut view,
                &into_slot(source, EquipmentSlot::Ring),
                EQUIP
            )
            .await,
            ItemMoveStep::Disconnect
        );
        Ok(())
    })
}

// Through the connection, with capabilities 4 and 6 selected.

/// Capability 4 is not offered in production until domain 9 can be read there (CP D738); this
/// offered set is the one that flip will add to.
const ITEM_MOVE_OFFERED: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES,
    },
    OfferedCapability {
        id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
        requires: &[],
    },
];

fn item_move_selected() -> SelectedCapabilities {
    SelectedCapabilities::select(ITEM_MOVE_OFFERED, &[4, 6]).expect("select")
}

fn session(item_view: ItemViewContinuity) -> Result<AdmittedSession, Box<dyn Error>> {
    let world_id = WorldId::decode(&uuid_v7(0x33))?;
    let channel_id = ChannelId::decode(&uuid_v7(0x44))?;
    Ok(AdmittedSession {
        game_session_id: GameSessionId::decode(&uuid_v7(0x22))?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: SessionContinuity {
            selected_capabilities: item_move_selected(),
            item_view,
            ..SessionContinuity::FRESH
        },
        item_fence: None,
    })
}

fn command(id: u64, command_type: u32, payload: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_client_command(
        1,
        &ClientCommandValue {
            command_id: id,
            command_type,
            payload,
        },
    )?)
}

fn move_result(
    sequence: u64,
    id: u64,
    outcome: ItemMoveOutcome,
) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_command_result(
        1,
        sequence,
        id,
        CommandStatus::Accepted,
        &encode_item_move_result(outcome).map_err(fault)?,
    )?)
}

fn delta_frame(sequence: u64, delta: &ItemViewDelta) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(encode_state_delta(
        1,
        sequence,
        delta.domain_id,
        delta.from,
        delta.to,
        delta.delta_type,
        &delta.payload,
    )?)
}

async fn serve(
    authority: &MoveAuthority,
    admitted: AdmittedSession,
    frames: &[Vec<u8>],
) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
    for frame in frames {
        let mut framed = (frame.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(frame);
        client.write_all(&framed).await?;
    }
    client.shutdown().await?;
    let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
        cursor += 4 + length;
    }
    Ok((end, frames))
}

fn ended(end: ConnectionEnd) -> Result<SessionContinuity, Box<dyn Error>> {
    match end {
        ConnectionEnd::AdmittedThenDisconnected(admitted) => Ok(admitted.continuity),
        other => Err(format!("unexpected end {other:?}").into()),
    }
}

#[test]
fn capabilities_4_and_6_open_a_corpse_and_loot_it_by_command_9() -> TestResult {
    run(async {
        assert_eq!(item_move_selected().as_slice(), [4, 6]);
        assert!(item_move_selected().command_selected(COMMAND_TYPE_ITEM_MOVE_INTENT));
        let carried = ItemViewContinuity {
            open_corpse: Some(corpse(7)),
            ..ItemViewContinuity::default()
        };
        let near = observe(HERE, NEAR, &[5, 6]);
        let mut mirror = SessionItemView::resume(carried);
        let domains = mirror
            .snapshot(inventory(&[1]), Some(near.clone()))
            .map_err(fault)?;
        let container = decode_open_container(&domains[1].payload)
            .map_err(fault)?
            .container_handle
            .expect("reopened");
        let (_, opened) = mirror.open(corpse(7), near.clone()).map_err(fault)?;
        let opened = opened.expect("opened");
        let source = entry(&mirror, 5);
        let backpack_delta = mirror
            .inventory_committed(inventory(&[1, 5]))
            .map_err(fault)?
            .expect("domain 9");
        let corpse_delta = mirror
            .open_corpse_committed(Some(observe(HERE, NEAR, &[6])))
            .map_err(fault)?
            .expect("domain 11");

        let authority = MoveAuthority::new(
            Replay::Nothing,
            vec![
                Some(near.clone()),
                Some(near.clone()),
                Some(near),
                Some(observe(HERE, NEAR, &[6])),
            ],
        )
        .taking(
            Ok(ItemTransferOutcome::Committed(committed())),
            Some(inventory(&[1, 5])),
        );
        let (end, frames) = serve(
            &authority,
            session(carried)?,
            &[
                command(
                    1,
                    COMMAND_TYPE_USE_INTENT,
                    &encode_use_item_intent(container),
                )?,
                command(2, COMMAND_TYPE_ITEM_MOVE_INTENT, &into_backpack(source))?,
            ],
        )
        .await?;
        let expected = [
            encode_command_result(
                1,
                1,
                1,
                CommandStatus::Accepted,
                &encode_use_result(UseDisposition::Committed).unwrap_or_default(),
            )?,
            delta_frame(2, &opened)?,
            move_result(3, 2, ItemMoveOutcome::Moved)?,
            delta_frame(4, &corpse_delta)?,
            delta_frame(5, &backpack_delta)?,
        ];
        assert!(frames.len() > expected.len());
        assert_eq!(frames[frames.len() - expected.len()..], expected);
        assert_eq!(*authority.takes.borrow(), [(2, corpse(7), instance(5))]);
        assert_eq!(mirror.table().handle(&instance(5)), Some(source));
        let ended = ended(end)?;
        assert_eq!(ended.next_command_id, 3);
        assert_eq!(ended.item_view, mirror.continuity());
        Ok(())
    })
}

#[test]
fn a_command_that_committed_before_a_reconnect_answers_moved_with_its_stale_handle() -> TestResult {
    run(async {
        let carried = ItemViewContinuity {
            handle_counter: 40,
            open_corpse: Some(corpse(7)),
            ..ItemViewContinuity::default()
        };
        let authority = MoveAuthority::new(
            Replay::Committed,
            vec![
                Some(observe(HERE, NEAR, &[6])),
                Some(observe(HERE, NEAR, &[6])),
            ],
        );
        let (end, frames) = serve(
            &authority,
            session(carried)?,
            &[command(
                1,
                COMMAND_TYPE_ITEM_MOVE_INTENT,
                &into_backpack(handle(40)),
            )?],
        )
        .await?;
        assert_eq!(
            frames.last(),
            Some(&move_result(1, 1, ItemMoveOutcome::Moved)?)
        );
        assert!(authority.takes.borrow().is_empty());
        assert_eq!(ended(end)?.next_command_id, 2);
        Ok(())
    })
}

#[test]
fn an_unknown_outcome_ends_the_connection_before_the_command_id_advances() -> TestResult {
    run(async {
        let carried = ItemViewContinuity {
            open_corpse: Some(corpse(7)),
            ..ItemViewContinuity::default()
        };
        let authority = MoveAuthority::new(
            Replay::Unavailable,
            vec![
                Some(observe(HERE, NEAR, &[5, 6])),
                Some(observe(HERE, NEAR, &[5, 6])),
            ],
        );
        let (end, frames) = serve(
            &authority,
            session(carried)?,
            &[command(
                1,
                COMMAND_TYPE_ITEM_MOVE_INTENT,
                &into_backpack(handle(1)),
            )?],
        )
        .await?;
        assert_eq!(ended(end)?.next_command_id, 1);
        // Only the snapshot was written.
        let (_, snapshot) = serve(&authority, session(carried)?, &[]).await?;
        assert_eq!(frames, snapshot);
        Ok(())
    })
}

/// EQUIP-WIRE-1: capabilities 4, 6 and 12 equip one backpack entry by command 9, and its
/// result and domain 9 delta carry the capability 12 encoding.
#[test]
fn capability_12_equips_a_backpack_entry_by_command_9() -> TestResult {
    run(async {
        const EQUIP_OFFERED: &[OfferedCapability] = &[
            OfferedCapability {
                id: CAPABILITY_ITEM_VIEW_MOVE_V1,
                requires: CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES,
            },
            OfferedCapability {
                id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
                requires: &[],
            },
            OfferedCapability {
                id: CAPABILITY_ITEM_EQUIP_DROP_V1,
                requires: &[CAPABILITY_ITEM_VIEW_MOVE_V1],
            },
        ];
        let selected = SelectedCapabilities::select(EQUIP_OFFERED, &[4, 6, 12]).expect("select");
        assert_eq!(selected.as_slice(), [4, 6, 12]);
        let shown = inventory(&[1, 2]);
        let after = worn(&[1], &[(EquipmentSlot::Armor, 2)]);
        let mut mirror = wearing(shown.clone());
        let source = entry(&mirror, 2);
        let delta = mirror
            .inventory_committed(after.clone())
            .map_err(fault)?
            .expect("domain 9");
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new()).equipping(
            shown,
            EquipmentMoveResult::Committed,
            Some(after),
        );
        let mut admitted = session(ItemViewContinuity::default())?;
        admitted.continuity.selected_capabilities = selected;
        let (end, frames) = serve(
            &authority,
            admitted,
            &[command(
                1,
                COMMAND_TYPE_ITEM_MOVE_INTENT,
                &into_slot(source, EquipmentSlot::Armor),
            )?],
        )
        .await?;
        let expected = [
            encode_command_result(
                1,
                1,
                1,
                CommandStatus::Accepted,
                &encode_item_move_result_with_equip_drop(ItemMoveOutcome::Moved, true)
                    .map_err(fault)?,
            )?,
            delta_frame(2, &delta)?,
        ];
        assert!(frames.len() > expected.len());
        assert_eq!(frames[frames.len() - expected.len()..], expected);
        assert_eq!(
            *authority.equip_moves.borrow(),
            [(
                1,
                EquipmentMove::Equip {
                    item: instance(2),
                    to: EquipmentSlot::Armor
                }
            )]
        );
        let ended = ended(end)?;
        assert_eq!(ended.next_command_id, 2);
        assert_eq!(ended.item_view, mirror.continuity());
        Ok(())
    })
}

// EQUIP-WIRE-1 (#2009 thread 4246911927): domain 9's two custody halves describe one committed
// state.

fn equip_view() -> SessionItemView {
    SessionItemView::resume(ItemViewContinuity::default()).with_equip_drop()
}

fn slots(worn: &[(EquipmentSlot, u16)]) -> Option<Slots> {
    Some(
        worn.iter()
            .map(|(slot, n)| (*slot, item(instance(*n))))
            .collect(),
    )
}

#[test]
fn an_equipment_move_between_the_two_halves_is_read_again() -> TestResult {
    run(async {
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new());
        authority.show(Some(worn(&[1], &[(EquipmentSlot::RightHand, 2)])));
        // Entry 2 was equipped while the backpack was read: the slots changed around it.
        *authority.equipment_reads.borrow_mut() =
            VecDeque::from([slots(&[]), slots(&[(EquipmentSlot::RightHand, 2)])]);
        let shown = observe_inventory(
            &authority,
            actor(),
            use_command(1).game_session_id,
            &equip_view(),
        )
        .await
        .ok_or("a consistent pair")?;
        assert_eq!(shown, worn(&[1], &[(EquipmentSlot::RightHand, 2)]));
        assert_eq!(authority.inventory_reads.get(), 2);
        Ok(())
    })
}

#[test]
fn an_item_in_both_halves_is_never_shown() -> TestResult {
    run(async {
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new());
        // Entry 1 is in the backpack and in a slot on every read: no committed state has both.
        authority.show(Some(worn(&[1], &[(EquipmentSlot::RightHand, 1)])));
        assert_eq!(
            observe_inventory(
                &authority,
                actor(),
                use_command(1).game_session_id,
                &equip_view()
            )
            .await,
            None
        );
        assert_eq!(
            authority.inventory_reads.get(),
            INVENTORY_SNAPSHOT_ATTEMPTS as u32
        );
        Ok(())
    })
}

#[test]
fn without_capability_12_the_backpack_is_read_once_and_no_slot_is_read() -> TestResult {
    run(async {
        let authority = MoveAuthority::new(Replay::Nothing, Vec::new());
        authority.show(Some(worn(&[1], &[(EquipmentSlot::RightHand, 1)])));
        *authority.equipment_reads.borrow_mut() = VecDeque::from([None]);
        let view = SessionItemView::resume(ItemViewContinuity::default());
        let shown = observe_inventory(&authority, actor(), use_command(1).game_session_id, &view)
            .await
            .ok_or("the backpack")?;
        assert_eq!(shown, inventory(&[1]));
        assert_eq!(authority.inventory_reads.get(), 1);
        assert_eq!(authority.equipment_reads.borrow().len(), 1);
        Ok(())
    })
}
