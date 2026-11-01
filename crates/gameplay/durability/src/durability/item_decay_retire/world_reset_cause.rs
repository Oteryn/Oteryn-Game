//! Reset admission on the existing one-item retirement owner and shared receipts.
use super::*;
use crate::durability::world_reset::RetiringWorldReset;

mod death_contents;
pub(super) use death_contents::retire_death_contents;
mod ground_admission;
use super::activity_retirement::ActivityRetirementBinding;
pub(super) use ground_admission::{admit, admit_ground};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RetireCause {
    CorpseDecay,
    WorldReset(RetiringWorldReset),
    ActivityRetirement(ActivityRetirementBinding),
}

impl RetireCause {
    pub(super) fn activity_generation(self) -> Option<u64> {
        match self {
            Self::ActivityRetirement(binding) => Some(binding.generation.get()),
            _ => None,
        }
    }
    pub(super) fn handoff_id(self) -> Option<[u8; 16]> {
        match self {
            Self::ActivityRetirement(binding) => Some(binding.handoff_id),
            _ => None,
        }
    }
    pub(super) fn tag(self) -> &'static str {
        match self {
            Self::CorpseDecay => "CORPSE_DECAY",
            Self::WorldReset(_) => "WORLD_RESET",
            Self::ActivityRetirement(_) => "ACTIVITY_RETIREMENT",
        }
    }
    pub(super) fn epoch(self) -> Option<String> {
        match self {
            Self::CorpseDecay | Self::ActivityRetirement(_) => None,
            Self::WorldReset(reset) => Some(reset.epoch.to_string()),
        }
    }
    pub(super) fn binding(self) -> Option<Vec<u8>> {
        match self {
            Self::CorpseDecay | Self::ActivityRetirement(_) => None,
            Self::WorldReset(reset) => Some(reset.binding.to_vec()),
        }
    }
    pub(super) fn target(self) -> Option<Vec<u8>> {
        match self {
            Self::CorpseDecay | Self::ActivityRetirement(_) => None,
            Self::WorldReset(reset) => Some(reset.target.to_vec()),
        }
    }
}

pub(super) fn decode_cause(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<RetireCause, DurabilityError> {
    match row.try_get::<&str, _>("cause_kind")? {
        "CORPSE_DECAY" => Ok(RetireCause::CorpseDecay),
        "WORLD_RESET" => Ok(RetireCause::WorldReset(RetiringWorldReset {
            world: WorldId::decode(&uuid_text(row.try_get("world_id")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            epoch: row
                .try_get::<String, _>("reset_epoch")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            binding: row
                .try_get::<Vec<u8>, _>("retirement_binding")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            target: row
                .try_get::<Vec<u8>, _>("target_digest")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        })),
        "ACTIVITY_RETIREMENT" => Ok(RetireCause::ActivityRetirement(ActivityRetirementBinding {
            handoff_id: uuid_text(row.try_get("activity_handoff_id")?)?,
            scope: decode_ground_scope(row)?,
            generation: ScopeOwnershipGeneration::new(
                row.try_get::<String, _>("activity_ownership_generation")?
                    .parse()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        })),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

pub(super) fn receipt_matches(
    row: &sqlx::postgres::PgRow,
    cause: RetireCause,
) -> std::result::Result<bool, DurabilityError> {
    Ok(decode_cause(row)? == cause)
}

pub(super) fn bind_message(message: &mut OneItemDecayRetireV1, cause: RetireCause) {
    let item_instance_id = message
        .before
        .as_ref()
        .map_or_else(Vec::new, |state| state.item_instance_id.clone());
    match cause {
        RetireCause::CorpseDecay => return,
        RetireCause::WorldReset(reset) => {
            message.world_reset = Some(audit::OneItemWorldResetV1 {
                world_id: reset.world.as_bytes().to_vec(),
                reset_epoch: reset.epoch,
                item_instance_id,
                target_digest: reset.target.to_vec(),
                retirement_binding: reset.binding.to_vec(),
            })
        }
        RetireCause::ActivityRetirement(binding) => {
            let RuntimeScopeRefV1::Instance {
                world_id,
                instance_id,
            } = binding.scope
            else {
                return;
            };
            message.activity_retirement = Some(audit::OneItemActivityRetirementV1 {
                world_id: world_id.as_bytes().to_vec(),
                instance_id: instance_id.to_vec(),
                handoff_id: binding.handoff_id.to_vec(),
                ownership_generation: binding.generation.get(),
                item_instance_id,
            });
        }
    }
    message.cause = None;
    message.ground_container_entry =
        message
            .corpse_entry
            .take()
            .map(|entry| audit::OneItemGroundContainerEntryV1 {
                parent_item_instance_id: entry.corpse_item_instance_id,
                placement_ordinal: entry.placement_ordinal,
                parent_ground: entry.corpse_ground,
            });
}

impl DecayRetireStep {
    pub(in crate::durability) fn for_world_reset(
        reset: RetiringWorldReset,
        root: [u8; 16],
        item: [u8; 16],
    ) -> Self {
        Self {
            corpse_item_instance_id: root,
            item_instance_id: item,
            cause: RetireCause::WorldReset(reset),
        }
    }
}
