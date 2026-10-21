//! House-cell rule for last-position admission (CHAR-POSITION §3.3).

/// A known non-house tile may restore a last position; unknown placement fails closed.
#[must_use]
pub const fn house_cell(placement: Option<bool>) -> bool {
    !matches!(placement, Some(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_non_house_cells_can_restore_a_position() {
        assert!(!house_cell(Some(false)));
        assert!(house_cell(Some(true)));
        assert!(house_cell(None));
    }
}
