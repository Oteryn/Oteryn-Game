//! Session task phases: receive one shell command, service the session, and publish its result.

use super::{
    Outbox, feature_0043_vip::protocol::send_vip, public_class, session_stepper::publish,
    step_direction,
};
use crate::combat_input::PlayCommand;
use oteryn_session::LogoutResult;

use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

/// How long the session task serves liveness probes while no step is pending; well inside the
/// server's probe cadence.
const IDLE_SLICE: Duration = Duration::from_millis(20);

pub use super::stepper::Stepper;
super::feature_local_account_achievements::protocol::session_trait! {
    fn target_walk(&mut self, _position: oteryn_session::ActorPosition) -> impl Future<Output = Result<oteryn_client_runtime::target_move::WalkResponse, SessionError>> + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 64 }) }
    }
    fn actor_vitals(&self) -> Option<oteryn_session::ActorVitals> {
        None
    }
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
    fn visible_appearances(&self) -> Vec<oteryn_session::VisibleAppearance> { Vec::new() }
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
    fn boss_rewards_selected(&self) -> bool {
        false
    }
    fn boss_rewards(
        &mut self,
        _command: &oteryn_client_runtime::boss_rewards::BossRewardsCommand,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::boss_rewards::BossRewardsResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 224 }) }
    }
    fn prey_selected(&self) -> bool {
        false
    }
    fn prey(
        &mut self,
        _command: &oteryn_client_runtime::prey::prey::Request,
    ) -> impl Future<Output = Result<oteryn_client_runtime::prey::PreyResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 140 }) }
    }
    fn daily_reward_selected(&self) -> bool {
        false
    }
    fn daily_reward(
        &mut self,
        _command: &oteryn_client_runtime::daily_reward::DailyRewardCommand,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::daily_reward::DailyRewardResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 182 }) }
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
    fn feature_selected(&self, _capability: u32) -> bool {
        false
    }
    fn feature(
        &mut self,
        _command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 0 }) }
    }
    fn feature_snapshot(&self, _domain: u32) -> Option<Vec<u8>> {
        None
    }
    fn server_mechanic_selected(&self, _capability: u32) -> bool {
        false
    }
    fn server_mechanic(
        &mut self,
        _request: &oteryn_client_runtime::server_mechanic::ServerMechanicCommand,
    ) -> impl Future<
        Output = Result<
            oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
            SessionError,
        >,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 0 }) }
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
    fn chat_public_selected(&self) -> bool {
        false
    }
    fn chat_state(&self) -> Option<(oteryn_session::ChatRoomSet, Vec<oteryn_session::ChatLine>)> {
        None
    }
    fn chat_public(
        &mut self,
        _intent: &oteryn_session::ChatIntent,
    ) -> impl Future<Output = Result<oteryn_session::ChatOutcome, SessionError>> + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 259 }) }
    }
    fn player_typing_selected(&self) -> bool {
        false
    }
    fn player_typing(
        &mut self,
        _command: &oteryn_client_runtime::player_typing::PlayerTypingCommand,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::player_typing::PlayerTypingResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 233 }) }
    }
}

/// Initializes the live map, sends commands and services session liveness.
pub async fn run_session<T: Stepper>(
    mut session: T,
    mut commands: UnboundedReceiver<PlayCommand>,
    outbox: Outbox,
) {
    outbox.equipment_initialize(session.equipment_selected());
    outbox.world_context_initialize(session.feature_selected(28), session.feature_selected(11));
    outbox.targeting_initialize(session.feature_selected(15));
    // Publish sources only after their equipment, movement and targeting routes are ready.
    outbox.inventory_initialize(session.inventory(), session.item_count_selected());
    outbox.container_initialize(session.container_views());
    outbox.target_walk_initialize(session.feature_selected(64));
    outbox.vitals(session.actor_vitals());
    outbox.world_light_initialize(session.feature_snapshot(18));
    outbox.world_weather_initialize(session.feature_snapshot(22));
    outbox.map_initialize(session.feature_snapshot(17));
    outbox.animus_initialize(session.animus_selected());
    outbox.wheel_initialize(session.wheel_selected());
    outbox.exiva_initialize(session.exiva_selected());
    outbox.loyalty_skills_initialize(session.loyalty_skills_selected());
    outbox.quest_initialize(session.quest_state());
    outbox.task_board_initialize(session.task_board_selected());
    outbox.stash_initialize(session.stash_selected());
    outbox.dialogue_initialize(&session);
    outbox.npc_shop_initialize(&session);
    outbox.prey_initialize(session.prey_selected());
    outbox.daily_reward_initialize(session.daily_reward_selected());
    outbox.boss_rewards_initialize(session.boss_rewards_selected());
    outbox.imbue_initialize(session.imbue_selected());
    outbox.player_trade_initialize(
        session.player_trade_selected(),
        session.player_trade_state(),
    );
    outbox.vip_initialize(session.vip_selected());
    outbox.guild_initialize(session.guild_selected());
    outbox.party_initialize(
        session.party_selected(),
        session.feature_selected(oteryn_session::CAPABILITY_CHAT_V1),
    );
    outbox.hireling_initialize(session.hireling_selected());
    outbox.market_initialize(session.market_selected());
    outbox.chat_public_initialize(session.chat_public_selected(), session.chat_state());
    outbox.chat_private_publish(&session);
    outbox.player_typing_initialize(session.player_typing_selected());
    outbox.boss_encounter_initialize(session.feature_selected(117));
    outbox.boss_difficulty_initialize(session.feature_selected(225));
    outbox.browse_field_initialize(session.server_mechanic_selected(230));
    outbox.guild_chat_publish(&session);
    outbox.chat_initialize(session.feature_selected(oteryn_session::CAPABILITY_CHAT_V1));
    outbox.chat_party_room_initialize(session.feature_selected(240));
    outbox.cyclopedia_character_initialize(session.feature_selected(199));
    outbox.cyclopedia_tracker_initialize(session.feature_selected(197));
    outbox.world_event_initialize(session.feature_selected(53));
    outbox.daily_boost_initialize(session.feature_selected(151));
    outbox.highscores_initialize(session.server_mechanic_selected(9));
    outbox.depot_search_initialize(session.server_mechanic_selected(153));
    outbox.hotkey_equip_initialize(session.server_mechanic_selected(179));
    outbox.item_forge_initialize(session.server_mechanic_selected(152));
    outbox.store_initialize(session.server_mechanic_selected(154));
    outbox.vip_presence_initialize(session.feature_selected(258));
    outbox.weapon_proficiency_initialize(session.feature_selected(195));
    outbox.depot_initialize(session.feature_selected(37));
    outbox.achievements_initialize();
    outbox.player_trade_items(session.player_trade_items());
    let mut held = Vec::new();
    loop {
        let ended = super::feature_0014_container_tree::protocol::dispatch! {
            outbox.death_command(&mut commands), session, outbox, held, publish;
            Ok(PlayCommand::Feature(command)) => match session.feature(&command).await {
                Ok(result) => {
                    match command.capability {
                        14 => outbox.player_trade_container_result(&result),
                        53 => outbox.world_event_result(&result),
                        117 => outbox.boss_encounter_result(&result),
                        151 => outbox.daily_boost_result(&result),
                        195 => outbox.weapon_proficiency_result(&result),
                        197 => outbox.cyclopedia_tracker_result(&result),
                        199 => outbox.cyclopedia_character_result(&result),
                        225 => outbox.boss_difficulty_result(&result),
                        _ => {}
                    }
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::ServerMechanic(request)) => {
                match session.server_mechanic(&request).await {
                    Ok(response) => {
                        match request.capability {
                            9 => outbox.highscores_result(&response),
                            152 => outbox.item_forge_result(&response),
                            153 => outbox.depot_search_result(&response),
                            154 => outbox.store_result(&response),
                            179 => outbox.hotkey_equip_result(&response),
                            230 => outbox.browse_field_result(&response),
                            _ => {}
                        }
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
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
            Ok(PlayCommand::Exiva(payload)) => match outbox.exiva_run(&mut session, &payload).await
            {
                Ok(()) => {
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
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
            Ok(PlayCommand::TargetWalk(position)) => {
                match outbox.target_walk_run(&mut session, position).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
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
            Ok(command @ (PlayCommand::Combat(_) | PlayCommand::Battle(_))) => {
                match outbox.battle_run(&mut session, command).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
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
            Ok(PlayCommand::NpcTalk(intent)) => {
                match outbox.dialogue_run(&mut session, &intent).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::NpcTrade(intent)) => {
                match outbox.npc_trade_run(&mut session, &intent).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::NpcShop(command)) => {
                match outbox.npc_shop_run(&mut session, &command).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::BossRewards(command)) => match session.boss_rewards(&command).await {
                Ok(response) => {
                    outbox.boss_rewards_result(&response);
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
            Ok(PlayCommand::DailyReward(command)) => match session.daily_reward(&command).await {
                Ok(response) => {
                    outbox.daily_reward_result(&response);
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
                    publish(&mut session, &outbox, &mut held);
                    outbox.player_trade_result(&response);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Vip(request)) => match send_vip(&mut session, &request).await {
                Ok(response) => {
                    outbox.vip_result(&response);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Guild(command)) => match outbox.guild_run(&mut session, &command).await
            {
                Ok(()) => {
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Party(request)) => match outbox.party_run(&mut session, &request).await
            {
                Ok(()) => {
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
            Ok(PlayCommand::Market(request)) => {
                match outbox.market_execute(&mut session, &request).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::ChatPublic(intent)) => match session.chat_public(&intent).await {
                Ok(outcome) => {
                    outbox.chat_result(&intent, &outcome);
                    publish(&mut session, &outbox, &mut held);
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::PlayerTyping(command)) => match session.player_typing(&command).await {
                Ok(response) => {
                    outbox.player_typing_result(command, response);
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
