//! Dedicated self-only blessing query using the existing authoritative StoreSummary wire.
use super::{CyclopediaCharacterPanel, decode_store};
use oteryn_client_runtime::{
    cyclopedia_character::{CharacterTab, MAX_TAB_RESULT_BYTES, StoreSummary},
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlessingsPanel {
    pub open: bool,
    pub pending: bool,
    pub snapshot: Option<StoreSummary>,
    pub feedback: Option<&'static str>,
}
impl BlessingsPanel {
    pub fn text(&self) -> String {
        let mut text = String::from("Blessings / death protection\r\nCtrl+Alt+Shift+F12\r\n\r\n");
        if let Some(snapshot) = &self.snapshot {
            text.push_str(&format!(
                "Owned blessings: {}\r\n",
                snapshot.blessings.len()
            ));
            if snapshot.blessings.is_empty() {
                text.push_str("No blessings owned\r\n");
            }
            for key in &snapshot.blessings {
                let label = key
                    .strip_prefix("oteryn:blessing.")
                    .unwrap_or(key)
                    .split('_')
                    .map(|word| {
                        let mut chars = word.chars();
                        chars.next().map_or_else(String::new, |first| {
                            first.to_uppercase().chain(chars).collect::<String>()
                        })
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                text.push_str(&format!("Owned: {label}\r\n"));
            }
            text.push_str(
                "Loss percentages are unavailable; ownership does not imply full protection.\r\n",
            );
        } else {
            text.push_str("No server blessing state loaded\r\n");
        }
        if self.pending {
            text.push_str("Waiting for the server\r\n");
        }
        if let Some(feedback) = self.feedback {
            text.push_str(feedback);
        }
        text
    }
}
pub fn request_blessings(p: &mut CyclopediaCharacterPanel) -> Option<FeatureCommand> {
    if !p.selected {
        p.blessings.feedback = Some("Blessings unavailable in this session");
        return None;
    }
    if p.pending || p.blessings.pending {
        p.blessings.feedback = Some("Waiting for the server");
        return None;
    }
    p.blessings.pending = true;
    p.blessings.feedback = None;
    Some(FeatureCommand {
        capability: 199,
        kind: 388,
        payload: vec![CharacterTab::StoreSummary as u8, 0, 0, 20],
    })
}
pub(super) fn apply(panel: &mut BlessingsPanel, result: &GatedCommandResult) -> bool {
    panel.pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused the blessings query");
        return false;
    }
    let Some(snapshot) = (result.payload.len() <= MAX_TAB_RESULT_BYTES)
        .then(|| decode_store(&result.payload))
        .flatten()
    else {
        panel.feedback = Some("Invalid blessings reply; refresh to retry");
        return false;
    };
    panel.snapshot = Some(snapshot);
    panel.feedback = None;
    true
}
