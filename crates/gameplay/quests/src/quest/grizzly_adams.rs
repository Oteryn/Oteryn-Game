//! Killing in the Name Of, Grizzly Adams: sourced rules over the existing quest tracks.
//! Crystal 201189f is the authority; `grizzly_source.py` owns the embedded fact catalogue.
//! The durable quest owner evaluates actions under its character lock and owns replay.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::QuestTrackChange;

#[path = "grizzly_adams/catalogue.rs"]
mod integration;
#[path = "grizzly_adams/rules.rs"]
mod rules;

pub use integration::{extend, journal};
pub use rules::{can_start_task, plan_action, points_reward, promotion_due};

pub const QUEST: &str = "oteryn:quest.killing_in_the_name_of_quest";
pub const NPC: &str = "oteryn:npc.grizzly_adams";
pub const TRANSITION_PREFIX: &str = "oteryn:quest-transition/grizzly/";
pub const QUEST_STORAGE_BASE: u32 = 1500;
pub const POINTS_STORAGE: u32 = 2500;
pub const KILLS_STORAGE_BASE: u32 = 65000;
pub const REPEAT_STORAGE_BASE: u32 = 48950;
pub const KILL_COUNTER_BASE: u32 = 42179;
pub const JOIN_STORAGE: u32 = 100157;
pub const BOSS_POINTS_STORAGE: u32 = 42169;
pub const QUEST_LOG_STORAGE: u32 = 42170;
pub const PROMOTION_STORAGE: u32 = 42171;
/// killing_in_the_name_of.lua:904 and movements_boss.lua:416.
pub const ACTIVE_TASK_LIMIT: usize = 3;
pub const BOSS_ROOM_SECONDS: u64 = 600;
const SOURCE_REVISION: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
const RULES_VERSION: &[u8] = b"oteryn-grizzly-rules-v2";
const EMBEDDED: &str = include_str!("../../../../../content/quests/killing-in-the-name-of.json");

pub type GrizzlyState = BTreeMap<String, i64>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrizzlyActor {
    pub level: u32,
    pub premium: bool,
    /// Completion grants XP before testing the point cap (NPC:462,485). The durable
    /// owner supplies the level projected by the current native progression policy;
    /// dialogue and actions without XP need no projection.
    pub level_after_reward: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrizzlyAction {
    Join,
    StartTask(u8),
    CancelTask(u8),
    CompleteTask(u8),
    Promote,
    SelectBoss(u8),
    ReportBoss(u8),
    StartSpecialTask,
    EnterBoss(u8),
    KillCreature {
        task_id: u8,
        creature: String,
        /// Coherent quest snapshot revision captured at the lethal event. The durable
        /// owner refuses a task run started after this revision.
        observed_revision: u64,
    },
    BossKill {
        boss_id: u8,
        /// A later NPC reset must erase an older queued death, while distinct
        /// deaths captured without an intervening reset can all increment.
        observed_revision: u64,
    },
}

impl GrizzlyAction {
    #[must_use]
    pub fn transition_key(&self) -> String {
        let tail = match self {
            Self::Join => "join".to_owned(),
            Self::StartTask(id) => format!("start-{id}"),
            Self::CancelTask(id) => format!("cancel-{id}"),
            Self::CompleteTask(id) => format!("complete-{id}"),
            Self::Promote => "promote".to_owned(),
            Self::SelectBoss(id) => format!("select-boss-{id}"),
            Self::ReportBoss(id) => format!("report-boss-{id}"),
            Self::StartSpecialTask => "special".to_owned(),
            Self::EnterBoss(id) => format!("enter-boss-{id}"),
            Self::KillCreature { task_id, .. } => format!("kill-{task_id}"),
            Self::BossKill { boss_id, .. } => format!("kill-boss-{boss_id}"),
        };
        format!("{TRANSITION_PREFIX}{tail}")
    }

    /// Stable request binding includes the creature and the captured task-run predecessor.
    #[must_use]
    pub fn binding_bytes(&self) -> Vec<u8> {
        let mut bytes = self.transition_key().into_bytes();
        if let Self::KillCreature {
            creature,
            observed_revision,
            ..
        } = self
        {
            bytes.push(0);
            bytes.extend_from_slice(creature.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(&observed_revision.to_be_bytes());
        }
        if let Self::BossKill {
            observed_revision, ..
        } = self
        {
            bytes.push(0);
            bytes.extend_from_slice(&observed_revision.to_be_bytes());
        }
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrizzlyRefusal {
    InvalidCatalogue,
    InvalidTask,
    InvalidBoss,
    NotJoined,
    AlreadyJoined,
    AlreadyStarted,
    NotStarted,
    Level,
    Rank,
    Premium,
    TaskLimit,
    RepeatLimit,
    KillsIncomplete,
    CreatureMismatch,
    BossActive,
    BossPoints,
    BossNotEntered,
    BossNotKilled,
    NoPromotion,
    SpecialTasksComplete,
    ProgressionUnavailable,
    RoomOccupied,
    OutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GrizzlyItemReward {
    pub source_id: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GrizzlyStorageReward {
    pub storage: u32,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GrizzlyCreature {
    pub name: String,
    pub key: String,
    pub alt_storage: Option<u32>,
}

impl GrizzlyCreature {
    #[must_use]
    pub fn matches(&self, creature: &str) -> bool {
        self.key == creature || self.name.eq_ignore_ascii_case(creature)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GrizzlyTask {
    pub id: u8,
    pub name: String,
    pub required: i64,
    pub minimum_level: u32,
    pub maximum_level: u32,
    pub premium: bool,
    pub minimum_rank: u8,
    pub repeat_limit: Option<u8>,
    pub boss_id: Option<u8>,
    pub creatures: Vec<GrizzlyCreature>,
    pub experience: Option<i64>,
    pub points: Option<i64>,
    pub item: Option<GrizzlyItemReward>,
    pub completion_storage: Option<GrizzlyStorageReward>,
    pub donor_line: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct GrizzlyPosition {
    pub x: u16,
    pub y: u16,
    pub z: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GrizzlyBoss {
    pub id: u8,
    pub task_id: Option<u8>,
    pub name: String,
    pub achievement: Option<String>,
    pub creature_name: String,
    pub creature_key: String,
    pub storage: u32,
    pub kill_storage: u32,
    pub portal: GrizzlyPosition,
    pub player_position: GrizzlyPosition,
    pub boss_position: GrizzlyPosition,
    pub center: GrizzlyPosition,
    pub range_x: u16,
    pub range_y: u16,
    pub donor_line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GrizzlyStorage {
    pub id: u32,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct GrizzlyCatalogue {
    schema: String,
    source_revision: String,
    pub tasks: Vec<GrizzlyTask>,
    pub bosses: Vec<GrizzlyBoss>,
    pub storages: Vec<GrizzlyStorage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GrizzlyTransition {
    pub changes: Vec<QuestTrackChange>,
    pub experience: Option<i64>,
    pub item: Option<GrizzlyItemReward>,
    pub achievement: Option<String>,
}

pub fn catalogue() -> Result<&'static GrizzlyCatalogue, GrizzlyRefusal> {
    static DATA: OnceLock<Result<GrizzlyCatalogue, GrizzlyRefusal>> = OnceLock::new();
    DATA.get_or_init(|| {
        let data: GrizzlyCatalogue =
            serde_json::from_str(EMBEDDED).map_err(|_| GrizzlyRefusal::InvalidCatalogue)?;
        if data.schema != "OTERYN_GRIZZLY_ADAMS/v1"
            || data.source_revision != SOURCE_REVISION
            || data.tasks.len() != 42
            || data.bosses.len() != 30
            || data
                .tasks
                .iter()
                .enumerate()
                .any(|(n, t)| usize::from(t.id) != n + 1)
            || data
                .bosses
                .iter()
                .enumerate()
                .any(|(n, b)| usize::from(b.id) != n + 1)
            || data
                .storages
                .windows(2)
                .any(|pair| pair[0].id >= pair[1].id)
        {
            return Err(GrizzlyRefusal::InvalidCatalogue);
        }
        Ok(data)
    })
    .as_ref()
    .map_err(|error| *error)
}

#[must_use]
pub fn definition_hash() -> [u8; 32] {
    Sha256::new()
        .chain_update(RULES_VERSION)
        .chain_update(EMBEDDED)
        .finalize()
        .into()
}

#[must_use]
pub fn task(id: u8) -> Option<&'static GrizzlyTask> {
    catalogue().ok()?.tasks.get(usize::from(id).checked_sub(1)?)
}

#[must_use]
pub fn boss(id: u8) -> Option<&'static GrizzlyBoss> {
    catalogue()
        .ok()?
        .bosses
        .get(usize::from(id).checked_sub(1)?)
}

/// Literal ids retain the existing raw-storage namespace; named donor storage keeps its
/// already lowered key. These are one set of tracks, not a second quest state.
#[must_use]
pub fn storage_track(id: u32) -> String {
    if let Ok(data) = catalogue()
        && let Ok(index) = data.storages.binary_search_by_key(&id, |row| row.id)
    {
        return data.storages[index].key.clone();
    }
    format!("oteryn:quest-progress/storage/{id}")
}

#[must_use]
pub fn storage_keys() -> Vec<String> {
    catalogue()
        .map(|data| data.storages.iter().map(|row| row.key.clone()).collect())
        .unwrap_or_default()
}

#[must_use]
pub fn storage_value(state: &GrizzlyState, id: u32) -> i64 {
    state.get(&storage_track(id)).copied().unwrap_or(-1)
}

#[must_use]
pub fn joined(state: &GrizzlyState) -> bool {
    storage_value(state, JOIN_STORAGE) == 1 && storage_value(state, QUEST_LOG_STORAGE) == 0
}

/// Point rank and the separately confirmed promotion track are intentionally distinct.
#[must_use]
pub fn rank(state: &GrizzlyState) -> u8 {
    match storage_value(state, POINTS_STORAGE) {
        100.. => 6,
        70..=99 => 5,
        40..=69 => 4,
        20..=39 => 3,
        10..=19 => 2,
        _ => u8::from(storage_value(state, JOIN_STORAGE) == 1),
    }
}

#[must_use]
pub fn started_tasks(state: &GrizzlyState) -> Vec<u8> {
    (1..=42)
        .filter(|id| storage_value(state, QUEST_STORAGE_BASE + u32::from(*id)) == 1)
        .collect()
}

/// The donor accepts the plural name or its final-letter-stripped form, case insensitive.
#[must_use]
pub fn task_by_name(name: &str) -> Option<&'static GrizzlyTask> {
    catalogue().ok()?.tasks.iter().find(|task| {
        task.name.eq_ignore_ascii_case(name)
            || task
                .name
                .get(..task.name.len().saturating_sub(1))
                .is_some_and(|singular| singular.eq_ignore_ascii_case(name))
    })
}

#[must_use]
pub fn boss_by_name(name: &str) -> Option<&'static GrizzlyBoss> {
    catalogue()
        .ok()?
        .bosses
        .iter()
        .find(|boss| boss.name.eq_ignore_ascii_case(name))
}

pub fn check_boss_room(state: &GrizzlyState, id: u8, occupied: bool) -> Result<(), GrizzlyRefusal> {
    let boss = boss(id).ok_or(GrizzlyRefusal::InvalidBoss)?;
    if storage_value(state, boss.storage) != 1 {
        return Err(GrizzlyRefusal::BossNotEntered);
    }
    if occupied {
        return Err(GrizzlyRefusal::RoomOccupied);
    }
    Ok(())
}

#[cfg(test)]
#[path = "grizzly_adams/tests.rs"]
mod tests;
