//! Atomic presentation entry points over the retained signed staff owner.
use super::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct StaffToolsSnapshot {
    pub panel: super::super::state::Panel,
    session: crate::play::mailbox::SharedMailbox,
}
impl StaffToolsSnapshot {
    pub fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.session, &other.session) && same(&self.panel, &other.panel)
    }
}
fn same(a: &super::super::state::Panel, b: &super::super::state::Panel) -> bool {
    a.selected == b.selected
        && a.tools == b.tools
        && a.open == b.open
        && a.pending == b.pending
        && a.pending_kind == b.pending_kind
        && a.fields == b.fields
        && a.summary == b.summary
        && a.feedback == b.feedback
}
impl PlayLink {
    pub fn staff_tools_snapshot(&self) -> StaffToolsSnapshot {
        StaffToolsSnapshot {
            panel: lock(&self.mailbox).properties.staff.clone(),
            session: Arc::clone(&self.mailbox),
        }
    }
    pub fn staff_tools_open_bound(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.shared_frontend
            || self.commands.is_closed()
            || !mailbox.properties.staff.selected
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        mailbox.properties.staff.open = true;
        true
    }
    pub fn staff_tools_close_bound(&self, expected: &StaffToolsSnapshot) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !Arc::ptr_eq(&self.mailbox, &expected.session)
            || self.commands.is_closed()
            || mailbox.custom_hotkeys_ended
        {
            return false;
        }
        mailbox.properties.staff.open = false;
        true
    }
    pub fn staff_tools_edit_bound(
        &self,
        expected: &StaffToolsSnapshot,
        fields: [String; 3],
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.staff_tools_current(&mailbox, expected)
            || mailbox.properties.staff.pending
            || fields
                .iter()
                .any(|value| value.len() > 8192 || value.contains('\0'))
        {
            return false;
        }
        mailbox.properties.staff.fields = fields;
        true
    }
    pub fn staff_tools_submit_bound(
        &self,
        expected: &StaffToolsSnapshot,
        receipt: bool,
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !confirmed
            || !self.staff_tools_current(&mailbox, expected)
            || mailbox.properties.staff.pending
        {
            return false;
        }
        super::super::input::submit(&mut mailbox, &self.commands, receipt);
        mailbox.properties.staff.pending
    }
    fn staff_tools_current(
        &self,
        mailbox: &crate::play::mailbox::Mailbox,
        expected: &StaffToolsSnapshot,
    ) -> bool {
        mailbox.shared_frontend
            && !self.commands.is_closed()
            && Arc::ptr_eq(&self.mailbox, &expected.session)
            && mailbox.properties.staff.selected
            && mailbox.properties.staff.open
            && !mailbox.server_modal.is_active()
            && same(&mailbox.properties.staff, &expected.panel)
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_staff_panel(&self, panel: super::super::state::Panel) {
        lock(&self.0).properties.staff = panel;
    }
    pub(crate) fn bevy_staff_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
    pub(crate) fn bevy_staff_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_session::property_services::staff as wire;
        let result = wire::StaffResult {
            code: wire::StaffCode::Ok,
            disposition: if matches!(command.kind, 61 | 561) {
                3
            } else {
                1
            },
            control_seq: 42,
            link_sha256: vec![7; 32],
            mute_until_ms: 0,
        };
        let payload = if command.capability == 330 {
            oteryn_client_runtime::raw_feature::staff_tools::encode_reply(
                &oteryn_client_runtime::raw_feature::staff_tools::StaffToolsReply {
                    result,
                    info: String::new(),
                },
            )?
        } else {
            wire::encode_result(&result)?
        };
        self.properties_result(
            command,
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
                payload,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn staff_tools_owner_rejects_closed_transport_and_invalid_fields_atomically() {
        let (link, receiver, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.bevy_staff_panel(super::super::super::state::Panel {
            selected: true,
            open: true,
            fields: ["operator".into(), "signed target".into(), "01".into()],
            ..Default::default()
        });
        let snapshot = link.staff_tools_snapshot();
        assert!(!link.staff_tools_edit_bound(
            &snapshot,
            ["operator".into(), "changed\0target".into(), "01".into()]
        ));
        assert!(snapshot.matches(&link.staff_tools_snapshot()));
        drop(receiver);
        assert!(!link.staff_tools_submit_bound(&snapshot, false, true));
        assert!(!link.staff_tools_close_bound(&snapshot));
        assert!(snapshot.matches(&link.staff_tools_snapshot()));
    }
}
