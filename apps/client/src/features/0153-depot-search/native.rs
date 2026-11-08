//! Native presentation of the existing feature request and reducer state.
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::depot_search::DepotSearchPanel;
use tokio::sync::mpsc::UnboundedSender;

pub(in crate::play) const CONTROLS: &[(&str, &str, &str)] = &[
    ("EDIT", "Item id", "100"),
    ("BUTTON", "Search", ""),
    ("EDIT", "Result row (starting at 0)", "0"),
    ("EDIT", "Tier", "0"),
    ("BUTTON", "Select result / details", ""),
    ("BUTTON", "List holdings", ""),
    ("BUTTON", "Next page", ""),
    ("BUTTON", "Retrieve one", ""),
    ("BUTTON", "Retrieve all (all tiers)", ""),
];
pub(in crate::play) struct Inputs(pub Vec<String>);
impl Default for Inputs {
    fn default() -> Self {
        Self(CONTROLS.iter().map(|c| c.2.to_owned()).collect())
    }
}
pub(in crate::play) fn enabled(panel: &DepotSearchPanel) -> bool {
    panel.selected && !panel.pending
}
pub(in crate::play) fn presentation(panel: &DepotSearchPanel) -> String {
    let mut text = oteryn_client_ui::depot_search::panel_text(panel);
    if panel.pending {
        text.push_str("\nWaiting for server…");
    }

    text
}
#[cfg(test)]
pub(in crate::play) fn submit(link: &PlayLink, inputs: &Inputs) {
    dispatch(link, inputs, false);
}
#[cfg(test)]
pub(in crate::play) fn dispatch(link: &PlayLink, inputs: &Inputs, detail: bool) {
    dispatch_panel(
        &mut lock(&link.mailbox).depot_search,
        &link.commands,
        inputs,
        detail,
    );
}
fn dispatch_panel(
    panel: &mut DepotSearchPanel,
    commands: &UnboundedSender<PlayCommand>,
    inputs: &Inputs,
    detail: bool,
) {
    if !enabled(panel) {
        return;
    }
    let parsed = (|| {
        let item_id = inputs.0.first()?.trim().parse().ok()?;
        let tier = inputs.0.get(3)?.trim().parse().ok()?;
        if detail {
            let row: usize = inputs.0.get(2)?.trim().parse().ok()?;
            let result = panel.reply.as_ref()?.results.get(row)?;
            if !result.tiers.iter().any(|entry| entry.tier == tier) {
                return None;
            }
            panel.item_id = result.client_id;
        } else {
            panel.item_id = item_id;
        }
        panel.tier = tier;
        Some(())
    })();
    if parsed.is_none() {
        panel.feedback = Some("Enter valid values for the labelled controls".into());
        return;
    }
    let command = if detail {
        oteryn_client_ui::depot_search::detail(panel)
    } else {
        oteryn_client_ui::depot_search::search(panel)
    };
    if let Some(command) = command
        && commands.send(PlayCommand::ServerMechanic(command)).is_err()
    {
        panel.pending = false;
        panel.feedback = Some("The session has ended".into());
    }
}

#[cfg(test)]
pub(in crate::play) fn activate(link: &PlayLink, inputs: &Inputs, action: usize) {
    activate_shared(&link.mailbox, &link.commands, inputs, action);
}
#[cfg(test)]
pub(in crate::play) fn activate_shared(
    shared: &SharedMailbox,
    commands: &UnboundedSender<PlayCommand>,
    inputs: &Inputs,
    action: usize,
) {
    activate_panel(&mut lock(shared).depot_search, commands, inputs, action);
}

pub(in crate::play) fn activate_displayed(
    shared: &SharedMailbox,
    commands: &UnboundedSender<PlayCommand>,
    inputs: &Inputs,
    action: usize,
    displayed: Option<&oteryn_client_runtime::cui3::depot_search::Reply>,
) {
    let mut mailbox = lock(shared);
    let panel = &mut mailbox.depot_search;
    if matches!(action, 4 | 7 | 8) && panel.reply.as_ref() != displayed {
        panel.feedback = Some("Results changed; select from the refreshed list".into());
        return;
    }
    activate_panel(panel, commands, inputs, action);
}

fn activate_panel(
    panel: &mut DepotSearchPanel,
    commands: &UnboundedSender<PlayCommand>,
    inputs: &Inputs,
    action: usize,
) {
    use oteryn_client_runtime::cui3::depot_search::{self as wire, Outcome, Request};
    if action == 1 {
        dispatch_panel(panel, commands, inputs, false);
        return;
    }
    if action == 4 {
        dispatch_panel(panel, commands, inputs, true);
        return;
    }
    if !enabled(panel) {
        return;
    }
    let request = match action {
        5 => Some(Request::Open { after: 0 }),
        6 => panel
            .reply
            .as_ref()
            .filter(|r| r.outcome == Outcome::More)
            .and_then(|r| r.results.last())
            .map(|row| Request::Open {
                after: row.client_id,
            }),
        7 | 8 => inputs
            .0
            .get(2)
            .and_then(|s| s.trim().parse::<usize>().ok())
            .and_then(|i| panel.reply.as_ref()?.results.get(i))
            .map(|row| Request::Retrieve {
                client_id: row.client_id,
                all: action == 8,
            }),
        _ => None,
    };
    let Some(payload) = request.and_then(|r| wire::encode_request(r).ok()) else {
        panel.feedback =
            Some("Select a current result, or list holdings for the first page".into());
        return;
    };
    if let Some(command) = oteryn_client_ui::depot_search::request(
        panel,
        oteryn_client_ui::depot_search::command(payload),
    ) && commands.send(PlayCommand::ServerMechanic(command)).is_err()
    {
        panel.pending = false;
        panel.feedback = Some("The session has ended".into());
    }
}

#[cfg(windows)]
pub(in crate::play) use windows::open_window;
#[cfg(windows)]
#[path = "native/windows.rs"]
mod windows;
