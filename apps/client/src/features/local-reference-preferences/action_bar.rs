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
    fn default() -> Self {
        Self {
            auto_insert: false,
            three_rows: false,
            labels: true,
            cooldowns: true,
        }
    }
}
impl ActionBarPreferences {
    pub fn grid(self) -> (usize, usize) {
        if self.three_rows { (4, 3) } else { (12, 1) }
    }
    pub(crate) fn project(self, labels: &mut [String]) {
        for label in labels {
            *label = label
                .lines()
                .enumerate()
                .filter_map(|(i, line)| {
                    let cooldown = line == "Ready"
                        || line == "Empty"
                        || line == "Cooldown ?"
                        || line
                            .strip_suffix(" ms")
                            .is_some_and(|value| value.parse::<u64>().is_ok());
                    ((i != 0 || self.labels) && (!cooldown || self.cooldowns)).then_some(line)
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hiding_cooldowns_retains_amounts_and_custom_binding_hints() {
        let preferences = ActionBarPreferences {
            cooldowns: false,
            ..Default::default()
        };
        let mut labels = vec!["1: Heal\n500 ms\nVisible: 8\nAlt+H".into()];
        preferences.project(&mut labels);
        assert_eq!(labels, ["1: Heal\nVisible: 8\nAlt+H"]);
    }
}
