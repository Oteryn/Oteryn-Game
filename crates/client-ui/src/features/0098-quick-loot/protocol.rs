use super::*;
use oteryn_client_runtime::inventory::ItemHandle;
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    LootAll,
    Corpse([u8; 16]),
    CorpseHandle(ItemHandle),
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
        Action::CorpseHandle(handle) => {
            if !panel.handles_selected {
                return None;
            }
            wire.action = QuickLootAction::Click;
            wire.corpse = Some(encode_target_handle(*handle));
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
    let command = if matches!(action, Action::CorpseHandle(_)) {
        crate::quick_loot_target_handles::request(&wire)
    } else {
        encode_quick_loot_request(&wire).map(|payload| ServerMechanicCommand {
            capability: CAPABILITY_QUICK_LOOT_V1,
            command_type: COMMAND_TYPE_QUICK_LOOT,
            payload,
        })
    };
    let command = match command {
        Ok(command) => command,
        Err(_) => {
            panel.feedback = tr!("Choose valid loot details").into();
            return None;
        }
    };
    panel.pending = Some(action);
    panel.feedback = tr!("Waiting for the server").into();
    Some(command)
}
/// Apply a server answer. Returns whether it confirmed a loot run (loot all or one corpse),
/// whose result line belongs in the Loot tab.
pub fn apply_result(panel: &mut Panel, response: &ServerMechanicResponse) -> bool {
    let Some(action) = panel.pending.take() else {
        return false;
    };
    if matches!(
        response.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && let Ok(result) = decode_quick_loot_result(&response.payload)
    {
        let looted = matches!(
            action,
            Action::LootAll | Action::Corpse(_) | Action::CorpseHandle(_)
        );
        if !looted {
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
        return looted;
    }
    panel.feedback = tr!("Loot request refused or invalid reply; try again").into();
    false
}

#[cfg(test)]
#[test]
fn target_encoding_follows_the_selected_command() -> Result<(), Box<dyn std::error::Error>> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    let handle = ItemHandle::new(42).ok_or("item handle")?;
    assert!(request(&mut panel, Action::CorpseHandle(handle)).is_none());

    panel.handles_selected = true;
    let encoded = request(&mut panel, Action::CorpseHandle(handle)).ok_or("handle command")?;
    assert_eq!(encoded.capability, CAPABILITY_QUICK_LOOT_TARGET_HANDLES_V1);
    assert_eq!(encoded.command_type, COMMAND_TYPE_QUICK_LOOT_TARGET_HANDLES);
    assert_eq!(
        decode_quick_loot_request(&encoded.payload).map(|decoded| decoded.corpse),
        Ok(Some(encode_target_handle(handle))),
    );

    panel.pending = None;
    let uuid = [7; 16];
    let legacy = request(&mut panel, Action::Corpse(uuid)).ok_or("legacy command")?;
    assert_eq!(legacy.capability, CAPABILITY_QUICK_LOOT_V1);
    assert_eq!(legacy.command_type, COMMAND_TYPE_QUICK_LOOT);
    assert_eq!(
        decode_quick_loot_request(&legacy.payload).map(|decoded| decoded.corpse),
        Ok(Some(uuid)),
    );
    Ok(())
}
