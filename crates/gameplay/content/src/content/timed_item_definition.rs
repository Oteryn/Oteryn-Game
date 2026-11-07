//! Content's timed-item schema adapter for the domain host.
use super::{
    ReferenceItemField, ReferenceItemSemantics, ReferenceItemTarget, ReferenceTemporalMode,
    ReferenceTransformKind,
};
use crate::domain::timed_item::TimedValues;
use crate::domain::timed_item_host::{TimedDefinitionSource, TimedHostError, TimedItemDefinition};

impl TimedDefinitionSource<ReferenceItemTarget> for ReferenceItemSemantics {
    fn resolve_timed_definition(
        &self,
        definition: &ReferenceItemTarget,
    ) -> Result<Option<TimedItemDefinition<ReferenceItemTarget>>, TimedHostError> {
        let semantics = self;
        use ReferenceItemField::{Conflict, Known};
        let (remaining_ms, temporal_target) = match &semantics.temporal {
            Conflict => return Err(TimedHostError::InvalidDefinition),
            Known(temporal) => {
                let target = match &temporal.decay_target {
                    Conflict => return Err(TimedHostError::InvalidDefinition),
                    Known(target) => Some(target),
                    _ => None,
                };
                match (&temporal.consumption_mode, &temporal.duration) {
                    (Conflict, _) | (_, Conflict) => return Err(TimedHostError::InvalidDefinition),
                    (
                        Known(ReferenceTemporalMode::AuthoritativeActiveTimeBudget),
                        Known(duration),
                    ) => (Some(duration.0), target),
                    _ => (None, target),
                }
            }
            _ => (None, None),
        };
        // §8: `transform {trigger: decay}` takes precedence; both set to different targets conflict.
        let transform_target = match &semantics.use_transform {
            Conflict => return Err(TimedHostError::InvalidDefinition),
            Known(transform) => match transform
                .targets
                .iter()
                .find(|entry| entry.kind == ReferenceTransformKind::Decay)
                .map(|entry| &entry.target)
            {
                Some(Conflict) => return Err(TimedHostError::InvalidDefinition),
                Some(Known(target)) => Some(target),
                _ => None,
            },
            _ => None,
        };
        let decay_target = match (transform_target, temporal_target) {
            (Some(transform), Some(temporal)) if transform != temporal => {
                return Err(TimedHostError::InvalidDefinition);
            }
            (Some(target), _) | (None, Some(target)) => Some(target.clone()),
            (None, None) => None,
        };
        let charges = match &semantics.charges {
            Conflict => return Err(TimedHostError::InvalidDefinition),
            Known(charges) => match charges.count {
                Conflict => return Err(TimedHostError::InvalidDefinition),
                Known(count) => Some(count),
                _ => None,
            },
            _ => None,
        };
        if charges.is_none() && remaining_ms.is_none() {
            return Ok(None);
        }
        Ok(Some(TimedItemDefinition {
            definition: definition.clone(),
            full: TimedValues::new(charges, remaining_ms)
                .map_err(|_| TimedHostError::InvalidDefinition)?,
            decay_target,
        }))
    }
}
