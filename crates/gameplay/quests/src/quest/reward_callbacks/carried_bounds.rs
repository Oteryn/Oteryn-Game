//! Local admission alignment with the reused QENGINE2 migration1552 proof.
use super::RewardCallback;

pub(super) fn valid(callback: &RewardCallback) -> bool {
    !callback.carried_exchange
        || ((1..=8).contains(&callback.take.len())
            && callback
                .take
                .iter()
                .all(|item| (1..=100).contains(&item.count)))
}
