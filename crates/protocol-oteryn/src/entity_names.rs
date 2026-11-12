//! S-ENTNAME: negotiated display names in spatial actor entries (field 13).
use crate::world_spatial::WorldSpatialError;

pub const CAPABILITY_WORLD_SPATIAL_ENTITIES_V2: u32 = 286;
pub const MAX_ENTITY_DISPLAY_NAME_BYTES: usize = 64;

/// A bounded UTF-8 value keeps spatial entries cheap to copy without truncating names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityDisplayName {
    bytes: [u8; MAX_ENTITY_DISPLAY_NAME_BYTES],
    len: u8,
}

impl EntityDisplayName {
    pub fn new(name: &str) -> Result<Self, WorldSpatialError> {
        if name.len() > MAX_ENTITY_DISPLAY_NAME_BYTES {
            return Err(WorldSpatialError::LimitExceeded);
        }
        if name.trim().is_empty() || name.chars().any(char::is_control) {
            return Err(WorldSpatialError::Malformed);
        }
        let mut value = Self {
            bytes: [0; MAX_ENTITY_DISPLAY_NAME_BYTES],
            len: name.len() as u8,
        };
        value.bytes[..name.len()].copy_from_slice(name.as_bytes());
        Ok(value)
    }

    pub fn as_str(&self) -> Result<&str, WorldSpatialError> {
        let bytes = self
            .bytes
            .get(..usize::from(self.len))
            .ok_or(WorldSpatialError::LimitExceeded)?;
        let name = std::str::from_utf8(bytes).map_err(|_| WorldSpatialError::Malformed)?;
        if name.trim().is_empty() || name.chars().any(char::is_control) {
            return Err(WorldSpatialError::Malformed);
        }
        Ok(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn entity_names_encode_rejects_an_overbound_value() {
        // Exercise the encode-side check even though public construction already rejects it.
        let invalid = EntityDisplayName {
            bytes: [b'x'; MAX_ENTITY_DISPLAY_NAME_BYTES],
            len: 65,
        };
        let entity = crate::world_spatial_entities::WorldSpatialEntity {
            display_name: Some(invalid),
            kind: crate::world_spatial_entities::EntityKind::Player,
            entity: crate::world_spatial_entities::EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: crate::world_spatial::ActorPosition {
                x: 0,
                y: 0,
                floor: 7,
            },
            detail: crate::world_spatial_entities::EntityDetail::Actor {
                direction: crate::world_spatial::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let snapshot = crate::world_spatial_entities::WorldSpatialEntitiesSnapshot {
            content_generation: [1; 32],
            actor_position: entity.position,
            own_identity: entity.entity.identity,
            entities: vec![entity],
        };
        assert_eq!(
            crate::world_spatial_entities::encode_world_spatial_entities_snapshot_with_fields(
                &snapshot,
                crate::world_spatial_entities::EntityFields::selected(&[6, 286]),
                &crate::world_spatial_entities::EntityLights::new(),
            ),
            Err(WorldSpatialError::LimitExceeded)
        );
    }
}
