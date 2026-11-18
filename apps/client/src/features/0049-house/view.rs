use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_panel(&self) -> super::state::Panel {
        lock(&self.mailbox).properties.house.clone()
    }
    /// Selects the catalogue row the house-row field names and returns the north-up entrance
    /// map of its floor; with no entrance there it leaves the feedback that says why. A row
    /// that names no catalogue entry shows no map and keeps the previous selection.
    #[cfg(bevy_shell)]
    pub(crate) fn house_map(
        &self,
        revision: u64,
        expected: &oteryn_client_ui::house::Panel,
        drafts: [String; 7],
    ) -> Vec<oteryn_client_ui::house::MapHouse> {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.properties.house;
        if panel.activity_revision != revision || panel.model != *expected {
            return Vec::new();
        }
        // An ACL read restores its selected row and draft together when it finishes.
        if panel.model.pending.is_some() && !panel.model.pending_acl_read {
            panel.model.feedback = tr!("Waiting for the server").into();
            return Vec::new();
        }
        let index = drafts[2]
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .filter(|index| *index < panel.model.page.rows.len());
        if panel.model.pending.is_none() {
            for (field, draft) in panel.fields.iter_mut().zip(drafts) {
                if oteryn_client_ui::modal_dialogs::TextEditor::accepts(&draft, 8192) {
                    *field = draft;
                }
            }
        }
        let Some(index) = index else {
            panel.model.feedback = tr!("Select a house row (starting at 1)").into();
            return Vec::new();
        };
        panel.model.index = index;
        let houses = oteryn_client_ui::house::map_houses(&panel.model);
        if houses.is_empty() {
            panel.model.feedback = "Query houses and select a row to view its floor".into();
        }
        houses
    }
}

#[cfg(all(test, bevy_shell))]
impl PlayLink {
    pub(crate) fn bevy_house_ownership_result(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
    ) {
        lock(&self.mailbox).properties.house.apply(command, result);
    }
    pub(crate) fn bevy_house_list_fixture(&self) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.properties.house;
        panel.initialize(true);
        panel.model.world = [1, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, 1];
        panel.model.town = "oteryn:content.area.city.thais".into();
        panel
            .model
            .page
            .rows
            .push(oteryn_client_runtime::house::Row {
                house: "oteryn:house.test".into(),
                acl_revision: 7,
                owned: true,
                ..Default::default()
            });
        panel.fields[0] = "01010101-0101-7101-8101-010101010101".into();
        panel.fields[1] = panel.model.town.clone();
        panel.fields[2] = "1".into();
    }
    /// The list fixture with three entrances: two on floor 7, one a floor below.
    pub(crate) fn bevy_house_map_fixture(&self) {
        self.bevy_house_list_fixture();
        let mut mailbox = lock(&self.mailbox);
        let rows = &mut mailbox.properties.house.model.page.rows;
        for row in rows.iter_mut() {
            row.name = "Home".into();
            row.entrance = Some((100, 110, 7));
        }
        for (name, entrance) in [
            ("Harbour Home", (120, 130, 7)),
            ("Cellar Flat", (100, 100, 6)),
        ] {
            rows.push(oteryn_client_runtime::house::Row {
                name: name.into(),
                house: format!(
                    "oteryn:house.{}",
                    name.to_ascii_lowercase().replace(' ', "_")
                ),
                entrance: Some(entrance),
                ..Default::default()
            });
        }
    }
}
