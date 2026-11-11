use super::*;
use atelier::v2;

/// Command 6210: the current Atelier, vessels and Wheel revision.
#[must_use]
pub fn query() -> Option<WheelCommand> {
    WheelCommand::new(v2::COMMAND_TYPE_ATELIER_V2_QUERY, v2::encode_query())
}

/// Command 6211: lock or unlock a gem (the v1 bytes).
pub fn set_lock(
    occurrence: [u8; 16],
    expected_revision: u64,
    gem: [u8; 16],
    locked: bool,
) -> Result<WheelCommand, &'static str> {
    let payload = v2::encode_set_lock(&atelier::SetLockRequest {
        occurrence,
        expected_revision,
        gem,
        locked,
    })
    .map_err(|_| "Gem lock change could not be sent")?;
    WheelCommand::new(v2::COMMAND_TYPE_ATELIER_V2_SET_LOCK, payload)
        .ok_or("Gem lock change could not be sent")
}

/// Command 6212: a paid change with the v1 bytes. Switch Domain no longer exists and is never
/// sent.
pub fn mutate(
    occurrence: [u8; 16],
    expected_revision: u64,
    mutation: &atelier::Mutation,
) -> Result<WheelCommand, &'static str> {
    let mutation = match *mutation {
        atelier::Mutation::Reveal { quality } => v2::Mutation::Reveal { quality },
        atelier::Mutation::Dismantle { gem } => v2::Mutation::Dismantle { gem },
        atelier::Mutation::GradeUp { kind, mod_id } => v2::Mutation::GradeUp { kind, mod_id },
        atelier::Mutation::SwitchDomain { .. } => return Err("Gems no longer have a domain"),
    };
    let payload = v2::encode_mutate(&v2::MutateRequest {
        occurrence,
        expected_revision,
        mutation,
    })
    .map_err(|_| "Atelier change could not be sent")?;
    WheelCommand::new(v2::COMMAND_TYPE_ATELIER_V2_MUTATE, payload)
        .ok_or("Atelier change could not be sent")
}

/// Command 6213: Replace Vessel puts `gem` into `slot` (0-11), or empties the slot with `None`.
pub fn replace_vessel(
    occurrence: [u8; 16],
    expected_wheel_revision: u64,
    slot: u8,
    gem: Option<[u8; 16]>,
) -> Result<WheelCommand, &'static str> {
    let payload = v2::encode_replace_vessel(&v2::ReplaceVesselRequest {
        occurrence,
        expected_wheel_revision,
        slot,
        gem,
    })
    .map_err(|_| "Vessel change could not be sent")?;
    WheelCommand::new(v2::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL, payload)
        .ok_or("Vessel change could not be sent")
}

/// Decodes a v2 reply into the panel projection.
#[must_use]
pub fn decode_reply(bytes: &[u8]) -> Option<Projection> {
    v2::decode_reply(bytes).ok().as_ref().map(project)
}
