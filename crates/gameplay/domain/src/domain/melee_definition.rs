/// A qualified physical melee entry; the active artifact supplies its digest.
/// This value is content, never an issuer, current actor or mutation grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeleeDefinition {
    pub content_digest: [u8; 32],
    pub interval_us: u64,
    pub chance_ppm: u32,
    pub entry_index: u16,
    pub minimum: u32,
    pub maximum: u32,
}
impl MeleeDefinition {
    pub fn new(
        content_digest: [u8; 32],
        interval_ms: u64,
        chance_ppm: u32,
        minimum: u32,
        maximum: u32,
    ) -> Option<Self> {
        if maximum == 0 {
            return None;
        }
        Self::new_native(content_digest, interval_ms, chance_ppm, minimum, maximum)
    }

    /// Native authored zero damage still attempts a melee and meets the target's blocks.
    /// This is content validation only; the native owner separately verifies MeleeSource
    /// and the current committed AI selection. Strict legacy admission keeps rejecting zero.
    pub fn new_native(
        content_digest: [u8; 32],
        interval_ms: u64,
        chance_ppm: u32,
        minimum: u32,
        maximum: u32,
    ) -> Option<Self> {
        if interval_ms == 0 || chance_ppm > 1_000_000 || minimum > maximum {
            return None;
        }
        Some(Self {
            content_digest,
            interval_us: interval_ms.checked_mul(1_000)?,
            chance_ppm,
            entry_index: 0,
            minimum,
            maximum,
        })
    }

    /// The melee entry's index in the profile's `attacks[]`: the `AI_ATTACK` draw index.
    #[must_use]
    pub const fn with_entry_index(mut self, entry_index: u16) -> Self {
        self.entry_index = entry_index;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::MeleeDefinition;

    #[test]
    fn native_zero_melee_preserves_authored_interval_without_loosening_legacy_admission() {
        assert!(MeleeDefinition::new([1; 32], 2000, 1_000_000, 0, 0).is_none());
        let source = MeleeDefinition::new_native([1; 32], 2000, 1_000_000, 0, 0);
        assert_eq!(source.map(|value| value.interval_us), Some(2_000_000));
        assert!(MeleeDefinition::new_native([1; 32], 0, 1_000_000, 0, 0).is_none());
        assert!(MeleeDefinition::new_native([1; 32], 2000, 1_000_001, 0, 0).is_none());
        assert!(MeleeDefinition::new_native([1; 32], 2000, 1_000_000, 1, 0).is_none());
    }
}
