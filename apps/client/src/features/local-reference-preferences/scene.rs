//! The character preference key comes from the admitted server observation.
impl crate::scene::Scene {
    pub(crate) const fn preference_character(&self) -> Option<[u8; 16]> {
        self.own_identity
    }
}
