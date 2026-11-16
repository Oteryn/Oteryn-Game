//! Native hotkey equip configuration presentation.
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_client_ui::hotkey_equip::{self, HotkeyEquipPanel};

pub(in crate::play) fn submit(
    panel: &mut HotkeyEquipPanel,
    input: &str,
) -> Option<ServerMechanicCommand> {
    if panel.pending || !panel.selected {
        return None;
    }
    let Ok(definition) = input.trim().parse::<u32>() else {
        panel.feedback = Some(tr!("Enter a positive item definition number").into());
        return None;
    };
    panel.item_definition = definition;
    hotkey_equip::control_request(panel)
}

pub(in crate::play) fn panel_text(panel: &HotkeyEquipPanel) -> String {
    if !panel.selected {
        return tr!("Hotkey equip unavailable in this session").into();
    }
    if panel.pending {
        return tr!("Pending equip…").into();
    }
    if let Some(feedback) = &panel.feedback {
        return feedback.clone();
    }
    panel.outcome.as_ref().map_or_else(
        || tr!("Enter an item definition to equip").into(),
        |outcome| lfmt!("Equip result: {outcome:?}", outcome = outcome),
    )
}


