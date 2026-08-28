//! Deterministic mutation of the non-authoritative client projection.

use oteryn_client_domain::{ClientWorldProjection, ProjectionEvent};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationError {
    RevisionOverflow,
}

impl Display for SimulationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("client projection revision reached its bound")
    }
}

impl std::error::Error for SimulationError {}

#[derive(Debug, Clone)]
pub struct ClientSimulation {
    projection: ClientWorldProjection,
}

impl ClientSimulation {
    #[must_use]
    pub const fn new(projection: ClientWorldProjection) -> Self {
        Self { projection }
    }

    pub fn apply(&mut self, event: ProjectionEvent) -> Result<(), SimulationError> {
        let next_revision = self
            .projection
            .revision
            .checked_add(1)
            .ok_or(SimulationError::RevisionOverflow)?;
        match event {
            ProjectionEvent::Upsert(entity) => {
                self.projection.entities.insert(entity.entity_ref, entity);
            }
            ProjectionEvent::Remove(entity_ref) => {
                self.projection.entities.remove(&entity_ref);
            }
        }
        self.projection.revision = next_revision;
        Ok(())
    }

    #[must_use]
    pub const fn snapshot(&self) -> &ClientWorldProjection {
        &self.projection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_domain::{ClientEntityRef, EntityProjection, Position};
    use oteryn_foundation::ClientSessionEpoch;

    #[test]
    fn projection_is_deterministic() -> Result<(), SimulationError> {
        let projection = ClientWorldProjection::empty(ClientSessionEpoch::new(1));
        let mut simulation = ClientSimulation::new(projection);
        simulation.apply(ProjectionEvent::Upsert(EntityProjection {
            entity_ref: ClientEntityRef::new(9),
            position: Position {
                x: 1,
                y: 2,
                floor: 7,
            },
            display_name: "Synthetic Rat".to_owned(),
        }))?;
        assert_eq!(simulation.snapshot().revision, 1);
        assert_eq!(simulation.snapshot().entities.len(), 1);
        Ok(())
    }
}
