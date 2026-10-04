//! The thin live loop body: one input event -> at most one session command -> model update.

use super::input::LiveInput;
use super::model::{LiveCommand, RenderModel, Viewport};
use oteryn_dev_client::{CastOutcome, DevClientError, DevClientSession};
use oteryn_input_actions::NormalizedInputEvent;
use oteryn_protocol_oteryn::actor_spell::ActorVitals;
use std::time::Duration;

/// Drives one admitted [`DevClientSession`] and keeps the [`RenderModel`] in step with it.
#[derive(Debug)]
pub struct LiveController {
    session: DevClientSession,
    model: RenderModel,
    view: Viewport,
    input: LiveInput,
    last_cast: Option<CastOutcome>,
}

impl LiveController {
    #[must_use]
    pub fn new(session: DevClientSession, view: Viewport, input: LiveInput) -> Self {
        let mut model = RenderModel::from_snapshot(session.join_snapshot());
        model.vitals = session.actor_vitals().copied();
        if let Some(store) = session.world_entities() {
            model = model.with_entities(store);
        }
        if let Some(log) = session.chat_log() {
            model.chat = super::model::ChatPane::from_log(log);
        }
        Self {
            session,
            model,
            view,
            input,
            last_cast: None,
        }
    }

    #[must_use]
    pub const fn model(&self) -> &RenderModel {
        &self.model
    }

    #[must_use]
    pub const fn view(&self) -> Viewport {
        self.view
    }

    /// The most recent cast's server result and applied own-actor vitals delta.
    #[must_use]
    pub const fn last_cast(&self) -> Option<&CastOutcome> {
        self.last_cast.as_ref()
    }

    /// The session's last real own-actor vitals. A denied cast does not replace them.
    #[must_use]
    pub fn actor_vitals(&self) -> Option<&ActorVitals> {
        self.session.actor_vitals()
    }

    /// Bounded terminal/scenario status from server outcomes, without simulated effects.
    #[must_use]
    pub fn status_text(&self) -> String {
        let mut status = self.model.notice.as_str().to_owned();
        if let Some(outcome) = self.last_cast() {
            status.push_str(&format!(
                " | last cast {:?} command {} server sequence {}",
                outcome.disposition, outcome.command_id, outcome.result_server_sequence
            ));
        }
        match self.actor_vitals() {
            Some(vitals) => status.push_str(&format!(
                " | health {}/{} mana {}/{} soul {} harmony {} serene {}",
                vitals.health,
                vitals.max_health,
                vitals.mana,
                vitals.max_mana,
                vitals.soul,
                vitals.harmony,
                vitals.serene
            )),
            None => status.push_str(" | vitals unavailable"),
        }
        status
    }

    /// Maps `event` and, if it asks for a command, runs it. Returns whether a command ran (so the
    /// caller knows to redraw).
    ///
    /// # Errors
    ///
    /// Any session error; the session is unusable afterwards.
    pub async fn handle_event(
        &mut self,
        event: &NormalizedInputEvent,
    ) -> Result<bool, DevClientError> {
        match self.input.map_event(event, self.view, &self.model) {
            Some(command) => {
                self.dispatch(command).await?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Runs one command and applies its outcome (result and deltas) to the model.
    ///
    /// # Errors
    ///
    /// Any session error; the session is unusable afterwards.
    pub async fn dispatch(&mut self, command: LiveCommand) -> Result<(), DevClientError> {
        self.model = match command {
            LiveCommand::Step(direction) => {
                let outcome = self.session.step(direction).await?;
                self.model.apply_step(&outcome)
            }
            LiveCommand::UseDoor { expected_revision } => {
                let outcome = self
                    .session
                    .use_object(super::model::DOOR_PLACEMENT, expected_revision)
                    .await?;
                self.model.apply_use(&outcome)
            }
            LiveCommand::Cast {
                spell,
                target,
                aim_at_target,
            } => {
                let outcome = self
                    .session
                    .cast_spell(spell, target, aim_at_target)
                    .await?;
                let model = self.model.apply_cast(&outcome);
                self.last_cast = Some(outcome);
                model
            }
            LiveCommand::Select(tile) => self.model.select_at(tile),
            LiveCommand::Chat(intent) => {
                let outcome = self.session.chat(&intent).await?;
                self.model.apply_chat(&outcome)
            }
        };
        self.drain_events();
        Ok(())
    }

    /// Applies the pushed deltas the session queued since the last call. Returns whether any
    /// arrived (so the caller knows to redraw).
    fn drain_events(&mut self) -> bool {
        let events = self.session.take_events();
        if events.is_empty() {
            return false;
        }
        self.model = self.model.apply_events(&events);
        true
    }

    /// Keeps the connection alive while nothing is happening, applying pushed deltas. Returns
    /// whether any changed the model (so the caller knows to redraw).
    ///
    /// # Errors
    ///
    /// Any session error; the session is unusable afterwards.
    pub async fn idle(&mut self, duration: Duration) -> Result<bool, DevClientError> {
        self.session.service_liveness(duration).await?;
        Ok(self.drain_events())
    }
}
