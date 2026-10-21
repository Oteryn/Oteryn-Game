//! Pure chase limits, search budgets and the order of waiting searches.

/// `RANGED0-RL-06` (RANGED-0 §8): at most one search per player per this interval.
pub const CHASE_SEARCH_INTERVAL_MICROS: u64 = 1_000_000;
/// `RANGED0-RL-07` (RANGED-0 §8): a failed search waits this long before the next one (Canary).
pub const CHASE_FAILED_SEARCH_WAIT_MICROS: u64 = 2_000_000;
/// `RANGED0-RL-08`'s TTL: how long a cell the Movement owner refused a chase step into as
/// blocked stays out of the search.
pub const CHASE_REFUSED_CELL_MICROS: u64 = CHASE_FAILED_SEARCH_WAIT_MICROS;
/// `RANGED0-RL-08`: at most this many refused cells are remembered; the oldest is forgotten
/// first.
pub const CHASE_REFUSED_CELLS: usize = 8;

/// `RANGED0-RL-03`: player chase searches per owner window per Channel, served oldest-waiting
/// request first, then actor id.
pub const CHASE_SEARCHES_PER_WINDOW: u32 = 64;
/// The owner window `RANGED0-RL-03` counts in (`CREATUREAI0-RL-09`'s 50 ms window).
pub const CHASE_WINDOW_MICROS: u64 = 50_000;
/// `RANGED0-RL-03`: the owner window whose player chase searches the Channel last drained.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChaseSearchBudget {
    drained: Option<u64>,
}

impl ChaseSearchBudget {
    /// Whether the owner window of `now_us` is still to be drained; if so it counts as drained.
    /// A caller's earlier window never reopens a drained one.
    pub fn open(&mut self, now_us: u64) -> bool {
        let window = now_us / CHASE_WINDOW_MICROS;
        if self.drained.is_some_and(|drained| window <= drained) {
            return false;
        }
        self.drained = Some(window);
        true
    }
}

/// `RANGED0-RL-03`: the order the drain of the owner window of `now_us` serves the waiting
/// searches in, each given as when it began waiting and its actor key: only those that waited
/// from an earlier window, so every search due in one window is known before any is served;
/// the oldest-waiting window first, then actor id. The drain serves at most
/// [`CHASE_SEARCHES_PER_WINDOW`] of them; the rest keep their places for the next window.
pub fn chase_search_order<K: Ord>(
    now_us: u64,
    waiting: impl IntoIterator<Item = (u64, K)>,
) -> Vec<K> {
    let window = now_us / CHASE_WINDOW_MICROS;
    let mut order: Vec<_> = waiting
        .into_iter()
        .map(|(since_us, key)| (since_us / CHASE_WINDOW_MICROS, key))
        .filter(|(since, _)| *since < window)
        .collect();
    order.sort();
    order.into_iter().map(|(_, key)| key).collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn the_refused_cell_cap_matches_its_registered_row() {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
        ))
        .unwrap();
        let entries = registry["entries"].as_array().unwrap();
        let row = entries
            .iter()
            .find(|entry| entry["id"] == "RANGED0-RL-08")
            .unwrap();
        assert_eq!(
            row["hard_maximum"].as_u64(),
            u64::try_from(CHASE_REFUSED_CELLS).ok()
        );
        assert_eq!(CHASE_REFUSED_CELL_MICROS, 2_000_000);
    }

    #[test]
    fn the_channel_drains_each_owner_window_once() {
        let mut budget = ChaseSearchBudget::default();
        assert!(budget.open(10_000));
        assert!(!budget.open(49_999));
        assert!(budget.open(50_000), "a new window is drained once more");
        assert!(
            !budget.open(49_999),
            "a late caller's earlier window never reopens a drained one"
        );
    }

    #[test]
    fn the_drain_serves_the_oldest_waiting_window_first_then_actor_id() {
        let order = chase_search_order(
            100_000,
            [(60_000, 1), (10_000, 3), (40_000, 2), (100_000, 0)],
        );
        assert_eq!(
            order,
            [2, 3, 1],
            "a search registered in the drained window waits for the next"
        );
    }
}
