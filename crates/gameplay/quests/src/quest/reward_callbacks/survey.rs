//! Native areas selected for a carried detector or thermometer.
use super::SelfTarget;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering::{Equal, Greater, Less};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Survey {
    selection: Selection,
    areas: Vec<SelfTarget>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Selection {
    Personal,
    WorldReset,
}

impl Survey {
    pub(super) fn valid(&self) -> bool {
        (1..=16).contains(&self.areas.len()) && self.areas.iter().all(SelfTarget::valid)
    }

    fn uses_world_reset(&self) -> bool {
        self.selection == Selection::WorldReset
    }

    pub fn clue(
        &self,
        character: &[u8; 16],
        epoch: u64,
        last_report: i64,
        x: i32,
        y: i32,
        floor: i16,
    ) -> Option<&'static str> {
        if self.areas.is_empty() {
            return Some("No reading.");
        }
        let seed = match self.selection {
            Selection::Personal => {
                let mut hash = Sha256::new();
                hash.update(character);
                hash.update(self.areas[0].minimum.floor.to_le_bytes());
                // Keep the first commission stable for characters already tracking.
                if last_report != 0 {
                    hash.update(last_report.to_le_bytes());
                }
                let digest = hash.finalize();
                u64::from(u16::from_le_bytes([digest[0], digest[1]]))
            }
            Selection::WorldReset => epoch,
        };
        let area = &self.areas[(seed % self.areas.len() as u64) as usize];
        if area.contains(x, y, floor) {
            return None;
        }
        if self.uses_world_reset() {
            return Some("This is not the hot spot!");
        }
        let depth = -i16::from(area.minimum.floor);
        if floor != depth {
            return Some(if floor < depth {
                "The detector points below you."
            } else {
                "The detector points above you."
            });
        }
        let dx = i64::from(area.minimum.x) + i64::from(area.maximum.x) - 2 * i64::from(x);
        let dy = i64::from(area.minimum.y) + i64::from(area.maximum.y) - 2 * i64::from(y);
        match (dx.cmp(&0), dy.cmp(&0)) {
            (Equal, Equal) => None,
            (Greater, Equal) => Some("The detector points east."),
            (Less, Equal) => Some("The detector points west."),
            (Equal, Greater) => Some("The detector points south."),
            (Equal, Less) => Some("The detector points north."),
            (Greater, Greater) => Some("The detector points southeast."),
            (Greater, Less) => Some("The detector points northeast."),
            (Less, Greater) => Some("The detector points southwest."),
            (Less, Less) => Some("The detector points northwest."),
        }
    }
}
