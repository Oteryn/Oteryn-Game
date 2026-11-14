use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn guildhall_panel(&self) -> oteryn_client_ui::guildhall::Panel {
        lock(&self.mailbox).properties.guildhall.clone()
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
            title: "Guildhall leadership",
            labels: &[
                "Guild UUID",
                "Successor character UUID",
                "Expected guild revision",
            ],
            buttons: &["Transfer"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.guildhall;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::guildhall::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.guildhall.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.guildhall;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::guildhall::Intent::Transfer,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

/// Atomic shared-frontend submission; the existing protocol owner remains authoritative.
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub(crate) fn guildhall_leadership_transfer_bound(
        &self,
        expected: &oteryn_client_ui::guildhall::Panel,
        fields: [String; 3],
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !confirmed
            || self.commands.is_closed()
            || mailbox.server_modal.is_active()
            || mailbox.world_input_blocked()
            || mailbox.properties.guildhall != *expected
            || !expected.open
            || !expected.selected
            || expected.pending
            || fields
                .iter()
                .any(|value| value.len() > 512 || value.contains('\0'))
        {
            return false;
        }
        let mut prepared = expected.clone();
        prepared.fields = fields;
        let Ok(command) =
            super::protocol::command(&prepared, oteryn_client_ui::guildhall::Intent::Transfer)
        else {
            return false;
        };
        if !oteryn_client_ui::guildhall::request(&mut prepared, command.kind)
            || self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_err()
        {
            return false;
        }
        mailbox.properties.guildhall = prepared;
        true
    }

    pub(crate) fn guildhall_leadership_close_bound(
        &self,
        expected: &oteryn_client_ui::guildhall::Panel,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.properties.guildhall != *expected
            || !expected.open
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        mailbox.properties.guildhall.open = false;
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn guildhall_leadership_selected(&self, selected: bool) {
        lock(&self.0).properties.guildhall.selected = selected;
    }
    pub(crate) fn guildhall_leadership_end(&self) {
        self.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    }
    pub(crate) fn guildhall_leadership_reply(
        &self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        code: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let request =
            oteryn_client_runtime::guildhall::decode_guildhall_leadership(&command.payload)
                .map_err(|error| format!("{error:?}"))?;
        let payload = oteryn_client_runtime::guildhall::encode_guild_intent_result(
            &oteryn_client_runtime::guildhall::GuildIntentResult {
                code,
                guild: request.guild,
                revision: Some(2),
            },
        )
        .map_err(|error| format!("{error:?}"))?;
        super::protocol::apply(
            &mut lock(&self.0).properties.guildhall,
            command,
            &oteryn_client_runtime::raw_feature::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload,
            },
        );
        Ok(())
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod guildhall_leadership_tests {
    #[test]
    fn guildhall_leadership_owner_rejects_unconfirmed_invalid_and_closed_requests() {
        let (link, mut commands, events) = crate::play::play_channel();
        events.guildhall_leadership_selected(true);
        link.guildhall_open();
        let expected = link.guildhall_panel();
        let valid = [
            "00000000-0000-7000-8000-000000000001".into(),
            "00000000-0000-7000-8000-000000000002".into(),
            "1".into(),
        ];
        assert!(!link.guildhall_leadership_transfer_bound(&expected, valid.clone(), false));
        assert!(!link.guildhall_leadership_transfer_bound(
            &expected,
            [String::new(), String::new(), String::new()],
            true
        ));
        assert_eq!(link.guildhall_panel(), expected);
        assert!(commands.try_recv().is_err());
        drop(commands);
        assert!(!link.guildhall_leadership_transfer_bound(&expected, valid, true));
        assert_eq!(link.guildhall_panel(), expected);
    }
}
