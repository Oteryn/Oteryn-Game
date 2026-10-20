//! Shared schedule proposal data; native Content and Ability types are supplied by callers.

use super::ExactActorRef;
use super::ability_data::{AbilityIntent, AbilityOccurrence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleList {
    Attack,
    Defence,
}

impl ScheduleList {
    pub(crate) const fn purpose(self) -> &'static str {
        match self {
            Self::Attack => "AI_ATTACK",
            Self::Defence => "AI_DEFENCE",
        }
    }

    pub(crate) const fn atom(self) -> &'static str {
        match self {
            Self::Attack => "attack",
            Self::Defence => "defence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileAbilityProposal<DefinitionRef, Magnitude> {
    pub issuer: ExactActorRef,
    pub target: ExactActorRef,
    pub ability: DefinitionRef,
    pub list: ScheduleList,
    pub entry_index: usize,
    pub magnitude: Option<Magnitude>,
    pub range_tiles: u16,
    pub occurrence: AbilityOccurrence,
    pub intent: AbilityIntent,
}
