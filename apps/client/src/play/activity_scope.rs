//! A committed scope replaces source projections and discards unsent source actions.
use super::{
    Outbox, PlayLink, PlayView, Stepper,
    mailbox::{self, lock},
};
use crate::combat_input::PlayCommand;
use oteryn_session::activity_scope::{ActivityScope, ActivityScopeSnapshot, decode_activity_scope};
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::mpsc::{UnboundedReceiver, error::TryRecvError};

type Receiver = Arc<Mutex<UnboundedReceiver<PlayCommand>>>;
fn discard_commands(mailbox: &mailbox::Mailbox) {
    if let Some(receiver) = mailbox.command_receiver.upgrade() {
        let mut receiver = receiver.lock().unwrap_or_else(PoisonError::into_inner);
        while receiver.try_recv().is_ok() {}
    }
}
impl Outbox {
    pub(super) fn scope_receiver(&self, receiver: &Receiver) {
        lock(&self.0).command_receiver = Arc::downgrade(receiver);
    }
    pub(super) fn scope_command(&self, receiver: &Receiver) -> Result<PlayCommand, TryRecvError> {
        // Both paths take mailbox then receiver; no lock lives across an await.
        let mut mailbox = lock(&self.0);
        let mut receiver = receiver.lock().unwrap_or_else(PoisonError::into_inner);
        if mailbox.scope_pending {
            loop {
                match receiver.try_recv() {
                    Ok(_) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        super::feature_local_death_respawn::protocol::receive_command(&mut mailbox, &mut receiver)
    }
    pub(super) fn scope_needs_initialize(&self) -> bool {
        let mut mailbox = lock(&self.0);
        std::mem::take(&mut mailbox.scope_initialize)
    }
    pub(super) fn scope_waiting(&self) -> bool {
        let mailbox = lock(&self.0);
        mailbox.scope_pending || mailbox.scope_initialize
    }

    pub(super) fn take_scope_modal_reset(&self) -> bool {
        std::mem::take(&mut lock(&self.0).scope_reset_modal)
    }

    pub(super) fn reset_unbound_scope(&self) {
        let mut mailbox = lock(&self.0);
        retire_source_requests(&mut mailbox);
        mailbox.reset_server_message_namespace();
        mailbox.activity_reset_commands = true;
    }

    pub(super) fn scope_publish<T: Stepper>(&self, session: &mut T) -> bool {
        let scope = session
            .feature_snapshot(99)
            .and_then(|bytes| decode_activity_scope(&bytes).ok());
        let binding = session
            .game_session_id()
            .or_else(|| scope.as_ref().map(|scope| scope.session));
        let scope = scope.filter(|scope| Some(scope.session) == binding);
        let reset = {
            let mut mailbox = lock(&self.0);
            if binding.is_some()
                && binding != mailbox.scope_session
                && (mailbox.scope_session.is_some() || scope.is_some())
            {
                let reset_modal = mailbox.scope_session.is_some();
                reset_scope(&mut mailbox, scope, session);
                mailbox.scope_session = binding;
                mailbox.scope_reset_modal = reset_modal;
                true
            } else {
                mailbox.scope_session = binding.or(mailbox.scope_session);
                false
            }
        };
        if reset || lock(&self.0).scope_pending {
            self.push(session.drain());
            if reset {
                let mut mailbox = lock(&self.0);
                mailbox.pushed.scope_session = binding;
                mailbox.pushed.scope_changed = true;
                mailbox.pushed.activity_scope = mailbox.activity_scope.clone();
                mailbox.world_context.pending = true;
            }
            return true;
        }
        false
    }
}
fn retire_source_requests(mailbox: &mut mailbox::Mailbox) {
    discard_commands(mailbox);
    mailbox.cancel_activity_requests();
    if let Some(launcher) = &mailbox.connection.coin_shop {
        launcher.cancel_active();
    }
}

fn reset_scope<T: Stepper>(
    mailbox: &mut mailbox::Mailbox,
    scope: Option<ActivityScopeSnapshot>,
    session: &T,
) {
    let receiver = mailbox.command_receiver.clone();
    let achievements = mailbox.achievement_notices.clone();
    let shared_frontend = mailbox.shared_frontend;
    let traffic = mailbox.traffic.take();
    let character = std::mem::take(&mut mailbox.party.own_character_name);
    let mut world_field = std::mem::take(&mut mailbox.properties.house.fields[0]);
    let house_transfer = scope.is_none().then(|| {
        (
            std::mem::take(&mut mailbox.properties.house_transfer.summary),
            std::mem::take(&mut mailbox.properties.house_transfer.feedback),
        )
    });
    if let Some(scope) = &scope {
        let world = match scope.scope {
            ActivityScope::Channel { world, channel } => {
                mailbox.connection.channel_current = Some(channel);
                world
            }
            ActivityScope::Instance { world, .. } => world,
        };
        world_field.clear();
        for (index, byte) in world.iter().enumerate() {
            if [4, 6, 8, 10].contains(&index) {
                world_field.push('-');
            }
            use std::fmt::Write;
            let _ = write!(world_field, "{byte:02x}");
        }
    }
    let chat_mode = match mailbox.chat.console.mode {
        oteryn_client_ui::chat::ComposerMode::Temporary => {
            oteryn_client_ui::chat::ComposerMode::Off
        }
        mode => mode,
    };
    #[cfg(bevy_shell)]
    let look_receipt = mailbox.look_receipt;
    retire_source_requests(mailbox);
    mailbox.reset_scope_state();
    mailbox.project_world_channels();
    mailbox.shared_frontend = shared_frontend;
    mailbox.traffic = traffic;
    mailbox.party.own_character_name = character;
    mailbox.properties.house.fields[0] = world_field;
    if let Some((summary, feedback)) = house_transfer {
        mailbox.properties.house_transfer.summary = summary;
        mailbox.properties.house_transfer.feedback = feedback;
    }
    mailbox.notices.tutorial_selected = session.feature_selected(31);
    mailbox.notices.save_selected = session.feature_selected(75);
    mailbox.chat.console.mode = chat_mode;
    #[cfg(bevy_shell)]
    {
        mailbox.look_receipt = look_receipt;
    }
    mailbox.command_receiver = receiver;
    mailbox.achievement_notices = achievements;
    mailbox.activity_scope = scope;
    mailbox.scope_pending = true;
    mailbox.world_context.pending = true;
}

impl PlayLink {
    pub(super) fn scope_presented(&self, session: [u8; 16]) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.scope_pending
            && mailbox
                .scope_session
                .or_else(|| mailbox.activity_scope.as_ref().map(|scope| scope.session))
                == Some(session)
        {
            discard_commands(&mailbox);
            // This presentation already drained the retired namespace. A later
            // legacy drain would discard the first fresh destination command.
            mailbox.activity_reset_commands = false;
            mailbox.scope_pending = false;
            mailbox.world_context.pending = false;
            mailbox.scope_initialize = true;
        }
    }
}
impl PlayView {
    pub(super) fn apply_activity_scope(
        &mut self,
        scope: &ActivityScopeSnapshot,
    ) -> Result<(), oteryn_renderer::BatchError> {
        self.inputs.clear();
        self.in_flight = false;
        self.cursor_spell = None;
        self.creature_drag = Default::default();
        self.entities.clear();
        self.appearances.clear();
        self.ground_items.clear();
        self.markers.clear();
        self.visuals.clear();
        self.sounds.clear();
        self.scene.clear_target()?;
        self.scene.set_drome_hazards(Vec::new())?;
        self.world_weather = None;
        self.world_clock = None;
        self.served_channel = None;
        self.target_walk_feedback = None;
        self.world_context_feedback = None;
        self.combat_line = None;
        self.logout_line = None;
        self.server_messages.clear();
        self.battle_instinct = None;
        self.battle_instinct_refresh = None;
        self.action_line = Some(match scope.scope {
            ActivityScope::Instance { .. } => "Entered boss instance",
            ActivityScope::Channel { .. } => "Returned to channel",
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::StepDir;

    fn id(value: u8) -> [u8; 16] {
        let mut id = [value; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }

    fn pending(link: &PlayLink) -> [u8; 16] {
        let scope = ActivityScopeSnapshot {
            handoff: id(1),
            source_session: id(2),
            session: id(3),
            scope: ActivityScope::Instance {
                world: id(4),
                instance: id(5),
            },
            scope_generation: 1,
            connection_generation: 1,
            lease_generation: 1,
            selection: vec![117],
            requires_ack: false,
        };
        assert!(oteryn_session::activity_scope::encode_activity_scope(&scope).is_ok());
        let session = scope.session;
        let mut mailbox = lock(&link.mailbox);
        mailbox.activity_scope = Some(scope);
        mailbox.scope_pending = true;
        mailbox.activity_reset_commands = true;
        session
    }

    #[test]
    fn m25_presented_scope_preserves_first_destination_step_after_legacy_reset() {
        let (link, commands, outbox) = crate::play::play_channel();
        let receiver = Arc::new(Mutex::new(commands));
        outbox.scope_receiver(&receiver);
        let session = pending(&link);
        assert!(
            link.commands
                .send(PlayCommand::Step(StepDir::North))
                .is_ok()
        );
        link.scope_presented(session);
        link.request(StepDir::South);
        assert!(matches!(
            outbox.scope_command(&receiver),
            Ok(PlayCommand::Step(StepDir::South))
        ));
        assert!(matches!(
            outbox.scope_command(&receiver),
            Err(TryRecvError::Empty)
        ));
    }

    #[test]
    fn m25_wrong_presentation_cannot_release_scope_or_legacy_reset() {
        let (link, mut commands, _) = crate::play::play_channel();
        pending(&link);
        link.scope_presented(id(9));
        link.request(StepDir::South);
        assert!(matches!(commands.try_recv(), Err(TryRecvError::Empty)));
        let mailbox = lock(&link.mailbox);
        assert!(mailbox.scope_pending);
        assert!(mailbox.activity_reset_commands);
    }
}
