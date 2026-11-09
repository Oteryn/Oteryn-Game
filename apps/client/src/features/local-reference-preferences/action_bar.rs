//! Local action-bar display choices do not change casting authority.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActionBarPreferences {
    pub auto_insert: bool,
    pub three_rows: bool,
    pub labels: bool,
    pub cooldowns: bool,
}
impl Default for ActionBarPreferences {
    fn default() -> Self { Self { auto_insert: false, three_rows: false, labels: true, cooldowns: true } }
}
impl ActionBarPreferences {
    pub fn grid(self) -> (usize,usize) { if self.three_rows { (4,3) } else { (12,1) } }
    pub(crate) fn project(self, labels: &mut [String]) {
        for label in labels {
            if !self.cooldowns { label.truncate(label.find('\n').unwrap_or(label.len())); }
            if !self.labels {
                *label = label.split_once('\n').map_or_else(String::new, |(_, state)| state.to_owned());
            }
        }
    }
}
