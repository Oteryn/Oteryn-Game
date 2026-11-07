//! Registered GlobalEvent DATA pinned through the actual source World capture.
//! This proof grants no actor, Item write, Node or runtime Scope authority.
use super::QualifiedNativeEntryRoom;
use super::project::native_spell_world::PinnedActivitySourceRoom;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedSpiteGlobalEvent {
    content: [u8; 32],
    body: [u8; 32],
}
impl QualifiedSpiteGlobalEvent {
    pub const fn content(&self) -> [u8; 32] {
        self.content
    }
    pub const fn body_digest(&self) -> [u8; 32] {
        self.body
    }
    pub const fn name(&self) -> &'static str {
        "CreateGoshnarSpiteFire"
    }
    pub const fn encounter_key(&self) -> &'static str {
        "oteryn:encounter.soul_war_taint_zones"
    }
    pub const fn positions(&self) -> [[i32; 3]; 4] {
        [
            [33743, 31628, 14],
            [33736, 31632, 14],
            [33750, 31632, 14],
            [33742, 31637, 14],
        ]
    }
    pub const fn interval_ms(&self) -> u64 {
        14_000
    }
    pub const fn expiry_ms(&self) -> u64 {
        5_000
    }
}

pub(in crate::content) fn matches_capture(value: &serde_json::Value) -> bool {
    static CAPTURE: std::sync::OnceLock<Option<serde_json::Value>> = std::sync::OnceLock::new();
    CAPTURE
        .get_or_init(|| {
            serde_json::from_slice(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/srx3-spite-global-event.json"
            ))
            .ok()
        })
        .as_ref()
        == Some(value)
}

impl super::native_gameplay::NativeGameplayState {
    /// Registered event DATA for the actual published World, independently of
    /// a private room's coordinate frame or a boss incarnation.
    pub fn qualify_spite_global_event_in_published_world(
        &self,
        room: &QualifiedNativeEntryRoom,
        pin: &super::channel_content_pin::ChannelContentPin,
        world: &super::QualifiedPublishedWorldContext,
    ) -> Option<QualifiedSpiteGlobalEvent> {
        if !world.matches(room, pin)
            || self.source_digest() != pin.server_artifact_digest()
            || !self.qualifies_soul_war_callbacks(self.source_digest())
        {
            return None;
        }
        // Neutral until the actual full published-World target proves this port.
        None
    }
    pub fn qualify_spite_global_event(
        &self,
        room: &QualifiedNativeEntryRoom,
    ) -> Option<QualifiedSpiteGlobalEvent> {
        if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::GoshnarsSpite)
            || self.source_digest() != room.compiled().server_digest()
            || !self.qualifies_soul_war_callbacks(self.source_digest())
        {
            return None;
        }
        // The production Source Bundle retains its role only after exact
        // capture-cell qualification. Its current collision index is Bundle,
        // so immutable recipe DATA comes from that sealed role, never an old
        // Source scope masquerading as the current physical map.
        let retained;
        let data = if let Some(map) = room.source_world() {
            map.registered_global_event_data()?
        } else {
            if !matches!(
                room.movement_cells().index(),
                super::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
            ) {
                return None;
            }
            retained = serde_json::from_slice::<serde_json::Value>(
                PinnedActivitySourceRoom::GoshnarsSpite.bytes(),
            )
            .ok()?;
            retained.get("registered_global_events")?
        };
        if !matches_capture(data) {
            return None;
        }
        self.item_index()?
            .definition_ref("Item", "oteryn:item.tibia.i33877", "definition-r1")?;
        let proof = QualifiedSpiteGlobalEvent {
            content: self.source_digest(),
            body: [0; 32],
        };
        for [x, y, floor] in proof.positions() {
            if let Some(map) = room.source_world() {
                map.immutable_tile(map.scope(), super::LogicalCell { x, y, z: -floor })
                    .ok()?;
            } else {
                room.movement_cells()
                    .index()
                    .lookup(
                        room.movement_cells().scope(),
                        super::LogicalCell { x, y, z: floor },
                    )
                    .ok()?;
            }
        }
        let hex = data
            .get("sources")?
            .as_array()?
            .first()?
            .get("sha256")?
            .as_str()?;
        if hex.len() != 64 {
            return None;
        }
        let mut body = [0; 32];
        for (index, byte) in body.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).ok()?;
        }
        Some(QualifiedSpiteGlobalEvent { body, ..proof })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    #[test]
    fn srx3_spite_registered_global_event_qualifies_actual_source_world_without_actor()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut id = [104; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let world = oteryn_protocol_oteryn::WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let source_room = pin
            .qualified_room()
            .pinned_activity_source_view(PinnedActivitySourceRoom::GoshnarsSpite)
            .map_err(|error| format!("{error:?}"))?;
        let room = &source_room;
        assert_eq!(
            room.pinned_activity_source_room(),
            Some(PinnedActivitySourceRoom::GoshnarsSpite)
        );
        let data = room
            .source_world()
            .and_then(|map| map.registered_global_event_data())
            .ok_or("registered GlobalEvent DATA must be retained by actual source map parser")?;
        assert!(matches_capture(data));
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("native generation")?;
        let proof = native
            .qualify_spite_global_event(room)
            .ok_or("actual registered source GlobalEvent requires no synthetic Creature parent")?;
        assert_eq!(proof.name(), "CreateGoshnarSpiteFire");
        assert_eq!((proof.interval_ms(), proof.expiry_ms()), (14_000, 5_000));
        assert_eq!(proof.content(), native.source_digest());
        assert_ne!(proof.body_digest(), [0; 32]);
        assert_eq!(proof.positions().len(), 4);
        Ok(())
    }
}
