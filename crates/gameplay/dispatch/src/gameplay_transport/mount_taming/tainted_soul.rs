//! Timing and credit for Sacred Antler Talisman redemption.

use std::time::Duration;

pub(super) struct RedemptionTiming {
    pub(super) credit: u8,
    pub(super) tainted_remove_after: Duration,
    pub(super) redeemed_remove_after: Option<Duration>,
}

pub(super) fn redemption_timing(turn: Duration, has_spawn_room: bool) -> RedemptionTiming {
    RedemptionTiming {
        credit: 1,
        tainted_remove_after: turn,
        redeemed_remove_after: has_spawn_room.then_some(Duration::from_secs(30)),
    }
}

#[cfg(test)]
mod tests {
    use super::redemption_timing;
    use std::time::Duration;

    #[test]
    fn tainted_soul_redemption_counts_without_room_and_bounds_actor_lifetimes() {
        let turn = Duration::from_millis(200);
        let blocked = redemption_timing(turn, false);
        assert_eq!(blocked.credit, 1);
        assert_eq!(blocked.tainted_remove_after, turn);
        assert_eq!(blocked.redeemed_remove_after, None);

        let open = redemption_timing(turn, true);
        assert_eq!(open.credit, blocked.credit);
        assert_eq!(open.tainted_remove_after, turn);
        assert_eq!(open.redeemed_remove_after, Some(Duration::from_secs(30)));
    }
}
