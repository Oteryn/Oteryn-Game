//! Own actor party effects, using the existing server-owned shared-experience shields.
pub const CAPABILITY_PARTY_EFFECTS_V1: u32 = 280;
pub const COMMAND_TYPE_PARTY_EFFECTS_QUERY: u32 = 496;
pub const MAX_PARTY_EFFECTS_REPLY_BYTES: usize = 41;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PartyEffects {
    pub in_party: bool,
    pub shared_enabled: bool,
    pub shared_active: bool,
    pub eligible: bool,
    /// Four percentages: damage reduction, auto attack, spell damage, spell healing;
    /// then the additive Guiding Presence Mantra points (not a percentage).
    pub bonuses: [u32; 5],
    /// Train, Protect, Enchant, Heal, Enlighten Party; zero means inactive.
    pub party_spell_remaining_ms: [u32; 5],
}
impl PartyEffects {
    fn valid(&self) -> bool {
        (self.in_party || !(self.shared_enabled || self.shared_active || self.eligible))
            && (!self.shared_active || (self.shared_enabled && self.eligible))
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let mut bytes = Vec::with_capacity(MAX_PARTY_EFFECTS_REPLY_BYTES);
        bytes.push(
            u8::from(self.in_party)
                | (u8::from(self.shared_enabled) << 1)
                | (u8::from(self.shared_active) << 2)
                | (u8::from(self.eligible) << 3),
        );
        for bonus in self
            .bonuses
            .into_iter()
            .chain(self.party_spell_remaining_ms)
        {
            bytes.extend_from_slice(&bonus.to_le_bytes());
        }
        Some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != MAX_PARTY_EFFECTS_REPLY_BYTES || bytes[0] & !15 != 0 {
            return None;
        }
        let flags = bytes[0];
        let mut words = [0; 10];
        for (bonus, word) in words.iter_mut().zip(bytes[1..].as_chunks::<4>().0) {
            *bonus = u32::from_le_bytes(*word);
        }
        let value = Self {
            in_party: flags & 1 != 0,
            shared_enabled: flags & 2 != 0,
            shared_active: flags & 4 != 0,
            eligible: flags & 8 != 0,
            bonuses: words[..5].try_into().ok()?,
            party_spell_remaining_ms: words[5..].try_into().ok()?,
        };
        value.valid().then_some(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn party_effects_codec_preserves_authoritative_flags_and_bonuses() -> Result<(), &'static str> {
        let value = PartyEffects {
            in_party: true,
            shared_enabled: true,
            shared_active: false,
            eligible: true,
            bonuses: [6, 12, 6, 12, 50],
            party_spell_remaining_ms: [0, 120000, 0, 0, 0],
        };
        let bytes = value.encode().ok_or("encode")?;
        assert_eq!(PartyEffects::decode(&bytes), Some(value));
        for end in 0..bytes.len() {
            assert!(PartyEffects::decode(&bytes[..end]).is_none());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(PartyEffects::decode(&trailing).is_none());
        let mut bad = bytes;
        bad[0] = 4;
        assert!(PartyEffects::decode(&bad).is_none());
        assert!(PartyEffects::decode(&[16; 41]).is_none());
        Ok(())
    }
}
