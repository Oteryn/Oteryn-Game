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
    fn task_board_selected(&self) -> bool {
        false
    }
    fn task_board(
        &mut self,
        _command: &oteryn_client_runtime::task_board::TaskBoardCommand,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::task_board::TaskBoardResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 72 }) }
    }
    fn stash_selected(&self) -> bool {
        false
    }
    fn stash(
        &mut self,
        _command: &oteryn_client_runtime::stash::StashCommand,
    ) -> impl Future<Output = Result<oteryn_client_runtime::stash::StashResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 62 }) }
    }
    fn npc_shop_selected(&self) -> bool {
        false
    }
    fn npc_shop(
        &mut self,
        _command: &oteryn_client_runtime::npc_shop::NpcShopCommand,
    ) -> impl Future<Output = Result<oteryn_client_runtime::npc_shop::NpcShopResponse, SessionError>>
    + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 81 }) }
    }
    fn prey_selected(&self) -> bool {
        false
    }
    fn prey(
        &mut self,
        _command: &oteryn_client_runtime::prey::PreyCommand,
    ) -> impl Future<Output = Result<oteryn_client_runtime::prey::PreyResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 140 }) }
    }
    fn imbue_selected(&self) -> bool {
        false
    }
    fn imbue(
        &mut self,
        _command: &oteryn_client_runtime::imbue::ImbueCommand,
    ) -> impl Future<Output = Result<oteryn_client_runtime::imbue::ImbueResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 68 }) }
    }
    fn player_trade_selected(&self) -> bool {
        false
    }
    fn player_trade_state(&self) -> Option<oteryn_session::player_trade::State> {
        None
    }
    fn player_trade(
        &mut self,
        _request: &oteryn_client_runtime::player_trade::PlayerTradeCommand,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::player_trade::PlayerTradeResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 60 }) }
    }
    fn vip_selected(&self) -> bool {
        false
    }
    fn vip(
        &mut self,
        _request: &oteryn_client_runtime::vip::VipCommand,
    ) -> impl Future<Output = Result<oteryn_client_runtime::vip::VipResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 43 }) }
    }
    fn guild_selected(&self) -> bool {
        false
    }
    fn guild(
        &mut self,
        _command: &oteryn_client_runtime::guild::GuildCommand,
    ) -> impl Future<Output = Result<oteryn_client_runtime::guild::GuildReply, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 40 }) }
    }
    fn party_selected(&self) -> bool {
        false
    }
    fn party(
        &mut self,
        _request: &oteryn_client_runtime::party::PartyCommand,
    ) -> impl Future<Output = Result<oteryn_client_runtime::party::PartyResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 42 }) }
    }
    fn hireling_selected(&self) -> bool {
        false
    }
    fn hireling(
        &mut self,
        _request: &oteryn_client_runtime::hireling::HirelingCommand,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::hireling::HirelingResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 165 }) }
    }
    fn market_selected(&self) -> bool {
        false
    }
    fn market(
        &mut self,
        _request: &oteryn_client_runtime::market::MarketCommand,
    ) -> impl Future<Output = Result<oteryn_client_runtime::market::MarketResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 63 }) }
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
    fn task_board_selected(&self) -> bool {
        self.is_selected(oteryn_session::task_board::CAPABILITY_TASK_BOARD_V1)
    }
    async fn task_board(
        &mut self,
        command: &oteryn_client_runtime::task_board::TaskBoardCommand,
    ) -> Result<oteryn_client_runtime::task_board::TaskBoardResponse, SessionError> {
        oteryn_client_runtime::task_board::send(self, command).await
    }
    fn stash_selected(&self) -> bool {
        self.is_selected(oteryn_session::stash::CAPABILITY_STASH_V1)
    }
    async fn stash(
        &mut self,
        command: &oteryn_client_runtime::stash::StashCommand,
    ) -> Result<oteryn_client_runtime::stash::StashResponse, SessionError> {
        oteryn_client_runtime::stash::send(self, command).await
    }
    fn npc_shop_selected(&self) -> bool {
        self.is_selected(oteryn_session::npc_shop::CAPABILITY_NPC_SHOP_V1)
    }
    async fn npc_shop(
        &mut self,
        command: &oteryn_client_runtime::npc_shop::NpcShopCommand,
    ) -> Result<oteryn_client_runtime::npc_shop::NpcShopResponse, SessionError> {
        oteryn_client_runtime::npc_shop::send(self, command).await
    }
    fn prey_selected(&self) -> bool {
        self.is_selected(oteryn_session::prey::CAPABILITY_PREY_V1)
    }
    async fn prey(
        &mut self,
        command: &oteryn_client_runtime::prey::PreyCommand,
    ) -> Result<oteryn_client_runtime::prey::PreyResponse, SessionError> {
        oteryn_client_runtime::prey::send(self, command).await
    }
    fn imbue_selected(&self) -> bool {
        self.is_selected(oteryn_session::imbuement::CAPABILITY_IMBUE_V1)
    }
    async fn imbue(
        &mut self,
        command: &oteryn_client_runtime::imbue::ImbueCommand,
    ) -> Result<oteryn_client_runtime::imbue::ImbueResponse, SessionError> {
        oteryn_client_runtime::imbue::send(self, command).await
    }
    fn player_trade_selected(&self) -> bool {
        self.is_selected(oteryn_session::player_trade::CAPABILITY_PLAYER_TRADE_V1)
    }
    fn player_trade_state(&self) -> Option<oteryn_session::player_trade::State> {
        self.gated_snapshot(oteryn_session::player_trade::STATE_DOMAIN_PLAYER_TRADE)
            .and_then(|snapshot| oteryn_session::player_trade::decode_state(&snapshot.payload).ok())
    }
    async fn player_trade(
        &mut self,
        request: &oteryn_client_runtime::player_trade::PlayerTradeCommand,
    ) -> Result<oteryn_client_runtime::player_trade::PlayerTradeResponse, SessionError> {
        oteryn_client_runtime::player_trade::send(self, request).await
    }
    fn vip_selected(&self) -> bool {
        self.is_selected(oteryn_session::vip::CAPABILITY_VIP_V1)
    }
    async fn vip(
        &mut self,
        request: &oteryn_client_runtime::vip::VipCommand,
    ) -> Result<oteryn_client_runtime::vip::VipResponse, SessionError> {
        oteryn_client_runtime::vip::send(self, request).await
    }
    fn guild_selected(&self) -> bool {
        self.is_selected(oteryn_session::guild::CAPABILITY_GUILD_V1)
    }
    async fn guild(
        &mut self,
        command: &oteryn_client_runtime::guild::GuildCommand,
    ) -> Result<oteryn_client_runtime::guild::GuildReply, SessionError> {
        oteryn_client_runtime::guild::send(self, command).await
    }
    fn party_selected(&self) -> bool {
        self.is_selected(oteryn_client_runtime::party::party::CAPABILITY_PARTY_V1)
    }
    async fn party(
        &mut self,
        request: &oteryn_client_runtime::party::PartyCommand,
    ) -> Result<oteryn_client_runtime::party::PartyResponse, SessionError> {
        oteryn_client_runtime::party::send(self, request).await
    }
    fn hireling_selected(&self) -> bool {
        self.is_selected(oteryn_client_runtime::hireling::hireling::CAPABILITY_HIRELING_V1)
    }
    async fn hireling(
        &mut self,
        request: &oteryn_client_runtime::hireling::HirelingCommand,
    ) -> Result<oteryn_client_runtime::hireling::HirelingResponse, SessionError> {
        oteryn_client_runtime::hireling::send(self, request).await
    }
    fn market_selected(&self) -> bool {
        self.is_selected(oteryn_client_runtime::market::market::CAPABILITY_MARKET_V1)
    }
    async fn market(
        &mut self,
        request: &oteryn_client_runtime::market::MarketCommand,
    ) -> Result<oteryn_client_runtime::market::MarketResponse, SessionError> {
        oteryn_client_runtime::market::send(self, request).await
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
    outbox.task_board_initialize(session.task_board_selected());
    outbox.stash_initialize(session.stash_selected());
    outbox.npc_shop_initialize(session.npc_shop_selected());
    outbox.prey_initialize(session.prey_selected());
    outbox.imbue_initialize(session.imbue_selected());
    outbox.player_trade_initialize(
        session.player_trade_selected(),
        session.player_trade_state(),
    );
    outbox.vip_initialize(session.vip_selected());
    outbox.guild_initialize(session.guild_selected());
    outbox.party_initialize(session.party_selected());
    outbox.hireling_initialize(session.hireling_selected());
    outbox.market_initialize(session.market_selected());
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
            Ok(PlayCommand::Stash(command)) => match session.stash(&command).await {
                Ok(response) => {
                    outbox.stash_result(&response);
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
            Ok(PlayCommand::TaskBoard(command)) => match session.task_board(&command).await {
                Ok(response) => {
                    outbox.task_board_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::NpcShop(command)) => match session.npc_shop(&command).await {
                Ok(result) => {
                    outbox.npc_shop_result(&result);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Prey(command)) => match session.prey(&command).await {
                Ok(response) => {
                    outbox.prey_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Imbue(command)) => match session.imbue(&command).await {
                Ok(response) => {
                    outbox.imbue_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::PlayerTrade(request)) => match session.player_trade(&request).await {
                Ok(response) => {
                    outbox.player_trade_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Vip(request)) => match session.vip(&request).await {
                Ok(response) => {
                    outbox.vip_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Guild(command)) => match session.guild(&command).await {
                Ok(reply) => {
                    outbox.guild_result(&reply);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Party(request)) => match session.party(&request).await {
                Ok(response) => {
                    outbox.party_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Hireling(request)) => match session.hireling(&request).await {
                Ok(response) => {
                    outbox.hireling_result(&response);
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
            Ok(PlayCommand::Market(request)) => match session.market(&request).await {
                Ok(response) => {
                    outbox.market_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
        };
        outbox.end(public_class(&ended));
        return;
    }
}
