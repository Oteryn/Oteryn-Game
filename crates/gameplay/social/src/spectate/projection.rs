//! World-tier inputs only. There is deliberately no opaque payload/envelope, inventory,
//! actor-vitals, cooldown, command-outcome, private-room or account-data input.
use super::{SpectateRuntime, TargetFacts};
use oteryn_protocol_oteryn::spectate::{EndReason, SpectateMessage};
use oteryn_protocol_oteryn::{
    chat::{ChatLine, encode_chat_line},
    presentation::{PresentationEvent, encode_presentation_batch},
    world_map::{
        MapViewBinding, WorldMapViewDelta, WorldMapViewSnapshot, encode_world_map_delta,
        encode_world_map_snapshot,
    },
    world_spatial_entities::{
        EntityFields, EntityLights, WorldSpatialEntitiesDelta, WorldSpatialEntitiesSnapshot,
        encode_world_spatial_entities_delta_with_fields,
        encode_world_spatial_entities_snapshot_with_fields,
    },
};

pub enum WorldProjection {
    SpatialSnapshot {
        value: WorldSpatialEntitiesSnapshot,
        fields: EntityFields,
        lights: EntityLights,
    },
    SpatialDelta {
        value: WorldSpatialEntitiesDelta,
        fields: EntityFields,
        lights: EntityLights,
    },
    MapSnapshot(WorldMapViewSnapshot),
    MapDelta {
        binding: MapViewBinding,
        value: WorldMapViewDelta,
    },
    Presentation(Vec<PresentationEvent>),
    Chat(ChatLine),
}
impl WorldProjection {
    fn encode(self) -> Option<SpectateMessage> {
        let (domain, payload_type, snapshot, payload) = match self {
            Self::SpatialSnapshot {
                value,
                fields,
                lights,
            } => (
                1,
                2,
                true,
                encode_world_spatial_entities_snapshot_with_fields(&value, fields, &lights).ok()?,
            ),
            Self::SpatialDelta {
                value,
                fields,
                lights,
            } => (
                1,
                2,
                false,
                encode_world_spatial_entities_delta_with_fields(&value, fields, &lights).ok()?,
            ),
            Self::MapSnapshot(value) => (17, 1, true, encode_world_map_snapshot(&value).ok()?),
            Self::MapDelta { binding, value } => {
                (17, 1, false, encode_world_map_delta(&binding, &value).ok()?)
            }
            Self::Presentation(value) => (19, 1, false, encode_presentation_batch(&value).ok()?),
            Self::Chat(line @ ChatLine::Local { .. }) => {
                (12, 1, false, encode_chat_line(&line).ok()?)
            }
            Self::Chat(_) => return None,
        };
        Some(SpectateMessage::Frame {
            domain,
            payload_type,
            snapshot,
            payload,
        })
    }
}
impl SpectateRuntime {
    pub(super) fn publish(&mut self, facts: TargetFacts, projection: WorldProjection) {
        self.refresh(facts);
        let target = facts.authority.current_game_session_id();
        if !self.grants.contains_key(&target) {
            return;
        }
        let Some(frame) = projection.encode() else {
            return;
        };
        let Ok(encoded) = Vec::<u8>::try_from(&frame) else {
            return;
        };
        let mut overloaded = Vec::new();
        for observer in self
            .observers
            .values_mut()
            .filter(|o| o.target == target && o.ended.is_none())
        {
            // NET03-OUTBOUND-QUEUE-ENTRIES / -BYTES. Fan-out performs no await or owner read.
            if observer.queue.len() >= 64
                || observer.bytes.saturating_add(encoded.len() + 32) > 1_048_576
            {
                overloaded.push(observer.identity);
            } else {
                observer.bytes += encoded.len() + 32;
                observer.queue.push_back(frame.clone());
            }
        }
        for identity in overloaded {
            self.end_observer(identity, EndReason::Overload);
        }
    }
}
