//! Stable personal target and Crystal's bearing text for a carried detector.
//! Donor 201189f8 data-global/scripts/quests/spike_tasks/actions_ghost_detector.lua:10-103.
use super::super::authored_bindings::kills::DeathArea;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CarriedTracker {
    pub area: DeathArea,
    pub radius: u16,
    pub heading_prefix: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground_search: Option<GroundSearch>,
    /// Same-quest track that changes between assignments and stays fixed during one run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_track: Option<String>,
}

/// First-target search uses exact admitted ground appearances. Crystal
/// data/libs/functions/position.lua:148-161 returns the final draw as fallback.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroundSearch {
    pub attempts: u8,
    pub appearances: Vec<[u16; 2]>,
}

impl GroundSearch {
    pub fn accepts(&self, appearance: u16) -> bool {
        self.appearances
            .iter()
            .any(|[from, to]| (*from..=*to).contains(&appearance))
    }

    fn valid(&self) -> bool {
        (2..=50).contains(&self.attempts)
            && self.appearances.len() <= 8
            && self
                .appearances
                .iter()
                .all(|[from, to]| *from != 0 && from <= to)
    }
}

impl CarriedTracker {
    pub(super) fn valid(&self, callback: &super::RewardCallback) -> bool {
        self.area.valid()
            && self.ground_search.as_ref().map_or_else(
                || self.area.minimum[2] == self.area.maximum[2],
                GroundSearch::valid,
            )
            && (1..=8).contains(&self.radius)
            && !self.heading_prefix.is_empty()
            && self.heading_prefix.chars().count() <= 80
            && !self.heading_prefix.chars().any(char::is_control)
            && super::hotspot::bare_tool(callback)
            && callback.server_save_hotspot.is_none()
            && callback.storage.len() == 1
            && callback.storage[0].minimum == callback.storage[0].maximum
            && callback.storage[0].minimum.checked_add(1) == Some(callback.storage[0].set)
            && self.cycle_track.as_ref().is_none_or(|track| {
                super::super::valid_quest_key(track) && track != &callback.storage[0].track
            })
    }

    pub fn target_at(
        &self,
        key: &str,
        world: &[u8; 16],
        character: &[u8; 16],
        epoch: u64,
        cycle: Option<i64>,
        probe: u8,
    ) -> [i32; 3] {
        let mut hash = Sha256::new()
            .chain_update(b"quest-carried-tracker-v1")
            .chain_update(world)
            .chain_update(character)
            .chain_update(epoch.to_be_bytes());
        if let Some(cycle) = cycle {
            hash.update(cycle.to_be_bytes());
        }
        hash.update(key);
        if self.ground_search.is_some() {
            hash.update([probe]);
        }
        let digest = hash.finalize();
        let coordinate = |axis: usize| {
            let first = self.area.minimum[axis];
            let last = self.area.maximum[axis];
            let sample = i32::from(u16::from_be_bytes([digest[axis * 2], digest[axis * 2 + 1]]));
            first + sample % (last - first + 1)
        };
        std::array::from_fn(coordinate)
    }

    pub fn reached(&self, from: [i32; 3], target: [i32; 3]) -> bool {
        let radius = u32::from(self.radius);
        from[2] == target[2]
            && from[0].abs_diff(target[0]) <= radius
            && from[1].abs_diff(target[1]) <= radius
    }

    pub fn heading(&self, from: [i32; 3], target: [i32; 3]) -> String {
        let [dx, dy, dz] = std::array::from_fn(|i| i64::from(from[i]) - i64::from(target[i]));
        let answer = if dx.unsigned_abs() <= u64::from(self.radius)
            && dy.unsigned_abs() <= u64::from(self.radius)
        {
            match dz.cmp(&0) {
                std::cmp::Ordering::Greater => "above you",
                std::cmp::Ordering::Less => "below you",
                std::cmp::Ordering::Equal => "next to you",
            }
            .to_owned()
        } else {
            let tangent = if dx == 0 { 10.0 } else { dy as f64 / dx as f64 };
            let direction = if tangent.abs() < 0.4142 {
                if dx > 0 { "west" } else { "east" }
            } else if tangent.abs() < 2.4142 {
                match (dx > 0, dy > 0) {
                    (true, true) => "north-west",
                    (false, false) => "south-east",
                    (true, false) => "south-west",
                    (false, true) => "north-east",
                }
            } else if dy > 0 {
                "north"
            } else {
                "south"
            };
            let squared_distance = i128::from(dx).pow(2) + i128::from(dy).pow(2);
            let distance = if squared_distance >= 75625 {
                "very far to the "
            } else if squared_distance >= 10000 {
                "far to the "
            } else if dz > 0 {
                "on a higher level to the "
            } else if dz < 0 {
                "on a lower level to the "
            } else {
                "to the "
            };
            format!("{distance}{direction}")
        };
        format!("{}{answer}.", self.heading_prefix)
    }
}
