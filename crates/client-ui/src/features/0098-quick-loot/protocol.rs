use super::*;
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    LootAll,
    LootAllFrom([u8; 16]),
    Corpse([u8; 16]),
    CorpseItem {
        corpse: [u8; 16],
        item: oteryn_client_runtime::inventory::ItemHandle,
    },
    Add {
        mode: QuickLootFilterMode,
        definition: u32,
    },
    Remove {
        mode: QuickLootFilterMode,
        definition: u32,
    },
    Clear(QuickLootFilterMode),
    Select(QuickLootFilterMode),
    ReviewDestinations,
    Distribute(oteryn_client_runtime::inventory::ItemHandle),
    Destination {
        category: Option<QuickLootCategory>,
        recipient: QuickLootRecipient,
        id: Option<[u8; 16]>,
        fallback: bool,
    },
}
pub fn request(panel: &mut Panel, action: Action) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending.is_some() {
        return None;
    }
    let mut wire = QuickLootRequest {
        action: QuickLootAction::LootAll,
        corpse: None,
        selected_item: None,
        review_filters: false,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    };
    match &action {
        Action::LootAll => {}
        Action::LootAllFrom(id) => wire.corpse = Some(*id),
        Action::Corpse(id) => {
            wire.action = QuickLootAction::Click;
            wire.corpse = Some(*id);
        }
        Action::CorpseItem { corpse, item } => {
            wire.action = QuickLootAction::Click;
            wire.corpse = Some(*corpse);
            wire.selected_item = Some(*item);
        }
        Action::Add { mode, definition } | Action::Remove { mode, definition } => {
            wire.action = if matches!(action, Action::Add { .. }) {
                QuickLootAction::FilterAdd
            } else {
                QuickLootAction::FilterRemove
            };
            wire.filter_mode = Some(*mode);
            wire.filter_definition_ref = Some(*definition);
        }
        Action::Clear(mode) => {
            wire.action = QuickLootAction::FilterClear;
            wire.filter_mode = Some(*mode);
        }
        Action::Select(mode) => {
            wire.action = QuickLootAction::FilterSelect;
            wire.filter_mode = Some(*mode);
        }
        Action::ReviewDestinations => wire = quick_loot_filter_review_request(),
        Action::Distribute(_) => wire.action = QuickLootAction::Distribute,
        Action::Destination {
            category,
            recipient,
            id,
            fallback,
        } => {
            wire.action = if panel.destinations_selected {
                QuickLootAction::AssignLoot
            } else {
                QuickLootAction::Assign
            };
            wire.category = *category;
            wire.recipient = Some(*recipient);
            wire.recipient_id = *id;
            wire.fallback_to_main = *fallback;
        }
    }
    let command_type = wire.action.command_type();
    let capability = if command_type == COMMAND_TYPE_QUICK_LOOT {
        CAPABILITY_QUICK_LOOT_V1
    } else if panel.destinations_selected {
        CAPABILITY_QUICK_LOOT_DESTINATIONS_V1
    } else {
        return None;
    };
    let encoded = match action {
        Action::Distribute(source) => encode_quick_loot_view_request(&wire, source),
        _ => encode_quick_loot_request(&wire),
    };
    let payload = match encoded {
        Ok(payload) => payload,
        Err(_) => {
            panel.feedback = tr!("Choose valid loot details").into();
            return None;
        }
    };
    panel.pending = Some(action);
    panel.feedback = tr!("Waiting for the server").into();
    panel.item_feedback = None;
    Some(ServerMechanicCommand {
        capability,
        command_type,
        payload,
    })
}
/// Apply a server answer. Returns whether it confirmed a loot run (loot all or one corpse),
/// whose result line belongs in the Loot tab.
pub fn apply_result(panel: &mut Panel, response: &ServerMechanicResponse) -> bool {
    use CommandStatus::{Accepted, DuplicateReplay};

    let Some(action) = panel.pending.take() else {
        return false;
    };
    if action == Action::ReviewDestinations {
        panel.feedback = match apply_destinations(panel, response) {
            Ok(()) => tr!("Confirmed by the server"),
            Err(message) => message,
        }
        .into();
        return false;
    }
    let can_confirm = matches!(response.status, Accepted | DuplicateReplay);
    if let Action::CorpseItem { corpse, .. } = action {
        let message = if can_confirm
            && QuickLootClickOutcome::decode(&response.payload) == Ok(QuickLootClickOutcome::Moved)
        {
            tr!("Item moved")
        } else {
            match refusal(response) {
                Some(QuickLootRefusal::Stale) => tr!("Item is no longer available"),
                Some(_) => tr!("Item move refused"),
                None => tr!("Loot request refused or invalid reply; try again"),
            }
        };
        panel.feedback = message.into();
        panel.item_feedback = Some((corpse, message));
        return false;
    }
    if can_confirm && let Ok(result) = decode_quick_loot_result(&response.payload) {
        if matches!(action, Action::Distribute(_)) {
            panel.distribution_result = Some(result);
            panel.feedback = tr!("Confirmed by the server").into();
            return false;
        }
        let looted = matches!(
            action,
            Action::LootAll | Action::LootAllFrom(_) | Action::Corpse(_)
        );
        if !looted {
            if panel.confirmed.len() == 100 {
                panel.confirmed.remove(0);
            }
            match action {
                Action::Add { mode, .. }
                | Action::Remove { mode, .. }
                | Action::Select(mode)
                | Action::Clear(mode) => panel.filter_mode = Some(mode),
                _ => {}
            }
            match &action {
                Action::Add { mode, definition } => {
                    let list = if *mode == QuickLootFilterMode::Accept {
                        &mut panel.accepted
                    } else {
                        &mut panel.skipped
                    };
                    list.insert(*definition);
                }
                Action::Remove { mode, definition } => {
                    let list = if *mode == QuickLootFilterMode::Accept {
                        &mut panel.accepted
                    } else {
                        &mut panel.skipped
                    };
                    list.remove(definition);
                }
                Action::Clear(mode) => {
                    if *mode == QuickLootFilterMode::Accept {
                        panel.accepted.clear();
                    } else {
                        panel.skipped.clear();
                    }
                }
                Action::Destination { category, .. } => {
                    panel.destinations.retain(|previous| {
                        !matches!(previous,
                        Action::Destination { category: previous, .. } if previous == category)
                    });
                    panel.destinations.push(action.clone());
                }
                _ => {}
            }
            panel.confirmed.push(action);
        }
        panel.result = Some(result);
        panel.feedback = tr!("Confirmed by the server").into();
        return looted;
    }
    panel.feedback = failure_message(response).into();
    false
}

/// Apply the saved Loot and Obtain routes from a destination review.
pub fn apply_destinations(
    panel: &mut Panel,
    response: &ServerMechanicResponse,
) -> Result<(), &'static str> {
    if !matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        return Err(failure_message(response));
    }
    let saved =
        decode_quick_loot_destinations(&response.payload).map_err(|_| failure_message(response))?;
    panel.destinations = saved
        .loot
        .into_iter()
        .map(|row| Action::Destination {
            category: row.category,
            recipient: row.recipient,
            id: row.recipient_id,
            fallback: saved.fallback_to_main,
        })
        .collect();
    panel.obtain_destinations = saved.obtain;
    if let Some(filters) = saved.filters {
        panel.filter_mode = Some(filters.mode);
        panel.skipped = filters.skipped.into_iter().collect();
        panel.accepted = filters.accepted.into_iter().collect();
    }
    Ok(())
}

fn failure_message(response: &ServerMechanicResponse) -> &'static str {
    if refusal(response).is_some() {
        tr!("The server refused the request")
    } else {
        tr!("Loot request refused or invalid reply; try again")
    }
}

fn refusal(response: &ServerMechanicResponse) -> Option<QuickLootRefusal> {
    // A replay carries the original terminal payload, including typed refusals.
    match response.status {
        CommandStatus::Rejected | CommandStatus::DuplicateReplay => {
            decode_quick_loot_refusal(&response.payload).ok()
        }
        CommandStatus::Accepted | CommandStatus::DuplicateOutcomeExpired => None,
    }
}

#[cfg(test)]
mod destination_request_tests {
    use super::*;

    #[test]
    fn quick_loot_gap_0607_legacy_rejects_managed_actions_without_changing_bytes() {
        let mut panel = Panel {
            selected: true,
            ..Panel::default()
        };
        assert!(request(&mut panel, Action::ReviewDestinations).is_none());
        assert!(request(&mut panel, Action::Distribute(std::num::NonZeroU64::MIN)).is_none());
        assert!(panel.pending.is_none());
        assert_eq!(
            request(&mut panel, Action::LootAll).map(|command| command.payload),
            Some(vec![8, 2])
        );
    }

    #[test]
    fn destination_assignment_uses_the_negotiated_wire() -> Result<(), &'static str> {
        for (destinations_selected, action, capability, command_type) in [
            (false, QuickLootAction::Assign, 98, 114),
            (true, QuickLootAction::AssignLoot, 607, 6376),
        ] {
            let mut panel = Panel {
                selected: true,
                destinations_selected,
                ..Panel::default()
            };
            let category = Some(QuickLootCategory::Gold);
            let recipient = QuickLootRecipient::MainContainer;
            let command = request(
                &mut panel,
                Action::Destination {
                    category,
                    recipient,
                    id: None,
                    fallback: true,
                },
            )
            .ok_or("destination request")?;
            if !destinations_selected {
                assert_eq!(command.payload, [8, 3, 24, 1, 32, 3, 48, 1]);
            }
            assert_eq!(
                (command.capability, command.command_type),
                (capability, command_type)
            );
            let wire = decode_quick_loot_request(&command.payload).map_err(|_| "wire")?;
            assert_eq!(wire.action, action);
            assert_eq!(wire.category, category);
            assert_eq!(wire.recipient, Some(recipient));
            assert!(wire.fallback_to_main);
        }
        Ok(())
    }
}
