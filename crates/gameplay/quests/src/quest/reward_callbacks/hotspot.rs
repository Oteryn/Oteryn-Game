//! One shared location per World reset. Crystal 201189f8
//! data-global/scripts/quests/spike_tasks/actions_thermometer.lua:1-32.
use super::super::authored_bindings::kills::DeathArea;
use super::RewardCallback;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ServerSaveHotspot {
    pub areas: Vec<DeathArea>,
    pub cold_reply: String,
    pub hot_effect: u8,
}

impl ServerSaveHotspot {
    pub(super) fn valid_for(&self, callback: &RewardCallback) -> bool {
        (2..=32).contains(&self.areas.len())
            && self
                .areas
                .iter()
                .all(|area| area.valid() && area.minimum[2] == area.maximum[2])
            && self.areas.iter().enumerate().all(|(index, area)| {
                self.areas[..index].iter().all(|other| {
                    (0..3).any(|axis| {
                        area.maximum[axis] < other.minimum[axis]
                            || other.maximum[axis] < area.minimum[axis]
                    })
                })
            })
            && self.hot_effect > 0
            && !self.cold_reply.is_empty()
            && self.cold_reply.chars().count() <= 255
            && !self.cold_reply.chars().any(char::is_control)
            && callback.carried_item.is_some()
            && !callback.carried_exchange
            && callback.carried_target.is_none()
            && callback.creature_target.is_none()
            && callback.corpse_target.is_none()
            && callback.self_target.is_none()
            && callback.peer_requires.is_none()
            && !callback.storage.is_empty()
    }

    pub fn contains(&self, key: &str, world: &[u8; 16], epoch: u64, position: [i32; 3]) -> bool {
        let digest = Sha256::new()
            .chain_update(b"quest-server-save-hotspot-v1")
            .chain_update(world)
            .chain_update(epoch.to_be_bytes())
            .chain_update(key.as_bytes())
            .finalize();
        let sample = u16::from_be_bytes([digest[0], digest[1]]);
        let Some(index) = usize::from(sample).checked_rem(self.areas.len()) else {
            return false;
        };
        self.areas[index].contains(position)
    }
}
