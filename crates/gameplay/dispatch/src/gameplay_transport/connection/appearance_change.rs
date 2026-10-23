//! One observer connection's appearance dictionary. Never part of resume continuity.
mod mapping_refresh;
use super::*;
use oteryn_protocol_oteryn::appearance as wire;
use oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1;
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub(super) struct SessionAppearance {
    selected: bool,
    item_view: bool,
    revision: u64,
    high_water: u32,
    catalogue: Option<wire::Catalogue>,
    bindings: BTreeMap<WireEntityRef, wire::Binding>,
    snapshot: Option<std::sync::Arc<[u8]>>,
}
impl SessionAppearance {
    pub(super) fn new(selected: SelectedCapabilities) -> Self {
        Self {
            selected: selected.contains(wire::CAPABILITY_APPEARANCE_V1),
            item_view: selected.contains(CAPABILITY_ITEM_VIEW_MOVE_V1),
            ..Self::default()
        }
    }
    fn players(
        entities: &[oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity],
    ) -> Vec<WireEntityRef> {
        entities
            .iter()
            .filter(|e| {
                e.kind == oteryn_protocol_oteryn::world_spatial_entities::EntityKind::Player
            })
            .map(|e| e.entity)
            .collect()
    }
    fn upserts(
        &mut self,
        observation: super::super::appearance_change::observation::Observation,
        players: &[WireEntityRef],
    ) -> Option<Vec<wire::Binding>> {
        if observation
            .bindings
            .iter()
            .map(|b| b.entity)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != players.len()
            || observation.bindings.len() != players.len()
            || observation.bindings.iter().any(|b| {
                !players.contains(&b.entity)
                    || b.context != observation.catalogue.context
                    || (!self.item_view && matches!(b.look, wire::Look::Item { .. }))
            })
        {
            return None;
        }
        if self
            .catalogue
            .as_ref()
            .is_some_and(|c| c.context != observation.catalogue.context)
        {
            return None;
        }
        let mut upserts = Vec::new();
        for mut binding in observation.bindings {
            binding.appearance_ref = match self.bindings.get(&binding.entity) {
                Some(prior) => prior.appearance_ref,
                None => {
                    self.high_water = self.high_water.checked_add(1)?;
                    self.high_water
                }
            };
            let unchanged = self.bindings.get(&binding.entity).is_some_and(|prior| {
                prior.root_revision == binding.root_revision
                    && prior.condition_revision == binding.condition_revision
                    && prior.look == binding.look
                    && prior.selection_revision == binding.selection_revision
            });
            if !unchanged {
                self.bindings.insert(binding.entity, binding.clone());
                upserts.push(binding);
            }
        }
        self.catalogue = Some(observation.catalogue);
        upserts.sort_by_key(|b| b.entity);
        Some(upserts)
    }
    fn attach(
        &self,
        entities: &mut [oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity],
    ) -> Option<()> {
        for entity in entities {
            if entity.kind == oteryn_protocol_oteryn::world_spatial_entities::EntityKind::Player {
                let reference = self.bindings.get(&entity.entity)?.appearance_ref;
                let oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Actor {
                    appearance_ref,
                    ..
                } = &mut entity.detail
                else {
                    return None;
                };
                *appearance_ref = reference;
            }
        }
        Some(())
    }
    pub(super) async fn initial<A: FreshAdmissionAuthority>(
        &mut self,
        authority: &A,
        actor: ExactActorRef,
        admitted: &AdmittedSession,
        view: &mut SessionVisibility,
        channel: &ChannelEntities,
        item_view: Option<&mut SessionItemView>,
    ) -> Option<(u32, Vec<u8>)> {
        let mut item_view = item_view;
        let mut snapshot = view
            .snapshot(channel, &mut |entities| {
                attach_spatial_handles(item_view.as_deref_mut(), entities)
            })
            .ok()?;
        if self.selected {
            let players = Self::players(&snapshot.entities);
            let observation = authority
                .appearance_observation(actor, admitted.game_session_id, players.clone())
                .await?;
            self.upserts(observation, &players)?;
            self.attach(&mut snapshot.entities)?;
            let payload = wire::Snapshot {
                catalogue: self.catalogue.clone()?,
                bindings: self.bindings.values().cloned().collect(),
                high_water: self.high_water,
            }
            .encode()
            .ok()?;
            self.snapshot = Some(payload.into());
            self.revision = 1;
        }
        encode_visibility_snapshot_lit(
            admitted.continuity.selected_capabilities.as_slice(),
            &snapshot,
            view.lights(),
        )
        .ok()
    }
    pub(super) fn domain<'a>(&'a self, domains: &mut Vec<DomainSnapshot<'a>>) {
        if let Some(payload) = &self.snapshot {
            domains.insert(
                0,
                DomainSnapshot {
                    domain_id: wire::STATE_DOMAIN_APPEARANCE,
                    revision: self.revision,
                    snapshot_type: wire::SNAPSHOT_TYPE_APPEARANCE,
                    payload,
                },
            );
        }
    }
}

pub(in crate::gameplay_transport) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    payload: &[u8],
) -> Option<wire::ChangeResult> {
    let change = match wire::Change::decode(payload) {
        Ok(change) => change,
        Err(_) => return Some(wire::ChangeResult::Rejected),
    };
    authority.appearance_change(actor, command, change).await
}
pub(super) fn result(result: wire::ChangeResult) -> Option<(CommandStatus, Vec<u8>)> {
    Some((
        if matches!(result, wire::ChangeResult::Committed(_)) {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        },
        result.encode().ok()?,
    ))
}
