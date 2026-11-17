use super::*;
use crate::gem_atelier;

#[path = "../0272-atelier-resources/protocol.rs"]
mod resource_projection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Save,
    Reset,
    Reveal,
    ToggleLock,
    Reshape,
    Dismantle,
    GradeUp,
    /// Capability 483: put the selected gem into the selected Gem Vessel slot.
    ReplaceVessel,
    /// Capability 483: empty the selected Gem Vessel slot.
    ClearVessel,
    Retry,
}

/// All controls use codecs for the selected feature; no cost or random outcome is submitted.
pub fn request(
    panel: &mut WheelPanel,
    intent: Intent,
    occurrence: [u8; 16],
) -> Option<WheelCommand> {
    let result = prepare(panel, intent, occurrence);
    match result {
        Ok(command) => {
            panel.pending = Some(command.clone());
            panel.feedback = None;
            Some(command)
        }
        Err(message) => {
            panel.feedback = Some(message);
            None
        }
    }
}

fn prepare(
    panel: &WheelPanel,
    intent: Intent,
    occurrence: [u8; 16],
) -> Result<WheelCommand, &'static str> {
    if !panel.selected {
        return Err(tr!("Wheel and Atelier are unavailable in this session"));
    }
    if panel.pending.is_some() {
        return Err(tr!("Waiting for the server"));
    }
    if intent == Intent::Retry {
        return panel.retry.clone().ok_or(tr!("No request needs retrying"));
    }
    if intent == Intent::Refresh {
        return WheelCommand::new(wheel::COMMAND_TYPE_WHEEL_ALLOCATION, vec![8, 0])
            .ok_or(tr!("Could not refresh the Wheel"));
    }
    if panel.retry.is_some() {
        return Err(tr!(
            "Retry the unresolved request before making another change"
        ));
    }
    if !panel.synchronized {
        return Err(tr!("Refresh the current Wheel and Atelier state first"));
    }
    let wheel = panel
        .confirmed_wheel
        .as_ref()
        .ok_or(tr!("Refresh the Wheel first"))?;
    let atelier = panel
        .confirmed_atelier
        .as_ref()
        .ok_or(tr!("Refresh the Atelier first"))?;
    if matches!(intent, Intent::Save | Intent::Reset) {
        let payload = wheel::encode_wheel_request(&wheel::WheelAllocationRequest {
            action: if intent == Intent::Save { 1 } else { 2 },
            occurrence,
            expected_revision: wheel.revision,
            points: if intent == Intent::Save {
                panel.points
            } else {
                [0; 36]
            },
            // With 483 the vessels change through Replace Vessel; an absent field keeps them.
            vessels: (intent == Intent::Save && !panel.atelier_v2).then_some(panel.vessels),
        })
        .map_err(|_| tr!("Wheel edits could not be sent"))?;
        return WheelCommand::new(wheel::COMMAND_TYPE_WHEEL_ALLOCATION, payload)
            .ok_or(tr!("Wheel edits could not be sent"));
    }
    if panel.dirty() {
        return Err(tr!("Save or refresh your pending Wheel edits first"));
    }
    let mutation = match intent {
        Intent::Reveal => {
            atelier
                .quality_quotes
                .iter()
                .find(|quote| quote.quality == panel.quality)
                .ok_or(tr!("Refresh current Atelier prices first"))?;
            atelier::Mutation::Reveal {
                quality: panel.quality,
            }
        }
        Intent::ToggleLock => {
            let gem = panel.selected_gem().ok_or(tr!("Select a gem first"))?;
            if panel.atelier_v2 {
                return gem_atelier::set_lock(occurrence, atelier.revision, gem.id, !gem.locked);
            }
            let payload = atelier::encode_set_lock(&atelier::SetLockRequest {
                occurrence,
                expected_revision: atelier.revision,
                gem: gem.id,
                locked: !gem.locked,
            })
            .map_err(|_| tr!("Gem lock change could not be sent"))?;
            return WheelCommand::new(atelier::COMMAND_TYPE_ATELIER_SET_LOCK, payload)
                .ok_or(tr!("Gem lock change could not be sent"));
        }
        Intent::ReplaceVessel | Intent::ClearVessel => {
            let board = panel
                .vessel_board
                .as_ref()
                .filter(|_| panel.atelier_v2)
                .ok_or("Gem Vessels are unavailable in this session")?;
            let slot = panel.vessel_slot;
            let gem = if intent == Intent::ReplaceVessel {
                let gem = panel.selected_gem().ok_or(tr!("Select a gem first"))?;
                if let Some(block) = gem_atelier::replace_block(board, gem, slot) {
                    return Err(block.reason());
                }
                Some(gem.id)
            } else {
                board
                    .slots
                    .get(usize::from(slot))
                    .and_then(|vessel| vessel.gem)
                    .ok_or("This vessel is already empty")?;
                None
            };
            return gem_atelier::replace_vessel(occurrence, board.wheel_revision, slot, gem);
        }
        Intent::Reshape => {
            let gem = panel.selected_gem().ok_or(tr!("Select a gem first"))?;
            if let Some(block) = super::reshape_block(panel, gem) {
                return Err(block.reason());
            }
            if !panel.atelier_v2 {
                return Err(tr!("Mod reshaping is unavailable in this session"));
            }
            let position = super::reshape_position(panel, gem)
                .and_then(|position| u8::try_from(position).ok())
                .ok_or(tr!("Select a modifier of the gem first"))?;
            return gem_atelier::reshape(occurrence, atelier.revision, gem.id, position);
        }
        Intent::Dismantle => {
            let gem = panel.selected_gem().ok_or(tr!("Select a gem first"))?;
            if gem.vessel {
                return Err(tr!("Remove the gem from its vessel and save first"));
            }
            if gem.locked {
                return Err(tr!("Unlock the gem first"));
            }
            atelier
                .quality_quotes
                .iter()
                .find(|quote| quote.quality == gem.quality)
                .ok_or(tr!("Refresh current Atelier prices first"))?;
            atelier::Mutation::Dismantle { gem: gem.id }
        }
        Intent::GradeUp => {
            let (kind, mod_id) = panel
                .modifier
                .filter(|id| panel.modifiers().contains(id))
                .ok_or(tr!("Select a Workshop modifier first"))?;
            let grade = panel.grade(kind, mod_id);
            if grade >= 3 {
                return Err(tr!("This modifier is already Grade IV"));
            }
            atelier
                .grade_quotes
                .iter()
                .find(|quote| quote.kind == kind && quote.grade == grade + 1)
                .ok_or(tr!("Refresh current Workshop prices first"))?;
            atelier::Mutation::GradeUp { kind, mod_id }
        }
        _ => return Err(tr!("Unknown Wheel action")),
    };
    if panel.atelier_v2 {
        return gem_atelier::mutate(occurrence, atelier.revision, &mutation);
    }
    let payload = atelier::encode_mutate(&atelier::MutateRequest {
        occurrence,
        expected_revision: atelier.revision,
        mutation,
    })
    .map_err(|_| tr!("Atelier change could not be sent"))?;
    WheelCommand::new(atelier::COMMAND_TYPE_ATELIER_MUTATE, payload)
        .ok_or(tr!("Atelier change could not be sent"))
}

fn operation_refused(response: &WheelResponse) -> Option<bool> {
    match response.kind {
        wheel::COMMAND_TYPE_WHEEL_ALLOCATION => wheel::decode_wheel_reply(&response.payload)
            .ok()
            .map(|v| v.refused),
        atelier::COMMAND_TYPE_ATELIER_QUERY
        | atelier::COMMAND_TYPE_ATELIER_SET_LOCK
        | atelier::COMMAND_TYPE_ATELIER_MUTATE => atelier::decode_reply(&response.payload)
            .ok()
            .map(|v| v.refused),
        atelier::v2::COMMAND_TYPE_ATELIER_V2_QUERY
        | atelier::v2::COMMAND_TYPE_ATELIER_V2_SET_LOCK
        | atelier::v2::COMMAND_TYPE_ATELIER_V2_MUTATE
        | atelier::v2::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL
        | atelier::v2::COMMAND_TYPE_ATELIER_V2_RESHAPE => {
            gem_atelier::decode_reply(&response.payload).map(|v| v.atelier.refused)
        }
        _ => None,
    }
}

/// Historical mutation replies settle identity; only the following live queries replace views.
pub fn apply_result(panel: &mut WheelPanel, response: &WheelResponse) -> bool {
    let Some(pending) = panel.pending.take() else {
        return false;
    };
    panel.synchronized = false;
    // An absent or malformed optional query must never leave old balances
    // looking like the current result, or turn an unavailable value into zero.
    panel.confirmed_resources = None;
    if pending.kind != response.kind {
        if pending.mutation {
            panel.retry = Some(pending);
        }
        panel.feedback = Some(tr!(
            "The response did not match this request; refresh and retry"
        ));
        return false;
    }
    let outcome = match response.status {
        CommandStatus::Accepted | CommandStatus::DuplicateReplay => operation_refused(response),
        CommandStatus::Rejected => Some(true),
        CommandStatus::DuplicateOutcomeExpired => None,
    };
    if pending.mutation {
        panel.retry = outcome.is_none().then_some(pending);
    }
    panel.feedback = match outcome {
        Some(false) => Some(tr!("The server confirmed the request")),
        Some(true) => Some(tr!(
            "The server refused the change; check Premium, points, location and resources"
        )),
        None => Some(tr!(
            "The outcome is not confirmed; Retry sends the same request without a second payment"
        )),
    };
    let Some((wheel, (atelier, board))) = response
        .wheel
        .as_deref()
        .and_then(|bytes| wheel::decode_wheel_reply(bytes).ok())
        .zip(response.atelier.as_deref().and_then(|bytes| {
            if panel.atelier_v2 {
                let projection = gem_atelier::decode_reply(bytes)?;
                Some((projection.atelier, Some(projection.board)))
            } else {
                Some((atelier::decode_reply(bytes).ok()?, None))
            }
        }))
    else {
        panel.feedback = Some(tr!(
            "The current view is unavailable; refresh before editing"
        ));
        return false;
    };
    if wheel.refused || atelier.refused {
        panel.feedback = Some(tr!(
            "Wheel or Atelier is unavailable for this character; check level, promotion and Premium"
        ));
        return false;
    }
    if panel
        .confirmed_wheel
        .as_ref()
        .is_some_and(|old| wheel.revision < old.revision)
        || panel
            .confirmed_atelier
            .as_ref()
            .is_some_and(|old| atelier.revision < old.revision)
    {
        panel.feedback = Some(tr!(
            "The response is older than the current view; refresh again"
        ));
        return false;
    }
    let mut vessels = [None; 4];
    // With 483 the draft vessels mirror the first four Gem Vessel slots; gems carry their slot
    // (0-11) in `domain`.
    if let Some(board) = &board {
        for (draft, slot) in vessels.iter_mut().zip(&board.slots) {
            *draft = slot.gem;
        }
    }
    for gem in atelier.gems.iter().filter(|_| board.is_none()) {
        if gem.vessel {
            let Some(slot) = vessels.get_mut(usize::from(gem.domain)) else {
                return false;
            };
            if slot.replace(gem.id).is_some() {
                return false;
            }
        }
    }
    panel.points = wheel.points;
    panel.vessels = vessels;
    panel.confirmed_wheel = Some(wheel);
    panel.confirmed_atelier = Some(atelier);
    if let Some(board) = board {
        panel.in_temple = Some(board.in_temple);
        panel.premium = Some(board.premium);
        panel.vessel_board = Some(board);
    }
    panel.confirmed_resources = response
        .resources
        .as_deref()
        .and_then(resource_projection::decode_reply);
    panel.synchronized = true;
    if panel.selected_gem().is_none() {
        panel.gem = panel
            .confirmed_atelier
            .as_ref()
            .and_then(|state| state.gems.first())
            .map(|gem| gem.id);
    }
    if !panel
        .modifier
        .is_some_and(|id| panel.modifiers().contains(&id))
    {
        panel.modifier = panel.modifiers().first().copied();
    }
    true
}

/// Read-only executed modifiers from the existing Wheel/Atelier capability.
pub fn gem_presentation_query(
    p: &mut WheelPanel,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    super::gems::query(p)
}

/// Read-only graph and draft projection via the existing Wheel capability.
pub fn wheel_presentation_query(
    p: &mut WheelPanel,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    super::diagram::query(p)
}
