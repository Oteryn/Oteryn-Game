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

/// Crystal 201189f8 movements_herald_red_floors.lua.
#[derive(Clone, Copy, Debug)]
pub struct HeraldRedFloors(());

impl HeraldRedFloors {
    pub const PLATES: [(u16, u16); 3] = [(32496, 32649), (32503, 32656), (32496, 32663)];
    pub const UNSTEPPED: u16 = 53096;
    pub const STEPPED: u16 = 53095;
    pub const BODY: [u8; 32] = [
        0xe0, 0x96, 0xce, 0x49, 0xef, 0xf3, 0x10, 0xb5, 0xdc, 0x37, 0x7f, 0x22, 0xf1, 0x98, 0xc6,
        0x58, 0x39, 0x1b, 0xcc, 0xbc, 0xa5, 0x74, 0xc3, 0xc3, 0x0f, 0xd7, 0x1f, 0x59, 0xf4, 0x8f,
        0xfb, 0xfd,
    ];

    pub fn contains_plate(position: LogicalCell) -> bool {
        position.z == 15
            && Self::PLATES
                .into_iter()
                .any(|(x, y)| (i32::from(x), i32::from(y)) == (position.x, position.y))
    }

    pub(super) fn qualify(room: &QualifiedNativeEntryRoom) -> Option<Self> {
        Self::PLATES
            .into_iter()
            .all(|(x, y)| {
                let cells = room.movement_cells();
                let plate = LogicalCell {
                    x: i32::from(x),
                    y: i32::from(y),
                    z: 15,
                };
                cells
                    .spell_tiles()
                    .lookup(cells.scope(), plate)
                    .is_ok_and(|tile| tile.ground_source_id() == Some(u32::from(Self::UNSTEPPED)))
            })
            .then_some(Self(()))
    }
}

impl QualifiedActivityRoomTemplate {
    pub fn herald_red_floors(&self) -> Option<HeraldRedFloors> {
        self.has_herald_callbacks().then_some(HeraldRedFloors(()))
    }
}
