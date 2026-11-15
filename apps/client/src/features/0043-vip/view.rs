use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    /// Shared presentation visibility; avoids copying the retained server view.
    #[must_use]
    pub fn vip_is_open(&self) -> bool {
        lock(&self.mailbox).vip.open
    }
    /// Closing the sidebar preserves authoritative data and an in-flight request.
    pub fn vip_close(&self) {
        lock(&self.mailbox).vip.open = false;
    }
    #[must_use]
    pub fn vip_panel(&self) -> oteryn_client_ui::vip::VipPanel {
        lock(&self.mailbox).vip.clone()
    }
}
#[cfg(windows)]
#[path = "native.rs"]
mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[cfg(any(windows, test))]
#[derive(Default)]
pub(in crate::play) struct ManagementView {
    pub entries: Vec<Row>,
    pub groups: Vec<Row>,
    pub consents: Vec<Row>,
    pub next_entry: u64,
}
#[cfg(any(windows, test))]
pub(in crate::play) struct Row {
    pub number: u64,
    pub name: String,
    pub label: String,
}
#[cfg(any(windows, test))]
pub(in crate::play) fn management_view(
    view: Option<&str>,
) -> Result<ManagementView, serde_json::Error> {
    let value: serde_json::Value = serde_json::from_str(view.unwrap_or("{}"))?;
    let rows = |key: &str, number: &str| {
        value[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|row| {
                let name = row["name"].as_str()?;
                let label = match key {
                    "entries" => format!(
                        "{name} — {} ({})",
                        row["description"].as_str().unwrap_or(""),
                        if row["confirmed"] == true {
                            "confirmed"
                        } else {
                            "pending"
                        }
                    ),
                    "consents" => format!(
                        "{name} ({})",
                        if row["accepted"] == true {
                            "accepted"
                        } else {
                            "pending"
                        }
                    ),
                    _ => name.to_owned(),
                };
                Some(Row {
                    number: row[number].as_u64()?,
                    name: name.to_owned(),
                    label,
                })
            })
            .collect()
    };
    Ok(ManagementView {
        entries: rows("entries", "entry"),
        groups: rows("groups", "group"),
        consents: rows("consents", "handle"),
        next_entry: value["next_entry"].as_u64().unwrap_or(0),
    })
}

#[cfg(any(windows, test))]
pub(in crate::play) fn management_presence(
    view: Option<&str>,
    presence: &oteryn_client_ui::vip_presence::VipPresencePanel,
) -> Result<ManagementView, serde_json::Error> {
    let mut view = management_view(view)?;
    for row in &mut view.entries {
        let status = presence
            .selected
            .then(|| {
                presence
                    .presence
                    .entries
                    .iter()
                    .find(|entry| entry.entry == row.number && entry.name == row.name)
            })
            .flatten()
            .map(|entry| entry.status);
        let label = match status {
            Some(oteryn_client_runtime::cl25a_protocol::VipPresenceStatus::Online) => "Online",
            Some(oteryn_client_runtime::cl25a_protocol::VipPresenceStatus::Offline) => "Offline",
            _ => "Unconfirmed",
        };
        row.label.push_str(&format!(" — {label}"));
    }
    Ok(view)
}

#[cfg(any(windows, test))]
impl PlayLink {
    pub(in crate::play) fn vip_row_notification(
        &self,
        displayed: &oteryn_client_ui::vip::VipPanel,
        row: usize,
        notification: usize,
    ) -> bool {
        if notification != 2 {
            return false;
        }
        let name = {
            let mailbox = lock(&self.mailbox);
            if !mailbox.vip.selected || mailbox.vip.view != displayed.view {
                return false;
            }
            let Ok(view) = management_view(mailbox.vip.view.as_deref()) else {
                return false;
            };
            let Some(row) = view.entries.get(row) else {
                return false;
            };
            row.name.clone()
        };
        self.open_chat();
        self.chat_open_conversation(&name)
    }
}
