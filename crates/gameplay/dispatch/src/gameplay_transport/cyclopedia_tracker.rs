//! P22: Cyclopedia Bestiary (383) and Bosstiary (384) tracker commands.
//!
//! Crystal `parseCyclopediaMonsterTracker` (protocolgame.cpp:2782-2816) acts only on a known
//! boss (with a credited kill) or a known Bestiary race; every accepted change answers with the
//! refreshed list (player.cpp:836, :857), built as `refreshCyclopediaMonsterTracker`
//! (protocolgame.cpp:3171-3209). A stored creature the loaded content no longer knows is left
//! out of the refresh, as Crystal's load skips it (iologindata_load_player.cpp:556, :973).
//!
//! State domain 98 pushes both trackers on join (player.cpp:11560-11563) and one tracker after
//! a credited kill of a creature on it (iobestiary.cpp:504-507, io_bosstiary.cpp:194).
use std::collections::BTreeSet;
use std::sync::Mutex;

use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::content::creature_reward::CreatureRewardTable;
use crate::domain::CharacterId;
use crate::domain::bosstiary::{kill_thresholds, progress};
use crate::durability::cyclopedia_tracker::{TrackedCreature, TrackerKind};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use oteryn_protocol_oteryn::cyclopedia_tracker::{
    COMMAND_TYPE_BESTIARY_TRACKER, COMMAND_TYPE_BOSSTIARY_TRACKER, TrackerEntry, TrackerRequest,
    TrackerSnapshot,
};

macro_rules! authority_methods {
    () => {
        fn cyclopedia_tracker(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _command_type: u32,
            _request: oteryn_protocol_oteryn::cyclopedia_tracker::TrackerRequest,
        ) -> impl std::future::Future<
            Output = Option<oteryn_protocol_oteryn::cyclopedia_tracker::TrackerSnapshot>,
        > {
            async { None }
        }

        /// The domain 98 join snapshot; `None` is a server fault.
        fn cyclopedia_tracker_snapshot(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }

        /// The trackers to push after settled kills; consumes the session's kill marks even
        /// when domain 98 is not `selected`.
        fn cyclopedia_tracker_refresh(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _selected: bool,
        ) -> impl std::future::Future<
            Output = Vec<oteryn_protocol_oteryn::cyclopedia_tracker::TrackerSnapshot>,
        > {
            async { Vec::new() }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn cyclopedia_tracker(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            command_type: u32,
            request: oteryn_protocol_oteryn::cyclopedia_tracker::TrackerRequest,
        ) -> Option<oteryn_protocol_oteryn::cyclopedia_tracker::TrackerSnapshot> {
            self.execute_cyclopedia_tracker(admitted, command_type, request)
                .await
        }

        async fn cyclopedia_tracker_snapshot(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
        ) -> Option<Vec<u8>> {
            self.cyclopedia_tracker_domain_snapshot(admitted).await
        }

        async fn cyclopedia_tracker_refresh(
            &self,
            admitted: $crate::gameplay_transport::connection::AdmittedSession,
            selected: bool,
        ) -> Vec<oteryn_protocol_oteryn::cyclopedia_tracker::TrackerSnapshot> {
            self.refresh_cyclopedia_tracker(admitted, selected).await
        }
    };
}
pub(super) use composed_methods;

/// P22d: the tracker entries whose credited kill was settled since the Character's last
/// refresh. A leaf lock, never held across an await.
#[derive(Default)]
pub struct TrackerRefresh(Mutex<BTreeSet<(CharacterId, bool, String)>>);

impl TrackerRefresh {
    pub(super) fn mark(&self, character: CharacterId, kind: TrackerKind, key: &str) {
        if let Ok(mut marked) = self.0.lock() {
            marked.insert((character, kind == TrackerKind::Bosstiary, key.to_owned()));
        }
    }

    /// Removes and returns the Character's marked keys of each kind (Bestiary, Bosstiary).
    fn take(&self, character: CharacterId) -> [Vec<String>; 2] {
        let mut taken = [Vec::new(), Vec::new()];
        if let Ok(mut marked) = self.0.lock() {
            marked.retain(|(marked_character, boss, key)| {
                if *marked_character != character {
                    return true;
                }
                taken[usize::from(*boss)].push(key.clone());
                false
            });
        }
        taken
    }
}

/// The content entry's stage thresholds and whether `kills` completes it, or `None` when the
/// loaded content has no such Bestiary race or boss.
fn stages(
    rewards: &CreatureRewardTable,
    kind: TrackerKind,
    key: &str,
    kills: u64,
) -> Option<([u32; 3], bool)> {
    match kind {
        TrackerKind::Bestiary => {
            let row = rewards.row(key).ok()?;
            let thresholds: [u32; 3] = row.race.as_ref()?.kill_thresholds().try_into().ok()?;
            // Crystal getKillStatus == 4: every stage reached (protocolgame.cpp:3200).
            Some((thresholds, kills >= u64::from(thresholds[2])))
        }
        TrackerKind::Bosstiary => {
            let category = &rewards.boss(key)?.category;
            let [first, second, third] = kill_thresholds(category)?;
            let thresholds = [
                u32::try_from(first).ok()?,
                u32::try_from(second).ok()?,
                u32::try_from(third).ok()?,
            ];
            // Crystal getBossCurrentLevel == 3 (protocolgame.cpp:3195).
            Some((thresholds, progress(category, kills)?.stage == 3))
        }
    }
}

fn snapshot(
    rewards: &CreatureRewardTable,
    kind: TrackerKind,
    tracked: Vec<TrackedCreature>,
) -> TrackerSnapshot {
    TrackerSnapshot {
        boss: kind == TrackerKind::Bosstiary,
        entries: tracked
            .into_iter()
            .filter_map(|creature| {
                let (thresholds, completed) =
                    stages(rewards, kind, &creature.creature_key, creature.kill_count)?;
                Some(TrackerEntry {
                    kill_count: u32::try_from(creature.kill_count).unwrap_or(u32::MAX),
                    creature_key: creature.creature_key,
                    thresholds,
                    completed,
                })
            })
            .collect(),
    }
}

/// The session's item fence, when it is still the current connection's.
fn current_fence(admitted: &AdmittedSession) -> Option<CurrentCharacterItemFence> {
    let fence = admitted.item_fence?;
    (fence.game_session_id == admitted.game_session_id
        && fence.connection_generation.get() == admitted.continuity.connection_generation)
        .then_some(fence)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn read_cyclopedia_tracker(
        &self,
        fence: CurrentCharacterItemFence,
        kind: TrackerKind,
    ) -> Option<TrackerSnapshot> {
        let _permit = self.spell_lane_permit().await?;
        let tracked = self
            .root
            .cyclopedia_tracker(self.character, self.holder, fence, kind, None)
            .await
            .ok()??;
        Some(snapshot(self.reward_table(), kind, tracked))
    }

    pub(in crate::gameplay_transport) async fn cyclopedia_tracker_domain_snapshot(
        &self,
        admitted: AdmittedSession,
    ) -> Option<Vec<u8>> {
        let fence = current_fence(&admitted)?;
        let bestiary = self
            .read_cyclopedia_tracker(fence, TrackerKind::Bestiary)
            .await?;
        let bosstiary = self
            .read_cyclopedia_tracker(fence, TrackerKind::Bosstiary)
            .await?;
        oteryn_protocol_oteryn::cyclopedia_tracker::encode_domain_snapshot(&bestiary, &bosstiary)
    }

    /// A tracker is pushed when one of its marked kills is on it. A failed read keeps the
    /// marks for the next cadence.
    pub(in crate::gameplay_transport) async fn refresh_cyclopedia_tracker(
        &self,
        admitted: AdmittedSession,
        selected: bool,
    ) -> Vec<TrackerSnapshot> {
        let Some(fence) = current_fence(&admitted) else {
            return Vec::new();
        };
        let marks = &self.states.cyclopedia_tracker_refresh;
        let taken = marks.take(fence.character_id);
        let mut refreshed = Vec::new();
        if !selected {
            return refreshed;
        }
        for (kind, keys) in [TrackerKind::Bestiary, TrackerKind::Bosstiary]
            .into_iter()
            .zip(taken)
        {
            if keys.is_empty() {
                continue;
            }
            let Some(tracker) = self.read_cyclopedia_tracker(fence, kind).await else {
                for key in &keys {
                    marks.mark(fence.character_id, kind, key);
                }
                continue;
            };
            if tracker
                .entries
                .iter()
                .any(|entry| keys.contains(&entry.creature_key))
            {
                refreshed.push(tracker);
            }
        }
        refreshed
    }

    pub(in crate::gameplay_transport) async fn execute_cyclopedia_tracker(
        &self,
        admitted: AdmittedSession,
        command_type: u32,
        request: TrackerRequest,
    ) -> Option<TrackerSnapshot> {
        let kind = match command_type {
            COMMAND_TYPE_BESTIARY_TRACKER => TrackerKind::Bestiary,
            COMMAND_TYPE_BOSSTIARY_TRACKER => TrackerKind::Bosstiary,
            _ => return None,
        };
        let fence = current_fence(&admitted)?;
        let rewards = self.reward_table();
        let change = match &request {
            TrackerRequest::Read => None,
            TrackerRequest::Change {
                action,
                creature_key,
            } => {
                stages(rewards, kind, creature_key, 0)?;
                Some((*action, creature_key.as_str()))
            }
        };
        let _permit = self.spell_lane_permit().await?;
        let tracked = self
            .root
            .cyclopedia_tracker(self.character, self.holder, fence, kind, change)
            .await
            .ok()??;
        Some(snapshot(rewards, kind, tracked))
    }
}
