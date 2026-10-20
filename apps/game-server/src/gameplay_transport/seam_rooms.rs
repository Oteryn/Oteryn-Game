//! BUNDLE-GATES-1: the two rooms of the gameplay seam, readable only through their gates.
//!
//! `node/serve.rs` hands the seam no `qualified` room on a bundle World and, as `combat`, the
//! entry room's bundle view (BUNDLE-COMBAT-ENABLE-1, #2025). Both fields are private to this
//! module, so every read of a room is a call of one of the gate-named accessors below: one that
//! returns `combat` is open on a bundle World, one that returns `qualified` is closed there.
//! `bundle_gates_tests.rs` inventories every accessor, its state and its call sites.

use crate::content::QualifiedNativeEntryRoom;

#[derive(Clone, Copy)]
pub(crate) struct SeamRooms<'a> {
    qualified: Option<&'a QualifiedNativeEntryRoom>,
    combat: Option<&'a QualifiedNativeEntryRoom>,
}

impl<'a> SeamRooms<'a> {
    /// `qualified`: the qualified entry room, `None` on a bundle World. `combat`: the room player
    /// and monster combat read, the qualified room itself or a bundle World's bundle view
    /// (`QualifiedNativeEntryRoom::bundle_view`), whose identities match the bundle pin.
    pub(crate) fn new(
        qualified: Option<&'a QualifiedNativeEntryRoom>,
        combat: Option<&'a QualifiedNativeEntryRoom>,
    ) -> Self {
        Self { qualified, combat }
    }

    /// The player attack target.
    pub(crate) fn gate_attack_target(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.combat
    }

    /// The auto-attack drain.
    pub(crate) fn gate_auto_attack(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.combat
    }

    /// The chase step.
    pub(crate) fn gate_chase_step(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.combat
    }

    /// The native combat cast and its commit.
    pub(crate) fn gate_native_combat_cast(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.combat
    }

    /// The ordinary combat timer continuation.
    pub(crate) fn gate_combat_timer_continuation(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.combat
    }

    /// The monster AI pass.
    pub(crate) fn gate_monster_ai(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.combat
    }

    /// The periodic player damage.
    pub(crate) fn gate_periodic_damage(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.combat
    }

    /// MAGIC-CAST-1: the room a parameter cast reads. A locate message reads it only for the
    /// caster's protection-zone tile, which the combat room serves on both World kinds. House
    /// access and vertical moves read the fixture room's house and floor facts, so they keep the
    /// qualified room and stay closed on a bundle World, which has none.
    pub(crate) fn gate_parameter_cast(&self, key: &str) -> Option<&'a QualifiedNativeEntryRoom> {
        match key {
            "locate_message" => self.combat,
            _ => self.qualified,
        }
    }

    /// The world item casts and their commit.
    pub(crate) fn gate_world_item_cast(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.qualified
    }

    /// The field step ingress and its commit.
    pub(crate) fn gate_field_step_ingress(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.qualified
    }

    /// The source map initialization: the durable custody of the map's Items.
    pub(crate) fn gate_source_map_initialization(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.qualified
    }

    /// The source-World branches (door overlay, step and chase step blocking, USE); a bundle
    /// World is served by `bundle_map` at the same sites.
    pub(crate) fn gate_source_world(&self) -> Option<&'a QualifiedNativeEntryRoom> {
        self.qualified
    }
}
