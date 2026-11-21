//! Bounded read-only source slots, distinct from permission to insert into a container.

/// A source view never truncates captured children or invents portable container semantics.
pub fn captured_source_capacity(
    native_capacity: Option<u32>,
    pickupable: bool,
    captured_children: usize,
    view_limit: usize,
) -> Option<u8> {
    let slots = match native_capacity {
        Some(capacity) => usize::try_from(capacity).ok()?.min(view_limit),
        None if !pickupable && captured_children > 0 => captured_children,
        None => return None,
    };
    if slots == 0 || slots > view_limit || captured_children > slots {
        return None;
    }
    u8::try_from(slots).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_captured_source_needs_no_portable_item_custody_policy() {
        assert_eq!(captured_source_capacity(None, false, 1, 20), Some(1));
        assert_eq!(captured_source_capacity(None, false, 3, 20), Some(3));
    }

    #[test]
    fn an_unknown_portable_or_empty_noncontainer_never_gets_a_capacity() {
        assert_eq!(captured_source_capacity(None, true, 1, 20), None);
        assert_eq!(captured_source_capacity(None, false, 0, 20), None);
    }

    #[test]
    fn source_views_keep_declared_capacity_within_the_protocol_window() {
        assert_eq!(captured_source_capacity(Some(20), true, 0, 20), Some(20));
        assert_eq!(captured_source_capacity(Some(25), false, 3, 20), Some(20));
        assert_eq!(captured_source_capacity(Some(25), false, 21, 20), None);
        assert_eq!(captured_source_capacity(Some(0), false, 1, 20), None);
    }
}
