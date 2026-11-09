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
        if valid(&self.skill_order, 8, 0) && valid(&self.condition_order, 27, 1) {
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
