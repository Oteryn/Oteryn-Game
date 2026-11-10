//! Local presentation order never rewrites the server's skill or condition snapshots.
use super::{PreferenceError, ReferencePreferences};

fn valid(order: &[u8], count: u8, first: u8) -> bool {
    order.is_empty()
        || (order.len() == usize::from(count)
            && (first..first + count)
                .all(|id| order.iter().filter(|&&value| value == id).count() == 1))
}

impl ReferencePreferences {
    pub(super) fn validate_status_order(&self) -> Result<(), PreferenceError> {
        let keys = Self::special_condition_keys();
        let valid_keys = |values: &[String]| {
            values
                .iter()
                .enumerate()
                .all(|(index, key)| keys.contains(key) && !values[..index].contains(key))
        };
        if valid(&self.skill_order, 8, 0)
            && valid(&self.condition_order, 27, 1)
            && valid_keys(&self.condition_status_hidden)
            && valid_keys(&self.condition_hud_hidden)
            && (self.special_condition_order.is_empty()
                || (self.special_condition_order.len() == keys.len()
                    && valid_keys(&self.special_condition_order)))
        {
            Ok(())
        } else {
            Err(PreferenceError)
        }
    }

    /// Move the selected display row, preserving every server-owned identity.
    pub fn move_status_entry(&mut self, skills: bool, index: usize, down: bool) -> bool {
        let (order, first, count) = if skills {
            (&mut self.skill_order, 0, 8)
        } else {
            (&mut self.condition_order, 1, 27)
        };
        let Some(next) = (if down {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        }) else {
            return false;
        };
        if index >= usize::from(count) || next >= usize::from(count) {
            return false;
        }
        if order.is_empty() {
            order.extend(first..first + count);
        }
        if !valid(order, count, first) {
            return false;
        }
        order.swap(index, next);
        true
    }
}

impl ReferencePreferences {
    pub fn special_condition_keys() -> Vec<String> {
        (1..=27)
            .map(|icon| oteryn_client_ui::condition_status::icon_key(icon).to_owned())
            .chain(["Resting Area", "Soul War", "Rotten Blood"].map(str::to_owned))
            .collect()
    }
    pub fn ordered_condition_keys(&self) -> Vec<String> {
        if !self.special_condition_order.is_empty() {
            return self.special_condition_order.clone();
        }
        let order: Vec<_> = if self.condition_order.is_empty() {
            (1..=27).collect()
        } else {
            self.condition_order.clone()
        };
        order
            .into_iter()
            .map(|icon| oteryn_client_ui::condition_status::icon_key(icon).to_owned())
            .chain(["Resting Area", "Soul War", "Rotten Blood"].map(str::to_owned))
            .collect()
    }
    pub fn move_special_condition(&mut self, index: usize, down: bool) -> bool {
        let mut order = self.ordered_condition_keys();
        let Some(next) = (if down {
            index.checked_add(1)
        } else {
            index.checked_sub(1)
        }) else {
            return false;
        };
        if index >= order.len() || next >= order.len() {
            return false;
        }
        order.swap(index, next);
        self.special_condition_order = order;
        true
    }
    pub fn set_condition_visible(&mut self, key: &str, hud: bool, visible: bool) {
        if !Self::special_condition_keys()
            .iter()
            .any(|value| value == key)
        {
            return;
        }
        let hidden = if hud {
            &mut self.condition_hud_hidden
        } else {
            &mut self.condition_status_hidden
        };
        hidden.retain(|value| value != key);
        if !visible {
            hidden.push(key.to_owned());
        }
    }
}
