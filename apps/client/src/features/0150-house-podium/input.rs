use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::house_podium::Intent;
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    let panel = &mut mailbox.properties.house_podium;
    if !panel.selected {
        panel.feedback = "Unavailable in this session".into();
        return;
    }
    if panel.pending {
        panel.feedback = "Waiting for the server".into();
        return;
    }
    match super::protocol::command(panel, intent) {
        Ok(command) => {
            if oteryn_client_ui::house_podium::request(panel, command.kind)
                && commands.send(PlayCommand::Feature(command)).is_err()
            {
                panel.failed("The session has ended");
            }
        }
        Err(message) => panel.feedback = message,
    }
}
impl PlayLink {
    /// Open the clicked live item; the server resolves its session-local handle.
    pub fn house_podium_select(&self, handle: u64) {
        {
            let mut mailbox = lock(&self.mailbox);
            let panel = &mut mailbox.properties.house_podium;
            if !panel.display_selected || panel.pending || handle == 0 {
                return;
            }
            edit(panel, 0, handle.to_string());
            submit(&mut mailbox, &self.commands, Intent::Refresh);
        }
        self.house_podium_open();
    }

    pub fn house_podium_edit(&self, field: usize, value: String) {
        edit(
            &mut lock(&self.mailbox).properties.house_podium,
            field,
            value,
        );
    }
    pub fn house_podium_request(&self, intent: Intent) {
        submit(&mut lock(&self.mailbox), &self.commands, intent);
    }
    pub fn house_podium_open(&self) {
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.house_podium.open = true;
        }
    }
    pub fn house_podium_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::house_podium::open_key(events) {
            return false;
        }
        self.house_podium_open();
        true
    }
}

pub(in crate::play) fn edit(
    panel: &mut oteryn_client_ui::house_podium::Panel,
    field: usize,
    value: String,
) {
    if panel.pending || value.len() > 7996 || value.contains('\0') {
        return;
    }
    if let Some(slot) = panel.fields.get_mut(field) {
        if *slot == value {
            return;
        }
        *slot = value;
        panel.preview = None;
        if field == 0 {
            panel.summary.clear();
            panel.projection.clear();
            panel.revision = None;
            panel.loaded_item.clear();
        }
    }
}

#[cfg(any(windows, test))]
pub(in crate::play) fn display_action(
    panel: &mut oteryn_client_ui::house_podium::Panel,
    action: usize,
) {
    if panel.pending {
        return;
    }
    if matches!(action, 4 | 5) && !panel.display_selected {
        panel.feedback = "The server does not support podium visibility controls".into();
        return;
    }
    match action {
        4 if panel.display_selected => panel.visibility.display = !panel.visibility.display,
        5 if panel.display_selected => panel.visibility.platform = !panel.visibility.platform,
        6..=8 => panel.fields[1] = (action - 5).to_string(),
        9 | 10 => {
            let direction = panel.fields[2].trim().parse::<u8>().unwrap_or(0).min(3);
            panel.fields[2] = ((direction + if action == 9 { 3 } else { 1 }) % 4).to_string();
        }
        _ => return,
    }
    panel.preview = None;
    panel.feedback = "Draft changed. Preview checks it; Save commits it".into();
}
