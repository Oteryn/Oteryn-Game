//! Falcon Bastion's six accepted 24-hour random reward sources.
use sha2::{Digest, Sha256};

pub const COOLDOWN_MS: i64 = 86_400_000;
pub const CLAIM_PREFIX: &str =
    "oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.";
pub const REWARDS: [(u16, u32); 7] = [
    (9081, 1),
    (28821, 1),
    (28823, 1),
    (9058, 1),
    (6299, 1),
    (3052, 1),
    (3035, 10),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chest {
    pub suffix: &'static str,
    pub unique: u16,
    pub appearance: u16,
    pub position: (u16, u16, i8),
}
impl Chest {
    pub fn key(self) -> String {
        format!("{CLAIM_PREFIX}{}", self.suffix)
    }
    pub fn source(self) -> String {
        // The existing native RewardClaim binder uses this same stable key.
        format!(
            "oteryn:placement/{}/0",
            self.key().trim_start_matches("oteryn:")
        )
    }
}

// Crystal 201189f:chest.lua:1383-1429, accepted 24h repeat ruling PR2770 Q3/A.
pub const CHESTS: [Chest; 6] = [
    Chest {
        suffix: "coast",
        unique: 6105,
        appearance: 23741,
        position: (33352, 31318, -7),
    },
    Chest {
        suffix: "island",
        unique: 6106,
        appearance: 23740,
        position: (33384, 31285, -7),
    },
    Chest {
        suffix: "throne_hall",
        unique: 6107,
        appearance: 23741,
        position: (33366, 31323, -5),
    },
    Chest {
        suffix: "shortcut",
        unique: 6108,
        appearance: 23741,
        position: (33374, 31340, -4),
    },
    Chest {
        suffix: "lower_bastion",
        unique: 6109,
        appearance: 23740,
        position: (33324, 31268, -8),
    },
    Chest {
        suffix: "underground_bastion",
        unique: 6110,
        appearance: 23740,
        position: (33308, 31304, -9),
    },
];

pub fn claim_key(key: &str) -> Option<Chest> {
    CHESTS.into_iter().find(|chest| chest.key() == key)
}

/// The accepted cooldown starts at successful delivery, not the first draw.
pub fn repeat_ready(claimed_at: i64, now: i64) -> bool {
    claimed_at
        .checked_add(COOLDOWN_MS)
        .is_some_and(|next| now >= next)
}

/// Uniform one-of-seven selection. Rejection sampling avoids modulo bias; hashing
/// a rejected block makes the frozen 32-byte entropy a complete replay recipe.
pub fn draw_index(entropy: &[u8; 32]) -> usize {
    let mut block = *entropy;
    loop {
        if let Some(byte) = block.iter().find(|byte| **byte < 252) {
            return usize::from(*byte % 7);
        }
        block = Sha256::digest(block).into();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn repeat_opens_at_exactly_twenty_four_hours() {
        let claimed = 10_000;
        assert!(!super::repeat_ready(
            claimed,
            claimed + super::COOLDOWN_MS - 1
        ));
        assert!(super::repeat_ready(claimed, claimed + super::COOLDOWN_MS));
        assert!(super::repeat_ready(
            claimed,
            claimed + super::COOLDOWN_MS + 1
        ));
        assert!(!super::repeat_ready(i64::MAX, i64::MAX));
    }
    #[test]
    fn seven_entry_draw_rejects_modulo_bias() {
        let mut counts = [0; 7];
        for byte in 0..252 {
            counts[super::draw_index(&[byte; 32])] += 1;
        }
        assert_eq!(counts, [36; 7]);
        assert!(super::draw_index(&[255; 32]) < 7);
    }
    #[test]
    fn six_chests_have_the_exact_seven_entry_pool() {
        assert_eq!(super::CHESTS.len(), 6);
        assert_eq!(
            super::REWARDS,
            [
                (9081, 1),
                (28821, 1),
                (28823, 1),
                (9058, 1),
                (6299, 1),
                (3052, 1),
                (3035, 10)
            ]
        );
    }
}
