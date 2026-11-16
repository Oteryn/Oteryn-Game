use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn house_panel(&self) -> super::state::Panel {
        lock(&self.mailbox).properties.house.clone()
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
        panel.model.page.rows.push(oteryn_client_runtime::house::Row {
            house: "oteryn:house.test".into(),
            acl_revision: 7,
            owned: true,
            ..Default::default()
        });
        panel.fields[0] = "01010101-0101-7101-8101-010101010101".into();
        panel.fields[1] = panel.model.town.clone();
        panel.fields[2] = "1".into();
    }
}
