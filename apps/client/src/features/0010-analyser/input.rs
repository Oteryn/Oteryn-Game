#[cfg(feature = "bevy-shell")]
#[path = "input/bound.rs"]
mod bound;
use crate::combat_input::PlayCommand;
use crate::play::PlayLink;
use crate::play::mailbox::{Mailbox, lock};
#[cfg(feature = "bevy-shell")]
pub(crate) use bound::{ANALYSER_TOOLS_LABELS, AnalyserToolsSnapshot, requires_confirmation};
use oteryn_input_actions::NormalizedInputEvent;
#[cfg(any(windows, test, feature = "bevy-shell"))]
pub(in crate::play) fn select_tool(panel: &mut oteryn_client_ui::analyser::Panel, index: usize) {
    if let Some(tool) = oteryn_client_ui::analyser::Tool::CYCLOPEDIA.get(index) {
        panel.tool = *tool;
    } else if index == oteryn_client_ui::analyser::Tool::CYCLOPEDIA.len() {
        oteryn_client_ui::analyser::protocol::reset(panel);
    }
}

use tokio::sync::mpsc::UnboundedSender;
#[cfg(any(windows, test, feature = "bevy-shell"))]
#[path = "input/bosses.rs"]
mod bosses;
#[path = "input/party_import.rs"]
mod party_import;
impl PlayLink {
    pub fn analyser_xp_goal(&self, value: &str) -> bool {
        oteryn_client_ui::analyser::set_xp_goal(
            &mut crate::play::mailbox::lock(&self.mailbox).analyser,
            value,
        )
    }
    pub fn analyser_item_price(&self, item: &str, price: &str) -> bool {
        oteryn_client_ui::analyser::set_item_price(
            &mut crate::play::mailbox::lock(&self.mailbox).analyser,
            item,
            price,
        )
    }
    pub fn analyser_resolve_items(&self) {
        resolve_items(&mut lock(&self.mailbox), &self.commands);
    }
    pub fn analyser_select(&self, key: &str) -> bool {
        oteryn_client_ui::analyser::select_tool(
            &mut crate::play::mailbox::lock(&self.mailbox).analyser,
            key,
        )
    }
    pub fn analyser_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::analyser::open_key(events) {
            return false;
        }
        self.analyser_open();
        true
    }
    pub fn analyser_open(&self) {
        self.analyser_resolve_items();
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            super::view::open_window(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            crate::play::mailbox::lock(&self.mailbox).analyser.open = true;
        }
    }
}

pub(in crate::play) fn resolve_items(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    if let Some(command) =
        oteryn_client_ui::analyser::protocol::catalogue_request(&mut mailbox.analyser)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.analyser.catalogue_pending.clear();
        mailbox.analyser.feedback = Some(tr!("Session ended"));
    }
}

/// Preserve the seven cyclopedia selector actions alongside the expanded analytics controls.
#[cfg(any(windows, test, feature = "bevy-shell"))]
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: usize,
    values: Vec<String>,
) {
    use oteryn_client_ui::analyser::Tool;
    let tools = Tool::ALL.len();
    if action < Tool::CYCLOPEDIA.len() {
        select_tool(&mut mailbox.analyser, action);
    } else if let Some(tool) = Tool::ALL.get(action) {
        oteryn_client_ui::analyser::select_tool(&mut mailbox.analyser, tool.key());
    } else if action == tools {
        select_tool(&mut mailbox.analyser, Tool::CYCLOPEDIA.len());
    } else if action == tools + 1
        && let Some(value) = values.first()
    {
        oteryn_client_ui::analyser::set_xp_goal(&mut mailbox.analyser, value);
    } else if action == tools + 2
        && let [_, item, price, ..] = values.as_slice()
    {
        oteryn_client_ui::analyser::set_item_price(&mut mailbox.analyser, item, price);
    } else if action == tools + 3 {
        resolve_items(mailbox, commands);
    } else if action == tools + 4
        && let Some(path) = values.get(3)
    {
        mailbox.analyser.export_path = path.clone();
        mailbox.analyser.feedback = Some(
            match save_session(&mailbox.analyser, std::path::Path::new(path)) {
                Ok(()) => tr!("Session report exported"),
                Err(_) => tr!("Session export failed; choose a writable file path"),
            },
        );
    } else if action == tools + 5 {
        refresh_party(mailbox, commands);
    } else if action == tools + 6
        && let [_, item, price, ..] = values.as_slice()
    {
        let tool = mailbox.analyser.tool;
        mailbox.analyser.tool = Tool::Supplies;
        oteryn_client_ui::analyser::set_item_price(&mut mailbox.analyser, item, price);
        mailbox.analyser.tool = tool;
    } else if (action == tools + 7 || action == tools + 8)
        && let Some(item) = values.get(1)
    {
        oteryn_client_ui::analyser::tracking::select(
            &mut mailbox.analyser,
            item,
            action == tools + 7,
        );
    } else if action == tools + 9 {
        mailbox.analyser.loot_only_tracked = !mailbox.analyser.loot_only_tracked;
    } else if action == tools + 10
        && let [_, race, target, ..] = values.as_slice()
    {
        oteryn_client_ui::analyser::kills::goal(&mut mailbox.analyser, race, Some(target));
    } else if action == tools + 11
        && let Some(race) = values.get(1)
    {
        oteryn_client_ui::analyser::kills::goal(&mut mailbox.analyser, race, None);
    } else if action == tools + 12 {
        refresh_tasks(mailbox, commands);
    } else if (tools + 13..=tools + 17).contains(&action) {
        bosses::action(
            mailbox,
            commands,
            action - tools - 13,
            values.get(1).map(String::as_str),
        );
    } else if (tools + 20..=tools + 22).contains(&action)
        && let [_, name, cost, ..] = values.as_slice()
    {
        oteryn_client_ui::analyser::party::edit(
            &mut mailbox.analyser,
            name,
            (action == tools + 20).then_some(cost.as_str()),
            (action != tools + 20).then_some(action == tools + 21),
        );
    } else if (tools + 18..=tools + 19).contains(&action) {
        party_import::submit(
            mailbox,
            action == tools + 18,
            values.get(3).map(String::as_str),
        );
    }
}

/// The player explicitly supplies a destination; remote identities never supply paths.
pub(in crate::play) fn save_session(
    panel: &oteryn_client_ui::analyser::Panel,
    path: &std::path::Path,
) -> std::io::Result<()> {
    if !panel.selected {
        return Err(std::io::Error::other(tr!("No active analyser session")));
    }
    std::fs::write(path, oteryn_client_ui::analyser::session_text(panel))
}

pub(in crate::play) fn refresh_party(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    mailbox.analyser.party_import = None;
    if !mailbox.party.selected {
        mailbox.analyser.party_feedback =
            Some(tr!("Party membership is unavailable in this session"));
        return;
    }
    if let Some(command) =
        oteryn_client_ui::analyser::protocol::party_request(&mut mailbox.analyser)
        && commands.send(PlayCommand::Feature(command)).is_err()
    {
        mailbox.analyser.party_pending = false;
        mailbox.analyser.party_feedback = Some(tr!("Session ended"));
    }
}

impl PlayLink {
    pub fn analyser_party_refresh(&self) {
        refresh_party(&mut lock(&self.mailbox), &self.commands);
    }
    pub fn analyser_save_session(&self, path: &std::path::Path) -> std::io::Result<()> {
        save_session(&lock(&self.mailbox).analyser, path)
    }
}

pub(in crate::play) fn refresh_tasks(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    let Some(command) = oteryn_client_ui::task_board::full_request(
        &mut mailbox.task_board,
        oteryn_client_ui::task_board::FullIntent::Refresh,
    ) else {
        mailbox.analyser.task_feedback = Some(tr!(
            "Hunting tasks unavailable or a request is already pending"
        ));
        return;
    };
    mailbox.analyser.task_feedback = Some(tr!("Reading confirmed hunting tasks"));
    if commands.send(PlayCommand::Feature(command)).is_err() {
        mailbox.task_board.pending = false;
        mailbox.analyser.task_feedback = Some(tr!("Session ended"));
    }
}
impl PlayLink {
    pub fn analyser_refresh_tasks(&self) {
        refresh_tasks(&mut lock(&self.mailbox), &self.commands);
    }
}

/// Prepare a window without constructing a second session-owning PlayLink.
#[cfg(any(windows, test))]
pub(in crate::play) fn begin_shared_window(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) -> bool {
    if mailbox.analyser.open {
        return false;
    }
    resolve_items(mailbox, commands);
    mailbox.analyser.open = true;
    true
}
