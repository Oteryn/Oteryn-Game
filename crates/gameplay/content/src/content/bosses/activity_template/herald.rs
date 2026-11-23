//! The pinned Crystal 201189f8 heraldoffirefields spell in Herald's source room.
use super::QualifiedActivityRoomTemplate;
use crate::content::{LogicalCell, QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};

#[derive(Clone, Copy, Debug)]
pub struct HeraldFireFieldAttack(());

impl HeraldFireFieldAttack {
    pub const INTERVAL_US: u64 = 8_000_000;
    pub const DELAY_US: u64 = 1_500_000;
    pub const EFFECT: u16 = 53557;
    pub const CREATURE: &str = super::HERALD_CREATURE;
    pub const ITEM: &str = "oteryn:item.tibia.i2118";
    pub const FIRE_ITEMS: [&str; 3] = [
        Self::ITEM,
        "oteryn:item.tibia.i2119",
        "oteryn:item.tibia.i2120",
    ];
    // data-global/scripts/spells/monster/herald_of_fire_firefields.lua
    pub const BODY: [u8; 32] = [
        0x8a, 0xec, 0xe1, 0x16, 0xed, 0x66, 0xf0, 0x1c, 0xf0, 0x40, 0x74, 0x3d, 0x75, 0x79, 0xcd,
        0xc1, 0xf3, 0x2e, 0x57, 0x2a, 0x01, 0x8b, 0xd4, 0xbe, 0xb5, 0x5c, 0x1b, 0xe0, 0x66, 0xda,
        0x9f, 0xab,
    ];
    pub(super) fn qualify(
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
    ) -> Option<Self> {
        let attack = Self(());
        (attack.cells().all(|cell| {
            room.movement_cells()
                .index()
                .lookup(room.movement_cells().scope(), cell)
                .is_ok()
        }) && Self::FIRE_ITEMS
            .into_iter()
            .all(|key| native.item_policy(key, "definition-r1").is_some()))
        .then_some(attack)
    }

    pub fn cells(self) -> impl Iterator<Item = LogicalCell> {
        (32492..=32500).step_by(2).flat_map(|x| {
            (32652..=32660)
                .step_by(2)
                .map(move |y| LogicalCell { x, y, z: 15 })
        })
    }
}

impl QualifiedActivityRoomTemplate {
    pub fn herald_fire_field_attack(&self) -> Option<HeraldFireFieldAttack> {
        self.has_herald_callbacks()
            .then_some(HeraldFireFieldAttack(()))
    }
}
