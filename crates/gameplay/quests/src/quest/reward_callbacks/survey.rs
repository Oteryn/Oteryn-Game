//! Native areas selected for a carried detector or thermometer.
use super::SelfTarget;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

    pub fn uses_world_reset(&self) -> bool {
        self.selection == Selection::WorldReset
    }

    pub fn clue(
        &self,
        character: &[u8; 16],
        epoch: u64,
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
        let east = 2 * x < i32::from(area.minimum.x) + i32::from(area.maximum.x);
        let south = 2 * y < i32::from(area.minimum.y) + i32::from(area.maximum.y);
        Some(match (east, south) {
            (true, true) => "The detector points southeast.",
            (true, false) => "The detector points northeast.",
            (false, true) => "The detector points southwest.",
            (false, false) => "The detector points northwest.",
        })
    }
}
