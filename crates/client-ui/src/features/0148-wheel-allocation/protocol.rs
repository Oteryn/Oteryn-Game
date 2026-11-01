use super::*;

#[path = "../0272-atelier-resources/protocol.rs"]
mod resource_projection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Save,
    Reset,
    Reveal,
    ToggleLock,
    SwitchDomain,
    Dismantle,
    GradeUp,
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
        return Err("Wheel and Atelier are unavailable in this session");
    }
    if panel.pending.is_some() {
        return Err("Waiting for the server");
    }
    if intent == Intent::Retry {
        return panel.retry.clone().ok_or("No request needs retrying");
    }
    if intent == Intent::Refresh {
        return WheelCommand::new(wheel::COMMAND_TYPE_WHEEL_ALLOCATION, vec![8, 0])
            .ok_or("Could not refresh the Wheel");
    }
    if panel.retry.is_some() {
        return Err("Retry the unresolved request before making another change");
    }
    if !panel.synchronized {
        return Err("Refresh the current Wheel and Atelier state first");
    }
    let wheel = panel
        .confirmed_wheel
        .as_ref()
        .ok_or("Refresh the Wheel first")?;
    let atelier = panel
        .confirmed_atelier
        .as_ref()
        .ok_or("Refresh the Atelier first")?;
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
            vessels: (intent == Intent::Save).then_some(panel.vessels),
        })
        .map_err(|_| "Wheel edits could not be sent")?;
        return WheelCommand::new(wheel::COMMAND_TYPE_WHEEL_ALLOCATION, payload)
            .ok_or("Wheel edits could not be sent");
    }
    if panel.dirty() {
        return Err("Save or refresh your pending Wheel edits first");
    }
    let mutation = match intent {
        Intent::Reveal => {
            atelier
                .quality_quotes
                .iter()
                .find(|quote| quote.quality == panel.quality)
                .ok_or("Refresh current Atelier prices first")?;
            atelier::Mutation::Reveal {
                quality: panel.quality,
            }
        }
        Intent::ToggleLock => {
            let gem = panel.selected_gem().ok_or("Select a gem first")?;
            let payload = atelier::encode_set_lock(&atelier::SetLockRequest {
                occurrence,
                expected_revision: atelier.revision,
                gem: gem.id,
                locked: !gem.locked,
            })
            .map_err(|_| "Gem lock change could not be sent")?;
            return WheelCommand::new(atelier::COMMAND_TYPE_ATELIER_SET_LOCK, payload)
                .ok_or("Gem lock change could not be sent");
        }
        Intent::SwitchDomain | Intent::Dismantle => {
            let gem = panel.selected_gem().ok_or("Select a gem first")?;
            if gem.vessel {
                return Err("Remove the gem from its vessel and save first");
            }
            if intent == Intent::Dismantle && gem.locked {
                return Err("Unlock the gem first");
            }
            atelier
                .quality_quotes
                .iter()
                .find(|quote| quote.quality == gem.quality)
                .ok_or("Refresh current Atelier prices first")?;
            if intent == Intent::SwitchDomain {
                atelier::Mutation::SwitchDomain { gem: gem.id }
            } else {
                atelier::Mutation::Dismantle { gem: gem.id }
            }
        }
        Intent::GradeUp => {
            let (kind, mod_id) = panel
                .modifier
                .filter(|id| panel.modifiers().contains(id))
                .ok_or("Select a Workshop modifier first")?;
            let grade = panel.grade(kind, mod_id);
            if grade >= 3 {
                return Err("This modifier is already Grade IV");
            }
            atelier
                .grade_quotes
                .iter()
                .find(|quote| quote.kind == kind && quote.grade == grade + 1)
                .ok_or("Refresh current Workshop prices first")?;
            atelier::Mutation::GradeUp { kind, mod_id }
        }
        _ => return Err("Unknown Wheel action"),
    };
    let payload = atelier::encode_mutate(&atelier::MutateRequest {
        occurrence,
        expected_revision: atelier.revision,
        mutation,
    })
    .map_err(|_| "Atelier change could not be sent")?;
    WheelCommand::new(atelier::COMMAND_TYPE_ATELIER_MUTATE, payload)
        .ok_or("Atelier change could not be sent")
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
        panel.feedback = Some("The response did not match this request; refresh and retry");
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
        Some(false) => Some("The server confirmed the request"),
        Some(true) => {
            Some("The server refused the change; check Premium, points, location and resources")
        }
        None => Some(
            "The outcome is not confirmed; Retry sends the same request without a second payment",
        ),
    };
    let Some((wheel, atelier)) = response
        .wheel
        .as_deref()
        .and_then(|bytes| wheel::decode_wheel_reply(bytes).ok())
        .zip(
            response
                .atelier
                .as_deref()
                .and_then(|bytes| atelier::decode_reply(bytes).ok()),
        )
    else {
        panel.feedback = Some("The current view is unavailable; refresh before editing");
        return false;
    };
    if wheel.refused || atelier.refused {
        panel.feedback = Some(
            "Wheel or Atelier is unavailable for this character; check level, promotion and Premium",
        );
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
        panel.feedback = Some("The response is older than the current view; refresh again");
        return false;
    }
    let mut vessels = [None; 4];
    for gem in &atelier.gems {
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
