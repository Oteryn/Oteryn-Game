//! Canary 6725cfa3 is failure evidence; native page boundaries remain authoritative.
use super::{TestResult, fixture::debug};
use crate::gameplay_transport::container_view::{
    ContainerObservation, ContainerPlan, ContainerViewState, browse_rules,
};
use crate::gameplay_transport::item_view::{
    InventoryItems, ItemKey, ItemViewContinuity, SessionItemView, ViewItem,
};
use oteryn_protocol_oteryn::browse_field::BrowseFieldIntent;
use oteryn_protocol_oteryn::container_tree::{
    ContainerViewIntent, ContainerViewOutcome, decode_container_views_for,
};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use std::num::NonZeroU32;

#[test]
fn donor_f33_shrink_resyncs_and_proximity_retires_handles() -> TestResult {
    let position = WorldTilePosition {
        x: 300,
        y: 300,
        floor: -7,
    };
    let key = ItemKey::BrowseField {
        x: 300,
        y: 300,
        floor: -7,
    };
    let observation = |total: u8| ContainerObservation::BrowseField {
        position,
        entries: (1..=total)
            .map(|n| ViewItem {
                key: ItemKey::Instance(super::fixture::id(n)),
                item_definition_ref: NonZeroU32::MIN,
                count: NonZeroU32::MIN,
                sub_type: 0,
            })
            .collect(),
        lease: None,
    };
    let mut view = SessionItemView::resume(ItemViewContinuity::default())
        .with_container_tree()
        .with_browse_field();
    view.snapshot(InventoryItems::default(), None)
        .map_err(debug)?;
    view.views_snapshot().map_err(debug)?;
    let mut state = ContainerViewState::default().with_browse_field();
    let ContainerPlan::Observe(open) = state.plan_browse(BrowseFieldIntent::Open { position })
    else {
        return Err("open must observe".into());
    };
    state
        .apply(&mut view, open, Some(observation(61)))
        .map_err(debug)?;
    // Offset 40 is not a native 30-entry page. Never reproduce the donor's
    // page size by changing the protocol: refuse it, then observe a valid page.
    assert!(!browse_rules::valid_seek(40, 61));
    assert_eq!(browse_rules::refresh_index(40, 21), 0);
    let ContainerPlan::Observe(seek) = state.plan_browse(BrowseFieldIntent::Seek {
        view_id: 15,
        index: 40,
    }) else {
        return Err("seek must reobserve".into());
    };
    let (outcome, delta) = state
        .apply(&mut view, seek, Some(observation(61)))
        .map_err(debug)?;
    assert_eq!(outcome, ContainerViewOutcome::Stale);
    assert!(delta.is_none());
    let ContainerPlan::Observe(seek) = state.plan_browse(BrowseFieldIntent::Seek {
        view_id: 15,
        index: 30,
    }) else {
        return Err("valid seek must observe".into());
    };
    let (_, delta) = state
        .apply(&mut view, seek, Some(observation(61)))
        .map_err(debug)?;
    let shown =
        decode_container_views_for(&delta.ok_or("seek delta")?.payload, true).map_err(debug)?;
    let old = shown.views[0].entries[0].handle;
    assert_eq!(shown.views[0].browse.ok_or("page")?.index, 30);
    let delta = state
        .refresh(&mut view, &[(key, Some(observation(21)))])
        .map_err(debug)?
        .ok_or("shrink delta")?;
    let shown = decode_container_views_for(&delta.payload, true).map_err(debug)?;
    assert_eq!(shown.views.len(), 1);
    let page = shown.views[0].browse.ok_or("shrunk page")?;
    assert_eq!((page.index, page.total), (0, 21));
    assert_eq!(shown.views[0].entries.len(), 21);
    assert!(view.resolve(old).is_none());
    let handles: Vec<_> = shown.views[0]
        .entries
        .iter()
        .map(|e| e.handle)
        .chain([shown.views[0].container_handle])
        .collect();
    let delta = state
        .refresh(&mut view, &[(key, Some(ContainerObservation::TooFar))])
        .map_err(debug)?
        .ok_or("reach loss delta")?;
    assert!(
        decode_container_views_for(&delta.payload, true)
            .map_err(debug)?
            .views
            .is_empty()
    );
    assert_eq!(state.open_views(), 0);
    for handle in handles {
        assert!(view.resolve(handle).is_none());
        assert!(matches!(
            state
                .plan(
                    &mut view,
                    ContainerViewIntent::Open {
                        handle,
                        replace_view: None
                    }
                )
                .map_err(debug)?,
            ContainerPlan::Decided(ContainerViewOutcome::Stale, None)
        ));
    }
    Ok(())
}
