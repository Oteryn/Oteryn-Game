//! The existing own-vitals lifecycle: zero health dies; restored health resumes play.
use crate::combat_input::PlayCommand;
use crate::play::{
    Outbox, PlayLink, PlayView,
    mailbox::{Mailbox, lock},
};
use oteryn_renderer::BatchError;
use oteryn_session::ActorVitals;
use tokio::sync::mpsc::{UnboundedReceiver, error::TryRecvError};

#[derive(Debug, Clone, Default)]
pub(crate) struct DeathState {
    pub(crate) dead: bool,
    deaths: u64,
    selected: bool,
    pending: bool,
    refresh: bool,
    pub(in crate::play) protection: Option<oteryn_client_ui::death_protection::DeathProtection>,
}

impl Mailbox {
    /// Refresh on actual server vitals events, including a coalesced death/respawn.
    /// The session loop also republishes cached vitals after commands: those must
    /// not schedule another query, otherwise query responses would feed themselves.
    pub(in crate::play) fn death_protection_events(
        &mut self,
        events: &[oteryn_session::SessionEvent],
    ) {
        if self.death.selected && events.iter().any(|event| matches!(event, oteryn_session::SessionEvent::ActorVitals(delta) if delta.value.max_health > 0 && delta.value.health > 0)) {
            self.death.refresh = true;
        }
    }
    pub(in crate::play) fn world_input_blocked(&self) -> bool {
        self.death.dead || self.death_reset_commands || self.scope_pending
    }

    pub(in crate::play) fn apply_death_vitals(&mut self, vitals: ActorVitals) {
        // An absent/default vitals snapshot is not a reported death.
        if vitals.max_health > 0 {
            let dead = vitals.health == 0;
            if dead && !self.death.dead {
                self.death.deaths = self.death.deaths.saturating_add(1);
                self.death_reset_commands = true;
                self.daily_reward.pending = false;
                self.target_walk.pending = false;
                self.target_walk.response = None;
                self.target_walk.feedback = None;
            }
            if self.death.dead && !dead {
                self.death.refresh = true;
            }
            self.death.dead = dead;
        }
        self.vitals = Some(vitals);
    }
}

impl PlayView {
    pub(in crate::play) fn project_death(&mut self, link: &PlayLink) -> Result<(), BatchError> {
        let (state, blocked) = {
            let mailbox = lock(&link.mailbox);
            (mailbox.death.clone(), mailbox.world_input_blocked())
        };
        if state.deaths != self.death.deaths {
            self.inputs.clear();
            self.cursor_spell = None;
            self.in_flight = false;
            self.scene.clear_target()?;
            self.target_walk_feedback = None;
            self.combat_line = None;
        }
        if blocked {
            self.in_flight = false;
        }
        self.death = state;
        self.death.dead = blocked;
        Ok(())
    }
}

impl Outbox {
    pub(in crate::play) fn death_protection_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.death.protection = None;
        mailbox.death.selected = selected;
        mailbox.death.refresh = selected;
        mailbox.death.pending = false;
    }
    pub(in crate::play) fn death_protection_result(
        &self,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let mut mailbox = lock(&self.0);
        mailbox.death.pending = false;
        if matches!(
            result.status,
            oteryn_session::CommandStatus::Accepted
                | oteryn_session::CommandStatus::DuplicateReplay
        ) && let Some(reply) = oteryn_client_ui::death_protection::decode(&result.payload)
        {
            mailbox.death.protection = reply.0;
        }
    }
    /// Discard pending world actions on death, even if respawn arrived before the next poll.
    #[cfg(test)]
    pub(in crate::play) fn death_command(
        &self,
        commands: &mut UnboundedReceiver<PlayCommand>,
    ) -> Result<PlayCommand, TryRecvError> {
        let mut mailbox = lock(&self.0);
        receive_command(&mut mailbox, commands)
    }
}

pub(in crate::play) fn receive_command(
    mailbox: &mut Mailbox,
    commands: &mut UnboundedReceiver<PlayCommand>,
) -> Result<PlayCommand, TryRecvError> {
    if mailbox.activity_reset_commands {
        while commands.try_recv().is_ok() {}
        mailbox.cancel_activity_requests();
        mailbox.death.pending = false;
        mailbox.death.refresh = mailbox.death.selected;
        mailbox.activity_reset_commands = false;
    }
    let blocked = mailbox.world_input_blocked();
    if mailbox.death.selected
        && mailbox.death.refresh
        && !mailbox.death.dead
        && !mailbox.death.pending
    {
        mailbox.death.refresh = false;
        mailbox.death.pending = true;
        return Ok(PlayCommand::Feature(
            oteryn_client_runtime::raw_feature::FeatureCommand {
                capability: 382,
                kind: 664,
                payload: Vec::new(),
            },
        ));
    }
    loop {
        let command = match commands.try_recv() {
            Ok(command) => command,
            Err(error) => {
                mailbox.death_reset_commands = false;
                return Err(error);
            }
        };
        let world_input = matches!(
            command,
            PlayCommand::Step(_)
                | PlayCommand::TargetWalk(_)
                | PlayCommand::Combat(_)
                | PlayCommand::DailyReward(_)
        );
        let movement_control = matches!(&command, PlayCommand::ServerMechanic(request) if matches!(request.capability, 64 | 82));
        if !blocked || !(world_input || movement_control) {
            return Ok(command);
        }
    }
}
