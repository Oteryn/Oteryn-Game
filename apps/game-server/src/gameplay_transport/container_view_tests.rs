#![allow(clippy::expect_used)]

use super::super::item_view::{InventoryItems, ItemViewContinuity, View};
use super::*;
use oteryn_protocol_oteryn::container_tree::{
    DELTA_TYPE_CONTAINER_VIEWS_V1, MAX_CONTAINER_VIEW_ENTRIES,
    MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE, STATE_DOMAIN_CONTAINER_VIEWS, decode_container_views,
};
use oteryn_protocol_oteryn::item_view::{ItemHandle, MAX_LIVE_ITEM_HANDLES};
use std::num::NonZeroU32;

fn instance(n: u16) -> ItemKey {
    let mut id = [0x5A; 16];
    id[..2].copy_from_slice(&n.to_be_bytes());
    ItemKey::Instance(id)
}

fn item(key: ItemKey) -> ViewItem {
    ViewItem {
        key,
        item_definition_ref: NonZeroU32::new(2854).expect("fixture"),
        count: NonZeroU32::new(1).expect("fixture"),
        sub_type: 0,
    }
}

/// The main backpack `instance(0)` holding `instance(1..=entries)`.
fn inventory(entries: u16) -> InventoryItems {
    InventoryItems {
        main_backpack: Some(item(instance(0))),
        entries: (1..=entries).map(|n| item(instance(n))).collect(),
    }
}

fn container(parent: Option<ItemKey>, entries: &[u16]) -> ContainerObservation {
    ContainerObservation::Container {
        parent,
        capacity: MAX_CONTAINER_VIEW_ENTRIES as u8,
        entries: entries.iter().map(|n| item(instance(*n))).collect(),
    }
}

/// A capability 14 view after its join snapshot: the backpack and `entries` have handles.
fn session(entries: u16) -> SessionItemView {
    let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_container_tree();
    view.snapshot(inventory(entries), None).expect("snapshot");
    view.views_snapshot().expect("views snapshot");
    view
}

fn handle_of(view: &SessionItemView, key: ItemKey) -> ItemHandle {
    view.table().handle(&key).expect("live handle")
}

/// Plans and applies one command with the given observation.
fn run(
    state: &mut ContainerViewState,
    view: &mut SessionItemView,
    intent: ContainerViewIntent,
    observation: Option<ContainerObservation>,
) -> (ContainerViewOutcome, Option<ItemViewDelta>) {
    match state.plan(view, intent).expect("plan") {
        ContainerPlan::Decided(outcome, delta) => (outcome, delta),
        ContainerPlan::Observe(pending) => state.apply(view, pending, observation).expect("apply"),
    }
}

fn open(handle: ItemHandle) -> ContainerViewIntent {
    ContainerViewIntent::Open {
        handle,
        replace_view: None,
    }
}

fn shown(delta: &ItemViewDelta) -> ContainerViews {
    assert_eq!(delta.domain_id, STATE_DOMAIN_CONTAINER_VIEWS);
    assert_eq!(delta.delta_type, DELTA_TYPE_CONTAINER_VIEWS_V1);
    assert_eq!(delta.to, delta.from + 1);
    decode_container_views(&delta.payload).expect("domain 14 payload")
}

#[test]
fn the_join_snapshot_has_no_open_view_above_every_seen_revision() {
    let mut view = SessionItemView::resume(ItemViewContinuity {
        views_revision: 6,
        ..ItemViewContinuity::default()
    })
    .with_container_tree();
    let snapshot = view.views_snapshot().expect("snapshot");
    assert_eq!(snapshot.domain_id, STATE_DOMAIN_CONTAINER_VIEWS);
    assert_eq!(snapshot.revision, 7);
    assert!(snapshot.payload.is_empty());
    assert_eq!(view.continuity().views_revision, 7);
    // A channel transfer carries the revision.
    assert_eq!(
        view.continuity().across_channel_transfer().views_revision,
        7
    );
}

#[test]
fn open_close_and_up_show_whole_views_with_inner_handles() {
    let mut view = session(2);
    let mut state = ContainerViewState::default();
    let bag = instance(1);

    // Open the backpack entry `bag`: its entries get handles from the session's table.
    let intent = open(handle_of(&view, bag));
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        intent,
        Some(container(Some(instance(0)), &[10, 11])),
    );
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let views = shown(&delta.expect("delta"));
    assert_eq!(views.views.len(), 1);
    let first = &views.views[0];
    assert_eq!(first.view_id, 0);
    assert_eq!(first.container_handle, handle_of(&view, bag));
    assert_eq!(first.parent_handle, Some(handle_of(&view, instance(0))));
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.handle)
            .collect::<Vec<_>>(),
        [
            handle_of(&view, instance(10)),
            handle_of(&view, instance(11))
        ]
    );

    // Open the inner bag `instance(10)` in place of view 0, then go up to `bag` again.
    let inner = handle_of(&view, instance(10));
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        ContainerViewIntent::Open {
            handle: inner,
            replace_view: Some(0),
        },
        Some(container(Some(bag), &[20])),
    );
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let views = shown(&delta.expect("delta"));
    assert_eq!(views.views[0].container_handle, inner);
    assert_eq!(views.views[0].parent_handle, Some(handle_of(&view, bag)));
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        ContainerViewIntent::Up { view_id: 0 },
        Some(container(Some(instance(0)), &[10, 11])),
    );
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let views = shown(&delta.expect("delta"));
    assert_eq!(views.views[0].container_handle, handle_of(&view, bag));
    // `instance(10)` stayed in domain 14 throughout, so it keeps its handle; `instance(20)` left
    // every view and its handle is STALE.
    assert_eq!(views.views[0].entries[0].handle, inner);
    assert_eq!(view.resolve(inner), Some(instance(10)));
    assert_eq!(view.table().handle(&instance(20)), None);

    // Close it: an empty domain 14. A second close is STALE and changes nothing.
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        ContainerViewIntent::Close { view_id: 0 },
        None,
    );
    assert_eq!(outcome, ContainerViewOutcome::Closed);
    assert_eq!(shown(&delta.expect("delta")), ContainerViews::default());
    assert_eq!(state.open_views(), 0);
    let revision = view.continuity().views_revision;
    assert_eq!(
        run(
            &mut state,
            &mut view,
            ContainerViewIntent::Close { view_id: 0 },
            None
        ),
        (ContainerViewOutcome::Stale, None)
    );
    assert_eq!(view.continuity().views_revision, revision);
}

#[test]
fn every_refusal_leaves_the_views_unchanged() {
    let mut view = session(2);
    let mut state = ContainerViewState::default();
    let bag = handle_of(&view, instance(1));
    let revision = view.continuity().views_revision;
    for (intent, observation, outcome) in [
        // A handle that is not live, or a view that is not open.
        (
            open(ItemHandle::new(999).expect("fixture")),
            None,
            ContainerViewOutcome::Stale,
        ),
        (
            ContainerViewIntent::Open {
                handle: bag,
                replace_view: Some(3),
            },
            None,
            ContainerViewOutcome::Stale,
        ),
        (
            ContainerViewIntent::Up { view_id: 0 },
            None,
            ContainerViewOutcome::Stale,
        ),
        // The Channel owner's observation.
        (open(bag), None, ContainerViewOutcome::Stale),
        (
            open(bag),
            Some(ContainerObservation::Gone),
            ContainerViewOutcome::Stale,
        ),
        (
            open(bag),
            Some(ContainerObservation::NotAContainer),
            ContainerViewOutcome::NotAContainer,
        ),
        (
            open(bag),
            Some(ContainerObservation::TooFar),
            ContainerViewOutcome::TooFar,
        ),
        // An observation that does not encode (more entries than its capacity).
        (
            open(bag),
            Some(ContainerObservation::Container {
                parent: None,
                capacity: 1,
                entries: vec![item(instance(10)), item(instance(11))],
            }),
            ContainerViewOutcome::Stale,
        ),
    ] {
        assert_eq!(
            run(&mut state, &mut view, intent, observation),
            (outcome, None),
            "{intent:?}"
        );
        assert_eq!(state.open_views(), 0);
        assert_eq!(view.continuity().views_revision, revision);
    }

    // Up from a view whose parent is not visible is STALE.
    let (outcome, _) = run(
        &mut state,
        &mut view,
        open(bag),
        Some(container(Some(instance(500)), &[])),
    );
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    assert_eq!(
        run(
            &mut state,
            &mut view,
            ContainerViewIntent::Up { view_id: 0 },
            None
        ),
        (ContainerViewOutcome::Stale, None)
    );
}

#[test]
fn a_container_shows_in_one_view_only() {
    let mut view = session(2);
    let mut state = ContainerViewState::default();
    let (first, second) = (handle_of(&view, instance(1)), handle_of(&view, instance(2)));
    run(
        &mut state,
        &mut view,
        open(first),
        Some(container(None, &[10])),
    );
    run(
        &mut state,
        &mut view,
        open(second),
        Some(container(None, &[20])),
    );
    assert_eq!(state.open_views(), 2);
    // Opening `first` again refreshes view 0 in place.
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        open(first),
        Some(container(None, &[11])),
    );
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let views = shown(&delta.expect("delta"));
    assert_eq!(
        views
            .views
            .iter()
            .map(|view| (view.view_id, view.container_handle))
            .collect::<Vec<_>>(),
        [(0, first), (1, second)]
    );
    // Opening `first` in place of view 1 moves it there and closes view 0.
    let (_, delta) = run(
        &mut state,
        &mut view,
        ContainerViewIntent::Open {
            handle: first,
            replace_view: Some(1),
        },
        Some(container(None, &[11])),
    );
    let views = shown(&delta.expect("delta"));
    assert_eq!(views.views.len(), 1);
    assert_eq!(
        (views.views[0].view_id, views.views[0].container_handle),
        (1, first)
    );
}

#[test]
fn a_seventeenth_open_is_too_many_views() {
    let mut view = session(MAX_CONTAINER_VIEWS as u16 + 1);
    let mut state = ContainerViewState::default();
    // max = 16 views, each with 20 entries.
    for n in 1..=MAX_CONTAINER_VIEWS as u16 {
        let contents: Vec<u16> = (0..MAX_CONTAINER_VIEW_ENTRIES as u16)
            .map(|entry| 1000 + n * 100 + entry)
            .collect();
        let intent = open(handle_of(&view, instance(n)));
        let (outcome, delta) = run(
            &mut state,
            &mut view,
            intent,
            Some(container(None, &contents)),
        );
        assert_eq!(outcome, ContainerViewOutcome::Opened);
        let views = shown(&delta.expect("delta"));
        assert_eq!(views.views.len(), usize::from(n));
        assert_eq!(views.views.last().expect("view").view_id, (n - 1) as u8);
    }
    // max + 1 is refused and changes nothing.
    let revision = view.continuity().views_revision;
    let seventeenth = handle_of(&view, instance(MAX_CONTAINER_VIEWS as u16 + 1));
    assert_eq!(
        run(
            &mut state,
            &mut view,
            open(seventeenth),
            Some(container(None, &[]))
        ),
        (ContainerViewOutcome::TooManyViews, None)
    );
    assert_eq!(state.open_views(), MAX_CONTAINER_VIEWS);
    assert_eq!(view.continuity().views_revision, revision);
    // Closing one frees its id for the next open.
    run(
        &mut state,
        &mut view,
        ContainerViewIntent::Close { view_id: 4 },
        None,
    );
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        open(seventeenth),
        Some(container(None, &[])),
    );
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let views = shown(&delta.expect("delta"));
    assert_eq!(views.views[4].container_handle, seventeenth);
}

#[test]
fn view_commands_are_limited_to_10_per_second() {
    let mut state = ContainerViewState::default();
    let start = Instant::now();
    // max = 10 within one second.
    for n in 0..MAX_VIEW_COMMANDS_PER_WINDOW as u64 {
        assert!(state.admit(start + Duration::from_millis(n * 10)));
    }
    // max + 1 within the window is refused, up to its last instant.
    assert!(!state.admit(start + Duration::from_millis(100)));
    assert!(!state.admit(start + VIEW_COMMAND_WINDOW - Duration::from_millis(1)));
    // The window slides: the first command leaves it after one second, freeing one place.
    assert!(state.admit(start + VIEW_COMMAND_WINDOW));
    assert!(!state.admit(start + VIEW_COMMAND_WINDOW));
    assert!(state.admit(start + VIEW_COMMAND_WINDOW + Duration::from_millis(10)));
}

#[test]
fn the_container_tree_bound_holds_637_handles_and_refuses_638() {
    let keys: Vec<ItemKey> = (0..=MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE as u16)
        .map(instance)
        .collect();
    // ITEMV0-RL-03 for the capability 4 views, plus 16 views of a container and 20 entries.
    let (base, tree) = keys.split_at(MAX_LIVE_ITEM_HANDLES);
    let mut table = ItemHandleTable::default().with_container_tree();
    assert_eq!(table.limit(), 637);
    table.replace(View::Spatial, base).expect("ITEMV0-RL-03");
    let max = &tree[..MAX_LIVE_ITEM_HANDLES_CONTAINER_TREE - MAX_LIVE_ITEM_HANDLES];
    table.replace(View::Tree, max).expect("max = 637");
    assert_eq!(table.live(), 637);
    assert_eq!(
        table.replace(View::Tree, tree),
        Err(ItemViewError::LimitExceeded)
    );
    assert_eq!(table.live(), 637);
    // Without capability 14 the bound stays ITEMV0-RL-03.
    assert_eq!(ItemHandleTable::default().limit(), MAX_LIVE_ITEM_HANDLES);
}

mod connection {
    use super::super::super::capabilities::{OfferedCapability, SelectedCapabilities};
    use super::super::super::connection::{
        AdmissionRefusal, AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAttempt,
        FreshAdmissionAuthority, IDLE_LIVENESS, SessionContinuity, StepOutcome, UseCommand,
        UseOutcome, serve_admitted,
    };
    use super::super::super::item_view::ItemViewSnapshotDomain;
    use super::super::super::world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, encode_use_item_intent,
        encode_use_result,
    };
    use super::super::super::world_spatial::{
        CAPABILITY_WORLD_SPATIAL_ENTITIES, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
        STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDirection, WorldSpatialObservation,
        encode_world_spatial,
    };
    use super::*;
    use crate::foundation::{
        ChannelId, CommandStatus, DomainSnapshot, ExactActorRef, GameSessionId, WorldId,
        encode_command_result, encode_single_chunk_snapshot, encode_state_delta,
    };
    use crate::gameplay_transport::world_spatial::ActorPosition;
    use oteryn_protocol_oteryn::container_tree::{
        CAPABILITY_CONTAINER_TREE_V1, CAPABILITY_CONTAINER_TREE_V1_REQUIRES,
        COMMAND_TYPE_CONTAINER_VIEW_INTENT, encode_container_view_intent,
        encode_container_view_result,
    };
    use oteryn_protocol_oteryn::item_view::{
        CAPABILITY_ITEM_EQUIP_DROP_V1, CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES,
        CAPABILITY_ITEM_VIEW_MOVE_V1, CAPABILITY_ITEM_VIEW_MOVE_V1_REQUIRES,
    };
    use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
    use std::cell::RefCell;
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

    const HERE: ActorPosition = ActorPosition {
        x: 10,
        y: 10,
        floor: 7,
    };
    const OFFERED: &[OfferedCapability] = &[
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
            requires: CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES,
        },
        OfferedCapability {
            id: CAPABILITY_CONTAINER_TREE_V1,
            requires: CAPABILITY_CONTAINER_TREE_V1_REQUIRES,
        },
    ];

    fn select(supported: &[u32]) -> SelectedCapabilities {
        SelectedCapabilities::select(OFFERED, supported).expect("select")
    }

    #[test]
    fn capability_14_is_selected_only_with_4_and_12() {
        assert_eq!(select(&[4, 6, 12, 14]).as_slice(), [4, 6, 12, 14]);
        assert_eq!(select(&[4, 6, 14]).as_slice(), [4, 6]);
        assert_eq!(select(&[6, 12, 14]).as_slice(), [6]);
    }

    /// The Channel owner as the container views see it: the backpack and one observation per
    /// container read.
    struct TreeAuthority {
        containers: RefCell<VecDeque<Option<ContainerObservation>>>,
        container_reads: RefCell<Vec<ItemKey>>,
    }

    impl TreeAuthority {
        fn new(containers: Vec<Option<ContainerObservation>>) -> Self {
            Self {
                containers: RefCell::new(containers.into()),
                container_reads: RefCell::new(Vec::new()),
            }
        }
    }

    fn observation() -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: HERE,
        }
    }

    impl FreshAdmissionAuthority for TreeAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(observation())
        }

        async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
            StepOutcome::rejected()
        }

        async fn use_object(
            &self,
            _actor: ExactActorRef,
            _use: UseCommand,
            _target: WorldObjectTarget,
        ) -> UseOutcome {
            UseOutcome::rejected()
        }

        async fn observe_character_inventory(
            &self,
            _actor: ExactActorRef,
            _game_session_id: GameSessionId,
        ) -> Option<InventoryItems> {
            Some(inventory(2))
        }

        async fn observe_container(
            &self,
            _actor: ExactActorRef,
            target: ItemKey,
        ) -> Option<ContainerObservation> {
            self.container_reads.borrow_mut().push(target);
            self.containers.borrow_mut().pop_front().flatten()
        }
    }

    fn session(supported: &[u32]) -> Result<AdmittedSession, Box<dyn Error>> {
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
                selected_capabilities: select(supported),
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

    fn view_command(id: u64, intent: ContainerViewIntent) -> Result<Vec<u8>, Box<dyn Error>> {
        command(
            id,
            COMMAND_TYPE_CONTAINER_VIEW_INTENT,
            &encode_container_view_intent(&intent).map_err(|error| format!("{error:?}"))?,
        )
    }

    fn result(sequence: u64, id: u64, status: CommandStatus, payload: &[u8]) -> Vec<u8> {
        encode_command_result(1, sequence, id, status, payload).expect("result")
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
        authority: &TreeAuthority,
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

    /// The join snapshot: domain 1, then 9 and 11, then 14 when given.
    fn snapshot(
        items: &[ItemViewSnapshotDomain; 2],
        views: Option<&ItemViewSnapshotDomain>,
    ) -> Vec<Vec<u8>> {
        let spatial = encode_world_spatial(&observation());
        let mut domains = vec![DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 1,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &spatial,
        }];
        for domain in items.iter().chain(views) {
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

    /// The server's view after the join snapshot, mirrored.
    fn mirror(tree: bool) -> (SessionItemView, Vec<Vec<u8>>) {
        let mut view = SessionItemView::resume(ItemViewContinuity::default());
        if tree {
            view = view.with_container_tree();
        }
        let items = view.snapshot(inventory(2), None).expect("snapshot");
        let views = tree.then(|| view.views_snapshot().expect("views"));
        let frames = snapshot(&items, views.as_ref());
        (view, frames)
    }

    #[test]
    fn domain_14_and_command_21_need_capability_14() -> TestResult {
        run(async {
            // Capabilities 4 and 12 alone: no domain 14, and command 21 is refused before any
            // authority call.
            let authority = TreeAuthority::new(vec![Some(container(None, &[]))]);
            let close = ContainerViewIntent::Close { view_id: 0 };
            let (_, frames) = serve(
                &authority,
                session(&[4, 6, 12])?,
                &[view_command(1, close)?],
            )
            .await?;
            let (_, mut expected) = mirror(false);
            expected.push(result(1, 1, CommandStatus::Rejected, &[]));
            assert_eq!(frames, expected);
            assert!(authority.container_reads.borrow().is_empty());

            // With capability 14 the join snapshot carries an empty domain 14.
            let authority = TreeAuthority::new(Vec::new());
            let (end, frames) = serve(&authority, session(&[4, 6, 12, 14])?, &[]).await?;
            let (view, expected) = mirror(true);
            assert_eq!(frames, expected);
            assert_eq!(ended(end)?.item_view, view.continuity());
            assert_eq!(view.continuity().views_revision, 1);
            Ok(())
        })
    }

    #[test]
    fn command_21_opens_and_closes_with_whole_view_deltas() -> TestResult {
        run(async {
            let authority = TreeAuthority::new(vec![
                Some(container(Some(instance(0)), &[10])),
                Some(ContainerObservation::TooFar),
            ]);
            let (mut view, mut expected) = mirror(true);
            let bag = handle_of(&view, instance(1));
            let frames = [
                view_command(1, open(bag))?,
                view_command(2, open(handle_of(&view, instance(2))))?,
                view_command(3, ContainerViewIntent::Close { view_id: 0 })?,
                // Malformed: REJECTED with an empty payload.
                command(4, COMMAND_TYPE_CONTAINER_VIEW_INTENT, &[0x22, 0x00])?,
            ];
            let (end, output) = serve(&authority, session(&[4, 6, 12, 14])?, &frames).await?;

            let mut state = ContainerViewState::default();
            let (_, opened) = super::run(
                &mut state,
                &mut view,
                open(bag),
                Some(container(Some(instance(0)), &[10])),
            );
            let opened = opened.expect("delta");
            let (_, closed) = super::run(
                &mut state,
                &mut view,
                ContainerViewIntent::Close { view_id: 0 },
                None,
            );
            let closed = closed.expect("delta");
            let accepted = |outcome| encode_container_view_result(outcome);
            expected.extend([
                result(
                    1,
                    1,
                    CommandStatus::Accepted,
                    &accepted(ContainerViewOutcome::Opened),
                ),
                delta_frame(2, &opened),
                result(
                    3,
                    2,
                    CommandStatus::Accepted,
                    &accepted(ContainerViewOutcome::TooFar),
                ),
                result(
                    4,
                    3,
                    CommandStatus::Accepted,
                    &accepted(ContainerViewOutcome::Closed),
                ),
                delta_frame(5, &closed),
                result(6, 4, CommandStatus::Rejected, &[]),
            ]);
            assert_eq!(output, expected);
            assert_eq!(
                *authority.container_reads.borrow(),
                [instance(1), instance(2)]
            );
            assert_eq!(ended(end)?.item_view, view.continuity());
            Ok(())
        })
    }

    #[test]
    fn view_commands_over_10_per_second_are_rejected_before_decoding() -> TestResult {
        run(async {
            let authority = TreeAuthority::new(Vec::new());
            let close = ContainerViewIntent::Close { view_id: 0 };
            let frames: Vec<Vec<u8>> = (1..=11)
                .map(|id| view_command(id, close))
                .collect::<Result<_, _>>()?;
            let (_, output) = serve(&authority, session(&[4, 6, 12, 14])?, &frames).await?;
            let (_, mut expected) = mirror(true);
            // max = 10 are decided (STALE: no view is open); max + 1 is REJECTED.
            for id in 1..=10 {
                expected.push(result(
                    id,
                    id,
                    CommandStatus::Accepted,
                    &encode_container_view_result(ContainerViewOutcome::Stale),
                ));
            }
            expected.push(result(11, 11, CommandStatus::Rejected, &[]));
            assert_eq!(output, expected);
            Ok(())
        })
    }

    #[test]
    fn use_on_a_container_opens_it_in_a_new_view() -> TestResult {
        run(async {
            let authority = TreeAuthority::new(vec![
                Some(container(None, &[10])),
                Some(ContainerObservation::TooFar),
                // Not a container: the corpse path, which observes nothing here (STALE).
                Some(ContainerObservation::NotAContainer),
            ]);
            let (mut view, mut expected) = mirror(true);
            let use_item = |id, key| {
                command(
                    id,
                    COMMAND_TYPE_USE_INTENT,
                    &encode_use_item_intent(handle_of(&view, key)),
                )
            };
            let frames = [
                use_item(1, instance(1))?,
                use_item(2, instance(2))?,
                use_item(3, instance(0))?,
            ];
            let (_, output) = serve(&authority, session(&[4, 6, 12, 14])?, &frames).await?;

            let mut state = ContainerViewState::default();
            let intent = open(handle_of(&view, instance(1)));
            let (_, opened) =
                super::run(&mut state, &mut view, intent, Some(container(None, &[10])));
            let used = |disposition| encode_use_result(disposition);
            expected.extend([
                result(
                    1,
                    1,
                    CommandStatus::Accepted,
                    &used(UseDisposition::Committed),
                ),
                delta_frame(2, &opened.expect("delta")),
                result(3, 2, CommandStatus::Accepted, &used(UseDisposition::TooFar)),
                result(
                    4,
                    3,
                    CommandStatus::Accepted,
                    &used(UseDisposition::StaleState),
                ),
            ]);
            assert_eq!(output, expected);
            Ok(())
        })
    }
}
