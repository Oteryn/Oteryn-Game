//! BUNDLE-GATES-1: the two rooms of the gameplay seam, readable only through their gates.
//!
//! `node/serve.rs` hands the seam no `qualified` room on a bundle World and, as `combat`, the
//! entry room's bundle view (BUNDLE-COMBAT-ENABLE-1, #2025). Both fields are private to this
//! module, so every read of a room is a call of one of the gate-named accessors below: one that
//! returns `combat` is open on a bundle World, one that returns `qualified` is closed there.
//! `bundle_gates_tests.rs` inventories every accessor, its state and its call sites.

use super::activity_runtime::SharedOwner;
use crate::content::QualifiedNativeEntryRoom;
use std::sync::Arc;

#[derive(Clone)]
pub struct SeamRooms<'a> {
    qualified: Option<SharedOwner<'a, QualifiedNativeEntryRoom>>,
    combat: Option<SharedOwner<'a, QualifiedNativeEntryRoom>>,
}

impl<'a> SeamRooms<'a> {
    /// `qualified`: the qualified entry room, `None` on a bundle World. `combat`: the room player
    /// and monster combat read, the qualified room itself or a bundle World's bundle view
    /// (`QualifiedNativeEntryRoom::bundle_view`), whose identities match the bundle pin.
    pub fn new(
        qualified: Option<&'a QualifiedNativeEntryRoom>,
        combat: Option<&'a QualifiedNativeEntryRoom>,
    ) -> Self {
        Self {
            qualified: qualified.map(SharedOwner::from),
            combat: combat.map(SharedOwner::from),
        }
    }

    /// An Instance keeps its immutable room and scope-local map alive through every
    /// handoff and recovery view. Borrowed boot rooms keep their existing gates.
    pub fn owned(room: Arc<QualifiedNativeEntryRoom>, qualified: bool) -> Self {
        Self {
            qualified: qualified.then(|| SharedOwner::Owned(Arc::clone(&room))),
            combat: Some(SharedOwner::Owned(room)),
        }
    }

    /// Spatial wire projection reads source-role metadata without granting map initialization.
    pub fn gate_wire_position(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// The player attack target.
    pub fn gate_attack_target(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// The auto-attack drain.
    pub fn gate_auto_attack(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// The chase step.
    pub fn gate_chase_step(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// The native combat cast and its commit.
    pub fn gate_native_combat_cast(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// The ordinary combat timer continuation.
    pub fn gate_combat_timer_continuation(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// The monster AI pass.
    pub fn gate_monster_ai(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// The periodic player damage.
    pub fn gate_periodic_damage(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// MAGIC-CAST-1, WAVE-A/M11 and WAVE-A2/C17c: locate messages, vertical moves and creature
    /// appearance read the combat room's tile facts on both World kinds. House access keeps the
    /// qualified room and stays closed on a bundle World, which has none.
    pub fn gate_parameter_cast(&self, key: &str) -> Option<&QualifiedNativeEntryRoom> {
        match key {
            "locate_message" => self.combat.as_deref(),
            "vertical_move" => self.combat.as_deref(),
            "creature_appearance" => self.combat.as_deref(),
            _ => self.qualified.as_deref(),
        }
    }

    /// The world item casts and their commit.
    pub fn gate_world_item_cast(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.qualified.as_deref()
    }

    /// The field step ingress and its commit.
    pub fn gate_field_step_ingress(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// W19: source-authored Ground item actions qualify the actual durable location
    /// against the current native or bundle room. This opens no spell or map-init gate.
    pub fn gate_ground_item_action(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.combat.as_deref()
    }

    /// The source map initialization: the durable custody of the map's Items.
    pub fn gate_source_map_initialization(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.qualified.as_deref()
    }

    /// The source-World branches (door overlay, step and chase step blocking, USE); a bundle
    /// World is served by `bundle_map` at the same sites.
    pub fn gate_source_world(&self) -> Option<&QualifiedNativeEntryRoom> {
        self.qualified.as_deref()
    }
}
