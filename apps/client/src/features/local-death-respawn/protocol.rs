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
        crate::play::feature_0086_house_transfer::input::automatic_exit_after_death(
            link,
            self.in_flight,
        );
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
            let movement_control = matches!(&command, PlayCommand::ServerMechanic(request) if matches!(request.capability, 64 | 82));
            if !blocked || !(world_input || movement_control) {
                return Ok(command);
            }
        }
    }
}

/// A server-committed House witness. Editable transfer-panel text is never used.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HouseDeathBinding {
    session: [u8; 16],
    connection_generation: u64,
    scope_generation: u64,
    lease_generation: u64,
    house: String,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct HouseDeathExitState {
    binding: Option<HouseDeathBinding>,
    vitals_binding: Option<HouseDeathBinding>,
    requested: Option<(HouseDeathBinding, u64)>,
    retry_at: Option<std::time::Instant>,
}
impl HouseDeathExitState {
    fn observe(&mut self, selected: bool, payload: Option<Vec<u8>>) -> bool {
        let binding = selected
            .then_some(payload)
            .flatten()
            .and_then(|bytes| {
                oteryn_client_runtime::house_transfer::decode_house_transfer_ack(&bytes).ok()
            })
            .and_then(|ack| {
                Some(HouseDeathBinding {
                    house: ack.house?,
                    session: ack.session,
                    connection_generation: ack.connection_generation,
                    scope_generation: ack.scope_generation,
                    lease_generation: ack.lease_generation,
                })
            });
        let cancelled = self.binding != binding && self.requested.is_some();
        if self.binding != binding {
            self.vitals_binding = None;
            self.requested = None;
            self.retry_at = None;
        }
        self.binding = binding;
        cancelled
    }
}

pub(in crate::play) fn prepare_house_death_exit(mailbox: &mut Mailbox) -> Option<PlayCommand> {
    if !mailbox.death.dead {
        mailbox.house_death_exit.retry_at = None;
        if mailbox.house_death_exit.requested.take().is_some()
            && mailbox.properties.house_transfer.pending_kind == 102
        {
            mailbox.properties.house_transfer.pending = false;
            mailbox.properties.house_transfer.feedback.clear();
        }
        return None;
    }
    if mailbox.world_context.pending
        || mailbox.properties.house_transfer.pending
        || !mailbox.properties.house_transfer.selected
        || !mailbox
            .vitals
            .is_some_and(|vitals| vitals.max_health > 0 && vitals.health == 0)
    {
        return None;
    }
    if mailbox
        .house_death_exit
        .retry_at
        .is_some_and(|deadline| std::time::Instant::now() < deadline)
    {
        return None;
    }
    let binding = mailbox.house_death_exit.binding.clone()?;
    if mailbox.house_death_exit.vitals_binding.as_ref() != Some(&binding) {
        return None;
    }
    let generation = mailbox.death.deaths;
    if mailbox.house_death_exit.requested.as_ref() == Some(&(binding.clone(), generation)) {
        return None;
    }
    let payload = oteryn_client_runtime::house_transfer::encode_house_transfer_intent(
        &oteryn_client_runtime::house_transfer::HouseTransferIntent {
            house: binding.house.clone(),
            exit: true,
        },
    )
    .ok()?;
    let command = oteryn_client_runtime::raw_feature::FeatureCommand {
        capability: 86,
        kind: 102,
        payload,
    };
    if !oteryn_client_ui::house_transfer::request(
        &mut mailbox.properties.house_transfer,
        command.kind,
    ) {
        return None;
    }
    mailbox.house_death_exit.retry_at = None;
    mailbox.house_death_exit.requested = Some((binding.clone(), generation));
    Some(PlayCommand::HouseDeathExit {
        command,
        house_session: binding.session,
        connection_generation: binding.connection_generation,
        scope_generation: binding.scope_generation,
        lease_generation: binding.lease_generation,
        house: binding.house,
        death_generation: generation,
    })
}

impl Outbox {
    pub(in crate::play) fn house_death_vitals_current(&self, current: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.house_death_exit.vitals_binding = current
            .then(|| mailbox.house_death_exit.binding.clone())
            .flatten();
    }
    pub(in crate::play) fn house_death_exit_rejected(
        &self,
        session: [u8; 16],
        connection_generation: u64,
        scope_generation: u64,
        lease_generation: u64,
        house: &str,
        death_generation: u64,
    ) {
        let mut mailbox = lock(&self.0);
        let binding = HouseDeathBinding {
            session,
            connection_generation,
            scope_generation,
            lease_generation,
            house: house.to_owned(),
        };
        if mailbox.house_death_exit.requested.as_ref() == Some(&(binding.clone(), death_generation))
            && mailbox.house_death_exit.binding.as_ref() == Some(&binding)
            && mailbox.house_death_exit.vitals_binding.as_ref() == Some(&binding)
            && mailbox.death.dead
            && mailbox.death.deaths == death_generation
        {
            mailbox.house_death_exit.requested = None;
            mailbox.house_death_exit.retry_at =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
        }
    }
    pub(in crate::play) fn house_death_scope(&self, selected: bool, payload: Option<Vec<u8>>) {
        let mut mailbox = lock(&self.0);
        if mailbox.house_death_exit.observe(selected, payload)
            && mailbox.properties.house_transfer.pending
            && mailbox.properties.house_transfer.pending_kind == 102
        {
            mailbox.properties.house_transfer.pending = false;
            mailbox.properties.house_transfer.feedback.clear();
        }
    }
    /// A revived actor or a newer House session cancels an obsolete queued exit.
    pub(in crate::play) fn house_death_exit_current(
        &self,
        house_session: [u8; 16],
        connection_generation: u64,
        scope_generation: u64,
        lease_generation: u64,
        house: &str,
        death_generation: u64,
    ) -> bool {
        let mut mailbox = lock(&self.0);
        let binding = HouseDeathBinding {
            session: house_session,
            connection_generation,
            scope_generation,
            lease_generation,
            house: house.to_owned(),
        };
        let matches_request = mailbox.house_death_exit.requested.as_ref()
            == Some(&(binding.clone(), death_generation));
        let current = matches_request
            && mailbox.house_death_exit.binding.as_ref() == Some(&binding)
            && mailbox.house_death_exit.vitals_binding.as_ref() == Some(&binding)
            && mailbox.death.dead
            && mailbox.death.deaths == death_generation
            && mailbox
                .vitals
                .is_some_and(|vitals| vitals.max_health > 0 && vitals.health == 0)
            && mailbox.properties.house_transfer.selected
            && mailbox.properties.house_transfer.pending;
        if !current && matches_request {
            mailbox.house_death_exit.requested = None;
            mailbox.properties.house_transfer.pending = false;
            mailbox.properties.house_transfer.feedback.clear();
        }
        current
    }
}
