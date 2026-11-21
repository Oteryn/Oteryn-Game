//! Self-only blessing details with a legacy ownership query when capability 503 is absent.
use super::{CyclopediaCharacterPanel, decode_store};
use oteryn_client_runtime::{
    cyclopedia_character::{CharacterTab, MAX_TAB_RESULT_BYTES, StoreSummary},
    raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult},
};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlessingsPanel {
    pub open: bool,
    pub details_selected: bool,
    pub details: Option<oteryn_client_runtime::cyclopedia_character::blessings_dialog::Snapshot>,
    details_pending: bool,
    pub pending: bool,
    pub snapshot: Option<StoreSummary>,
    pub feedback: Option<&'static str>,
}
fn blessing_label(key: &str) -> String {
    key.strip_prefix("oteryn:blessing.")
        .unwrap_or(key)
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect::<String>()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}
impl BlessingsPanel {
    pub fn holdings(&self) -> Option<&[String]> {
        self.details
            .as_ref()
            .map(|s| s.blessings.as_slice())
            .or_else(|| self.snapshot.as_ref().map(|s| s.blessings.as_slice()))
    }
    pub fn text(&self) -> String {
        let mut text = String::from("Blessings / death protection\r\nCtrl+Alt+Shift+F12\r\n\r\n");
        if let Some(holdings) = self.holdings() {
            text.push_str(&format!("Owned blessings: {}\r\n", holdings.len()));
            if holdings.is_empty() {
                text.push_str("No blessings owned\r\n");
            }
            for key in holdings {
                let label = blessing_label(key);
                text.push_str(&format!("Owned: {label}\r\n"));
            }
            if self.details.is_none() {
                text.push_str("Loss percentages are unavailable; ownership does not imply full protection.\r\n");
            }
        } else {
            text.push_str("No server blessing state loaded\r\n");
        }
        if let Some(details) = &self.details {
            text.push_str(&format!(
                "Experience loss protection: {}%\r\nSkill/magic progress protection: {}%\r\n",
                details.experience_protection_percent, details.skill_protection_percent
            ));
            for (heading, body) in super::super::protection_cards(&details.forecast) {
                text.push_str(&format!("{heading}\r\n{body}\r\n"));
            }
            text.push_str(super::super::PROTECTION_NOTE);
            text.push_str("\r\nBlessing history (latest 20 recorded events, UTC)\r\n");
            if details.history.is_empty() {
                text.push_str("No recorded blessing events\r\n");
            }
            for entry in &details.history {
                let date = crate::house::format_ownership_date(entry.at_unix_ms as i64);
                text.push_str(&format!(
                    "{date} · {}: {}\r\n",
                    if entry.acquired {
                        "Acquired"
                    } else {
                        "Consumed on death"
                    },
                    entry
                        .blessings
                        .iter()
                        .map(|key| blessing_label(key))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        if self.pending {
            text.push_str(tr!("Waiting for the server\r\n"));
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
        p.blessings.feedback = Some(tr!("Waiting for the server"));
        return None;
    }
    p.blessings.pending = true;
    p.blessings.details_pending = p.blessings.details_selected;
    p.blessings.feedback = None;
    Some(FeatureCommand {
        capability: 199,
        kind: 388,
        payload: if p.blessings.details_selected {
            vec![oteryn_client_runtime::cyclopedia_character::blessings_dialog::SELECTOR]
        } else {
            vec![CharacterTab::StoreSummary as u8, 0, 0, 20]
        },
    })
}
pub(super) fn apply(panel: &mut BlessingsPanel, result: &GatedCommandResult) -> bool {
    panel.pending = false;
    let details_pending = std::mem::take(&mut panel.details_pending);
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused the blessings query");
        return false;
    }
    if details_pending {
        let Some(details) = crate::blessings_dialog::decode(&result.payload) else {
            panel.feedback = Some("Invalid blessings reply; refresh to retry");
            return false;
        };
        panel.details = Some(details);
        panel.feedback = None;
        return true;
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
