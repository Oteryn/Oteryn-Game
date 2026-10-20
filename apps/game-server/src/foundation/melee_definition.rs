/// A qualified physical melee entry; the active artifact supplies its digest.
/// This value is content, never an issuer, current actor or mutation grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MeleeDefinition {
    pub(crate) content_digest: [u8; 32],
    pub(crate) interval_us: u64,
    pub(crate) chance_ppm: u32,
    pub(crate) entry_index: u16,
    pub(crate) minimum: u32,
    pub(crate) maximum: u32,
}
impl MeleeDefinition {
    pub(crate) fn new(
        content_digest: [u8; 32],
        interval_ms: u64,
        chance_ppm: u32,
        minimum: u32,
        maximum: u32,
    ) -> Option<Self> {
        if interval_ms == 0 || chance_ppm > 1_000_000 || maximum == 0 || minimum > maximum {
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
    pub(crate) const fn with_entry_index(mut self, entry_index: u16) -> Self {
        self.entry_index = entry_index;
        self
    }
}
