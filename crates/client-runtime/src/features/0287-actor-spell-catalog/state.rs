//! oteryn-choice: connection-local catalogue state, shared by the panel and slot bindings.
use super::{SpellCatalog, SpellCatalogEntry};
#[derive(Debug, Clone, Default)]
pub struct CatalogState {
    pub(crate) snapshot: Option<SpellCatalog>,
}
impl CatalogState {
    pub fn replace(&mut self, snapshot: SpellCatalog) {
        self.snapshot = Some(snapshot);
    }
    #[must_use]
    pub fn entries(&self) -> &[SpellCatalogEntry] {
        self.snapshot
            .as_ref()
            .map_or(&[], |snapshot| snapshot.entries.as_slice())
    }
}
