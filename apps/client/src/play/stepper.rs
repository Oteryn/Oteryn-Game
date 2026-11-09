//! Session seam and feature defaults used by the production driver and synthetic scenarios.
use crate::combat_input::{CombatCommand, CombatReport};
use oteryn_client_runtime::{
    imbue::ImbueResponse, party::PartyResponse, prey::PreyResponse, stash::StashResponse,
};
use oteryn_session::{
    LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome, WorldSpatialEntity,
};
use std::{future::Future, time::Duration};
super::feature_0014_container_tree::protocol::session_trait! {
    fn refusal_detail(&self) -> Option<oteryn_session::refusal_detail::RefusalDetail> { None }
    fn target_walk(
        &mut self,
        _position: oteryn_session::ActorPosition,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::target_move::WalkResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 64 }) }
    }
    fn hotkey_equip_items(&self) -> Vec<u32> { Vec::new() }
    fn inventory(&self) -> Option<oteryn_session::CharacterInventory> {
        None
    }
    fn battle_floor(&self) -> Option<i16> {
        None
    }
    fn battle_state(&self) -> Option<oteryn_session::ActorCombatState> {
        None
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
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send
    {
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
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send
    {
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
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send
    {
        async {
            Err(SessionError::CapabilityNotSelected {
                capability: oteryn_session::loyalty_skills::CAPABILITY_LOYALTY_SKILLS_V1,
            })
        }
    }
    fn quest_log(
        &mut self,
        _payload: &[u8],
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send
    {
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
    fn logout(
        &mut self,
    ) -> impl Future<Output = Result<Option<LogoutResult>, SessionError>> + Send;
    /// The other visible entities when they differ from `held`.
    fn entities_changed(
        &self,
        _held: &[WorldSpatialEntity],
    ) -> Option<Vec<WorldSpatialEntity>> {
        None
    }
    fn visible_appearances(&self) -> Vec<oteryn_session::VisibleAppearance> { Vec::new() }
    fn own_identity(&self) -> Option<[u8; 16]> { None }
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
    ) -> impl Future<Output = Result<StashResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 62 }) }
    }
    fn npc_shop_selected(&self) -> bool {
        false
    }
    fn npc_shop(
        &mut self,
        _command: &oteryn_client_runtime::npc_shop::NpcShopCommand,
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::npc_shop::NpcShopResponse, SessionError>,
    > + Send {
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
    ) -> impl Future<Output = Result<PreyResponse, SessionError>> + Send
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
    ) -> impl Future<Output = Result<ImbueResponse, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 68 }) }
    }
    fn player_trade_items(&self) -> oteryn_client_runtime::player_trade::TradeItems {
        Default::default()
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
    ) -> impl Future<Output = Result<PartyResponse, SessionError>> + Send
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
    ) -> impl Future<Output = Result<oteryn_session::GatedCommandResult, SessionError>> + Send
    {
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
    ) -> impl Future<
        Output = Result<oteryn_client_runtime::market::MarketResponse, SessionError>,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 63 }) }
    }
    fn chat_private_state(&self) -> (bool, Option<oteryn_session::ChatChannelList>) {
        (false, None)
    }
    fn guild_chat_state(&self) -> oteryn_session::GuildRoomState {
        Default::default()
    }
    fn chat_public_selected(&self) -> bool {
        false
    }
    fn chat_state(
        &self,
    ) -> Option<(oteryn_session::ChatRoomSet, Vec<oteryn_session::ChatLine>)> {
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
        Output = Result<
            oteryn_client_runtime::player_typing::PlayerTypingResponse,
            SessionError,
        >,
    > + Send {
        async { Err(SessionError::CapabilityNotSelected { capability: 233 }) }
    }

    fn npc_talk(
        &mut self,
        _intent: &oteryn_session::npc_service::NpcTalkIntent,
    ) -> impl Future<Output = Result<oteryn_session::NpcIntentDisposition, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 3 }) }
    }
    fn npc_resources(
        &mut self,
    ) -> impl Future<
        Output = Result<
            Option<oteryn_client_runtime::npc_shop::resources::ResourceBalances>,
            SessionError,
        >,
    > + Send {
        async { Ok(None) }
    }
    fn npc_inventory(&self) -> Option<oteryn_session::CharacterInventory> {
        None
    }
    fn npc_trade(
        &mut self,
        _intent: &oteryn_session::npc_service::NpcTradeIntent,
    ) -> impl Future<Output = Result<oteryn_session::NpcIntentDisposition, SessionError>> + Send
    {
        async { Err(SessionError::CapabilityNotSelected { capability: 3 }) }
    }
}
