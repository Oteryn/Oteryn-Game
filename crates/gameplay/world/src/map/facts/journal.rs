//! Export root catalogue from admitted placement facts, never from player rows.
use super::BundleFacts;
impl BundleFacts {
    pub fn journal_depot_towns(&self) -> Vec<u16> {
        self.placements
            .iter()
            .filter_map(|(_, p)| p.attrs.depot)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}
