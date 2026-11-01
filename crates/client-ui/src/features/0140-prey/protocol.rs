use super::{CommandStatus, PickConfirmation, PreyPanel, PreyResponse, prey};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    SetTier { slot: u8, tier: u8 },
    ListReroll { slot: u8 },
    Select { slot: u8 },
    BonusReroll { slot: u8 },
    SetOption { slot: u8, option: u8 },
    OfferedToday { slot: u8, after: String },
    ConfirmPickAgain(PickConfirmation),
}

pub fn request(panel: &mut PreyPanel, intent: Intent) -> Option<prey::Request> {
    if !panel.selected || panel.pending.is_some() {
        return None;
    }
    let request = match intent {
        Intent::Refresh => prey::Request::Query,
        Intent::SetTier { slot, tier } => {
            if !panel.unlocked(slot)
                || !(1..=5).contains(&tier)
                || panel
                    .slot(slot)
                    .is_none_or(|state| tier > state.allowed_tier)
            {
                return None;
            }
            prey::Request::SetTier { slot, tier }
        }
        Intent::ListReroll { slot } if panel.unlocked(slot) => prey::Request::ListReroll { slot },
        Intent::Select { slot } if panel.unlocked(slot) => {
            let index = panel.candidate[usize::from(slot - 1)]?;
            if panel.slot(slot)?.state != 1
                || usize::from(index) > panel.slot(slot)?.creatures.len()
            {
                return None;
            }
            prey::Request::Select { slot, index }
        }
        Intent::BonusReroll { slot } if panel.unlocked(slot) => prey::Request::BonusReroll { slot },
        Intent::SetOption { slot, option } if panel.unlocked(slot) => {
            prey::Request::SetOption { slot, option }
        }
        Intent::OfferedToday { slot, after } if panel.unlocked(slot) => {
            prey::Request::OfferedToday { slot, after }
        }
        Intent::ConfirmPickAgain(confirmation) => {
            if panel.confirmation.as_ref() != Some(&confirmation)
                || !panel.unlocked(confirmation.slot)
                || !panel.offered_today.iter().any(|row| {
                    row.creature == confirmation.creature
                        && row.price_gold == confirmation.price_gold
                        && (row.occupied_slot == 0 || row.occupied_slot == confirmation.slot)
                })
            {
                return None;
            }
            prey::Request::PickAgain {
                slot: confirmation.slot,
                creature: confirmation.creature,
                expected_price_gold: confirmation.price_gold,
            }
        }
        _ => return None,
    };
    prey::encode_request(&request).ok()?;
    panel.confirmation = None;
    panel.feedback = None;
    panel.pending = Some(request.clone());
    Some(request)
}

fn status_text(status: u8) -> Option<&'static str> {
    match status {
        0 => None,
        prey::STATUS_CONTENT_UNAVAILABLE => Some("This creature is unavailable for Prey."),
        prey::STATUS_NO_CANDIDATES => {
            Some("No complete list of unoffered creatures is available today. No gold was charged.")
        }
        prey::STATUS_INVALID_TIER => Some("This difficulty is unavailable at your current level."),
        prey::STATUS_NOT_OFFERED => Some("This creature has not been offered to you today."),
        prey::STATUS_OCCUPIED => Some("This creature is already in another Prey slot."),
        prey::STATUS_ALREADY_OWNED => Some("Prey List Expansion is already owned."),
        _ => Some("The server refused this Prey action."),
    }
}

pub fn apply_result(panel: &mut PreyPanel, response: &PreyResponse) -> bool {
    if panel.pending.as_ref() != Some(&response.request) {
        return false;
    }
    panel.pending = None;
    panel.confirmation = None;
    let accepted = matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    );
    let operation = accepted
        .then(|| prey::decode_reply(&response.payload).ok())
        .flatten();
    panel.feedback = if accepted {
        operation.as_ref().map_or(
            Some("The server returned an invalid Prey reply."),
            |reply| status_text(reply.status),
        )
    } else {
        Some("The server refused this action; refresh the list before trying again.")
    };
    let current = match &response.current {
        Some(bytes) => prey::decode_reply(bytes).ok(),
        None if matches!(
            response.request,
            prey::Request::Query | prey::Request::OfferedToday { .. }
        ) =>
        {
            operation.clone()
        }
        None => None,
    };
    if let (prey::Request::OfferedToday { slot, .. }, Some(reply)) = (&response.request, &operation)
        && reply.status == 0
    {
        panel.offered_slot = Some(*slot);
        panel.offered_today.clone_from(&reply.offered_today);
        panel.offered_next.clone_from(&reply.offered_next);
    } else if !matches!(response.request, prey::Request::Query) {
        panel.offered_slot = None;
        panel.offered_today.clear();
        panel.offered_next.clear();
    }
    if let Some(current) = current {
        for slot in &current.slots {
            let index = usize::from(slot.slot - 1);
            if panel
                .confirmed
                .as_ref()
                .and_then(|reply| reply.slots.iter().find(|old| old.slot == slot.slot))
                .is_none_or(|old| old.creatures != slot.creatures)
            {
                panel.candidate[index] = None;
            }
        }
        panel.confirmed = Some(current);
        true
    } else {
        if accepted
            && !matches!(
                response.request,
                prey::Request::Query | prey::Request::OfferedToday { .. }
            )
        {
            // The mutation may have committed, but its receipt can describe an old replay.
            // Require a current query before enabling another action against that state.
            panel.confirmed = None;
            panel.candidate = [None; 3];
            panel.feedback =
                Some("Current Prey state is unavailable. Refresh before sending another action.");
        }
        false
    }
}
