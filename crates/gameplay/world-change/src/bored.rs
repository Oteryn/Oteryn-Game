//! Wyda's Bored world change and the illusion spider's death reward.
pub const WYDA: &str = "oteryn:npc.wyda";
pub const BLOOD_HERB: &str = "oteryn:item.tibia.i3734";
pub const TORN_TEDDY: &str = "oteryn:item.tibia.i12617";
pub const WITCHESBROOM: &str = "oteryn:item.tibia.i3211";
pub const TORN_TREASURES: &str = "oteryn:achievement/torn_treasures";

pub fn exchange_reward(draw: u64) -> &'static str {
    if (u128::from(draw) * 1000) >> 64 < 36 {
        TORN_TEDDY
    } else {
        WITCHESBROOM
    }
}

/// Crystal data/scripts/creaturescripts/monster/giant_spider_wyda_death.lua:3–8.
pub fn death_achievement(creature: &str) -> Option<&'static str> {
    (creature == "oteryn:creature.giant_spider_wyda")
        .then_some("oteryn:achievement/someone_s_bored")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bored_exchange_reward_threshold() {
        let threshold = ((1_u128 << 64) * 36).div_ceil(1000) as u64;
        assert_eq!(exchange_reward(0), TORN_TEDDY);
        assert_eq!(exchange_reward(threshold - 1), TORN_TEDDY);
        assert_eq!(exchange_reward(threshold), WITCHESBROOM);
        assert_eq!(exchange_reward(u64::MAX), WITCHESBROOM);
    }

    #[test]
    fn bored_achievement_is_only_for_wydas_spider() {
        assert_eq!(
            death_achievement("oteryn:creature.giant_spider_wyda"),
            Some("oteryn:achievement/someone_s_bored")
        );
        assert_eq!(death_achievement("oteryn:creature.giant_spider"), None);
    }
}
