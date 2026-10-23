//! The existing real command-9 scenario at the booted epoch, mounted once.
use super::*;

#[test]
fn world_reset_map_pickup_dispatch_uses_the_active_epoch_for_views_and_mint() -> TestResult {
    scenario(false, false, 1)
}

#[test]
fn world_reset_map_pickup_dispatch_refuses_cross_epoch_handles_without_writes() -> TestResult {
    scenario(false, true, 1)
}
