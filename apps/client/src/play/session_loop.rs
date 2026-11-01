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
    fn animus_selected(&self) -> bool {
        false
    }
    fn animus(
        &mut self,
        _payload: &[u8],
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send {
        async {
            Err(SessionError::CapabilityNotSelected {
                capability: oteryn_session::animus_mastery::CAPABILITY_ANIMUS_MASTERY_V1,
            })
        }
    }
    fn wheel_selected(&self) -> bool {
        false
    }
    fn wheel(
        &mut self,
        _request: &oteryn_client_runtime::wheel_allocation::WheelCommand,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::wheel_allocation::WheelResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 148 }) }
    }
    fn exiva_selected(&self) -> bool {
        false
    }
    fn exiva(
        &mut self,
        _payload: &[u8],
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 147 }) }
    }
    fn quest_state(&self) -> oteryn_client_runtime::quest_log::QuestState {
        oteryn_client_runtime::quest_log::QuestState::default()
    }
    fn loyalty_skills_selected(&self) -> bool {
        false
    }
    fn loyalty_skills(
        &mut self,
        _payload: &[u8],
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send {
        async {
            Err(SessionError::CapabilityNotSelected {
                capability: oteryn_session::loyalty_skills::CAPABILITY_LOYALTY_SKILLS_V1,
            })
        }
    }
    fn quest_log(
        &mut self,
        _payload: &[u8],
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 16 }) }
    }
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
    fn animus_selected(&self) -> bool {
        self.is_selected(oteryn_session::animus_mastery::CAPABILITY_ANIMUS_MASTERY_V1)
    }
    async fn animus(
        &mut self,
        payload: &[u8],
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        self.gated_command(
            oteryn_session::animus_mastery::CAPABILITY_ANIMUS_MASTERY_V1,
            oteryn_session::animus_mastery::COMMAND_TYPE_ANIMUS_MASTERY_QUERY,
            payload,
        )
        .await
    }
    fn wheel_selected(&self) -> bool {
        self.is_selected(oteryn_session::wheel::CAPABILITY_WHEEL_ALLOCATION_V1)
    }
    async fn wheel(
        &mut self,
        request: &oteryn_client_runtime::wheel_allocation::WheelCommand,
    ) -> Result<oteryn_client_runtime::wheel_allocation::WheelResponse, SessionError> {
        oteryn_client_runtime::wheel_allocation::send(self, request).await
    }
    fn exiva_selected(&self) -> bool {
        self.is_selected(147)
    }
    async fn exiva(
        &mut self,
        payload: &[u8],
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        self.gated_command(147, 170, payload).await
    }
    fn loyalty_skills_selected(&self) -> bool {
        self.is_selected(oteryn_session::loyalty_skills::CAPABILITY_LOYALTY_SKILLS_V1)
    }
    async fn loyalty_skills(
        &mut self,
        payload: &[u8],
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        self.gated_command(
            oteryn_session::loyalty_skills::CAPABILITY_LOYALTY_SKILLS_V1,
            oteryn_session::loyalty_skills::COMMAND_TYPE_LOYALTY_SKILLS_QUERY,
            payload,
        )
        .await
    }
    fn quest_state(&self) -> oteryn_client_runtime::quest_log::QuestState {
        super::feature_0016_quest_log::protocol::initial(self)
    }
    async fn quest_log(
        &mut self,
        payload: &[u8],
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        super::feature_0016_quest_log::protocol::send(self, payload).await
    }
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
    outbox.animus_initialize(session.animus_selected());
    outbox.wheel_initialize(session.wheel_selected());
    outbox.exiva_initialize(session.exiva_selected());
    outbox.loyalty_skills_initialize(session.loyalty_skills_selected());
    outbox.quest_initialize(session.quest_state());
    let mut held = Vec::new();
    loop {
        let ended = match commands.try_recv() {
            Ok(PlayCommand::Animus(payload)) => match session.animus(&payload).await {
                Ok(result) => {
                    outbox.animus_result(&result);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Wheel(request)) => match session.wheel(&request).await {
                Ok(response) => {
                    outbox.wheel_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Exiva(payload)) => {
                let result = session.exiva(&payload).await;
                match result {
                    Ok(result) => {
                        // An accepted edit is followed by a fresh server read, including
                        // mode changes and UUID removal. Never infer the list locally.
                        let result = if payload != [5] && result.payload == [1] {
                            session.exiva(&[5]).await
                        } else {
                            Ok(result)
                        };
                        match result {
                            Ok(result) => {
                                outbox.exiva_result(&result);
                                publish(&mut session, &outbox, &mut held);
                                continue;
                            }
                            Err(error) => error,
                        }
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::QuestLog(payload)) => match session.quest_log(&payload).await {
                Ok(result) => {
                    outbox.quest_result(&result);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::LoyaltySkills(payload)) => match session.loyalty_skills(&payload).await
            {
                Ok(result) => {
                    outbox.loyalty_skills_result(&result);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
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
