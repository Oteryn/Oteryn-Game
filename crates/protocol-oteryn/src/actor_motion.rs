//! Negotiated actor movement and riding facts carried by visibility entries.
use crate::world_spatial::{MAX_DIAGONAL_STEP_WAIT_MS, WorldSpatialError};

pub const CAPABILITY_ACTOR_MOTION_V1: u32 = 502;
pub const COMMAND_TYPE_MOUNT: u32 = 6242;

pub fn encode_mount(riding: bool) -> Vec<u8> {
    vec![8, u8::from(riding)]
}

pub fn decode_mount(bytes: &[u8]) -> Result<bool, WorldSpatialError> {
    match bytes {
        [8, 0] => Ok(false),
        [8, 1] => Ok(true),
        _ => Err(WorldSpatialError::Malformed),
    }
}

/// A zero duration denotes a relocation, never a walking animation. Forecasts
/// are for N,E,S,W,NE,SE,SW,NW from this entry's position; zero means unknown or
/// blocked. Only the observer's own entry carries forecasts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActorMotion {
    pub translation_revision: u64,
    pub step_duration_ms: u32,
    pub mount_look_type: u32,
    pub prewalk_ms: [u32; 8],
}

impl ActorMotion {
    pub(crate) fn encode(self) -> Result<Vec<u8>, WorldSpatialError> {
        self.validate()?;
        let mut bytes = Vec::with_capacity(48);
        bytes.extend_from_slice(&self.translation_revision.to_le_bytes());
        for value in [self.step_duration_ms, self.mount_look_type]
            .into_iter()
            .chain(self.prewalk_ms)
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Ok(bytes)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, WorldSpatialError> {
        let bytes: &[u8; 48] = bytes.try_into().map_err(|_| WorldSpatialError::Malformed)?;
        let word =
            |at| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        let mut prewalk_ms = [0; 8];
        for (index, duration) in prewalk_ms.iter_mut().enumerate() {
            *duration = word(16 + index * 4);
        }
        let value = Self {
            translation_revision: u64::from_le_bytes(
                bytes[..8]
                    .try_into()
                    .map_err(|_| WorldSpatialError::Malformed)?,
            ),
            step_duration_ms: word(8),
            mount_look_type: word(12),
            prewalk_ms,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), WorldSpatialError> {
        if self.step_duration_ms > MAX_DIAGONAL_STEP_WAIT_MS
            || self
                .prewalk_ms
                .iter()
                .any(|ms| *ms > MAX_DIAGONAL_STEP_WAIT_MS)
            || self.mount_look_type > u16::MAX.into()
        {
            return Err(WorldSpatialError::Malformed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actor_motion_visibility_extension_is_gated_and_legacy_bytes_are_unchanged()
    -> Result<(), WorldSpatialError> {
        use crate::{
            world_spatial::{ActorPosition, StepDirection},
            world_spatial_entities::*,
        };
        let mut entity = WorldSpatialEntity {
            motion: Some(ActorMotion {
                translation_revision: 1,
                step_duration_ms: 200,
                mount_look_type: 139,
                prewalk_ms: [550; 8],
            }),
            display_name: None,
            kind: EntityKind::Player,
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 1,
                y: 2,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let snapshot = |entity: WorldSpatialEntity| WorldSpatialEntitiesSnapshot {
            content_generation: [0; 32],
            actor_position: entity.position,
            own_identity: [1; 16],
            entities: vec![entity],
        };
        let fields = EntityFields::selected(&[502]);
        let bytes = encode_world_spatial_entities_snapshot_with_fields(
            &snapshot(entity),
            fields,
            &EntityLights::new(),
        )?;
        let (decoded, _) = decode_world_spatial_entities_snapshot_with_fields(&bytes, fields)?;
        assert_eq!(decoded.entities[0].motion, entity.motion);
        assert!(decode_world_spatial_entities_snapshot(&bytes).is_err());
        let legacy = encode_world_spatial_entities_snapshot(&snapshot(entity))?;
        entity.motion = None;
        assert_eq!(
            legacy,
            encode_world_spatial_entities_snapshot(&snapshot(entity))?
        );
        assert_eq!(
            decode_world_spatial_entities_snapshot(&legacy)?.entities[0].motion,
            None
        );
        Ok(())
    }
    #[test]
    fn actor_motion_and_native_icons_share_an_entry_without_colliding()
    -> Result<(), WorldSpatialError> {
        use crate::{
            world_spatial::{ActorPosition, StepDirection},
            world_spatial_entities::*,
        };
        let icon = NativeCreatureIcon {
            category: 0,
            id: 22,
            count: 5,
        };
        let entity = WorldSpatialEntity {
            motion: Some(ActorMotion {
                translation_revision: 3,
                step_duration_ms: 200,
                mount_look_type: 0,
                prewalk_ms: [0; 8],
            }),
            display_name: None,
            kind: EntityKind::Player,
            entity: EntityRef {
                identity: [2; 16],
                generation: 1,
            },
            position: ActorPosition {
                x: 1,
                y: 2,
                floor: -7,
            },
            detail: EntityDetail::ActorWithIcons {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
                icons: NativeCreatureIcons::new(&[icon])?,
            },
        };
        let own = WorldSpatialEntity {
            motion: Some(ActorMotion {
                prewalk_ms: [550; 8],
                ..entity.motion.unwrap_or_default()
            }),
            entity: EntityRef {
                identity: [1; 16],
                generation: 1,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
            ..entity
        };
        let snapshot = WorldSpatialEntitiesSnapshot {
            content_generation: [0; 32],
            actor_position: own.position,
            own_identity: [1; 16],
            entities: vec![own, entity],
        };
        let fields = EntityFields::selected(&[6, CAPABILITY_NATIVE_CREATURE_ICONS_V1, 502]);
        let bytes = encode_world_spatial_entities_snapshot_with_fields(
            &snapshot,
            fields,
            &EntityLights::new(),
        )?;
        let (decoded, _) = decode_world_spatial_entities_snapshot_with_fields(&bytes, fields)?;
        assert_eq!(decoded.entities, snapshot.entities);
        Ok(())
    }
    #[test]
    fn actor_motion_wire_bounds_and_mount_boolean_are_strict() -> Result<(), WorldSpatialError> {
        let motion = ActorMotion {
            translation_revision: u64::MAX,
            step_duration_ms: 200,
            mount_look_type: 139,
            prewalk_ms: [550; 8],
        };
        assert_eq!(ActorMotion::decode(&motion.encode()?)?, motion);
        assert!(ActorMotion::decode(&[0; 47]).is_err());
        assert!(ActorMotion::decode(&[0; 49]).is_err());
        assert!(
            ActorMotion {
                step_duration_ms: MAX_DIAGONAL_STEP_WAIT_MS + 1,
                ..motion
            }
            .encode()
            .is_err()
        );
        assert!(
            ActorMotion {
                mount_look_type: 65536,
                ..motion
            }
            .encode()
            .is_err()
        );
        for mounted in [false, true] {
            assert_eq!(decode_mount(&encode_mount(mounted))?, mounted);
        }
        for invalid in [&[][..], &[8][..], &[8, 2][..], &[8, 1, 8, 1][..]] {
            assert!(decode_mount(invalid).is_err());
        }
        Ok(())
    }
}
