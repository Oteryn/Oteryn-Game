use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn cast_chat_panel(&self) -> oteryn_client_ui::cast_chat::Panel {
        lock(&self.mailbox).cast_chat.clone()
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
            title: "Live cast chat",
            labels: &[
                "Reply to cast room",
                "Viewer handle (32 hex digits, from the live lines)",
            ],
            buttons: &["Reply", "Mute", "Unmute", "Kick", "Ban"],
            snapshot: |m| {
                (
                    m.cast_chat.open,
                    m.cast_chat.pending,
                    vec![String::new(), String::new()],
                    oteryn_client_ui::cast_chat::panel_text(&m.cast_chat),
                )
            },
            set_open: |m, value| m.cast_chat.open = value,
            submit: |m, commands, action, values| {
                if let Ok(action) = u8::try_from(action) {
                    super::input::submit(
                        m,
                        commands,
                        values.first().map_or("", String::as_str),
                        values.get(1).map_or("", String::as_str),
                        action,
                    );
                }
            },
        },
    );
}
/// A presentation snapshot fenced to the actual retained session mailbox.
#[cfg(feature = "bevy-shell")]
#[derive(Clone)]
pub struct CastChatBound {
    session: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    pub panel: oteryn_client_ui::cast_chat::Panel,
    caster: (bool, bool, bool, u64, u64),
}
#[cfg(feature = "bevy-shell")]
impl CastChatBound {
    fn matches(&self, mailbox: &crate::play::mailbox::Mailbox) -> bool {
        let panel = &mailbox.cast_chat;
        self.caster == caster_key(mailbox)
            && self.panel.selected == panel.selected
            && self.panel.open == panel.open
            && self.panel.pending == panel.pending
            && self.panel.roster_revision == panel.roster_revision
            && self.panel.subjects == panel.subjects
            && oteryn_client_ui::cast_chat::panel_text(&self.panel)
                == oteryn_client_ui::cast_chat::panel_text(panel)
    }
    pub fn same_view(&self, other: &Self) -> bool {
        self.session.ptr_eq(&other.session)
            && self.caster == other.caster
            && self.panel.selected == other.panel.selected
            && self.panel.open == other.panel.open
            && self.panel.pending == other.panel.pending
            && self.panel.roster_revision == other.panel.roster_revision
            && self.panel.subjects == other.panel.subjects
            && oteryn_client_ui::cast_chat::panel_text(&self.panel)
                == oteryn_client_ui::cast_chat::panel_text(&other.panel)
    }
}
#[cfg(feature = "bevy-shell")]
fn caster_key(mailbox: &crate::play::mailbox::Mailbox) -> (bool, bool, bool, u64, u64) {
    let control = &mailbox.cast_control;
    (
        control.selected,
        control.active,
        control.pending,
        control.revision,
        control.epoch,
    )
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn cast_chat_bound(&self) -> CastChatBound {
        let mailbox = lock(&self.mailbox);
        CastChatBound {
            session: std::sync::Arc::downgrade(&self.mailbox),
            panel: mailbox.cast_chat.clone(),
            caster: caster_key(&mailbox),
        }
    }
    pub fn cast_chat_set_open(&self, open: bool) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        mailbox.cast_chat.open = open;
        true
    }
    pub fn cast_chat_close_bound(&self, expected: &CastChatBound) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || !expected.matches(&mailbox)
            || mailbox.server_modal.is_active()
        {
            return false;
        }
        mailbox.cast_chat.open = false;
        true
    }
    /// Rechecks the session, cast, roster and pending/modal fences under the owner lock.
    /// The legacy submitter remains the only command builder and dispatcher.
    pub fn cast_chat_submit_bound(
        &self,
        expected: &CastChatBound,
        text: &str,
        subject: &str,
        action: u8,
        confirmed: bool,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !expected
            .session
            .ptr_eq(&std::sync::Arc::downgrade(&self.mailbox))
            || !expected.matches(&mailbox)
            || self.commands.is_closed()
            || mailbox.world_input_blocked()
            || mailbox.server_modal.is_active()
            || !mailbox.cast_chat.open
            || !mailbox.cast_chat.selected
            || !mailbox.cast_control.active
            || mailbox.cast_chat.pending
            || mailbox.cast_control.pending
            || action > 4
            || (action != 0 && !confirmed)
        {
            return false;
        }
        super::input::submit(&mut mailbox, &self.commands, text, subject, action);
        mailbox.cast_chat.pending
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn cast_chat_test_end(&self) {
        self.logged_out();
    }
    pub(crate) fn cast_chat_test_start(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.cast_control_initialize(true);
        self.cast_chat_initialize(true);
        self.cast_control_result(&cast_chat_test_response(9)?);
        self.cast_chat_test_roster(vec![[4; 16]]);
        self.push(vec![oteryn_session::SessionEvent::CastCasterLine(
            oteryn_session::AppliedDelta {
                server_sequence: 2,
                base_revision: 1,
                new_revision: 2,
                value: oteryn_client_runtime::CastCasterLine {
                    handle: [4; 16],
                    text: "Viewer-1: server line".into(),
                },
            },
        )]);
        Ok(())
    }
    pub(crate) fn cast_chat_test_roster(&self, subjects: Vec<[u8; 16]>) {
        self.push(vec![oteryn_session::SessionEvent::CastRoster(
            oteryn_session::AppliedDelta {
                server_sequence: 2,
                base_revision: 1,
                new_revision: 2,
                value: oteryn_client_runtime::CastRoster {
                    revision: 9,
                    subjects,
                },
            },
        )]);
    }
    pub(crate) fn cast_chat_test_result(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.cast_chat_result(&cast_chat_test_response(10)?);
        Ok(())
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
fn cast_chat_test_response(
    revision: u64,
) -> Result<
    oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    Box<dyn std::error::Error>,
> {
    Ok(
        oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: oteryn_client_runtime::cast::Message::CasterState {
                cast_id: [1; 16],
                revision,
                epoch: 1,
                delay: 300,
                count: 1,
            }
            .encode()
            .map_err(|e| format!("{e:?}"))?,
        },
    )
}
