//! Connection-local appearance presentation. VIS-2 alone owns visibility.
mod item_dictionary;
mod mapping_reducer;
use super::*;
pub use item_dictionary::AppearanceItemDictionary;
use oteryn_protocol_oteryn::appearance as wire;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppearanceUpdate {
    Mappings(wire::Mappings),
    Catalogue(wire::Catalogue),
}
#[derive(Debug, Clone)]
pub(crate) struct AppearanceState {
    revision: u64,
    high_water: u32,
    catalogue: wire::Catalogue,
    items: Option<item_dictionary::ItemDictionary>,
    bindings: BTreeMap<world_spatial_entities::EntityRef, wire::Binding>,
}
fn invalid() -> SessionError {
    SessionError::Protocol(FoundationProtocolError::MalformedEnvelope)
}
fn codec(_: wire::AppearanceError) -> SessionError {
    invalid()
}

pub(crate) fn from_snapshots(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
    generation: [u8; 32],
    items: Option<AppearanceItemDictionary<'_>>,
) -> Result<Option<AppearanceState>, SessionError> {
    if !selected.contains(&wire::CAPABILITY_APPEARANCE_V1) {
        return Ok(None);
    }
    let domain = snapshots
        .iter()
        .find(|s| s.domain_id == wire::STATE_DOMAIN_APPEARANCE)
        .ok_or(SessionError::MissingDomain(wire::STATE_DOMAIN_APPEARANCE))?;
    if domain.snapshot_type != wire::SNAPSHOT_TYPE_APPEARANCE || domain.revision == 0 {
        return Err(invalid());
    }
    let snapshot = wire::Snapshot::decode(&domain.payload).map_err(codec)?;
    if snapshot.catalogue.context.content_generation != generation {
        return Err(invalid());
    }
    let state = AppearanceState {
        items: item_dictionary::admit(items, generation)?,
        revision: domain.revision,
        high_water: snapshot.high_water,
        catalogue: snapshot.catalogue,
        bindings: snapshot
            .bindings
            .into_iter()
            .map(|b| (b.entity, b))
            .collect(),
    };
    state.validate_looks(selected.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1))?;
    Ok(Some(state))
}
pub(crate) fn validate_entities(
    state: Option<&AppearanceState>,
    entities: Option<&WorldEntities>,
) -> Result<(), SessionError> {
    if let Some(state) = state {
        let entities = entities.ok_or_else(invalid)?;
        for entity in entities
            .entities
            .values()
            .filter(|e| e.kind == world_spatial_entities::EntityKind::Player)
        {
            let world_spatial_entities::EntityDetail::Actor { appearance_ref, .. } = entity.detail
            else {
                return Err(invalid());
            };
            state
                .resolve(entity.entity, appearance_ref)
                .ok_or_else(invalid)?;
        }
    }
    Ok(())
}
pub(crate) fn validate_result(
    command: u32,
    result: &GatedCommandResult,
) -> Result<(), SessionError> {
    if command == wire::COMMAND_TYPE_APPEARANCE_CHANGE {
        let disposition = wire::ChangeResult::decode(&result.payload).map_err(codec)?;
        let accepted = matches!(disposition, wire::ChangeResult::Committed(_));
        if accepted != (result.status == CommandStatus::Accepted) {
            return Err(invalid());
        }
    }
    Ok(())
}
impl AppearanceState {
    pub(crate) fn resolve(
        &self,
        entity: world_spatial_entities::EntityRef,
        reference: u32,
    ) -> Option<&wire::Look> {
        let binding = self.bindings.get(&entity)?;
        (binding.appearance_ref == reference).then_some(&binding.look)
    }
}

impl<S: SessionStream> Session<S> {
    pub(crate) fn appearance_delta(
        &mut self,
        sequence: u64,
        delta: oteryn_protocol_oteryn::StateDeltaView<'_>,
    ) -> Result<SessionEvent, SessionError> {
        if !self.is_selected(wire::CAPABILITY_APPEARANCE_V1) {
            return Err(SessionError::UnselectedDomain {
                domain_id: wire::STATE_DOMAIN_APPEARANCE,
            });
        }
        let mut state = self.appearance.clone().ok_or_else(invalid)?;
        let value = state.apply(
            delta.base_revision,
            delta.new_revision,
            delta.delta_type,
            delta.payload,
            self.is_selected(CAPABILITY_ITEM_VIEW_MOVE_V1),
        )?;
        validate_entities(Some(&state), self.entities.as_ref())?;
        self.appearance = Some(state);
        Ok(SessionEvent::Appearance(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value,
        }))
    }
}

#[cfg(test)]
mod mapping_tests;
