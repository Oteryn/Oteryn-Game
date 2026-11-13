//! A captured source can be opened without granting portable custody.

/// Returns `(movable, openable_source)` for one immutable base entry.
pub(super) fn base_handles(
    pickupable: bool,
    bound: bool,
    contents: bool,
    house: bool,
) -> (bool, bool) {
    (
        pickupable && !bound && !contents && !house,
        contents && !bound && !house,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_captured_source_has_an_open_handle_without_move_permission() {
        assert_eq!(base_handles(false, false, true, false), (false, true));
    }

    #[test]
    fn full_portable_container_is_openable_but_not_a_leaf_pickup() {
        assert_eq!(base_handles(true, false, true, false), (false, true));
    }

    #[test]
    fn bound_and_house_sources_never_gain_shared_container_handles() {
        for pickupable in [false, true] {
            for contents in [false, true] {
                assert_eq!(
                    base_handles(pickupable, true, contents, false),
                    (false, false)
                );
                assert_eq!(
                    base_handles(pickupable, false, contents, true),
                    (false, false)
                );
            }
        }
    }

    #[test]
    fn ordinary_movable_and_fixed_empty_entries_keep_their_policy() {
        assert_eq!(base_handles(true, false, false, false), (true, false));
        assert_eq!(base_handles(false, false, false, false), (false, false));
    }
}
