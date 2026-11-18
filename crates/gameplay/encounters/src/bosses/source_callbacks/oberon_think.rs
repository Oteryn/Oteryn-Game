//! Crystal grand_master_oberon.lua:125-136 and grand_master_oberon_functions.lua:43-60.

pub const OBERON_KEY: &str = "oteryn:creature.grand_master_oberon";
pub const OBERON_LIVES: u8 = 3;
pub const OBERON_HEAL_THRESHOLD_PERCENT: u64 = 20;
pub const OBERON_ASKING: [&str; 9] = [
    "You appear like a worm among men!",
    "The world will suffer for its iddle laziness!",
    "People fall at my feet when they see me coming!",
    "This will be the end of mortal man!",
    "I will remove you from this plane of existence!",
    "Dragons will soon rule this world, I am their herald!",
    "The true virtue of chivalry are my belief!",
    "I lead the most honourable and formidable following of knights!",
    "ULTAH SALID'AR, ESDO LO!",
];
pub const OBERON_SUMMONS: [&str; 2] = [
    "oteryn:creature.falcon_knight",
    "oteryn:creature.falcon_paladin",
];

// Exact case-insensitive response digests, Crystal grand_master_oberon_functions.lua:1-28.
const REPLY_GROUPS: [&str; 2] = [
    "e6332c37255759c0f2ee62f8ca1b59fc615a70918c20c36d271727f3baf01414",
    "70ed3a32ff5240592f9129ed47d7f49a5d9ef57445ccd69e3131ef206124bd62",
];
const REPLIES: [(&str, usize); 9] = [
    (
        "f0bf8e1d376556f5ea29532334cf994cfcdb129a1d1627656f853a04f2fa0cd5",
        1,
    ),
    (
        "7d3dca770c3988b28ebe286b1833f72b66b8267007798f81f13ec5dc32c70060",
        0,
    ),
    (
        "c4c6dbe3d7ad37e9a127b6b919face6b0da54e7449da3e3901b6ef1fe1adbc84",
        1,
    ),
    (
        "52bdc5fdb8d61329c4dbe19add2db4a347251147d025281b603741099f281354",
        0,
    ),
    (
        "cf08c4c240949ac89d626dd0a6a45e4e4e5e12bf062c6e00012158f9b4b35791",
        1,
    ),
    (
        "8001f7512d69cb5ce96629e96b2547f9cc39106bb217c8b44ee4d0d621e73c02",
        1,
    ),
    (
        "cb4c33ec2ee11daf8e0baff5e1c1b25036c4958e5565255fd877d306b9f661d2",
        0,
    ),
    (
        "f8deb48b21dbdcdee362614686dfd55c41efe1d160ad8e4ec23da179bc315ca8",
        1,
    ),
    (
        "afbd908a30a7d9184635dbc6b49eea65c8a0a759543453117eb145a993d1ec55",
        1,
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OberonEffect {
    /// Register the OberonImmunity health-change visual (HOLYAREA) until a correct reply.
    RegisterImmunityEvent,
    /// Monster speech (TALKTYPE_MONSTER_SAY).
    Say {
        text: &'static str,
    },
    HealToFull,
    /// Created on Oberon's tile with extended, forced placement.
    Summon {
        creature_key: &'static str,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OberonRefusal {
    InvalidDraw,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OberonThinkOwner {
    lives_used: u8,
    asking: Option<u8>,
    immune: bool,
    exhaust_until_seconds: u64,
}

impl OberonThinkOwner {
    /// One think. `draw(n)` returns a value in `0..n` (refused otherwise); it is called first
    /// with 9 (taunt) and then with 2 (summon), only when the asking fires.
    pub fn think(
        &mut self,
        health: u64,
        max_health: u64,
        mut draw: impl FnMut(u32) -> u32,
    ) -> Result<Vec<OberonEffect>, OberonRefusal> {
        if self.lives_used >= OBERON_LIVES
            || max_health == 0
            || u128::from(health) * 100
                > u128::from(OBERON_HEAL_THRESHOLD_PERCENT) * u128::from(max_health)
        {
            return Ok(vec![]);
        }

        let taunt = draw(OBERON_ASKING.len() as u32);
        if taunt >= OBERON_ASKING.len() as u32 {
            return Err(OberonRefusal::InvalidDraw);
        }
        let summon = draw(OBERON_SUMMONS.len() as u32);
        if summon >= OBERON_SUMMONS.len() as u32 {
            return Err(OberonRefusal::InvalidDraw);
        }

        self.asking = Some(taunt as u8 + 1);
        self.immune = true;
        self.lives_used += 1;

        Ok(vec![
            OberonEffect::RegisterImmunityEvent,
            OberonEffect::Say {
                text: OBERON_ASKING[taunt as usize],
            },
            OberonEffect::HealToFull,
            OberonEffect::Summon {
                creature_key: OBERON_SUMMONS[summon as usize],
            },
        ])
    }

    /// A nearby player's ordinary say consumes the boss's shared one-second response window.
    pub fn answer(&mut self, message: &str, now_seconds: u64) -> Option<bool> {
        if now_seconds < self.exhaust_until_seconds {
            return None;
        }
        self.exhaust_until_seconds = now_seconds.saturating_add(1);
        let index = usize::from(self.asking? - 1);
        let (reply, group) = REPLIES[index];
        use sha2::{Digest, Sha256};
        let digest: String = Sha256::digest(message.to_ascii_lowercase())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let accepted = digest == reply || digest == REPLY_GROUPS[group];
        if accepted {
            self.immune = false;
        }
        Some(accepted)
    }

    pub const fn immune(&self) -> bool {
        self.immune
    }

    #[must_use]
    pub const fn lives_used(&self) -> u8 {
        self.lives_used
    }

    /// The donor `Asking` storage: 1-based index into OBERON_ASKING of the last taunt.
    #[must_use]
    pub const fn asking(&self) -> Option<u8> {
        self.asking
    }
}

#[cfg(test)]
#[path = "oberon_think_tests.rs"]
mod tests;
