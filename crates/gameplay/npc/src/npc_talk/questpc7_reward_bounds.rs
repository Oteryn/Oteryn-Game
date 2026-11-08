//! Multiple registered sources retain the original per-document admission cap.
pub(super) const PER_DOCUMENT: usize = 256;

pub(super) fn within_registered_total(records: usize, sources: usize) -> bool {
    sources > 0 && records <= PER_DOCUMENT.saturating_mul(sources)
}
