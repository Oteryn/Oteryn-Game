//! Native presentation of the existing feature request and reducer state.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::item_forge::ItemForgePanel;

pub(in crate::play) const CONTROLS: &[(&str, &str, &str)] = &[
    (
        "EDIT",
        "Operation (8 current dust; 0 history; 1 fusion; 2 convergence fusion; 3 transfer; 4 convergence transfer; 5 slivers; 6 cores; 7 dust limit)",
        "0",
    ),
    ("EDIT", "First item handle", "0"),
    ("EDIT", "Second item handle", "0"),
    (
        "EDIT",
        "Occurrence (32 hex digits; required for mutations)",
        "00000000000000000000000000000000",
    ),
    ("EDIT", "Boost (0 or 1)", "0"),
    ("EDIT", "Protect (0 or 1)", "0"),
    ("EDIT", "History limit (0–50)", "20"),
    ("BUTTON", "Submit", ""),
];
pub(in crate::play) struct Inputs(pub Vec<String>);
impl Default for Inputs {
    fn default() -> Self {
        Self(CONTROLS.iter().map(|c| c.2.to_owned()).collect())
    }
}
pub(in crate::play) fn enabled(panel: &ItemForgePanel) -> bool {
    panel.selected && !panel.pending
}
pub(in crate::play) fn presentation(panel: &ItemForgePanel) -> String {
    let mut text = oteryn_client_ui::item_forge::panel_text(panel);
    if panel.pending {
        text.push_str(tr!("\nWaiting for server…"));
    }
    if panel.decoded.is_none() && !panel.response.is_empty() {
        match serde_json::from_slice::<serde_json::Value>(&panel.response) {
            Ok(value) => text.push_str(&lfmt!("\nResult: {value:#}", value = value)),
            Err(_) => text.push_str(tr!("\nThe server returned invalid forge results")),
        }
    }
    text
}
pub(in crate::play) fn submit(link: &PlayLink, inputs: &Inputs) {
    dispatch(link, inputs, false);
}
pub(in crate::play) fn dispatch(link: &PlayLink, inputs: &Inputs, detail: bool) {
    let mut mailbox = lock(&link.mailbox);
    let panel = &mut mailbox.item_forge;
    if !enabled(panel) {
        return;
    }
    let parsed = (|| {
        let _ = detail;
        use oteryn_client_runtime::cui3::item_forge::Operation;
        let chosen = inputs.0.first()?.trim().parse::<u8>().ok()?;
        panel.query_current_dust = chosen == 8;
        if panel.query_current_dust {
            return Some(());
        }
        let operation = match chosen {
            0 => Operation::QueryHistory,
            1 => Operation::Fusion,
            2 => Operation::ConvergenceFusion,
            3 => Operation::Transfer,
            4 => Operation::ConvergenceTransfer,
            5 => Operation::DustToSlivers,
            6 => Operation::SliversToCores,
            7 => Operation::DustLimit,
            _ => return None,
        };
        let first = inputs.0.get(1)?.trim().parse().ok()?;
        let second = inputs.0.get(2)?.trim().parse().ok()?;
        let hex = inputs.0.get(3)?.trim();
        if hex.len() != 32 || !hex.is_ascii() {
            return None;
        }
        let mut occurrence = [0; 16];
        for (i, byte) in occurrence.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        }
        let boolean = |index: usize| match inputs.0.get(index)?.trim() {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        };
        let boost = boolean(4)?;
        let protect = boolean(5)?;
        let limit = inputs.0.get(6)?.trim().parse().ok()?;
        panel.operation = operation;
        panel.first_item = first;
        panel.second_item = second;
        panel.occurrence = occurrence;
        panel.boost = boost;
        panel.protect = protect;
        panel.history_limit = limit;
        Some(())
    })();
    if parsed.is_none() {
        panel.feedback = Some(tr!("Enter valid values for the labelled controls").into());
        return;
    }
    let command = oteryn_client_ui::item_forge::control_request(panel);
    if let Some(command) = command
        && link
            .commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(command))
            .is_err()
    {
        panel.pending = false;
        panel.feedback = Some(tr!("The session has ended").into());
    }
}
