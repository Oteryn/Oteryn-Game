//! Each registered document retains its original closed admission bound.
pub(super) const PER_DOCUMENT: usize = 256;

pub(super) fn combined_capacity(documents: usize) -> Result<usize, String> {
    if documents == 0 || documents > super::DOCUMENTS.len() {
        return Err("reward callback document bounds".into());
    }
    PER_DOCUMENT
        .checked_mul(documents)
        .ok_or_else(|| "reward callback bounds".into())
}
