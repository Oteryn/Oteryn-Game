//! W11 Rotten Blood progression, owned by Quest rather than an encounter engine.
//!
//! Source: Crystal 201189f, `rotten_blood_quest/actions_sacrifice.lua:4-29`,
//! `movements_blood_entrance.lua:20-32`, `bloodshade_rotten.lua:143-151`;
//! Global manual captured in `docs/reference/tibia-manual/characters.md:165`.
//! PR2784 owner answers select persistent distinct boss flags, a 68-hour final
//! cooldown and cleanse cooldown, and the first base Decaying Defender outfit.

use serde::{Deserialize, Serialize};

pub const REQUIRED_LEVEL: u32 = 250;
pub const ACCESS_TRACK: &str = "oteryn:quest-progress/rotten-blood/access";
pub const QUEST: &str = "oteryn:quest/rotten-blood";
pub const ACCESS_DOOR_TRACK: &str = "oteryn:quest-progress/rotten-blood/access-door";
/// Crystal 201189f data-global/lib/core/storages.lua:3069.
pub const ACCESS_DOOR_STORAGE: u16 = 47_952;
pub const START: &str = "oteryn:quest-transition/rotten-blood/mission";
pub const RESTORE_DOOR: &str = "oteryn:quest-transition/rotten-blood/restore-door";
pub const ACKNOWLEDGE_CHAIN: &str = "oteryn:quest-transition/rotten-blood/acknowledge-chain";
pub const SACRIFICE: &str = "oteryn:quest-callback/rotten-blood/sacrifice";
pub const PURCHASE_ACCESS: &str = "oteryn:quest-transition/rotten-blood/purchase-access";
pub const SACRIFICIAL_PLATE_APPEARANCE: u16 = 43_891;
pub const BLOODY_TEARS_ITEM: u32 = 32_594;
pub const ACCESS_PRICE: u64 = 5_000_000;
pub const CLEANSE_COOLDOWN_SECONDS: u64 = 68 * 60 * 60;

/// Fixed source callback, registered through the existing literal USE owner.
/// Crystal registers item 43891 globally, so the selected native map placement
/// (not client item identity) is the selector. The stateful item rule is applied
/// by the existing quest callback consumption owner under the Character lock.
pub(crate) fn altar_callback() -> super::world_callbacks::Callback {
    use super::world_callbacks::{Callback, ItemOnTarget, Selector, Storage, Trigger};
    Callback {
        key: SACRIFICE.into(), quest: QUEST.into(), trigger: Trigger::Use,
        selector: Selector { position: None, unique: None, action: None,
            appearance: Some(SACRIFICIAL_PLATE_APPEARANCE) },
        lever: None, reversible: false, passages: Vec::new(), teleport: None,
        storage: vec![Storage { track: ACCESS_TRACK.into(), minimum: 0, maximum: 3, set: 1 }],
        minimum_level: None, experience: None, monk_reward: None, offerings: Vec::new(),
        item_on_target: Some(ItemOnTarget { item: "oteryn:item.tibia.i32594".into(), count: 1,
            replacement: None, target_replacement: None }),
        return_on_failure: false,
        failure_message: Some("You need a bloody tear for the sacrifice.".into()),
        sources: vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/rotten_blood_quest/actions_sacrifice.lua:4-33".into()],
    }
}

/// Extend the existing fixed Quest catalogue, keeping access and resources in
/// its established receipt owner. Paid access cannot use the generic public
/// transition entry point; the paid NPC transaction supplies its fee proof.
pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition};
    let mut tracks = catalogue.tracks.values().cloned().collect::<Vec<_>>();
    if let Some(track) = tracks.iter_mut().find(|track| track.key == ACCESS_TRACK) {
        *track = QuestTrack {
            key: ACCESS_TRACK.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: 5,
        };
    } else {
        tracks.push(QuestTrack {
            key: ACCESS_TRACK.into(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: 5,
        });
    }
    tracks.push(QuestTrack {
        key: ACCESS_DOOR_TRACK.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: 1,
    });
    let mut transitions = catalogue
        .transitions
        .values()
        .filter(|transition| transition.key != SACRIFICE)
        .cloned()
        .collect::<Vec<_>>();
    let effect = |track: &str, from, effect| QuestEffect {
        track: track.into(),
        from,
        effect,
    };
    for (key, effects) in [
        (
            START,
            vec![
                effect(
                    ACCESS_TRACK,
                    QuestComparison::Eq(0),
                    QuestEffectKind::Set(1),
                ),
                effect(
                    ACCESS_DOOR_TRACK,
                    QuestComparison::Eq(0),
                    QuestEffectKind::Set(1),
                ),
            ],
        ),
        (
            RESTORE_DOOR,
            vec![
                effect(
                    ACCESS_TRACK,
                    QuestComparison::Between(1, 5),
                    QuestEffectKind::Add(0),
                ),
                effect(
                    ACCESS_DOOR_TRACK,
                    QuestComparison::Eq(0),
                    QuestEffectKind::Set(1),
                ),
            ],
        ),
        // Crystal bloodshade_rotten.lua:136-142 acknowledges the four-child
        // chain after the entrance's already-earned access prerequisite.
        (
            ACKNOWLEDGE_CHAIN,
            vec![effect(
                ACCESS_TRACK,
                QuestComparison::Eq(4),
                QuestEffectKind::Set(5),
            )],
        ),
        (
            SACRIFICE,
            vec![effect(
                ACCESS_TRACK,
                QuestComparison::Between(0, 3),
                QuestEffectKind::Add(1),
            )],
        ),
        (
            PURCHASE_ACCESS,
            vec![effect(
                ACCESS_TRACK,
                QuestComparison::Between(0, 2),
                QuestEffectKind::Set(4),
            )],
        ),
    ] {
        transitions.push(QuestTransition {
            key: key.into(),
            quest: QUEST.into(),
            effects,
            completes: false,
            experience: None,
        });
    }
    super::QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}

/// Each bit names a distinct child. Retry timers belong to the shared encounter
/// owner and must never be used as the lifetime of these Quest flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum RottenBloodBoss {
    Ichgahal = 0,
    Murcion = 1,
    Vemiath = 2,
    Chagorz = 3,
    Bakragore = 4,
}

impl RottenBloodBoss {
    pub const CHILDREN: [Self; 4] = [Self::Ichgahal, Self::Murcion, Self::Vemiath, Self::Chagorz];

    pub fn from_creature_key(key: &str) -> Option<Self> {
        Self::CHILDREN
            .into_iter()
            .chain([Self::Bakragore])
            .find(|boss| boss.creature_key() == key)
    }

    pub const fn creature_key(self) -> &'static str {
        match self {
            Self::Ichgahal => "oteryn:creature.ichgahal",
            Self::Murcion => "oteryn:creature.murcion",
            Self::Vemiath => "oteryn:creature.vemiath",
            Self::Chagorz => "oteryn:creature.chagorz",
            Self::Bakragore => "oteryn:creature.bakragore",
        }
    }

    const fn flag(self) -> Option<u8> {
        match self {
            Self::Bakragore => None,
            child => Some(1 << child as u8),
        }
    }
}

/// Explicit durable fields; a missing row represents this default state.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "StoredRottenBloodState")]
pub struct RottenBloodState {
    access_stage: u8,
    regular_taints: u8,
    child_flags: u8,
    final_taint: bool,
    completed: bool,
    last_cleanse_at: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRottenBloodState {
    access_stage: u8,
    regular_taints: u8,
    child_flags: u8,
    final_taint: bool,
    completed: bool,
    last_cleanse_at: Option<u64>,
}

impl TryFrom<StoredRottenBloodState> for RottenBloodState {
    type Error = &'static str;

    fn try_from(state: StoredRottenBloodState) -> Result<Self, Self::Error> {
        Self::from_stored(
            state.access_stage,
            state.regular_taints,
            state.child_flags,
            state.final_taint,
            state.completed,
            state.last_cleanse_at,
        )
        .map_err(|_| "invalid stored Rotten Blood state")
    }
}

impl RottenBloodState {
    pub fn from_stored(
        access_stage: u8,
        regular_taints: u8,
        child_flags: u8,
        final_taint: bool,
        completed: bool,
        last_cleanse_at: Option<u64>,
    ) -> Result<Self, RottenBloodRefusal> {
        if access_stage > 5 || regular_taints > 4 || child_flags > 15 {
            return Err(RottenBloodRefusal::InvalidState);
        }
        Ok(Self {
            access_stage,
            regular_taints,
            child_flags,
            final_taint,
            completed,
            last_cleanse_at,
        })
    }

    pub const fn access(&self) -> bool {
        self.access_stage >= 4
    }

    pub const fn access_stage(&self) -> u8 {
        self.access_stage
    }

    pub const fn regular_taints(&self) -> u8 {
        self.regular_taints
    }

    pub const fn child_flags(&self) -> u8 {
        self.child_flags
    }

    pub const fn final_taint(&self) -> bool {
        self.final_taint
    }

    pub const fn completed(&self) -> bool {
        self.completed
    }

    pub const fn last_cleanse_at(&self) -> Option<u64> {
        self.last_cleanse_at
    }

    pub const fn chain_ready(&self) -> bool {
        self.child_flags == 15
    }

    pub const fn holds_child(&self, boss: RottenBloodBoss) -> bool {
        match boss.flag() {
            Some(flag) => self.child_flags & flag != 0,
            None => false,
        }
    }

    pub fn validate_entry(
        &self,
        level: u32,
        boss: Option<RottenBloodBoss>,
    ) -> Result<(), RottenBloodRefusal> {
        if level < REQUIRED_LEVEL {
            return Err(RottenBloodRefusal::LevelTooLow);
        }
        if !self.access() {
            return Err(RottenBloodRefusal::AccessMissing);
        }
        if boss == Some(RottenBloodBoss::Bakragore) && !self.chain_ready() {
            return Err(RottenBloodRefusal::ChainIncomplete);
        }
        Ok(())
    }

    /// Crystal torkada.lua:58-88. The callback supplies the durable level.
    pub fn begin_mission(self, level: u32) -> Result<Self, RottenBloodRefusal> {
        if level < REQUIRED_LEVEL {
            return Err(RottenBloodRefusal::LevelTooLow);
        }
        if self.access_stage != 0 {
            return Err(RottenBloodRefusal::WrongStage);
        }
        Ok(Self {
            access_stage: 1,
            ..self
        })
    }

    /// Crystal actions_sacrifice.lua:4-29: stage 1 -> 2 only checks possession;
    /// the next two altar uses consume one bloody tear each.
    pub fn sacrifice(self, tears_held: u32) -> Result<AccessChange, RottenBloodRefusal> {
        if self.access() {
            return Err(RottenBloodRefusal::AlreadyHasAccess);
        }
        if tears_held == 0 {
            return Err(RottenBloodRefusal::MissingBloodyTear);
        }
        Ok(AccessChange {
            state: Self {
                access_stage: self.access_stage + 1,
                ..self
            },
            tears_consumed: u32::from(self.access_stage != 1),
            gold_consumed: 0,
        })
    }

    /// Crystal bloodshade_sacrifice.lua:69-89. Resource consumption is performed
    /// by the existing wallet/item owner in the same transaction as this change.
    pub fn purchase_access(self, available_gold: u64) -> Result<AccessChange, RottenBloodRefusal> {
        if self.access_stage > 2 {
            return Err(RottenBloodRefusal::WrongStage);
        }
        if available_gold < ACCESS_PRICE {
            return Err(RottenBloodRefusal::InsufficientGold);
        }
        Ok(AccessChange {
            state: Self {
                access_stage: 4,
                ..self
            },
            tears_consumed: 0,
            gold_consumed: ACCESS_PRICE,
        })
    }

    /// Only a qualifying, damaging participant receives a taint. The shared
    /// native kill owner determines party membership and contribution; no wire
    /// payload is permitted to assert these facts.
    pub fn defeat_child(
        self,
        boss: RottenBloodBoss,
        party_minimum: u8,
        dealt_damage: bool,
    ) -> Result<Self, RottenBloodRefusal> {
        if !self.access() {
            return Err(RottenBloodRefusal::AccessMissing);
        }
        let flag = boss.flag().ok_or(RottenBloodRefusal::NotChildBoss)?;
        if party_minimum > 4 || party_minimum > self.regular_taints {
            return Err(RottenBloodRefusal::InvalidPartyMinimum);
        }
        if !dealt_damage {
            return Err(RottenBloodRefusal::NoDamageContribution);
        }
        if self.regular_taints != party_minimum {
            return Err(RottenBloodRefusal::PartyMinimumMismatch);
        }
        if self.child_flags & flag != 0 {
            return Err(RottenBloodRefusal::AlreadyHoldsChild);
        }
        if self.regular_taints == 4 {
            return Err(RottenBloodRefusal::TaintLimit);
        }
        Ok(Self {
            regular_taints: self.regular_taints + 1,
            child_flags: self.child_flags | flag,
            ..self
        })
    }

    /// Owner ANSWER6547536027: first verified victory grants the base outfit,
    /// no addon; reset only the regular cycle and retain the final taint.
    pub fn defeat_final(self, dealt_damage: bool) -> Result<FinalVictory, RottenBloodRefusal> {
        if !self.access() {
            return Err(RottenBloodRefusal::AccessMissing);
        }
        if !self.chain_ready() {
            return Err(RottenBloodRefusal::ChainIncomplete);
        }
        if !dealt_damage {
            return Err(RottenBloodRefusal::NoDamageContribution);
        }
        Ok(FinalVictory {
            state: Self {
                regular_taints: 0,
                child_flags: 0,
                final_taint: true,
                completed: true,
                ..self
            },
            grant_base_outfit: !self.completed,
        })
    }

    /// Owner ANSWER6547661530, pending a structured Global transcript: only the
    /// shared owner's unsuccessful terminal outcome may choose this pair once.
    /// A player death or disconnect alone never clears a flag or taint.
    pub fn fail_final(self, removed_child_flags: u8) -> Result<Self, RottenBloodRefusal> {
        if removed_child_flags > 15
            || removed_child_flags.count_ones() != 2
            || self.child_flags & removed_child_flags != removed_child_flags
        {
            return Err(RottenBloodRefusal::InvalidFailureSelection);
        }
        Ok(Self {
            regular_taints: self.regular_taints.saturating_sub(2),
            child_flags: self.child_flags & !removed_child_flags,
            ..self
        })
    }

    /// Owner ANSWER6547535436: first cleanse is immediate; only a successful
    /// cleanse advances the 68h clock. A regular cleanse keeps the final taint.
    pub fn cleanse(self, mode: CleanseMode, now: u64) -> Result<Self, RottenBloodRefusal> {
        if self.regular_taints == 0
            && self.child_flags == 0
            && (!matches!(mode, CleanseMode::All) || !self.final_taint)
        {
            return Err(RottenBloodRefusal::NothingToCleanse);
        }
        if let Some(last) = self.last_cleanse_at {
            let available = last
                .checked_add(CLEANSE_COOLDOWN_SECONDS)
                .ok_or(RottenBloodRefusal::InvalidTimestamp)?;
            if now < available {
                return Err(RottenBloodRefusal::CleanseCooldown {
                    available_at: available,
                });
            }
        }
        Ok(Self {
            regular_taints: 0,
            child_flags: 0,
            final_taint: self.final_taint && !matches!(mode, CleanseMode::All),
            last_cleanse_at: Some(now),
            ..self
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CleanseMode {
    Regular,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessChange {
    pub state: RottenBloodState,
    pub tears_consumed: u32,
    pub gold_consumed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalVictory {
    pub state: RottenBloodState,
    pub grant_base_outfit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RottenBloodRefusal {
    InvalidState,
    InvalidTimestamp,
    LevelTooLow,
    AccessMissing,
    WrongStage,
    AlreadyHasAccess,
    MissingBloodyTear,
    InsufficientGold,
    NotChildBoss,
    InvalidPartyMinimum,
    NoDamageContribution,
    PartyMinimumMismatch,
    AlreadyHoldsChild,
    TaintLimit,
    ChainIncomplete,
    InvalidFailureSelection,
    NothingToCleanse,
    CleanseCooldown { available_at: u64 },
}

impl std::fmt::Display for RottenBloodRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Rotten Blood progression refused: {self:?}")
    }
}

impl std::error::Error for RottenBloodRefusal {}

#[cfg(test)]
#[path = "rotten_blood_tests.rs"]
mod tests;
