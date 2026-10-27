//! Session task phases: receive one shell command, service the session, and publish its result.

use super::{Outbox, public_class, step_direction};
use crate::combat_input::{CombatCommand, CombatReport, PlayCommand, run_combat};
use crate::spell::SpellFeedback;
use oteryn_session::{
    LogoutResult, Session, SessionError, SessionEvent, SessionStream, StepDirection, StepOutcome,
    WorldSpatialEntity,
};
use std::future::Future;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

/// How long the session task serves liveness probes while no step is pending; well inside the
/// server's probe cadence.
const IDLE_SLICE: Duration = Duration::from_millis(20);

/// The session calls the task needs; implemented by `Session`.
pub trait Stepper: Send {
    fn step(
        &mut self,
        direction: StepDirection,
    ) -> impl Future<Output = Result<StepOutcome, SessionError>> + Send;
    fn serve(
        &mut self,
        duration: Duration,
    ) -> impl Future<Output = Result<(), SessionError>> + Send;
    /// The pushed deltas applied since the last call; the session fails closed when this queue
    /// is left to fill.
    fn drain(&mut self) -> Vec<SessionEvent>;
    /// Runs one combat command (attack target, cast).
    fn combat(
        &mut self,
        command: CombatCommand,
    ) -> impl Future<Output = Result<CombatReport, SessionError>> + Send;
    /// Sends one logout: the server's result, `None` when it refused the command before its
    /// logout authority.
    fn logout(&mut self)
    -> impl Future<Output = Result<Option<LogoutResult>, SessionError>> + Send;
    /// The other visible entities when they differ from `held`.
    fn entities_changed(&self, _held: &[WorldSpatialEntity]) -> Option<Vec<WorldSpatialEntity>> {
        None
    }
}

impl<S: SessionStream + Send> Stepper for Session<S> {
    fn step(
        &mut self,
        direction: StepDirection,
    ) -> impl Future<Output = Result<StepOutcome, SessionError>> + Send {
        self.step_retrying(direction)
    }

    fn serve(
        &mut self,
        duration: Duration,
    ) -> impl Future<Output = Result<(), SessionError>> + Send {
        self.service_liveness(duration)
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        self.take_events()
    }

    async fn combat(&mut self, command: CombatCommand) -> Result<CombatReport, SessionError> {
        run_combat(self, &mut SpellFeedback::default(), command).await
    }

    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(Session::logout(self).await?.result)
    }

    fn entities_changed(&self, held: &[WorldSpatialEntity]) -> Option<Vec<WorldSpatialEntity>> {
        let entities = self.world_entities()?;
        (!entities.others().eq(held.iter())).then(|| entities.others().copied().collect())
    }
}

/// Hands the shell what the session pushed, and the visible entities when they changed.
fn publish<T: Stepper>(session: &mut T, outbox: &Outbox, held: &mut Vec<WorldSpatialEntity>) {
    outbox.push(session.drain());
    if let Some(entities) = session.entities_changed(held) {
        held.clone_from(&entities);
        outbox.entities(entities);
    }
}

/// The session task, run on the client runtime: sends requested steps and, while none is
/// pending, keeps servicing liveness so an idle session is not dropped. It ends with the first
/// session error (reported with its public class) or when the shell drops its link.
pub async fn run_session<T: Stepper>(
    mut session: T,
    mut commands: UnboundedReceiver<PlayCommand>,
    outbox: Outbox,
) {
    let mut held = Vec::new();
    loop {
        let ended = match commands.try_recv() {
            Ok(PlayCommand::Step(direction)) => {
                match session.step(step_direction(direction)).await {
                    Ok(outcome) => {
                        outbox.stepped(outcome);
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::Combat(command)) => match session.combat(command).await {
                Ok(report) => {
                    outbox.combat(report);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Logout) => match session.logout().await {
                Ok(Some(LogoutResult::Accepted)) => {
                    // The server ended the session and closes the transport.
                    outbox.logged_out();
                    return;
                }
                Ok(refused) => {
                    outbox.logout_refused(refused);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                match session.serve(IDLE_SLICE).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return,
        };
        outbox.end(public_class(&ended));
        return;
    }
}
