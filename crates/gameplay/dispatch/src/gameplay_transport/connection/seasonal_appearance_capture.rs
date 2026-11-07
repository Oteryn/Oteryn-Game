//! Test-only access to the unchanged production observer dictionary and wire refresher.
use super::*;
use crate::gameplay_transport::item_view::SessionItemView;
use crate::gameplay_transport::world_spatial::{ChannelEntities, SessionVisibility};

pub(in crate::gameplay_transport) struct SeasonalAppearanceDomain {
    pub domain_id: u32,
    pub revision: u64,
    pub snapshot_type: u32,
    pub payload: Vec<u8>,
}
/// Opaque: profile tests cannot assign appearance references or mapping revisions.
pub(in crate::gameplay_transport) struct SeasonalAppearanceCapture {
    appearances: SessionAppearance,
    visibility: SessionVisibility,
    items: Option<SessionItemView>,
    map: Option<crate::gameplay_transport::world_map::SessionMapView>,
}
impl SeasonalAppearanceCapture {
    pub(in crate::gameplay_transport) async fn begin<A: FreshAdmissionAuthority>(
        authority: &A,
        actor: ExactActorRef,
        admitted: &mut AdmittedSession,
        channel: &ChannelEntities,
    ) -> Option<(Self, Vec<SeasonalAppearanceDomain>)> {
        let selected = admitted.continuity.selected_capabilities;
        let mut items = selected
            .contains(oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1)
            .then(|| {
                let items = SessionItemView::resume(admitted.continuity.item_view);
                if selected.contains(18) {
                    items.with_map_view()
                } else {
                    items
                }
            });
        let mut visibility =
            SessionVisibility::with_objects(items.is_some()).lit(selected.contains(19));
        let mut appearances = SessionAppearance::new(selected);
        let (snapshot_type, payload) = appearances
            .initial(
                authority,
                actor,
                admitted,
                &mut visibility,
                channel,
                items.as_mut(),
            )
            .await?;
        if let Some(items) = &items {
            admitted.continuity.item_view = items.continuity();
        }
        let mut domains = vec![SeasonalAppearanceDomain {
            domain_id: 1,
            revision: admitted.continuity.spatial_revision,
            snapshot_type,
            payload,
        }];
        let mut borrowed = Vec::new();
        appearances.domain(&mut borrowed);
        domains.extend(borrowed.into_iter().map(|domain| SeasonalAppearanceDomain {
            domain_id: domain.domain_id,
            revision: domain.revision,
            snapshot_type: domain.snapshot_type,
            payload: domain.payload.to_vec(),
        }));
        Some((
            Self {
                appearances,
                visibility,
                items,
                map: selected.contains(18).then(Default::default),
            },
            domains,
        ))
    }
    /// Initial map comes from the same handle directory as appearance/VIS publication.
    pub(in crate::gameplay_transport) async fn initial_map<A: FreshAdmissionAuthority>(
        &mut self,
        authority: &A,
        actor: ExactActorRef,
        admitted: &mut AdmittedSession,
    ) -> Option<SeasonalAppearanceDomain> {
        let map = self.map.as_mut()?;
        let items = self.items.as_mut()?;
        let crate::gameplay_transport::world_map::MapUpdate::Snapshot(snapshot) = authority
            .observe_world_map(actor, map, items)
            .await?
            .ok()??
        else {
            return None;
        };
        admitted.continuity.world_map_revision = map.revision();
        admitted.continuity.item_view = items.continuity();
        Some(SeasonalAppearanceDomain {
            domain_id: snapshot.domain_id,
            revision: snapshot.revision,
            snapshot_type: snapshot.snapshot_type,
            payload: snapshot.payload,
        })
    }
    /// Unchanged production map replacement/delta producer, including sequence barriers.
    pub(in crate::gameplay_transport) async fn refresh_map<A: FreshAdmissionAuthority>(
        &mut self,
        authority: &A,
        actor: ExactActorRef,
        admitted: &mut AdmittedSession,
    ) -> Option<Vec<u8>> {
        let map = self.map.as_mut()?;
        let items = self.items.as_mut()?;
        let update = world_map_frames(
            authority,
            actor,
            admitted.continuity.connection_generation,
            admitted.continuity.server_sequence,
            map,
            items,
        )
        .await?;
        let mut bytes = Vec::new();
        if let Some((sequence, frames)) = update {
            for frame in frames {
                crate::gameplay_transport::tcp_tls::write_frame(&mut bytes, &frame)
                    .await
                    .ok()?;
            }
            admitted.continuity.server_sequence = sequence;
        }
        admitted.continuity.world_map_revision = map.revision();
        admitted.continuity.item_view = items.continuity();
        Some(bytes)
    }
    /// Returns the exact length-prefixed bytes written by the production mapping+VIS refresher.
    pub(in crate::gameplay_transport) async fn refresh<A: FreshAdmissionAuthority>(
        &mut self,
        authority: &A,
        actor: ExactActorRef,
        admitted: &mut AdmittedSession,
        channel: &ChannelEntities,
    ) -> Option<Vec<u8>> {
        let mut bytes = Vec::new();
        let sequence = admitted.continuity.server_sequence;
        let revision = admitted.continuity.spatial_revision;
        let (sequence, revision) = self
            .appearances
            .refresh(
                &mut bytes,
                authority,
                actor,
                admitted,
                &mut self.visibility,
                channel,
                self.items.as_mut(),
                sequence,
                revision,
            )
            .await?;
        admitted.continuity.server_sequence = sequence;
        admitted.continuity.spatial_revision = revision;
        Some(bytes)
    }
}
