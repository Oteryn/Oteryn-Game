//! Session-view regressions for browse fields, using the production plan/apply/refresh owner.

use super::*;
use crate::gameplay_transport::item_view::{InventoryItems, ItemViewContinuity};
use oteryn_protocol_oteryn::container_tree::{
    MAX_CONTAINER_VIEW_ENTRIES, decode_container_views_for,
};
use std::num::NonZeroU32;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn error(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

fn position() -> WorldTilePosition {
    WorldTilePosition {
        x: 300,
        y: 300,
        floor: -7,
    }
}

fn field_key() -> ItemKey {
    ItemKey::BrowseField {
        x: 300,
        y: 300,
        floor: -7,
    }
}

fn key(value: u16) -> ItemKey {
    let mut bytes = [0x6A; 16];
    bytes[..2].copy_from_slice(&value.to_be_bytes());
    ItemKey::Instance(bytes)
}

fn item(value: u16) -> ViewItem {
    ViewItem {
        key: key(value),
        item_definition_ref: NonZeroU32::MIN,
        count: NonZeroU32::MIN,
        sub_type: 0,
    }
}

fn observation(ids: impl IntoIterator<Item = u16>) -> ContainerObservation {
    ContainerObservation::BrowseField {
        position: position(),
        entries: ids.into_iter().map(item).collect(),
        lease: None,
    }
}

fn fixture(browse: bool) -> TestResult<(ContainerViewState, SessionItemView)> {
    let mut view = SessionItemView::resume(ItemViewContinuity::default()).with_container_tree();
    let mut state = ContainerViewState::default();
    if browse {
        view = view.with_browse_field();
        state = state.with_browse_field();
    }
    view.snapshot(InventoryItems::default(), None)
        .map_err(error)?;
    view.views_snapshot().map_err(error)?;
    Ok((state, view))
}

fn run(
    state: &mut ContainerViewState,
    view: &mut SessionItemView,
    intent: BrowseFieldIntent,
    observation: Option<ContainerObservation>,
) -> TestResult<(ContainerViewOutcome, Option<ItemViewDelta>)> {
    match state.plan_browse(intent) {
        ContainerPlan::Observe(pending) => state
            .apply(view, pending, observation)
            .map_err(|e| error(e).into()),
        ContainerPlan::Decided(outcome, delta) => Ok((outcome, delta)),
    }
}

fn open(
    state: &mut ContainerViewState,
    view: &mut SessionItemView,
    observation: ContainerObservation,
) -> TestResult<ItemViewDelta> {
    let (outcome, delta) = run(
        state,
        view,
        BrowseFieldIntent::Open {
            position: position(),
        },
        Some(observation),
    )?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    Ok(delta.ok_or("open delta")?)
}

fn single(delta: &ItemViewDelta, selected: bool) -> TestResult<ContainerView> {
    let mut views = decode_container_views_for(&delta.payload, selected)
        .map_err(error)?
        .views;
    assert_eq!(views.len(), 1);
    Ok(views.pop().ok_or("single view")?)
}

fn ids(view: &SessionItemView, shown: &ContainerView) -> TestResult<Vec<u16>> {
    shown
        .entries
        .iter()
        .map(|entry| {
            let Some(ItemKey::Instance(bytes)) = view.resolve(entry.handle) else {
                return Err("entry handle must resolve to its instance".into());
            };
            Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
        })
        .collect()
}

#[test]
fn browse_open_and_seek_issue_only_current_page_handles() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    let delta = open(&mut state, &mut view, observation(1..=65))?;
    let first = single(&delta, true)?;
    assert_eq!(first.view_id, 15);
    assert_eq!(first.capacity, 30);
    assert_eq!(first.browse.ok_or("page metadata")?.total, 65);
    assert_eq!(ids(&view, &first)?, (1..=30).collect::<Vec<_>>());
    assert!(view.table().handle(&key(31)).is_none());
    let first_handle = first.entries[0].handle;
    let container_handle = first.container_handle;
    for (offset, expected) in [
        (30, (31..=60).collect::<Vec<_>>()),
        (60, (61..=65).collect()),
    ] {
        let (outcome, delta) = run(
            &mut state,
            &mut view,
            BrowseFieldIntent::Seek {
                view_id: 15,
                index: offset,
            },
            Some(observation(1..=65)),
        )?;
        assert_eq!(outcome, ContainerViewOutcome::Opened);
        let shown = single(&delta.ok_or("seek delta")?, true)?;
        assert_eq!(shown.container_handle, container_handle);
        assert_eq!(shown.browse.ok_or("page metadata")?.index, offset);
        assert_eq!(ids(&view, &shown)?, expected);
        assert!(view.resolve(first_handle).is_none());
    }
    let before = view.continuity();
    for (view_id, index) in [(15, 61), (15, 90), (14, 0)] {
        let (outcome, delta) = run(
            &mut state,
            &mut view,
            BrowseFieldIntent::Seek { view_id, index },
            Some(observation(1..=65)),
        )?;
        assert_eq!(outcome, ContainerViewOutcome::Stale);
        assert!(delta.is_none());
        assert_eq!(view.continuity(), before);
    }
    Ok(())
}

#[test]
fn browse_empty_field_opens_but_cannot_be_sought() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    let shown = single(&open(&mut state, &mut view, observation([]))?, true)?;
    assert!(shown.entries.is_empty());
    assert_eq!(shown.browse.ok_or("empty metadata")?.total, 0);
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Seek {
            view_id: 15,
            index: 0,
        },
        Some(observation([])),
    )?;
    assert_eq!(outcome, ContainerViewOutcome::Stale);
    assert!(delta.is_none());
    Ok(())
}

#[test]
fn browse_toggle_requires_current_reach_before_closing_its_reserved_slot() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    let first = single(&open(&mut state, &mut view, observation([1]))?, true)?;
    for (observed, expected) in [
        (
            Some(ContainerObservation::TooFar),
            ContainerViewOutcome::TooFar,
        ),
        (
            Some(ContainerObservation::Gone),
            ContainerViewOutcome::Stale,
        ),
        (None, ContainerViewOutcome::Stale),
    ] {
        let (outcome, delta) = run(
            &mut state,
            &mut view,
            BrowseFieldIntent::Open {
                position: position(),
            },
            observed,
        )?;
        assert_eq!(outcome, expected);
        assert!(delta.is_none());
        assert_eq!(state.open_views(), 1);
    }
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Open {
            position: position(),
        },
        Some(observation([1])),
    )?;
    assert_eq!(outcome, ContainerViewOutcome::Closed);
    assert!(
        decode_container_views_for(&delta.ok_or("close delta")?.payload, true)
            .map_err(error)?
            .views
            .is_empty()
    );
    assert_eq!(state.open_views(), 0);
    assert!(view.resolve(first.container_handle).is_none());
    assert!(view.resolve(first.entries[0].handle).is_none());
    Ok(())
}

#[test]
fn browse_toggle_closes_all_aliases_without_reopening_on_seek_or_refresh() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    open(&mut state, &mut view, observation(1..=31))?;
    let (outcome, _) = state
        .apply(
            &mut view,
            PendingOpen::new_view(field_key()),
            Some(observation(1..=31)),
        )
        .map_err(error)?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let (outcome, _) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Seek {
            view_id: 0,
            index: 30,
        },
        Some(observation(1..=31)),
    )?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let retired = [field_key(), key(1), key(31)]
        .map(|key| view.table().handle(&key).ok_or("visible alias handle"));
    let ordinary = ContainerObservation::Container {
        parent: None,
        capacity: 20,
        entries: vec![item(2001)],
    };
    let (outcome, _) = state
        .apply(
            &mut view,
            PendingOpen::new_view(key(2000)),
            Some(ordinary.clone()),
        )
        .map_err(error)?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let other_position = WorldTilePosition {
        x: 301,
        ..position()
    };
    let other_key = ItemKey::BrowseField {
        x: other_position.x,
        y: other_position.y,
        floor: other_position.floor,
    };
    let other = ContainerObservation::BrowseField {
        position: other_position,
        entries: vec![item(3001)],
        lease: None,
    };
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Open {
            position: other_position,
        },
        Some(other.clone()),
    )?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let before = decode_container_views_for(&delta.ok_or("all views")?.payload, true)
        .map_err(error)?
        .views;
    assert_eq!(
        before.iter().map(|view| view.view_id).collect::<Vec<_>>(),
        [0, 1, 12, 15]
    );
    let unrelated: Vec<_> = before
        .into_iter()
        .filter(|view| matches!(view.view_id, 1 | 12))
        .collect();

    // Crystal game.cpp:5077 broadcasts closure to the aliases. ANSWER6543254285
    // also retires them in the complete snapshot, so no hidden alias can reopen.
    // x+3 selects the same dummy ID; closure belongs to its existing occupant.
    let collision = WorldTilePosition {
        x: 303,
        ..position()
    };
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Open {
            position: collision,
        },
        Some(ContainerObservation::BrowseField {
            position: collision,
            entries: vec![item(4001)],
            lease: None,
        }),
    )?;
    assert_eq!(outcome, ContainerViewOutcome::Closed);
    assert_eq!(
        decode_container_views_for(&delta.ok_or("close all aliases")?.payload, true)
            .map_err(error)?
            .views,
        unrelated
    );
    assert_eq!(state.containers(), [key(2000), other_key]);
    for handle in retired {
        assert!(view.resolve(handle?).is_none());
    }
    let continuity = view.continuity();
    for view_id in [0, 15] {
        let (outcome, delta) = run(
            &mut state,
            &mut view,
            BrowseFieldIntent::Seek { view_id, index: 0 },
            Some(observation(1..=31)),
        )?;
        assert_eq!(outcome, ContainerViewOutcome::Stale);
        assert!(delta.is_none());
    }
    // The donor's retained page-30 alias would reopen when size31 shrinks to30
    // (player.cpp:9077-9080); the approved snapshot adaptation keeps it closed.
    assert!(
        state
            .refresh(
                &mut view,
                &[
                    (field_key(), Some(observation(1..=30))),
                    (key(2000), Some(ordinary)),
                    (other_key, Some(other)),
                ],
            )
            .map_err(error)?
            .is_none()
    );
    assert_eq!(state.containers(), [key(2000), other_key]);
    assert_eq!(view.continuity(), continuity);
    Ok(())
}

#[test]
fn browse_up_replaces_the_ground_container_in_the_same_slot_at_first_page() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    let (outcome, delta) = state
        .apply(
            &mut view,
            PendingOpen::new_view(key(1000)),
            Some(ContainerObservation::Container {
                parent: Some(field_key()),
                capacity: 20,
                entries: vec![item(1001)],
            }),
        )
        .map_err(error)?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let ground = single(&delta.ok_or("ground container delta")?, true)?;
    assert_eq!(ground.view_id, 0);
    assert_eq!(
        ground.parent_handle.and_then(|handle| view.resolve(handle)),
        Some(field_key())
    );
    let pending = match state
        .plan(
            &mut view,
            ContainerViewIntent::Up {
                view_id: ground.view_id,
            },
        )
        .map_err(error)?
    {
        ContainerPlan::Observe(pending) => pending,
        _ => return Err("Up must observe the field".into()),
    };
    assert_eq!(pending.key(), field_key());
    let (outcome, delta) = state
        .apply(&mut view, pending, Some(observation(1..=40)))
        .map_err(error)?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let field = single(&delta.ok_or("up delta")?, true)?;
    assert_eq!(field.view_id, ground.view_id);
    assert!(field.parent_handle.is_none());
    assert_eq!(field.browse.ok_or("field page")?.index, 0);
    assert_eq!(ids(&view, &field)?, (1..=30).collect::<Vec<_>>());
    assert!(matches!(
        state
            .plan(
                &mut view,
                ContainerViewIntent::Up {
                    view_id: field.view_id
                }
            )
            .map_err(error)?,
        ContainerPlan::Decided(ContainerViewOutcome::Stale, None)
    ));
    Ok(())
}

#[test]
fn browse_extension_does_not_expose_legacy_parent_or_expand_ordinary_capacity() -> TestResult {
    let (mut state, mut view) = fixture(false)?;
    assert!(matches!(
        state.plan_browse(BrowseFieldIntent::Open {
            position: position()
        }),
        ContainerPlan::Decided(ContainerViewOutcome::Stale, None)
    ));
    let (_, delta) = state
        .apply(
            &mut view,
            PendingOpen::new_view(key(1000)),
            Some(ContainerObservation::Container {
                parent: Some(field_key()),
                capacity: MAX_CONTAINER_VIEW_ENTRIES as u8,
                entries: vec![],
            }),
        )
        .map_err(error)?;
    let legacy = single(&delta.ok_or("legacy delta")?, false)?;
    assert!(legacy.parent_handle.is_none());
    assert!(view.table().handle(&field_key()).is_none());
    for selected in [false, true] {
        let (mut state, mut view) = fixture(selected)?;
        let (outcome, delta) = state
            .apply(
                &mut view,
                PendingOpen::new_view(key(1000)),
                Some(ContainerObservation::Container {
                    parent: None,
                    capacity: 30,
                    entries: vec![],
                }),
            )
            .map_err(error)?;
        assert_eq!(outcome, ContainerViewOutcome::Stale);
        assert!(delta.is_none());
        assert_eq!(state.open_views(), 0);
    }
    Ok(())
}

#[test]
fn browse_refresh_coalesces_removals_additions_counts_and_range_closure() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    let first = single(&open(&mut state, &mut view, observation([1, 2, 3]))?, true)?;
    let removed = first.entries[0].handle;
    let surviving = first.entries[1].handle;
    let mut third = item(3);
    third.count = NonZeroU32::new(7).ok_or("count")?;
    let current = ContainerObservation::BrowseField {
        position: position(),
        entries: vec![item(5), item(4), third, item(2)],
        lease: None,
    };
    let delta = state
        .refresh(&mut view, &[(field_key(), Some(current.clone()))])
        .map_err(error)?
        .ok_or("refresh delta")?;
    let changed = single(&delta, true)?;
    assert_eq!(ids(&view, &changed)?, [2, 3, 4, 5]);
    assert_eq!(changed.entries[0].handle, surviving);
    assert_eq!(changed.entries[1].count.get(), 7);
    assert!(view.resolve(removed).is_none());
    assert!(
        state
            .refresh(&mut view, &[(field_key(), Some(current))])
            .map_err(error)?
            .is_none()
    );
    let closed = state
        .refresh(
            &mut view,
            &[(field_key(), Some(ContainerObservation::TooFar))],
        )
        .map_err(error)?
        .ok_or("range-close delta")?;
    assert!(
        decode_container_views_for(&closed.payload, true)
            .map_err(error)?
            .views
            .is_empty()
    );
    assert!(view.resolve(surviving).is_none());
    Ok(())
}

#[test]
fn browse_refresh_rewinds_to_surviving_page_after_coalesced_removals() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    open(&mut state, &mut view, observation(1..=65))?;
    let (_, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Seek {
            view_id: 15,
            index: 60,
        },
        Some(observation(1..=65)),
    )?;
    let old = single(&delta.ok_or("last-page delta")?, true)?;
    let delta = state
        .refresh(&mut view, &[(field_key(), Some(observation(1..=20)))])
        .map_err(error)?
        .ok_or("shrunk delta")?;
    let shrunk = single(&delta, true)?;
    let page = shrunk.browse.ok_or("shrunk page")?;
    assert_eq!((page.index, page.total), (0, 20));
    assert_eq!(ids(&view, &shrunk)?, (1..=20).collect::<Vec<_>>());
    assert!(view.resolve(old.entries[0].handle).is_none());
    Ok(())
}

#[test]
fn browse_seek_preserves_append_order_for_coalesced_new_items() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    open(&mut state, &mut view, observation([1, 2]))?;
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Seek {
            view_id: 15,
            index: 0,
        },
        Some(observation([4, 3, 2, 1])),
    )?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let shown = single(&delta.ok_or("seek delta")?, true)?;
    assert_eq!(ids(&view, &shown)?, [1, 2, 3, 4]);
    Ok(())
}

#[test]
fn browse_views_share_tile_order_until_the_last_connection_closes() -> TestResult {
    let cache = browse_cache::BrowseFields::default();
    let shared = |values: &[u16]| -> TestResult<ContainerObservation> {
        let (entries, lease) = cache
            .observe(
                [1; 32],
                0,
                crate::map::overlay::TilePos {
                    x: 300,
                    y: 300,
                    floor: -7,
                },
                values.iter().copied().map(item).collect(),
            )
            .ok_or("shared observation")?;
        Ok(ContainerObservation::BrowseField {
            position: position(),
            entries,
            lease: Some(lease),
        })
    };
    let (mut state_a, mut view_a) = fixture(true)?;
    let initial = single(&open(&mut state_a, &mut view_a, shared(&[2, 1])?)?, true)?;
    assert_eq!(ids(&view_a, &initial)?, [2, 1]);
    let (mut state_b, mut view_b) = fixture(true)?;
    let later = single(&open(&mut state_b, &mut view_b, shared(&[3, 2, 1])?)?, true)?;
    assert_eq!(ids(&view_b, &later)?, [2, 1, 3]);
    let delta = state_a
        .refresh(&mut view_a, &[(field_key(), Some(shared(&[3, 2, 1])?))])
        .map_err(error)?
        .ok_or("first connection refresh")?;
    assert_eq!(ids(&view_a, &single(&delta, true)?)?, [2, 1, 3]);
    for (state, view) in [(&mut state_a, &mut view_a), (&mut state_b, &mut view_b)] {
        assert!(matches!(
            state
                .plan(view, ContainerViewIntent::Close { view_id: 15 })
                .map_err(error)?,
            ContainerPlan::Decided(ContainerViewOutcome::Closed, Some(_))
        ));
    }
    let (mut state_c, mut view_c) = fixture(true)?;
    let reopened = single(&open(&mut state_c, &mut view_c, shared(&[3, 2, 1])?)?, true)?;
    assert_eq!(ids(&view_c, &reopened)?, [3, 2, 1]);
    Ok(())
}

#[test]
fn browse_up_preserves_existing_alias_with_independent_pages_and_shared_refresh() -> TestResult {
    let cache = browse_cache::BrowseFields::default();
    let shared = |values: &[u16]| -> TestResult<ContainerObservation> {
        let (entries, lease) = cache
            .observe(
                [1; 32],
                0,
                crate::map::overlay::TilePos {
                    x: 300,
                    y: 300,
                    floor: -7,
                },
                values.iter().copied().map(item).collect(),
            )
            .ok_or("shared observation")?;
        Ok(ContainerObservation::BrowseField {
            position: position(),
            entries,
            lease: Some(lease),
        })
    };
    let decode = |delta: Option<ItemViewDelta>| -> TestResult<Vec<ContainerView>> {
        Ok(
            decode_container_views_for(&delta.ok_or("view delta")?.payload, true)
                .map_err(error)?
                .views,
        )
    };
    let full: Vec<_> = (1..=65).chain([1000]).collect();
    let (mut state, mut view) = fixture(true)?;
    open(&mut state, &mut view, shared(&full)?)?;
    let (_, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Seek {
            view_id: 15,
            index: 60,
        },
        Some(shared(&full)?),
    )?;
    let last_page = decode(delta)?;
    let high_handle = last_page[0].entries[0].handle;
    let bag = view.table().handle(&key(1000)).ok_or("ground bag handle")?;
    let ContainerPlan::Observe(pending) = state
        .plan(
            &mut view,
            ContainerViewIntent::Open {
                handle: bag,
                replace_view: None,
            },
        )
        .map_err(error)?
    else {
        return Err("ground bag must open in another ID".into());
    };
    let (outcome, _) = state
        .apply(
            &mut view,
            pending,
            Some(ContainerObservation::Container {
                parent: Some(field_key()),
                capacity: 20,
                entries: vec![],
            }),
        )
        .map_err(error)?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let ContainerPlan::Observe(pending) = state
        .plan(&mut view, ContainerViewIntent::Up { view_id: 0 })
        .map_err(error)?
    else {
        return Err("Up must observe the shared field".into());
    };
    let (outcome, delta) = state
        .apply(&mut view, pending, Some(shared(&full)?))
        .map_err(error)?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let shown = decode(delta)?;
    assert_eq!(
        shown.iter().map(|view| view.view_id).collect::<Vec<_>>(),
        [0, 15]
    );
    assert_eq!(shown[0].container_handle, shown[1].container_handle);
    assert_eq!(shown[0].browse.ok_or("Up page")?.index, 0);
    assert_eq!(shown[1].browse.ok_or("existing page")?.index, 60);
    assert_eq!(ids(&view, &shown[0])?, (1..=30).collect::<Vec<_>>());
    assert_eq!(
        ids(&view, &shown[1])?,
        (61..=65).chain([1000]).collect::<Vec<_>>()
    );
    assert_eq!(shown[1].entries[0].handle, high_handle);
    let low_handle = shown[0].entries[0].handle;

    // A seek is also a fresh tile observation: synchronize the other alias's total and page,
    // while retaining its independent index and any handles still visible there.
    let added: Vec<_> = std::iter::once(66).chain(full.iter().copied()).collect();
    let (_, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Seek {
            view_id: 0,
            index: 30,
        },
        Some(shared(&added)?),
    )?;
    let shown = decode(delta)?;
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[0].browse.ok_or("sought page")?.index, 30);
    assert_eq!(shown[1].browse.ok_or("unchanged page")?.index, 60);
    assert!(
        shown
            .iter()
            .all(|view| view.browse.is_some_and(|page| page.total == 67))
    );
    assert_eq!(ids(&view, &shown[0])?, (31..=60).collect::<Vec<_>>());
    assert_eq!(
        ids(&view, &shown[1])?,
        (61..=65).chain([1000, 66]).collect::<Vec<_>>()
    );
    assert_eq!(view.resolve(high_handle), Some(key(61)));
    assert!(view.resolve(low_handle).is_none());

    let smaller: Vec<_> = (1..=40).chain([1000]).collect();
    let (_, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Seek {
            view_id: 0,
            index: 0,
        },
        Some(shared(&smaller)?),
    )?;
    let shown = decode(delta)?;
    assert_eq!(shown[0].browse.ok_or("first page")?.index, 0);
    assert_eq!(shown[1].browse.ok_or("rewound alias")?.index, 30);
    assert!(
        shown
            .iter()
            .all(|view| view.browse.is_some_and(|page| page.total == 41))
    );
    assert_eq!(
        ids(&view, &shown[1])?,
        (31..=40).chain([1000]).collect::<Vec<_>>()
    );
    assert!(view.resolve(high_handle).is_none());
    let page_thirty_handle = shown[1].entries[0].handle;

    let smallest: Vec<_> = (1..=20).chain([1000]).collect();
    let delta = state
        .refresh(&mut view, &[(field_key(), Some(shared(&smallest)?))])
        .map_err(error)?;
    let shown = decode(delta)?;
    assert_eq!(shown.len(), 2);
    assert!(shown.iter().all(|view| {
        view.browse
            .is_some_and(|page| (page.index, page.total) == (0, 21))
    }));
    assert_eq!(shown[0].entries, shown[1].entries);
    assert!(view.resolve(page_thirty_handle).is_none());
    let retained = shown[0].entries[0].handle;
    // Explicit Close remains per ID (Crystal game.cpp:4445-4452), unlike the
    // Browse-toggle broadcast normalized by ANSWER6543254285.
    let ContainerPlan::Decided(ContainerViewOutcome::Closed, delta) = state
        .plan(&mut view, ContainerViewIntent::Close { view_id: 15 })
        .map_err(error)?
    else {
        return Err("explicit Close must close the requested ID".into());
    };
    let shown = decode(delta)?;
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].view_id, 0);
    assert_eq!(view.resolve(retained), Some(key(1)));
    let reopened = decode(Some(open(&mut state, &mut view, shared(&smallest)?)?))?;
    assert_eq!(
        reopened.iter().map(|view| view.view_id).collect::<Vec<_>>(),
        [0, 15]
    );
    Ok(())
}

#[test]
fn browse_aliases_reuse_previous_order_when_an_observation_has_no_cache_lease() -> TestResult {
    let (mut state, mut view) = fixture(true)?;
    open(&mut state, &mut view, observation([1, 2]))?;
    let (outcome, _) = state
        .apply(
            &mut view,
            PendingOpen::new_view(field_key()),
            Some(observation([1, 2])),
        )
        .map_err(error)?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let (outcome, delta) = run(
        &mut state,
        &mut view,
        BrowseFieldIntent::Seek {
            view_id: 0,
            index: 0,
        },
        Some(observation([4, 3, 2, 1])),
    )?;
    assert_eq!(outcome, ContainerViewOutcome::Opened);
    let shown = decode_container_views_for(&delta.ok_or("aliases delta")?.payload, true)
        .map_err(error)?
        .views;
    assert_eq!(shown.len(), 2);
    for alias in &shown {
        assert_eq!(ids(&view, alias)?, [1, 2, 3, 4]);
        assert_eq!(alias.browse.ok_or("alias metadata")?.total, 4);
    }
    assert_eq!(shown[0].entries, shown[1].entries);
    Ok(())
}
