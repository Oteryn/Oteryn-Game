//! Selected-instance exchange admission, reusing questpc3 9936a44dd's bounded input rule.
use super::{ItemAmount, RewardCallback};

impl RewardCallback {
    pub(super) fn valid_carried_use(&self) -> bool {
        let Some(item) = &self.carried_item else {
            return !self.carried_exchange;
        };
        self.fee.is_none()
            && self.requires_actor_conditions.is_empty()
            && if self.carried_exchange {
                self.give.is_some()
                    && self
                        .take
                        .iter()
                        .any(|take| &take.item == item && take.count == 1)
            } else {
                self.give.is_none()
                    && self.take
                        == [ItemAmount {
                            item: item.clone(),
                            count: 1,
                            fluid_subtype: None,
                        }]
            }
    }
}
