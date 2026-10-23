//! Reset admission on the existing one-item retirement owner and shared receipts.
use super::*;
use crate::durability::world_reset::RetiringWorldReset;

mod ground_admission;
pub(super) use ground_admission::admit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RetireCause {
    CorpseDecay,
    WorldReset(RetiringWorldReset),
}

impl RetireCause {
    pub(super) fn tag(self) -> &'static str {
        match self {
            Self::CorpseDecay => "CORPSE_DECAY",
            Self::WorldReset(_) => "WORLD_RESET",
        }
    }
    pub(super) fn epoch(self) -> Option<String> {
        match self {
            Self::CorpseDecay => None,
            Self::WorldReset(reset) => Some(reset.epoch.to_string()),
        }
    }
    pub(super) fn binding(self) -> Option<Vec<u8>> {
        match self {
            Self::CorpseDecay => None,
            Self::WorldReset(reset) => Some(reset.binding.to_vec()),
        }
    }
    pub(super) fn target(self) -> Option<Vec<u8>> {
        match self {
            Self::CorpseDecay => None,
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
    let RetireCause::WorldReset(reset) = cause else {
        return;
    };
    message.world_reset = Some(audit::OneItemWorldResetV1 {
        world_id: reset.world.as_bytes().to_vec(),
        reset_epoch: reset.epoch,
        item_instance_id: message
            .before
            .as_ref()
            .map_or_else(Vec::new, |state| state.item_instance_id.clone()),
        target_digest: reset.target.to_vec(),
        retirement_binding: reset.binding.to_vec(),
    });
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
