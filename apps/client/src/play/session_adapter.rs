use super::*;

impl<S: SessionStream + Send> Stepper for Session<S> {
    fn hotkey_own_actor(&self) -> Option<WorldSpatialEntity> {
        let entities = self.world_entities()?;
        entities.iter().find_map(|entity| {
            (entity.entity.identity == *entities.own_identity()).then_some(*entity)
        })
    }
    fn refusal_detail(&self) -> Option<oteryn_session::refusal_detail::RefusalDetail> {
        self.last_refusal_detail()
    }
    fn equipment_selected(&self) -> bool {
        self.is_selected(12)
    }
    fn item_count_selected(&self) -> bool {
        self.is_selected(61)
    }
    async fn move_item(
        &mut self,
        request: oteryn_client_runtime::inventory::MoveRequest,
    ) -> Result<Option<oteryn_client_runtime::inventory::MoveResponse>, SessionError> {
        oteryn_client_runtime::inventory::send(self, request).await
    }
    fn container_views(&self) -> Option<oteryn_client_runtime::container_tree::ContainerViews> {
        oteryn_client_runtime::container_tree::initial(self)
    }
    async fn container_view(
        &mut self,
        intent: oteryn_client_runtime::container_tree::ContainerViewIntent,
    ) -> Result<Option<oteryn_client_runtime::container_tree::ContainerViewOutcome>, SessionError>
    {
        oteryn_client_runtime::container_tree::send(self, intent).await
    }
    fn battle_floor(&self) -> Option<i16> {
        Some(self.world_spatial().actor_position.floor)
    }

    fn battle_state(&self) -> Option<oteryn_session::ActorCombatState> {
        Session::combat_state(self).copied()
    }
    async fn depot_open_container(
        &mut self,
        handle: oteryn_session::ItemHandle,
    ) -> Result<Option<oteryn_client_runtime::depot::ContainerViewOutcome>, SessionError> {
        oteryn_client_runtime::depot::open_container(self, handle).await
    }
    fn depot_items(&self) -> oteryn_client_runtime::depot::Items {
        oteryn_client_runtime::depot::items(self)
    }
    async fn depot(
        &mut self,
        intent: oteryn_session::DepotIntent,
    ) -> Result<Option<oteryn_session::DepotReply>, SessionError> {
        oteryn_client_runtime::depot::send(self, intent).await
    }

    async fn target_walk(
        &mut self,
        position: oteryn_session::ActorPosition,
    ) -> Result<oteryn_client_runtime::target_move::WalkResponse, SessionError> {
        oteryn_client_runtime::target_move::send(self, position).await
    }

    async fn account_achievements(
        &mut self,
        page: u32,
    ) -> Result<oteryn_client_runtime::account_achievements::Response, SessionError> {
        oteryn_client_runtime::account_achievements::send(self, page).await
    }

    fn inventory(&self) -> Option<oteryn_session::CharacterInventory> {
        oteryn_client_runtime::inventory::initial(self)
    }
    fn actor_vitals(&self) -> Option<oteryn_session::ActorVitals> {
        Session::actor_vitals(self).copied()
    }
    fn player_typing_selected(&self) -> bool {
        self.is_selected(oteryn_session::player_typing::CAPABILITY_PLAYER_TYPING_V1)
    }
    async fn player_typing(
        &mut self,
        command: &oteryn_client_runtime::player_typing::PlayerTypingCommand,
    ) -> Result<oteryn_client_runtime::player_typing::PlayerTypingResponse, SessionError> {
        oteryn_client_runtime::player_typing::send(self, command).await
    }
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
        super::super::feature_0016_quest_log::protocol::initial(self)
    }
    async fn quest_log(
        &mut self,
        payload: &[u8],
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        super::super::feature_0016_quest_log::protocol::send(self, payload).await
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
    fn visible_appearances(&self) -> Vec<oteryn_session::VisibleAppearance> {
        Session::visible_appearances(self)
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
    async fn npc_talk(
        &mut self,
        intent: &oteryn_session::npc_service::NpcTalkIntent,
    ) -> Result<oteryn_session::NpcIntentDisposition, SessionError> {
        oteryn_client_runtime::npc_dialogue::send_talk(self, intent).await
    }
    async fn npc_resources(
        &mut self,
    ) -> Result<Option<oteryn_client_runtime::npc_shop::resources::ResourceBalances>, SessionError>
    {
        oteryn_client_runtime::npc_shop::balances(self).await
    }
    fn npc_inventory(&self) -> Option<oteryn_session::CharacterInventory> {
        self.inventory().cloned()
    }
    async fn npc_trade(
        &mut self,
        intent: &oteryn_session::npc_service::NpcTradeIntent,
    ) -> Result<oteryn_session::NpcIntentDisposition, SessionError> {
        oteryn_client_runtime::npc_shop::trade(self, intent).await
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
    fn boss_rewards_selected(&self) -> bool {
        self.is_selected(oteryn_session::boss_rewards::CAPABILITY_BOSS_REWARDS_V1)
    }
    async fn boss_rewards(
        &mut self,
        command: &oteryn_client_runtime::boss_rewards::BossRewardsCommand,
    ) -> Result<oteryn_client_runtime::boss_rewards::BossRewardsResponse, SessionError> {
        oteryn_client_runtime::boss_rewards::send(self, command).await
    }
    fn prey_selected(&self) -> bool {
        self.is_selected(oteryn_session::prey::CAPABILITY_PREY_V1)
    }
    async fn prey(
        &mut self,
        command: &oteryn_client_runtime::prey::prey::Request,
    ) -> Result<oteryn_client_runtime::prey::PreyResponse, SessionError> {
        oteryn_client_runtime::prey::send(self, command).await
    }
    fn daily_reward_selected(&self) -> bool {
        self.is_selected(
            oteryn_client_runtime::daily_reward::daily_reward::CAPABILITY_DAILY_REWARD_V2,
        )
    }
    async fn daily_reward(
        &mut self,
        command: &oteryn_client_runtime::daily_reward::DailyRewardCommand,
    ) -> Result<oteryn_client_runtime::daily_reward::DailyRewardResponse, SessionError> {
        oteryn_client_runtime::daily_reward::send(self, command).await
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
    fn player_trade_items(&self) -> oteryn_client_runtime::player_trade::TradeItems {
        super::super::feature_0060_player_trade::protocol::initial_items(self)
    }
    fn player_trade_selected(&self) -> bool {
        self.is_selected(oteryn_session::player_trade::CAPABILITY_PLAYER_TRADE_V1)
    }
    fn player_trade_state(&self) -> Option<oteryn_session::player_trade::State> {
        oteryn_session::Session::player_trade_state(self)
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
    fn targeting_source_current(&self, source: oteryn_session::ItemHandle) -> bool {
        oteryn_client_runtime::world_interaction::item_current(self, source)
    }
    async fn world_context(
        &mut self,
        command: &oteryn_client_runtime::world_interaction::Command,
    ) -> Result<String, SessionError> {
        oteryn_client_runtime::world_interaction::send(self, command).await
    }
    fn feature_selected(&self, capability: u32) -> bool {
        self.is_selected(capability)
    }
    async fn feature(
        &mut self,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
    ) -> Result<oteryn_session::GatedCommandResult, SessionError> {
        oteryn_client_runtime::raw_feature::send(self, command).await
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        self.gated_snapshot(domain)
            .map(|snapshot| snapshot.payload.clone())
    }
    fn hotkey_equip_items(&self) -> Vec<u32> {
        let mut items = Vec::new();
        if let Some(inventory) = self.inventory() {
            items.extend(
                inventory
                    .entries
                    .iter()
                    .map(|item| item.item_definition_ref.get()),
            );
            items.extend(
                inventory
                    .equipment
                    .iter()
                    .map(|entry| entry.item.item_definition_ref.get()),
            );
        }
        if let Some(views) = Session::container_views(self) {
            for view in &views.views {
                items.extend(
                    view.entries
                        .iter()
                        .map(|item| item.item_definition_ref.get()),
                );
            }
        }
        items
    }
    fn server_mechanic_selected(&self, capability: u32) -> bool {
        self.is_selected(capability)
    }
    async fn server_mechanic(
        &mut self,
        request: &oteryn_client_runtime::server_mechanic::ServerMechanicCommand,
    ) -> Result<oteryn_client_runtime::server_mechanic::ServerMechanicResponse, SessionError> {
        self.gated_command(request.capability, request.command_type, &request.payload)
            .await
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
    fn chat_private_state(&self) -> (bool, Option<oteryn_session::ChatChannelList>) {
        (self.is_selected(80), self.private_channel_list().cloned())
    }
    fn guild_chat_state(&self) -> oteryn_session::GuildRoomState {
        self.chat_log()
            .map_or_else(Default::default, |log| log.guild_room().clone())
    }
    fn chat_public_selected(&self) -> bool {
        self.is_selected(259)
    }
    fn chat_state(&self) -> Option<(oteryn_session::ChatRoomSet, Vec<oteryn_session::ChatLine>)> {
        self.chat_log()
            .map(|log| (log.rooms(), log.lines().cloned().collect()))
    }
    async fn chat_public(
        &mut self,
        intent: &oteryn_session::ChatIntent,
    ) -> Result<oteryn_session::ChatOutcome, SessionError> {
        self.chat(intent).await
    }
}
