//! Immutable projection of the canonical VIP response; absent metadata stays absent.
use serde::Deserialize;
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(super) struct Contact {
    pub entry: u64,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<u8>,
    pub notify: Option<bool>,
    pub groups: Option<Vec<u8>>,
}
impl Contact {
    pub fn editable(&self) -> bool {
        self.description.is_some()
            && self.icon.is_some_and(|icon| icon <= 19)
            && self.notify.is_some()
            && self.groups.as_ref().is_some_and(|groups| groups.len() <= 8)
    }
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(super) struct Group {
    pub group: u8,
    pub name: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub entries: Vec<Contact>,
    pub groups: Vec<Group>,
    pub next_entry: Option<u64>,
}
impl Snapshot {
    pub fn read(panel: &oteryn_client_ui::vip::VipPanel) -> Result<Self, &'static str> {
        let raw = panel.view.as_deref().ok_or("Loading VIP list…")?;
        let value: Self =
            serde_json::from_str(raw).map_err(|_| "VIP reply is invalid; refresh.")?;
        let mut ids = std::collections::HashSet::new();
        if value
            .entries
            .iter()
            .any(|row| row.entry == 0 || row.name.is_empty() || !ids.insert(row.entry))
        {
            return Err("VIP reply has invalid contacts; refresh.");
        }
        let mut groups = std::collections::HashSet::new();
        if value
            .groups
            .iter()
            .any(|row| row.name.is_empty() || !groups.insert(row.group))
        {
            return Err("VIP reply has invalid groups; refresh.");
        }
        Ok(value)
    }
}
