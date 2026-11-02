//! The production `Stepper`: every session call the play task needs, served by `Session`.

use super::Stepper;
use crate::combat_input::{CombatCommand, CombatReport, run_combat};
use crate::spell::SpellFeedback;
use oteryn_session::{
    LogoutResult, Session, SessionError, SessionEvent, SessionStream, StepDirection, StepOutcome,
    WorldSpatialEntity,
};
use std::future::Future;
use std::time::Duration;

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
