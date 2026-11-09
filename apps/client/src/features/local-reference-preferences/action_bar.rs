//! Local action-bar display choices do not change casting authority.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActionBarPreferences {
    pub visible: bool,
    pub origin: [u16; 2],
    pub columns: u8,
    pub size: [u16; 2],
    pub gap: u16,
    pub auto_insert: bool,
    pub three_rows: bool,
    pub labels: bool,
    pub cooldowns: bool,
}
impl Default for ActionBarPreferences {
    fn default() -> Self {
        Self {
            visible: true, origin: [8, 120], columns: 4, size: [64, 30], gap: 6,
            auto_insert: false,
            three_rows: false,
            labels: true,
            cooldowns: true,
        }
    }
}
impl ActionBarPreferences {
    pub(super) fn valid(&self) -> bool {
        (1..=4).contains(&self.columns)
            && self.origin.iter().all(|v| *v <= 4096)
            && self.size.iter().all(|v| (24..=160).contains(v))
            && self.gap <= 32
    }
    pub fn rects(&self, viewport: [u32; 2]) -> [[i32; 4]; 4] {
        let columns = u32::from(self.columns).clamp(1, 4);
        let [width, height] = self.size.map(u32::from);
        let gap = u32::from(self.gap);
        let x = u32::from(self.origin[0])
            .min(viewport[0].saturating_sub(columns * (width + gap) - gap));
        let y = u32::from(self.origin[1])
            .min(viewport[1].saturating_sub(4_u32.div_ceil(columns) * (height + gap) - gap));
        std::array::from_fn(|slot| {
            [
                (x + slot as u32 % columns * (width + gap)) as i32,
                (y + slot as u32 / columns * (height + gap)) as i32,
                width as i32,
                height as i32,
            ]
        })
    }

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
