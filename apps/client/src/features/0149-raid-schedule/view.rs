use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn raid_schedule_panel(&self) -> oteryn_client_ui::raid_schedule::Panel {
        lock(&self.mailbox).properties.raid_schedule.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Raid schedule",
            labels: &[],
            buttons: &["Refresh"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.raid_schedule;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::raid_schedule::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.raid_schedule.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.raid_schedule;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::raid_schedule::Intent::Query,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(feature = "bevy-shell")]
#[derive(Clone, PartialEq, Eq)]
pub enum RaidScheduleControl { Refresh, Close }
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn raid_schedule_dispatch_bound(&self, expected: &oteryn_client_ui::raid_schedule::Panel, control: RaidScheduleControl) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed() || mailbox.properties.raid_schedule != *expected || !expected.open {
            return false;
        }
        if control == RaidScheduleControl::Close {
            mailbox.properties.raid_schedule.open = false;
            return true;
        }
        if !expected.selected || expected.pending { return false; }
        super::input::submit(&mut mailbox, &self.commands, oteryn_client_ui::raid_schedule::Intent::Query);
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_raid_schedule_reply(&self, command: &oteryn_client_runtime::raw_feature::FeatureCommand, result: &oteryn_client_runtime::raw_feature::GatedCommandResult) {
        self.properties_result(command, result);
    }
    pub(crate) fn bevy_raid_schedule_selected(&self, selected: bool) {
        lock(&self.0).properties.raid_schedule.selected = selected;
    }
}
