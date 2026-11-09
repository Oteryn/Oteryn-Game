//! Crystal201189f rottenEssences.lua30-111: fixed targets and ordinary storage burn.
use super::{ESSENCES, QUEST};
use crate::quest::world_callbacks::{Callback, Selector, Storage, Trigger};

pub const PREFIX: &str = "oteryn:quest-callback/rotten-blood/prepare-";
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlameSource {
    pub key: &'static str,
    pub flavor: &'static str,
    pub storage: &'static str,
    pub inactive: u16,
    pub active: u16,
    /// Literal donor positions; the qualified current World frame projects them.
    pub flame: [i32; 3],
    pub pillar: [i32; 3],
}
pub const PILLAR_INACTIVE: u16 = 43507;
pub const PILLAR_ACTIVE: u16 = 43508;
pub const FLAMES: [FlameSource; 4] = [
    FlameSource {
        key: "oteryn:quest-callback/rotten-blood/prepare-murcion",
        flavor: "Murcion",
        storage: "oteryn:quest-progress/storage/52280",
        inactive: 43983,
        active: 43984,
        flame: [33085, 32398, 15],
        pillar: [33081, 32398, 15],
    },
    FlameSource {
        key: "oteryn:quest-callback/rotten-blood/prepare-ichgahal",
        flavor: "Ichgahal",
        storage: "oteryn:quest-progress/storage/52281",
        inactive: 43497,
        active: 43498,
        flame: [33076, 32389, 15],
        pillar: [33076, 32395, 15],
    },
    FlameSource {
        key: "oteryn:quest-callback/rotten-blood/prepare-chagorz",
        flavor: "Chagorz",
        storage: "oteryn:quest-progress/storage/52282",
        inactive: 43987,
        active: 43988,
        flame: [33067, 32398, 15],
        pillar: [33071, 32398, 15],
    },
    FlameSource {
        key: "oteryn:quest-callback/rotten-blood/prepare-vemiath",
        flavor: "Vemiath",
        storage: "oteryn:quest-progress/storage/52283",
        inactive: 43985,
        active: 43986,
        flame: [33076, 32407, 15],
        pillar: [33076, 32401, 15],
    },
];
pub fn by_key(key: &str) -> Option<&'static FlameSource> {
    FLAMES.iter().find(|source| source.key == key)
}
pub fn by_inactive(id: u16) -> Option<&'static FlameSource> {
    FLAMES.iter().find(|source| source.inactive == id)
}

pub(crate) fn callbacks() -> Vec<Callback> {
    FLAMES.iter().map(|source| Callback { track_initials: Default::default(), character_requirements: None, required_completed_quests: Vec::new(), requires_premium: false,
        admission: None,
        world_change: None,
        actor_kind: Default::default(), opening_only: false, requires_world_admission: false,
        state_ops: Vec::new(), world_requirements: Vec::new(), facing: None,
        key:source.key.into(), quest:QUEST.into(), trigger:Trigger::Use,
        selector:Selector { native: None, position:None, unique:None, action:None, appearance:Some(source.inactive) },
        // Fixed optional source targets are resolved by the sealed current World
        // caller. They are neither a portable Item nor the clicked object's lever.
        lever:None, reversible:false, passages:Vec::new(), teleport:None,
        storage:vec![Storage { track:source.storage.into(), minimum:1, maximum:1, set:0, preserve:false, increment: false, at_least: false }],
        minimum_level:None, experience:None, monk_reward:None, offerings:Vec::new(), item_on_target:None,
        return_on_failure:false, failure_message:None,
        success_message:Some(format!("essence of {} activated.",source.flavor)), native_rule:None, effects:Default::default(),
        sources:vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/monster/quests/rotten_blood_quest/rottenEssences.lua:70-111".into()],
    }).collect()
}

/// Protect a renamed authored alias as well as the four literal callback keys.
pub fn source_only_transition(transition: &crate::quest::QuestTransition) -> bool {
    transition.key.starts_with(PREFIX)
        || transition.quest == QUEST
            && transition.effects.iter().any(|effect| {
                ESSENCES.iter().any(|source| source.track == effect.track)
                    && effect.effect == crate::quest::QuestEffectKind::Set(0)
            })
}
