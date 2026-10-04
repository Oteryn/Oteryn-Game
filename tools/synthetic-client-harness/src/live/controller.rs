//! The thin live loop body: one input event -> at most one session command -> model update.

use super::input::LiveInput;
use super::model::{LiveCommand, Notice, RenderModel, Viewport};
use oteryn_dev_client::{DevClientError, DevClientSession};
use oteryn_input_actions::NormalizedInputEvent;
use std::time::Duration;

/// Drives one admitted [`DevClientSession`] and keeps the [`RenderModel`] in step with it.
#[derive(Debug)]
pub struct LiveController {
    session: DevClientSession,
    model: RenderModel,
    view: Viewport,
    input: LiveInput,
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
            model = model.with_chat(log);
        }
        if let (Some(backpack), Some(corpse)) = (session.inventory(), session.open_container()) {
            model = model.with_items(backpack, corpse);
        }
        Self {
            session,
            model,
            view,
            input,
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

    /// Runs one command and applies its outcome (result and deltas) to the model. A chat or item
    /// input whose capability the server did not select is a notice, not an error: nothing is
    /// sent and the session stays usable.
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
            LiveCommand::Select(tile) => self.model.select_at(tile),
            LiveCommand::Chat(_) if self.model.chat.is_none() => {
                self.model.with_notice(Notice::ChatUnavailable)
            }
            LiveCommand::UseItem { .. } | LiveCommand::Loot { .. }
                if self.model.items.is_none() =>
            {
                self.model.with_notice(Notice::ItemsUnavailable)
            }
            LiveCommand::UseItem { handle, entity } => {
                let outcome = self.session.use_item(handle).await?;
                self.model.with_selected(entity).apply_use_item(&outcome)
            }
            LiveCommand::Loot { entry } => match self.model.loot_intent(entry) {
                Some(intent) => {
                    let outcome = self.session.move_item(&intent).await?;
                    self.model.apply_move(&outcome)
                }
                None => self.model.with_notice(Notice::NoSuchEntry),
            },
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
