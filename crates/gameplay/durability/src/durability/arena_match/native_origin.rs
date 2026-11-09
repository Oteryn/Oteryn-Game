//! Real source callbacks share exact native receipt provenance; none
//! is decoded as an Arena command received from a client.
use super::*;
use crate::foundation::CreatureDeathOccurrenceKey;

#[derive(Clone, Debug)]
pub enum NativeArenaOrigin {
    Step(NativeArenaStep),
    Death(NativeArenaDeath),
    Departure(NativeArenaDeparture),
}

impl NativeArenaOrigin {
    pub fn command(&self) -> CommandRef {
        match self {
            Self::Step(step) => step.command(),
            Self::Death(death) => death.command(),
            Self::Departure(departure) => departure.command(),
        }
    }
    pub(super) fn binding(&self) -> &[u8] {
        match self {
            Self::Step(step) => step.binding(),
            Self::Death(death) => death.binding(),
            Self::Departure(departure) => departure.binding(),
        }
    }
    pub(super) fn requires_live_pit(&self) -> bool {
        !matches!(self, Self::Departure(_))
    }
    pub(super) fn payload(&self) -> Option<Vec<u8>> {
        use oteryn_protocol_oteryn::arena::ArenaAction;
        oteryn_protocol_oteryn::arena::encode_match_request(match self {
            Self::Step(_) => ArenaAction::Advance,
            Self::Death(_) => ArenaAction::Status,
            Self::Departure(_) => ArenaAction::Leave,
        })
        .ok()
    }
    pub(super) fn validates(
        &self,
        before: &MatchState,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        payload: &[u8],
        victory: Option<CreatureDeathOccurrenceKey>,
    ) -> bool {
        if self.command() != command || self.payload().as_deref() != Some(payload) {
            return false;
        }
        match self {
            Self::Step(step) => victory.is_none() && step.validates(before, fence),
            Self::Death(death) => victory == Some(death.death()) && death.validates(before, fence),
            Self::Departure(departure) => victory.is_none() && departure.validates(before, fence),
        }
    }
}
