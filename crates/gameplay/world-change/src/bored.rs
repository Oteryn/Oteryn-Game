//! Wyda's Bored world change and the illusion spider's death reward.
use sha2::{Digest, Sha256};

pub const WYDA: &str = "oteryn:npc.wyda";
pub const BLOOD_HERB: &str = "oteryn:item.tibia.i3734";
pub const TORN_TEDDY: &str = "oteryn:item.tibia.i12617";
pub const WITCHESBROOM: &str = "oteryn:item.tibia.i3211";
pub const TORN_TREASURES: &str = "oteryn:achievement/torn_treasures";

pub fn exchange_reward(occurrence: &[u8; 16]) -> &'static str {
    let digest = Sha256::new()
        .chain_update(b"oteryn.world_change.bored.exchange.v1\0")
        .chain_update(occurrence)
        .finalize();
    let mut sample = [0; 8];
    sample.copy_from_slice(&digest[..8]);
    reward(u64::from_be_bytes(sample))
}

fn reward(draw: u64) -> &'static str {
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
        assert_eq!(reward(0), TORN_TEDDY);
        assert_eq!(reward(threshold - 1), TORN_TEDDY);
        assert_eq!(reward(threshold), WITCHESBROOM);
        assert_eq!(reward(u64::MAX), WITCHESBROOM);
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
