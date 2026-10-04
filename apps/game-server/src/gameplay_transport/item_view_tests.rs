#![allow(clippy::expect_used)]

use super::*;
use crate::durability::item_transfer::{BackpackEntry, InventoryItem};
use oteryn_protocol_oteryn::item_view::{decode_character_inventory, decode_open_container};

const fn at(x: i32, y: i32, floor: i16) -> ActorPosition {
    ActorPosition { x, y, floor }
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
        count: NonZeroU32::new(1).expect("fixture"),
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

#[test]
fn live_handles_are_bounded_at_max_and_refused_at_max_plus_one() {
    let mut table = ItemHandleTable::default();
    let keys: Vec<ItemKey> = (0..=MAX_LIVE_ITEM_HANDLES as u16).map(instance).collect();
    table
        .replace(View::Inventory, &keys[..MAX_LIVE_ITEM_HANDLES])
        .expect("fixture");
    assert_eq!(table.live(), MAX_LIVE_ITEM_HANDLES);
    // One more in another view is refused and changes nothing.
    assert_eq!(
        table.replace(View::Container, &keys[MAX_LIVE_ITEM_HANDLES..]),
        Err(ItemViewError::LimitExceeded)
    );
    assert_eq!(table.live(), MAX_LIVE_ITEM_HANDLES);
    assert_eq!(table.last_issued(), MAX_LIVE_ITEM_HANDLES as u64);
    // An item already live in another view costs no extra handle.
    table.replace(View::Container, &keys[..1]).expect("fixture");
    assert_eq!(table.live(), MAX_LIVE_ITEM_HANDLES);
    assert_eq!(
        table.replace(View::Inventory, &keys),
        Err(ItemViewError::LimitExceeded)
    );
}

#[test]
fn handles_are_monotonic_never_reused_and_stale_once_out_of_every_view() {
    let mut table = ItemHandleTable::default();
    table
        .replace(View::Inventory, &[instance(1), instance(2)])
        .expect("fixture");
    let first = table.handle(&instance(1)).expect("fixture");
    assert_eq!(first, handle(1));
    assert_eq!(table.handle(&instance(2)), Some(handle(2)));
    // In two views the handle is shared; leaving one keeps it live.
    table
        .replace(View::Container, &[instance(1)])
        .expect("fixture");
    table
        .replace(View::Inventory, &[instance(2)])
        .expect("fixture");
    assert_eq!(table.resolve(first), Some(instance(1)));
    // Leaving every view makes it stale; returning issues a new handle.
    table.replace(View::Container, &[]).expect("fixture");
    assert_eq!(table.resolve(first), None);
    table
        .replace(View::Container, &[instance(1)])
        .expect("fixture");
    assert_eq!(table.handle(&instance(1)), Some(handle(3)));
    assert_eq!(table.resolve(first), None);
    // The counter never wraps.
    let mut table = ItemHandleTable::starting_after(u64::MAX);
    assert_eq!(
        table.replace(View::Inventory, &[instance(1)]),
        Err(ItemViewError::Exhausted)
    );
    assert_eq!(table.live(), 0);
}

#[test]
fn domain_nine_shows_the_backpack_from_read_character_backpack_without_internal_ids() {
    let definition = |key: &str| TypedDefinitionRef {
        family: "item".to_owned(),
        production_key: key.to_owned(),
        revision_ref: "r1".to_owned(),
    };
    let stored = |id: u8, key: &str, quantity| InventoryItem {
        item_instance_id: [id; 16],
        definition: definition(key),
        quantity,
    };
    let backpack = CharacterBackpack {
        backpack: stored(1, "backpack", 1),
        entries: vec![
            BackpackEntry {
                item: stored(2, "gold", 7),
                placement_ordinal: 9,
            },
            BackpackEntry {
                item: stored(3, "sword", 1),
                placement_ordinal: 4,
            },
        ],
    };
    let refs = |definition: &TypedDefinitionRef| match definition.production_key.as_str() {
        "backpack" => NonZeroU32::new(2854),
        "gold" => NonZeroU32::new(3031),
        "sword" => NonZeroU32::new(3264),
        _ => None,
    };
    let items = inventory_from_backpack(Some(&backpack), refs).expect("fixture");
    let mut view = SessionItemView::resume(ItemViewContinuity::default());
    let [nine, eleven] = view.snapshot(items, None).expect("fixture");
    assert_eq!(nine.domain_id, STATE_DOMAIN_CHARACTER_INVENTORY);
    assert_eq!(eleven.domain_id, STATE_DOMAIN_OPEN_CONTAINER);
    let shown = decode_character_inventory(&nine.payload).expect("fixture");
    let main = shown.main_backpack.expect("fixture");
    assert_eq!(
        (main.handle, main.item_definition_ref.get()),
        (handle(1), 2854)
    );
    let entries: Vec<_> = shown
        .entries
        .iter()
        .map(|entry| {
            (
                entry.handle.get(),
                entry.item_definition_ref.get(),
                entry.count.get(),
            )
        })
        .collect();
    assert_eq!(entries, [(2, 3031, 7), (3, 3264, 1)]);
    // No instance id, placement key or row reaches the wire.
    for id in 1..=3_u8 {
        assert!(!nine.payload.windows(16).any(|window| window == [id; 16]));
    }
    assert!(
        decode_open_container(&eleven.payload)
            .expect("fixture")
            .entries
            .is_empty()
    );
    // No backpack is an empty view; an unmapped definition or an empty stack fails closed.
    assert_eq!(
        inventory_from_backpack(None, refs),
        Some(InventoryItems::default())
    );
    let mut unmapped = backpack.clone();
    unmapped.entries[0].item.definition = definition("unknown");
    assert_eq!(inventory_from_backpack(Some(&unmapped), refs), None);
    let mut empty = backpack;
    empty.entries[1].item.quantity = 0;
    assert_eq!(inventory_from_backpack(Some(&empty), refs), None);
}

#[test]
fn an_inventory_delta_follows_only_a_change_with_a_monotonic_revision() {
    let mut view = SessionItemView::resume(ItemViewContinuity::default());
    let [nine, _] = view.snapshot(inventory(&[1]), None).expect("fixture");
    assert_eq!(nine.revision, 1);
    assert_eq!(
        view.inventory_committed(inventory(&[1])).expect("fixture"),
        None
    );
    let delta = view
        .inventory_committed(inventory(&[2, 1]))
        .expect("fixture")
        .expect("fixture");
    assert_eq!((delta.domain_id, delta.from, delta.to), (9, 1, 2));
    let shown = decode_character_inventory(&delta.payload).expect("fixture");
    // The kept entry keeps its handle; the new one gets the next.
    let handles: Vec<u64> = shown
        .entries
        .iter()
        .map(|entry| entry.handle.get())
        .collect();
    assert_eq!(handles, [3, 2]);
    let delta = view
        .inventory_committed(inventory(&[]))
        .expect("fixture")
        .expect("fixture");
    assert_eq!((delta.from, delta.to), (2, 3));
    assert_eq!(view.resolve(handle(2)), None);
}

#[test]
fn resume_carries_the_counter_and_revisions_and_every_older_handle_is_stale() {
    let mut first = SessionItemView::resume(ItemViewContinuity::default());
    first.snapshot(inventory(&[1, 2]), None).expect("fixture");
    first
        .inventory_committed(inventory(&[1, 2, 3]))
        .expect("fixture");
    let old = first.table().handle(&instance(1)).expect("fixture");
    let carried = first.continuity();
    assert_eq!(
        (
            carried.handle_counter,
            carried.inventory_revision,
            carried.container_revision
        ),
        (4, 2, 1)
    );
    for continuity in [carried, carried.across_channel_transfer()] {
        let mut next = SessionItemView::resume(continuity);
        // Before the snapshot no handle is live.
        assert_eq!(next.resolve(old), None);
        let [nine, eleven] = next.snapshot(inventory(&[1, 2, 3]), None).expect("fixture");
        assert!(nine.revision > carried.inventory_revision);
        assert!(eleven.revision > carried.container_revision);
        let shown = decode_character_inventory(&nine.payload).expect("fixture");
        assert!(shown.main_backpack.expect("fixture").handle.get() > carried.handle_counter);
        for entry in &shown.entries {
            assert!(entry.handle.get() > carried.handle_counter);
        }
        for old in 1..=carried.handle_counter {
            assert_eq!(next.resolve(handle(old)), None);
        }
    }
}

#[test]
fn spatial_objects_get_handles_and_lose_them_on_leaving() {
    use super::super::world_spatial::{EntityKind, EntityRef};
    let object = |n: u8| WorldSpatialEntity {
        kind: EntityKind::Corpse,
        entity: EntityRef {
            identity: [n; ENTITY_IDENTITY_BYTES],
            generation: 0,
        },
        position: at(10, 10, 7),
        detail: EntityDetail::Object {
            item_definition_ref: 4240,
            quantity: 1,
            item_handle: None,
        },
    };
    let mut view = SessionItemView::resume(ItemViewContinuity::default());
    let mut entities = [object(7), object(8)];
    view.attach_spatial_handles(&mut entities).expect("fixture");
    let handles: Vec<_> = entities
        .iter()
        .map(|entity| match entity.detail {
            EntityDetail::Object { item_handle, .. } => item_handle,
            EntityDetail::Actor { .. } => None,
        })
        .collect();
    assert_eq!(handles, [Some(handle(1)), Some(handle(2))]);
    assert_eq!(view.resolve(handle(1)), Some(corpse(7)));
    let mut entities = [object(8)];
    view.attach_spatial_handles(&mut entities).expect("fixture");
    assert_eq!(view.resolve(handle(1)), None);
    assert_eq!(view.resolve(handle(2)), Some(corpse(8)));
}

#[test]
fn use_opens_a_corpse_in_reach_and_reports_every_disposition() {
    let mut view = SessionItemView::resume(ItemViewContinuity::default());
    view.snapshot(inventory(&[]), None).expect("fixture");
    let here = at(10, 10, 7);
    let (decision, delta) = view
        .open(corpse(7), observe(at(12, 10, 7), here, &[5]))
        .expect("fixture");
    assert_eq!((decision, delta), (OpenDecision::TooFar, None));
    let gone = ItemTargetObservation {
        actor: here,
        target: UseItemTarget::Gone,
    };
    assert_eq!(
        view.open(corpse(7), gone).expect("fixture"),
        (OpenDecision::StaleState, None)
    );
    let other = ItemTargetObservation {
        actor: here,
        target: UseItemTarget::NotACorpse,
    };
    assert_eq!(
        view.open(corpse(7), other).expect("fixture"),
        (OpenDecision::NothingToUse, None)
    );
    let (decision, delta) = view
        .open(corpse(7), observe(at(11, 11, 7), here, &[5, 6]))
        .expect("fixture");
    assert_eq!(decision, OpenDecision::Open);
    let delta = delta.expect("fixture");
    assert_eq!((delta.domain_id, delta.from, delta.to), (11, 1, 2));
    let shown = decode_open_container(&delta.payload).expect("fixture");
    // The corpse's handle names it; its entries get their own.
    let container = shown.container_handle.expect("fixture");
    assert_eq!(view.resolve(container), Some(corpse(7)));
    assert_eq!(shown.entries.len(), 2);
    assert_eq!(view.continuity().open_corpse, Some(corpse(7)));
}

#[test]
fn opening_another_corpse_closes_the_first() {
    let mut view = SessionItemView::resume(ItemViewContinuity::default());
    view.snapshot(inventory(&[]), None).expect("fixture");
    let here = at(10, 10, 7);
    let (_, first) = view
        .open(corpse(7), observe(here, here, &[5]))
        .expect("fixture");
    let first = decode_open_container(&first.expect("fixture").payload).expect("fixture");
    let (_, second) = view
        .open(corpse(8), observe(here, at(11, 10, 7), &[6]))
        .expect("fixture");
    let second = second.expect("fixture");
    assert_eq!((second.from, second.to), (2, 3));
    let shown = decode_open_container(&second.payload).expect("fixture");
    assert_eq!(
        view.resolve(shown.container_handle.expect("fixture")),
        Some(corpse(8))
    );
    // The first corpse and its entries left the view.
    assert_eq!(view.resolve(first.container_handle.expect("fixture")), None);
    assert_eq!(view.resolve(first.entries[0].handle), None);
    assert_eq!(view.continuity().open_corpse, Some(corpse(8)));
}

#[test]
fn every_closing_trigger_empties_domain_eleven() {
    let here = at(10, 10, 7);
    let triggers = [
        CloseTrigger::Moved { to: at(12, 10, 7) },
        CloseTrigger::Moved { to: at(10, 10, 6) },
        CloseTrigger::Teleported,
        CloseTrigger::Died,
        CloseTrigger::LoggedOut,
        CloseTrigger::ChannelTransfer,
        CloseTrigger::Decayed { corpse: corpse(7) },
    ];
    for trigger in triggers {
        let mut view = SessionItemView::resume(ItemViewContinuity::default());
        view.snapshot(inventory(&[]), None).expect("fixture");
        view.open(corpse(7), observe(here, here, &[5]))
            .expect("fixture");
        // A step in reach and another corpse decaying leave it open.
        assert_eq!(
            view.close(&CloseTrigger::Moved { to: at(11, 9, 7) })
                .expect("fixture"),
            None
        );
        assert_eq!(
            view.close(&CloseTrigger::Decayed { corpse: corpse(8) })
                .expect("fixture"),
            None
        );
        let delta = view.close(&trigger).expect("fixture").expect("closes");
        assert_eq!((delta.domain_id, delta.from, delta.to), (11, 2, 3));
        let shown = decode_open_container(&delta.payload).expect("fixture");
        assert_eq!(shown.container_handle, None);
        assert!(shown.entries.is_empty());
        assert_eq!(view.continuity().open_corpse, None);
        // Nothing left to close.
        assert_eq!(view.close(&trigger).expect("fixture"), None);
    }
}

#[test]
fn a_reconnect_reopens_the_corpse_only_while_it_is_in_reach() {
    let here = at(10, 10, 7);
    let mut first = SessionItemView::resume(ItemViewContinuity::default());
    first.snapshot(inventory(&[]), None).expect("fixture");
    first
        .open(corpse(7), observe(here, here, &[5]))
        .expect("fixture");
    let carried = first.continuity();
    assert_eq!(carried.open_corpse, Some(corpse(7)));

    let mut kept = SessionItemView::resume(carried);
    assert_eq!(kept.carried_open_corpse(), Some(corpse(7)));
    let [_, eleven] = kept
        .snapshot(inventory(&[]), Some(observe(at(11, 10, 7), here, &[5])))
        .expect("fixture");
    let shown = decode_open_container(&eleven.payload).expect("fixture");
    assert_eq!(
        kept.resolve(shown.container_handle.expect("fixture")),
        Some(corpse(7))
    );
    assert_eq!(kept.continuity().open_corpse, Some(corpse(7)));

    for reopen in [
        Some(observe(at(13, 10, 7), here, &[5])),
        Some(ItemTargetObservation {
            actor: here,
            target: UseItemTarget::Gone,
        }),
        None,
    ] {
        let mut dropped = SessionItemView::resume(carried);
        let [_, eleven] = dropped.snapshot(inventory(&[]), reopen).expect("fixture");
        let shown = decode_open_container(&eleven.payload).expect("fixture");
        assert_eq!(shown.container_handle, None);
        assert_eq!(dropped.continuity().open_corpse, None);
    }
    // A channel transfer closes it before the next snapshot.
    assert_eq!(carried.across_channel_transfer().open_corpse, None);
}

// The connection path: `serve_admitted` with capability 4 negotiated directly (it stays
// `offered: false` in production).
mod connection {
    use super::super::super::capabilities::{OfferedCapability, SelectedCapabilities};
    use super::super::super::connection::{
        AdmissionRefusal, AdmittedSession, ConnectionEnd, EarnedNotice, FirstEntryOutcome,
        FreshAdmissionAttempt, FreshAdmissionAuthority, IDLE_LIVENESS, SessionContinuity,
        StepOutcome, UseCommand, UseOutcome, serve_admitted,
    };
    use super::super::super::world_object::{
        COMMAND_TYPE_USE_INTENT, SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
        STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition, WorldObjectTarget, encode_use_intent,
        encode_use_item_intent, encode_use_result,
    };
    use super::super::super::world_spatial::{
        CAPABILITY_WORLD_SPATIAL_ENTITIES, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        DELTA_TYPE_WORLD_SPATIAL_V1, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDirection, StepDisposition,
        WorldSpatialObservation, encode_step_intent, encode_step_result, encode_world_spatial,
    };
    use super::*;
    use crate::foundation::{
        ChannelId, CommandStatus, DomainSnapshot, ExactActorRef, GameSessionId, WorldId,
        encode_command_result, encode_single_chunk_snapshot, encode_state_delta,
    };
    use oteryn_protocol_oteryn::item_view::{
        CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES,
    };
    use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;
    use std::error::Error;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    type TestResult = Result<(), Box<dyn Error>>;

    const fn uuid_v7(tag: u8) -> [u8; 16] {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70 | (tag & 0x0f);
        bytes[8] = 0x80 | (tag & 0x3f);
        bytes
    }

    const HERE: ActorPosition = at(10, 10, 7);
    const OFFERED: &[OfferedCapability] = &[
        OfferedCapability {
            id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
            requires: &[],
        },
        OfferedCapability {
            id: CAPABILITY_ITEM_VIEW_MOVE_V1,
            requires: CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES,
        },
    ];

    fn item_view_selected() -> SelectedCapabilities {
        SelectedCapabilities::select(OFFERED, &[CAPABILITY_ITEM_VIEW_MOVE_V1, 6]).expect("select")
    }

    #[test]
    fn capability_4_is_selected_only_with_6() {
        assert_eq!(item_view_selected().as_slice(), [4, 6]);
        let alone =
            SelectedCapabilities::select(OFFERED, &[CAPABILITY_ITEM_VIEW_MOVE_V1]).expect("select");
        assert!(alone.as_slice().is_empty());
    }

    /// The Channel owner as the item view sees it: the backpack, one observation per item
    /// target read, a step and a world-object `USE` that may change the backpack.
    struct ViewAuthority {
        inventory: RefCell<Option<InventoryItems>>,
        inventory_reads: Cell<u32>,
        targets: RefCell<VecDeque<Option<ItemTargetObservation>>>,
        target_reads: RefCell<Vec<ItemKey>>,
        step_to: Option<ActorPosition>,
        after_use: Option<InventoryItems>,
    }

    impl ViewAuthority {
        fn new(inventory: Option<InventoryItems>) -> Self {
            Self {
                inventory: RefCell::new(inventory),
                inventory_reads: Cell::new(0),
                targets: RefCell::new(VecDeque::new()),
                target_reads: RefCell::new(Vec::new()),
                step_to: None,
                after_use: None,
            }
        }

        fn targets(self, targets: Vec<Option<ItemTargetObservation>>) -> Self {
            *self.targets.borrow_mut() = targets.into();
            self
        }
    }

    fn observation(position: ActorPosition) -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: position,
        }
    }

    impl FreshAdmissionAuthority for ViewAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(observation(HERE))
        }

        async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
            StepOutcome {
                disposition: StepDisposition::Moved,
                moved_to: self.step_to.map(observation),
            }
        }

        async fn use_object(
            &self,
            _actor: ExactActorRef,
            _use: UseCommand,
            _target: WorldObjectTarget,
        ) -> UseOutcome {
            if let Some(after) = &self.after_use {
                *self.inventory.borrow_mut() = Some(after.clone());
            }
            UseOutcome {
                disposition: UseDisposition::Committed,
                committed: None,
                earned: EarnedNotice::NoneEarned,
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

        async fn observe_item_target(
            &self,
            _actor: ExactActorRef,
            target: ItemKey,
        ) -> Option<ItemTargetObservation> {
            self.target_reads.borrow_mut().push(target);
            self.targets.borrow_mut().pop_front().flatten()
        }
    }

    fn session(continuity: SessionContinuity) -> Result<AdmittedSession, Box<dyn Error>> {
        let world_id = WorldId::decode(&uuid_v7(0x33))?;
        let channel_id = ChannelId::decode(&uuid_v7(0x44))?;
        Ok(AdmittedSession {
            game_session_id: GameSessionId::decode(&uuid_v7(0x22))?,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity,
            item_fence: None,
        })
    }

    fn selected(item_view: ItemViewContinuity) -> SessionContinuity {
        SessionContinuity {
            selected_capabilities: item_view_selected(),
            item_view,
            ..SessionContinuity::FRESH
        }
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

    fn use_item(id: u64, handle: ItemHandle) -> Result<Vec<u8>, Box<dyn Error>> {
        command(id, COMMAND_TYPE_USE_INTENT, &encode_use_item_intent(handle))
    }

    fn use_result(sequence: u64, id: u64, disposition: UseDisposition) -> Vec<u8> {
        encode_command_result(
            1,
            sequence,
            id,
            CommandStatus::Accepted,
            &encode_use_result(disposition),
        )
        .expect("result")
    }

    fn delta_frame(sequence: u64, delta: &ItemViewDelta) -> Vec<u8> {
        encode_state_delta(
            1,
            sequence,
            delta.domain_id,
            delta.from,
            delta.to,
            delta.delta_type,
            &delta.payload,
        )
        .expect("delta")
    }

    async fn serve(
        authority: &ViewAuthority,
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

    fn run(test: impl std::future::Future<Output = TestResult>) -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(test)
    }

    fn ended(end: ConnectionEnd) -> Result<SessionContinuity, Box<dyn Error>> {
        match end {
            ConnectionEnd::AdmittedThenDisconnected(admitted) => Ok(admitted.continuity),
            other => Err(format!("unexpected end {other:?}").into()),
        }
    }

    /// The join snapshot the connection must send: domain 1, the empty overlay, then 9 and 11
    /// from `mirror`.
    fn snapshot(item_domains: Option<&[ItemViewSnapshotDomain; 2]>) -> Vec<Vec<u8>> {
        let spatial = encode_world_spatial(&observation(HERE));
        let mut domains = vec![
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 0,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &[],
            },
        ];
        for domain in item_domains.into_iter().flatten() {
            domains.push(DomainSnapshot {
                domain_id: domain.domain_id,
                revision: domain.revision,
                snapshot_type: domain.snapshot_type,
                payload: &domain.payload,
            });
        }
        encode_single_chunk_snapshot(1, 1, 0, &domains)
            .expect("snapshot")
            .into()
    }

    #[test]
    fn the_snapshot_carries_domains_9_and_11_only_with_capability_4() -> TestResult {
        run(async {
            // Without capability 4 the backpack is never read.
            let authority = ViewAuthority::new(Some(inventory(&[1, 2])));
            let (_, frames) = serve(&authority, session(SessionContinuity::FRESH)?, &[]).await?;
            assert_eq!(frames, snapshot(None));
            assert_eq!(authority.inventory_reads.get(), 0);

            let (end, frames) = serve(
                &authority,
                session(selected(ItemViewContinuity::default()))?,
                &[],
            )
            .await?;
            let mut mirror = SessionItemView::resume(ItemViewContinuity::default());
            let domains = mirror.snapshot(inventory(&[1, 2]), None).expect("snapshot");
            assert_eq!(frames, snapshot(Some(&domains)));
            assert_eq!(ended(end)?.item_view, mirror.continuity());
            assert_eq!(mirror.continuity().handle_counter, 3);

            // An unreadable backpack fails closed before anything is sent.
            let authority = ViewAuthority::new(None);
            let (end, frames) = serve(
                &authority,
                session(selected(ItemViewContinuity::default()))?,
                &[],
            )
            .await?;
            assert!(frames.is_empty());
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            Ok(())
        })
    }

    #[test]
    fn use_with_an_item_target_maps_every_disposition_and_writes_nothing() -> TestResult {
        run(async {
            let carried = ItemViewContinuity {
                handle_counter: 40,
                inventory_revision: 3,
                container_revision: 2,
                open_corpse: Some(corpse(7)),
                views_revision: 0,
            };
            let near = observe(HERE, at(11, 10, 7), &[5]);
            let far = observe(at(13, 10, 7), at(11, 10, 7), &[5]);
            let other = ItemTargetObservation {
                actor: HERE,
                target: UseItemTarget::NotACorpse,
            };
            let mut mirror = SessionItemView::resume(carried);
            let domains = mirror
                .snapshot(inventory(&[1]), Some(near.clone()))
                .expect("snapshot");
            let container = decode_open_container(&domains[1].payload)
                .expect("decode")
                .container_handle
                .expect("reopened");
            let backpack = mirror.table().handle(&instance(0)).expect("backpack");
            assert!(container.get() > 40 && backpack.get() > 40);

            let authority = ViewAuthority::new(Some(inventory(&[1]))).targets(vec![
                Some(near.clone()),
                Some(near.clone()),
                Some(far),
                Some(other),
                None,
            ]);
            let (end, frames) = serve(
                &authority,
                session(selected(carried))?,
                &[
                    // A handle from before this snapshot is stale without any read.
                    use_item(1, handle(40))?,
                    use_item(2, container)?,
                    use_item(3, container)?,
                    use_item(4, backpack)?,
                    use_item(5, backpack)?,
                ],
            )
            .await?;
            let (_, opened) = mirror.open(corpse(7), near).expect("open");
            let opened = opened.expect("opened");
            let mut expected = snapshot(Some(&domains));
            expected.extend([
                use_result(1, 1, UseDisposition::StaleState),
                use_result(2, 2, UseDisposition::Committed),
                delta_frame(3, &opened),
                use_result(4, 3, UseDisposition::TooFar),
                use_result(5, 4, UseDisposition::NothingToUse),
                use_result(6, 5, UseDisposition::StaleState),
            ]);
            assert_eq!(frames, expected);
            assert_eq!(
                *authority.target_reads.borrow(),
                [corpse(7), corpse(7), corpse(7), instance(0), instance(0)]
            );
            let ended = ended(end)?;
            assert_eq!(ended.item_view, mirror.continuity());
            assert_eq!(ended.item_view.container_revision, 4);
            Ok(())
        })
    }

    #[test]
    fn a_step_out_of_reach_closes_the_open_corpse() -> TestResult {
        run(async {
            let carried = ItemViewContinuity {
                open_corpse: Some(corpse(7)),
                ..ItemViewContinuity::default()
            };
            let near = observe(HERE, at(11, 10, 7), &[5]);
            let mut authority =
                ViewAuthority::new(Some(inventory(&[]))).targets(vec![Some(near.clone())]);
            authority.step_to = Some(at(9, 10, 7));
            let (end, frames) = serve(
                &authority,
                session(selected(carried))?,
                &[command(
                    1,
                    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                    &encode_step_intent(StepDirection::West),
                )?],
            )
            .await?;
            let mut mirror = SessionItemView::resume(carried);
            let domains = mirror
                .snapshot(inventory(&[]), Some(near))
                .expect("snapshot");
            let closed = mirror
                .close(&CloseTrigger::Moved { to: at(9, 10, 7) })
                .expect("close")
                .expect("closed");
            let mut expected = snapshot(Some(&domains));
            expected.extend([
                encode_command_result(
                    1,
                    1,
                    1,
                    CommandStatus::Accepted,
                    &encode_step_result(StepDisposition::Moved),
                )?,
                encode_state_delta(
                    1,
                    2,
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    1,
                    2,
                    DELTA_TYPE_WORLD_SPATIAL_V1,
                    &encode_world_spatial(&observation(at(9, 10, 7))),
                )?,
                delta_frame(3, &closed),
            ]);
            assert_eq!(frames, expected);
            assert_eq!(ended(end)?.item_view.open_corpse, None);
            Ok(())
        })
    }

    #[test]
    fn a_committed_use_sends_the_inventory_delta_only_when_the_backpack_changed() -> TestResult {
        run(async {
            let mut authority = ViewAuthority::new(Some(inventory(&[1])));
            authority.after_use = Some(inventory(&[2, 1]));
            let chest = encode_use_intent(&WorldObjectTarget {
                placement: vec![1, 2, 3],
                expected_revision: 0,
            })
            .expect("intent");
            let (_, frames) = serve(
                &authority,
                session(selected(ItemViewContinuity::default()))?,
                &[
                    command(1, COMMAND_TYPE_USE_INTENT, &chest)?,
                    command(2, COMMAND_TYPE_USE_INTENT, &chest)?,
                ],
            )
            .await?;
            let mut mirror = SessionItemView::resume(ItemViewContinuity::default());
            let domains = mirror.snapshot(inventory(&[1]), None).expect("snapshot");
            let changed = mirror
                .inventory_committed(inventory(&[2, 1]))
                .expect("commit")
                .expect("changed");
            let mut expected = snapshot(Some(&domains));
            expected.extend([
                use_result(1, 1, UseDisposition::Committed),
                delta_frame(2, &changed),
                use_result(3, 2, UseDisposition::Committed),
            ]);
            assert_eq!(frames, expected);
            Ok(())
        })
    }

    #[test]
    fn a_resumed_connection_reissues_handles_and_every_older_one_is_stale() -> TestResult {
        run(async {
            let other = ItemTargetObservation {
                actor: HERE,
                target: UseItemTarget::NotACorpse,
            };
            let authority = ViewAuthority::new(Some(inventory(&[1])))
                .targets(vec![Some(other.clone()), Some(other)]);
            let (end, _) = serve(
                &authority,
                session(selected(ItemViewContinuity::default()))?,
                &[use_item(1, handle(1))?],
            )
            .await?;
            let lost = ended(end)?;
            assert_eq!(lost.item_view.handle_counter, 2);
            // The resumed connection carries the counter and revisions; the old handle 1 no
            // longer names the backpack, so it is stale without any read.
            let resumed = SessionContinuity {
                connection_generation: 1,
                next_command_id: 1,
                server_sequence: 0,
                ..lost
            };
            let (end, frames) =
                serve(&authority, session(resumed)?, &[use_item(1, handle(1))?]).await?;
            let mut mirror = SessionItemView::resume(lost.item_view);
            let domains = mirror.snapshot(inventory(&[1]), None).expect("snapshot");
            assert!(domains[0].revision > lost.item_view.inventory_revision);
            let mut expected = snapshot(Some(&domains));
            expected.push(use_result(1, 1, UseDisposition::StaleState));
            assert_eq!(frames, expected);
            assert_eq!(authority.target_reads.borrow().len(), 1);
            assert_eq!(ended(end)?.item_view.handle_counter, 4);
            Ok(())
        })
    }
}
