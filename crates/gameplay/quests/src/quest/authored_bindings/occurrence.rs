//! Explicit save-scoped authored missions; the durable writer supplies current authority.
use super::{BoundQuest, QuestTransition, RewardCallback};
use crate::quest::QuestComparison;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Occurrence {
    ServerSave {
        world_change: String,
        restart: Box<RewardCallback>,
    },
}

impl Occurrence {
    pub(super) fn restart(&self) -> &RewardCallback {
        match self {
            Self::ServerSave { restart, .. } => restart,
        }
    }
    pub(super) fn gate(&self) -> &str {
        match self {
            Self::ServerSave { world_change, .. } => world_change,
        }
    }
    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        let callback = self.restart();
        if self.gate().is_empty()
            || self.gate().len() > 128
            || !self
                .gate()
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_@".contains(&b))
            || callback.storage.is_empty()
            || callback.storage.len() > crate::quest::QUESTSTATE0_RL_02
        {
            return Err("authored save occurrence gate/reset bounds".into());
        }
        let mut tracks = BTreeSet::new();
        for storage in &callback.storage {
            let declared = quest
                .tracks
                .iter()
                .find(|track| track.key == storage.track)
                .ok_or("save reset requires an owned track")?;
            if !tracks.insert(&storage.track)
                || storage.minimum != declared.minimum
                || storage.maximum != declared.maximum
                || storage.set != declared.initial
                || storage.preserve
                || storage.increment
                || storage.at_least
            {
                return Err("save reset must restore declared mission defaults".into());
            }
        }
        if !callback.is_save_restart_only() {
            return Err("authored save restart must be a pure owned NPC reset".into());
        }
        crate::quest::reward_callbacks::validate_callbacks(std::slice::from_ref(callback))
    }
    pub(super) fn qualify(&self, transition: &mut QuestTransition) {
        transition.world_change = Some(self.gate().into());
    }
    pub(super) fn transition(&self) -> Result<QuestTransition, String> {
        let mut transition = crate::quest::reward_callbacks::primary_transition(self.restart())?;
        for effect in &mut transition.effects {
            effect.from = QuestComparison::Any;
        }
        self.qualify(&mut transition);
        Ok(transition)
    }
}

/// Personal mission policy from Crystal 201189f8. Shared event recurrence never
/// invents a personal requirement or turns an absent donor mission into an offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersonalEventProfile {
    Unavailable { source: &'static str, line: u32 },
    SharedWorldOnly { source: &'static str, line: u32 },
    Counter { helper: &'static str },
}
const STORAGES: &str =
    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/lib/core/storages.lua";

/// All eighteen M55 event identities. Empty donor entries are typed unavailable;
/// unrelated Item/taming cooldowns never manufacture personal event missions.
pub fn personal_event_profile(quest: &str) -> Option<PersonalEventProfile> {
    use PersonalEventProfile as P;
    let unavailable = |line| P::Unavailable {
        source: STORAGES,
        line,
    };
    Some(match quest {
        "oteryn:quest.awash_world_change" => P::SharedWorldOnly {
            source: "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/dronk.lua",
            line: 53,
        },
        "oteryn:quest.authored.bank_robbery_mini_world_change" => unavailable(1881),
        // Wyda's separate BloodHerbQuest dialogue is not this empty event mission.
        "oteryn:quest.bored_mini_world_change" => unavailable(1882),
        "oteryn:quest.authored.chakoya_iceberg_mini_world_change" => unavailable(2263),
        // Tinder Box actions_reward.lua:11-16 is a separate Item cooldown.
        "oteryn:quest.chyllfroest_mini_world_change" => unavailable(2149),
        "oteryn:quest.deeplings_worldchange" => P::Counter {
            helper: crate::quest::shared_world_progress::DEEPLING_QUEST,
        },
        "oteryn:quest.authored.demon_wars_world_change" => P::SharedWorldOnly {
            source: STORAGES,
            line: 1821,
        },
        "oteryn:quest.authored.devovorga_s_essence_mini_world_change" => unavailable(1963),
        "oteryn:quest.authored.down_the_drain_mini_world_change" => unavailable(1884),
        "oteryn:quest.authored.fire_from_the_earth_mini_world_change" => unavailable(1885),
        "oteryn:quest.fury_gates_mini_world_change" => P::SharedWorldOnly {
            source: "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/world_changes/fury_gates.lua",
            line: 18,
        },
        // Orockle's mission dialogue is commented out: npc/orockle.lua:56-70.
        "oteryn:quest.authored.hive_born_world_change" => unavailable(1886),
        "oteryn:quest.authored.hive_outpost_mini_world_change" => unavailable(2271),
        "oteryn:quest.horse_station_world_change" => P::SharedWorldOnly {
            source: "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/globalevents/spawn/horse_station_world_change.lua",
            line: 58,
        },
        "oteryn:quest.authored.insectoid_invasion_world_change" => P::SharedWorldOnly {
            source: STORAGES,
            line: 1880,
        },
        "oteryn:quest.authored.jungle_camp_mini_world_change" => unavailable(2272),
        "oteryn:quest.authored.kingsday_mini_world_change" => unavailable(1888),
        "oteryn:quest.authored.lumberjack_mini_world_change" => unavailable(1893),
        _ => return None,
    })
}

/// Helpers retain their durable identity; only their guarded completion can
/// complete the corresponding canonical quest. This mapping grants no progress.
pub fn counter_canonical_quest(helper: &str) -> Option<&'static str> {
    match helper {
        crate::quest::shared_world_progress::HIVE_QUEST => {
            Some("oteryn:quest.authored.hive_born_world_change")
        }
        crate::quest::shared_world_progress::DEEPLING_QUEST => {
            Some("oteryn:quest.deeplings_worldchange")
        }
        _ => None,
    }
}

/// The current source-cycle snapshot backs both helper and canonical observers.
pub fn counter_helper_quest(quest: &str) -> Option<&'static str> {
    use crate::quest::shared_world_progress::{DEEPLING_QUEST, HIVE_QUEST};
    match quest {
        HIVE_QUEST | "oteryn:quest.authored.hive_born_world_change" => Some(HIVE_QUEST),
        DEEPLING_QUEST | "oteryn:quest.deeplings_worldchange" => Some(DEEPLING_QUEST),
        _ => None,
    }
}

/// Do not advertise absent donor missions through the native NPC adapter.
pub fn personal_event_offered(callback: &RewardCallback) -> bool {
    !matches!(
        personal_event_profile(&callback.quest),
        Some(
            PersonalEventProfile::Unavailable { .. } | PersonalEventProfile::SharedWorldOnly { .. }
        )
    ) && crate::quest::shared_world_progress::accepted_task(&callback.key).is_none()
}
