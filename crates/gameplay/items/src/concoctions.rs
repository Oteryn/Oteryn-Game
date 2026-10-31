//! W09 concoction identities from Crystal 201189f8.
//!
//! `data/scripts/actions/items/drome_cube.lua:2-23` owns the twenty item identities.
//! Activation and online-time accounting belong to the existing item-use durable owner.

pub const CONCOCTION_COUNT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Concoction {
    KooldownAid = 36723,
    StrikeEnhancement = 36724,
    StaminaExtension = 36725,
    CharmUpgrade = 36726,
    WealthDuplex = 36727,
    BestiaryBetterment = 36728,
    FireResilience = 36729,
    IceResilience = 36730,
    EarthResilience = 36731,
    EnergyResilience = 36732,
    HolyResilience = 36733,
    DeathResilience = 36734,
    PhysicalResilience = 36735,
    FireAmplification = 36736,
    IceAmplification = 36737,
    EarthAmplification = 36738,
    EnergyAmplification = 36739,
    HolyAmplification = 36740,
    DeathAmplification = 36741,
    PhysicalAmplification = 36742,
}

/// Donor pool order, not a bag overlay: every entry has the same probability and count one.
pub const ALL: [Concoction; CONCOCTION_COUNT] = [
    Concoction::KooldownAid,
    Concoction::StrikeEnhancement,
    Concoction::StaminaExtension,
    Concoction::CharmUpgrade,
    Concoction::WealthDuplex,
    Concoction::BestiaryBetterment,
    Concoction::FireResilience,
    Concoction::IceResilience,
    Concoction::EarthResilience,
    Concoction::EnergyResilience,
    Concoction::HolyResilience,
    Concoction::DeathResilience,
    Concoction::PhysicalResilience,
    Concoction::FireAmplification,
    Concoction::IceAmplification,
    Concoction::EarthAmplification,
    Concoction::EnergyAmplification,
    Concoction::HolyAmplification,
    Concoction::DeathAmplification,
    Concoction::PhysicalAmplification,
];

impl Concoction {
    pub fn from_item_id(id: u32) -> Option<Self> {
        let offset = id.checked_sub(Self::KooldownAid as u32)?;
        ALL.get(usize::try_from(offset).ok()?).copied()
    }

    pub fn from_production_key(key: &str) -> Option<Self> {
        ALL.into_iter().find(|kind| kind.production_key() == key)
    }

    pub const fn item_id(self) -> u32 {
        self as u32
    }

    pub const fn index(self) -> usize {
        (self as u32 - Self::KooldownAid as u32) as usize
    }

    pub const fn production_key(self) -> &'static str {
        match self {
            Self::KooldownAid => "oteryn:item.tibia.i36723",
            Self::StrikeEnhancement => "oteryn:item.tibia.i36724",
            Self::StaminaExtension => "oteryn:item.tibia.i36725",
            Self::CharmUpgrade => "oteryn:item.tibia.i36726",
            Self::WealthDuplex => "oteryn:item.tibia.i36727",
            Self::BestiaryBetterment => "oteryn:item.tibia.i36728",
            Self::FireResilience => "oteryn:item.tibia.i36729",
            Self::IceResilience => "oteryn:item.tibia.i36730",
            Self::EarthResilience => "oteryn:item.tibia.i36731",
            Self::EnergyResilience => "oteryn:item.tibia.i36732",
            Self::HolyResilience => "oteryn:item.tibia.i36733",
            Self::DeathResilience => "oteryn:item.tibia.i36734",
            Self::PhysicalResilience => "oteryn:item.tibia.i36735",
            Self::FireAmplification => "oteryn:item.tibia.i36736",
            Self::IceAmplification => "oteryn:item.tibia.i36737",
            Self::EarthAmplification => "oteryn:item.tibia.i36738",
            Self::EnergyAmplification => "oteryn:item.tibia.i36739",
            Self::HolyAmplification => "oteryn:item.tibia.i36740",
            Self::DeathAmplification => "oteryn:item.tibia.i36741",
            Self::PhysicalAmplification => "oteryn:item.tibia.i36742",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_pool_and_every_identity_are_bijective() {
        for (index, kind) in ALL.into_iter().enumerate() {
            assert_eq!(kind.index(), index);
            assert_eq!(kind.item_id(), 36723 + index as u32);
            assert_eq!(Concoction::from_item_id(kind.item_id()), Some(kind));
            assert_eq!(Concoction::from_production_key(kind.production_key()), Some(kind));
        }
        for id in [0, 36722, 36743, 36827, u32::MAX] {
            assert_eq!(Concoction::from_item_id(id), None);
        }
        for key in ["", "36727", crate::drome_cube::DROME_CUBE_KEY, "oteryn:item.tibia.i036727"] {
            assert_eq!(Concoction::from_production_key(key), None);
        }
    }
}
