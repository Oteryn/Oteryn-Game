//! Basic Oteryn world gathering: alternate reset epochs, until the next reset.
//! The reset owner selects this once; players can only read its announcement.

/// Epoch zero is uninitialized gameplay. Odd epochs announce a gathering; the
/// following even epoch closes it. This is Oteryn content, not a sourced map event.
pub fn announcement_for_epoch(epoch: u64) -> Option<&'static str> {
    (epoch % 2 == 1).then_some("World gathering is active until the next server save.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_world_event_starts_and_stops_only_at_reset() {
        assert_eq!(announcement_for_epoch(0), None);
        assert!(announcement_for_epoch(1).is_some());
        assert_eq!(announcement_for_epoch(2), None);
        assert_eq!(announcement_for_epoch(3), announcement_for_epoch(1));
        assert!(announcement_for_epoch(u64::MAX).is_some());
    }
}
