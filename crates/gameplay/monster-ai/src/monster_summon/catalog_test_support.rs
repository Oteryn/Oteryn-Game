//! Pin substitution for negative authority tests, preserving all other catalog facts.
use super::*;
impl NativeSummonCatalog {
    pub fn requalified_for_test(&self, content: [u8; 32]) -> Result<Self, SummonError> {
        let mut catalog = self.clone();
        catalog.content = content;
        for spec in catalog.entries.iter_mut().flatten() {
            spec.content = content;
            spec.admission = NativeSummonAdmissionSpec::qualified(
                &spec.parent.key,
                &spec.child.key,
                spec.health,
                spec.index,
                spec.total,
                spec.count,
                content,
            )
            .map_err(SummonError::Carrier)?;
        }
        Ok(catalog)
    }
}
