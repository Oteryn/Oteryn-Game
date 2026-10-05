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
    CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES,
    COMMAND_TYPE_ITEM_MOVE_INTENT, EquipmentSlot, ItemHandle, ItemMoveIntent,
    decode_open_container, encode_item_move_intent, encode_item_move_intent_for,
    encode_item_move_result,
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
        }
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
        self.inventory.borrow().clone()
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
fn a_committed_take_answers_moved_then_the_corpse_and_backpack_deltas() -> TestResult {
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
        let corpse_delta = mirror
            .open_corpse_committed(Some(observe(HERE, NEAR, &[6])))
            .map_err(fault)?
            .expect("domain 11");
        let backpack_delta = mirror
            .inventory_committed(inventory(&[1, 5]))
            .map_err(fault)?
            .expect("domain 9");
        assert_eq!(
            moved,
            ItemMoveStep::Result(ItemMoveOutcome::Moved, vec![corpse_delta, backpack_delta])
        );
        assert_eq!(*authority.takes.borrow(), [(1, corpse(7), instance(5))]);
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
        let corpse_delta = mirror
            .open_corpse_committed(Some(observe(HERE, NEAR, &[6])))
            .map_err(fault)?
            .expect("domain 11");
        let backpack_delta = mirror
            .inventory_committed(inventory(&[1, 5]))
            .map_err(fault)?
            .expect("domain 9");

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
                &encode_use_result(UseDisposition::Committed),
            )?,
            delta_frame(2, &opened)?,
            move_result(3, 2, ItemMoveOutcome::Moved)?,
            delta_frame(4, &corpse_delta)?,
            delta_frame(5, &backpack_delta)?,
        ];
        assert!(frames.len() > expected.len());
        assert_eq!(frames[frames.len() - expected.len()..], expected);
        assert_eq!(*authority.takes.borrow(), [(2, corpse(7), instance(5))]);
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
