use super::*;
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    LootAll,
    Corpse([u8; 16]),
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
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    };
    match &action {
        Action::LootAll => {}
        Action::Corpse(id) => {
            wire.action = QuickLootAction::Click;
            wire.corpse = Some(*id);
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
        Action::Destination {
            category,
            recipient,
            id,
            fallback,
        } => {
            wire.action = QuickLootAction::Assign;
            wire.category = *category;
            wire.recipient = Some(*recipient);
            wire.recipient_id = *id;
            wire.fallback_to_main = *fallback;
        }
    }
    let payload = match encode_quick_loot_request(&wire) {
        Ok(payload) => payload,
        Err(_) => {
            panel.feedback = tr!("Choose valid loot details").into();
            return None;
        }
    };
    panel.pending = Some(action);
    panel.feedback = tr!("Waiting for the server").into();
    Some(ServerMechanicCommand {
        capability: 98,
        command_type: 114,
        payload,
    })
}
pub fn apply_result(panel: &mut Panel, response: &ServerMechanicResponse) {
    let Some(action) = panel.pending.take() else {
        return;
    };
    if matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && let Ok(result) = decode_quick_loot_result(&response.payload)
    {
        if !matches!(action, Action::LootAll | Action::Corpse(_)) {
            if panel.confirmed.len() == 100 {
                panel.confirmed.remove(0);
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
        return;
    }
    panel.feedback = tr!("Loot request refused or invalid reply; try again").into();
}
