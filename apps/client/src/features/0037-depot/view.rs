use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::depot::{DepotPanel, Intent};
impl PlayLink {
    pub fn depot_panel(&self) -> DepotPanel {
        lock(&self.mailbox).depot.clone()
    }
    pub fn open_depot(&self) {
        self.depot_request(Intent::Browse {
            box_number: 1,
            page: 0,
        });
        lock(&self.mailbox).depot.open = true;
    }
}

/// Shared frontend actions are bound to the exact displayed inventory and depot page.
#[cfg(bevy_shell)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DepotUiAction {
    Close,
    Request(Intent),
}
#[cfg(bevy_shell)]
impl PlayLink {
    pub(crate) fn depot_action_bound(&self, expected: &DepotPanel, action: DepotUiAction) {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.depot.open || &mailbox.depot != expected {
            return;
        }
        match action {
            DepotUiAction::Close => mailbox.depot.open = false,
            DepotUiAction::Request(intent) => {
                super::input::enqueue(&mut mailbox, &self.commands, intent)
            }
        }
    }
}

#[cfg(bevy_shell)]
#[derive(serde::Deserialize)]
pub(crate) struct DepotTown {
    pub town_id: u16,
    pub name: String,
}

#[cfg(bevy_shell)]
pub(crate) fn depot_towns() -> &'static [DepotTown] {
    #[derive(serde::Deserialize)]
    struct TownCatalogue {
        towns: Vec<DepotTown>,
    }

    static TOWNS: std::sync::OnceLock<Vec<DepotTown>> = std::sync::OnceLock::new();
    TOWNS.get_or_init(|| {
        let source = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/content-schema/area-authoring/samples/crystal-world-towns-00ce02a5.json"
        ));
        let Ok(TownCatalogue { mut towns }) = serde_json::from_str(source) else {
            return Vec::new();
        };
        towns.sort_by(|left, right| left.name.cmp(&right.name));
        towns
    })
}
