use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn weapon_proficiency_state_initialize(&self, selected: bool) {
        lock(&self.0).weapon_proficiency.state_selected = selected;
    }
    pub(in crate::play) fn weapon_proficiency_initialize(&self, selected: bool) {
        lock(&self.0).weapon_proficiency =
            oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel {
                selected,
                ..Default::default()
            };
    }
    pub(in crate::play) fn weapon_proficiency_result(&self, result: &GatedCommandResult) {
        let mut mailbox = lock(&self.0);
        let panel = &mut mailbox.weapon_proficiency;
        #[cfg(bevy_shell)]
        let before: std::collections::BTreeMap<_, _> = panel
            .catalogue
            .iter()
            .flat_map(|p| &p.entries)
            .map(|entry| {
                (
                    entry.definition,
                    oteryn_client_ui::weapon_proficiency::progress_level(entry),
                )
            })
            .collect();
        oteryn_client_ui::weapon_proficiency::apply_result(panel, result);
        #[cfg(bevy_shell)]
        {
            let notices: Vec<_> = panel
                .catalogue
                .iter()
                .flat_map(|p| &p.entries)
                .filter(|entry| {
                    before.get(&entry.definition).is_some_and(|old| {
                        oteryn_client_ui::weapon_proficiency::progress_level(entry) > *old
                    })
                })
                .map(|entry| format!("Weapon proficiency improved: {}", entry.name))
                .collect();
            mailbox.achievement_banners.extend(notices);
        }
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_proficiency_initialize(&self) {
        self.weapon_proficiency_initialize(true);
    }
    pub(crate) fn bevy_proficiency_result(&self, result: &GatedCommandResult) {
        self.weapon_proficiency_result(result);
    }
    pub(crate) fn bevy_proficiency_state_initialize(&self) {
        self.weapon_proficiency_state_initialize(true);
    }
}
