use super::super::character_progression::CharacterProgressionError;
use oteryn_protocol_oteryn::prey::{self as wire, Request};
use serde::{Deserialize, Serialize};

/// Trusted deltas bound to one selected race and one selection lifetime.
/// A zero-time delta records settled kills/loot without consuming a minute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreyCheckpoint {
    pub slot: u8,
    pub creature: String,
    pub selection_epoch: u64,
    pub seconds: u32,
    pub kills: u32,
    pub loot_gold: u64,
}

#[derive(Debug, Clone)]
pub enum PreyMutation {
    Initialize,
    Command(Request),
    Checkpoint(Vec<PreyCheckpoint>),
}

impl PreyMutation {
    pub(super) fn binding(&self) -> Result<Vec<u8>, CharacterProgressionError> {
        Ok(match self {
            Self::Initialize => vec![0],
            Self::Command(request) => [
                vec![1],
                wire::encode_request(request)
                    .map_err(|_| CharacterProgressionError::InvalidInput)?,
            ]
            .concat(),
            Self::Checkpoint(deltas) => [
                vec![2],
                serde_json::to_vec(deltas).map_err(|_| CharacterProgressionError::InvalidInput)?,
            ]
            .concat(),
        })
    }
}

/// Only the accepted Store delivery owner constructs this immutable binding.
#[derive(Debug, Clone)]
pub struct PreyStoreDelivery {
    pub operation_id: [u8; 16],
    pub account_id: [u8; 16],
    pub world_id: [u8; 16],
    pub fulfilment_unit_id: String,
}

#[derive(Debug)]
pub enum PreyExpansionError {
    AlreadyOwned,
    Rejected(CharacterProgressionError),
}

impl From<CharacterProgressionError> for PreyExpansionError {
    fn from(value: CharacterProgressionError) -> Self {
        Self::Rejected(value)
    }
}
