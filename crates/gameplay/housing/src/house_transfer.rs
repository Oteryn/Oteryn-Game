//! Accepted flat House census and custody work bounds (HOUSE-OWN-0 §18).
//! Every native step must preserve its captured reclaim subjects, in item identity order.
pub const CENSUS_MAX: u64 = 2_000;
pub const STEP_MAX: usize = 100;

/// Validate the exact native batch before its audit and custody transaction may commit.
/// The native writer supplies the immutable census subjects; this checks identities and order.
pub fn valid_step(items: &[([u8; 16], [u8; 16])]) -> bool {
    items.len() <= STEP_MAX
        && items.windows(2).all(|pair| pair[0].0 < pair[1].0)
        && items.iter().all(|(item, subject)| {
            [item, subject]
                .iter()
                .all(|id| id[6] >> 4 == 7 && id[8] & 0xc0 == 0x80)
        })
}

#[cfg(test)]
#[path = "house_transfer_tests.rs"]
mod tests;
