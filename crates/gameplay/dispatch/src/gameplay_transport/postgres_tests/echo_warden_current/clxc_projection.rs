//! CLX-C: the real PostgreSQL encounter's portal crosses the production map wire.
use super::*;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::item_view::{ItemViewContinuity, SessionItemView};
use crate::gameplay_transport::world_map::{MapUpdate, SessionMapView};
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin, decode_world_map_snapshot};

pub(super) async fn portal_snapshot(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    present: bool,
) -> TestResult {
    let index = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("native Item index")?;
    let key = "oteryn:item.tibia.i54133";
    let reference = index
        .definition_ref("Item", key, index.revision(key).ok_or("portal revision")?)
        .ok_or("portal definition")?;
    let mut view = SessionMapView::default();
    let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    let update = owner
        .observe_world_map(actor, &mut view, &mut items)
        .await
        .ok_or("map observation")?
        .map_err(debug)?
        .ok_or("map snapshot")?;
    let MapUpdate::Snapshot(snapshot) = update else {
        return Err("expected fresh map snapshot".into());
    };
    let decoded = decode_world_map_snapshot(&snapshot.payload).map_err(debug)?;
    let portals = decoded
        .tiles
        .iter()
        .flat_map(|tile| &tile.items)
        .filter(|item| item.definition == MapDefinition::Item(reference))
        .collect::<Vec<_>>();
    assert_eq!(portals.len(), usize::from(present));
    for portal in portals {
        assert!(matches!(portal.origin, MapOrigin::Handle(_)));
        assert_eq!(portal.appearance_id, 54133);
        assert_eq!(portal.count, 1);
    }
    eprintln!("CLXC_ECHO_PORTAL_WIRE POSTGRES_ASSERTION_EXECUTED present={present}");
    Ok(())
}
