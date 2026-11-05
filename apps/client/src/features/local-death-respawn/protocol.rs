//! The existing own-vitals lifecycle: zero health dies; restored health resumes play.
use crate::combat_input::PlayCommand;
use crate::play::{
    Outbox, PlayLink, PlayView,
    mailbox::{Mailbox, lock},
};
use oteryn_renderer::BatchError;
use oteryn_session::ActorVitals;
use tokio::sync::mpsc::{UnboundedReceiver, error::TryRecvError};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DeathState {
    pub(crate) dead: bool,
    deaths: u64,
}

impl Mailbox {
    pub(in crate::play) fn world_input_blocked(&self) -> bool {
        self.death.dead || self.death_reset_commands
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
            self.death.dead = dead;
        }
        self.vitals = Some(vitals);
    }
}

impl PlayView {
    pub(in crate::play) fn project_death(&mut self, link: &PlayLink) -> Result<(), BatchError> {
        let (state, blocked) = {
            let mailbox = lock(&link.mailbox);
            (mailbox.death, mailbox.world_input_blocked())
        };
        if state.deaths != self.death.deaths {
            self.inputs.clear();
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
    /// Discard pending world actions on death, even if respawn arrived before the next poll.
    pub(in crate::play) fn death_command(
        &self,
        commands: &mut UnboundedReceiver<PlayCommand>,
    ) -> Result<PlayCommand, TryRecvError> {
        let mut mailbox = lock(&self.0);
        let blocked = mailbox.world_input_blocked();
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
            if !blocked || !world_input {
                return Ok(command);
            }
        }
    }
}
