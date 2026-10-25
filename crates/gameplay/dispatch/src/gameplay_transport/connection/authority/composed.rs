//! Production delegation to the existing mechanic owners.

use crate::gameplay_transport::*;

impl FreshAdmissionAuthority for ComposedFreshAdmission<'_, '_, '_> {
    mechanic_composed_methods!();
    async fn party_command(
        &self,
        admitted: connection::AdmittedSession,
        command_id: u64,
        command_type: u32,
        payload: &[u8],
        view: &mut party_command::SessionPartyView,
    ) -> Option<Vec<u8>> {
        self.party_wire_command(admitted, command_id, command_type, payload, view)
            .await
    }

    async fn timed_item(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        payload: &[u8],
        target: Option<item_view::ItemKey>,
    ) -> Option<Vec<u8>> {
        self.timed_item_command(actor, command, payload, target)
            .await
    }

    fn session_commands(&self) -> Option<&session_ingress::SessionCommands> {
        Some(&self.states.command_ingress)
    }
    async fn observe_quest_log(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        since: Option<u64>,
    ) -> quest_log::QuestLogObservation {
        self.quest_log_observation(actor, session, since).await
    }

    fn world_weather(&self) -> Option<&crate::world_weather::WorldWeatherSource> {
        // WEATHER-1 (D1027 signed off, D1448): the shipped Oteryn-designed data, `None` (never
        // offered) when it is missing or invalid. The data is public and stateless: its binding
        // token is the content revision, so no per-World secret is read.
        crate::world_weather::shipped()
    }

    async fn chat_admit(
        &self,
        _actor: ExactActorRef,
        session: GameSessionId,
    ) -> oteryn_protocol_oteryn::chat::ChatRoomSet {
        self.chat_open(session).await
    }

    async fn chat_intent(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: oteryn_protocol_oteryn::chat::ChatIntent,
    ) -> oteryn_protocol_oteryn::chat::ChatIntentResult {
        self.complete_chat_intent(actor, session, command_id, &intent)
            .await
            .map_or(
                oteryn_protocol_oteryn::chat::ChatIntentResult {
                    disposition: oteryn_protocol_oteryn::chat::ChatDisposition::ChatUnavailable,
                    wait_seconds: 0,
                },
                |(result, _)| result,
            )
    }

    async fn chat_intent_completion(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        intent: oteryn_protocol_oteryn::chat::ChatIntent,
    ) -> Option<(
        oteryn_protocol_oteryn::chat::ChatIntentResult,
        Option<actor_spell::SpellCastOutcome>,
    )> {
        self.complete_chat_intent(actor, session, command_id, &intent)
            .await
    }

    async fn chat_guild_admit(
        &self,
        admitted: AdmittedSession,
    ) -> oteryn_protocol_oteryn::chat::guild_room::GuildRoomState {
        self.guild_chat_admit(admitted).await
    }

    async fn chat_guild_intent(
        &self,
        admitted: AdmittedSession,
        text: String,
    ) -> oteryn_protocol_oteryn::chat::ChatIntentResult {
        self.guild_chat_send(admitted, text).await
    }

    async fn chat_next_bound(
        &self,
        admitted: AdmittedSession,
        remaining: &mut usize,
    ) -> Option<oteryn_protocol_oteryn::chat::ChatLine> {
        self.guild_chat_next(admitted, remaining).await
    }

    async fn chat_delivered(&self, session: GameSessionId) {
        self.states.chat.delivered(session);
    }

    async fn chat_rooms_changed(
        &self,
        session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::chat::ChatRoomSet> {
        self.states.chat.take_rooms_change(session)
    }

    async fn npc_session_open(&self, session: GameSessionId) {
        self.open_npc_session(session).await
    }

    async fn npc_follow(
        &self,
        _actor: ExactActorRef,
        session: GameSessionId,
        shown: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    ) -> Option<npc_command::NpcUpdate> {
        self.follow_npc_session(_actor, session, shown).await
    }

    async fn resume_npc_bank(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Option<npc_command::NpcTalkOutcome> {
        npc_command::resume_npc_talk(self, actor, command).await
    }

    async fn npc_talk(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::npc_service::NpcTalkIntent,
    ) -> npc_command::NpcTalkOutcome {
        self.vocation_talk(actor, command, intent).await
    }

    fn take_tutorial_notice(
        &self,
        session: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice> {
        self.states.tutorial_sessions.take(session)
    }

    async fn npc_trade(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::npc_service::NpcTradeIntent,
    ) -> npc_trade_command::NpcTradeStep {
        npc_trade_command::npc_trade(self, actor, command, intent).await
    }

    /// MAP-CUTOVER-1b: a bundle World offers the Item view set plus capability 18.
    /// MAP-ITEM-REF-1: any other World offers capability 4 (once Part B sets its gate) only when
    /// the active generation pins a non-empty Item key set. ADMIT0: boot validated
    /// `required_capabilities` against this same offer; a bundle World's floor holds 18, so
    /// admission requires it through the authenticated check in `admit`, not here.
    fn offered_capabilities(&self) -> &'static [capabilities::OfferedCapability] {
        loop_training::offer(
            self,
            world_required_capabilities::offered_for_scope(
                self.bundle_map().is_some(),
                self.active_generation,
            ),
        )
    }

    /// MAP-CUTOVER-1b: domain 17 of a bundle World at the actor's current position. The position
    /// is read under the runtime lock `step` takes (§1.4); the window is composed and encoded
    /// after the lock is released.
    async fn observe_world_map(
        &self,
        actor: ExactActorRef,
        view: &mut world_map::SessionMapView,
        items: &mut item_view::SessionItemView,
    ) -> Option<Result<Option<world_map::MapUpdate>, world_map::MapViewError>> {
        bundle_map_position::observe(self, actor, view, items).await
    }

    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        self.observe_actor_position(actor).await
    }

    /// MAP-ITEM-REF-1: domain 9 from the durable backpack, each definition through the Channel
    /// content pin's Item definition index.
    async fn observe_character_inventory(
        &self,
        _actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<item_view::InventoryItems> {
        item_ref_admission::observe_character_inventory(self, game_session_id).await
    }

    async fn observe_character_equipment(
        &self,
        _actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<
        Vec<(
            oteryn_protocol_oteryn::item_view::EquipmentSlot,
            item_view::ViewItem,
        )>,
    > {
        equipment_move::observe_character_equipment(self, game_session_id).await
    }

    async fn committed_equipment_move(
        &self,
        _actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Option<item_move::EquipmentMoveReplay> {
        equipment_move::committed_equipment_move(self, command).await
    }

    async fn move_equipment(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        request: item_move::EquipmentMove,
    ) -> item_move::EquipmentMoveResult {
        equipment_move::move_equipment(self, actor, command, request).await
    }

    async fn stash(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::stash::StashIntent,
        item: Option<item_view::ItemKey>,
        row_open: bool,
    ) -> Option<oteryn_protocol_oteryn::stash::StashReply> {
        stash::execute(self, actor, command, intent, item, row_open).await
    }

    async fn depot(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::depot::DepotIntent,
        item: Option<item_view::ItemKey>,
    ) -> Option<oteryn_protocol_oteryn::depot::DepotReply> {
        depot::execute(self, actor, command, intent, item).await
    }

    async fn committed_loop_move(
        &self,
        _actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Option<item_move::LoopMoveReplay> {
        loop_move::committed_loop_move(self, command).await
    }

    async fn move_loop_item(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        request: item_move::LoopMove,
    ) -> item_move::LoopMoveResult {
        loop_move::move_loop_item(self, actor, command, request).await
    }

    /// BAGS-1: a container of the carried tree (capability 14).
    async fn observe_container(
        &self,
        _actor: ExactActorRef,
        fence: Option<crate::durability::item_transfer::CurrentCharacterItemFence>,
        target: item_view::ItemKey,
    ) -> Option<container_view::ContainerObservation> {
        loop_move::observe_container(self, fence, target).await
    }

    /// VIS-3: the Channel's players, live creatures and (NPC-LOAD-1) NPC actors, read in one
    /// owner work item, and (MAP-ITEM-REF-1 Part B) its bound corpses as objects in the same
    /// read.
    async fn observe_visible_entities(
        &self,
        actor: ExactActorRef,
    ) -> Option<world_spatial::ChannelEntities> {
        self.observe_channel_entities(actor).await
    }

    /// The Channel's current door `WORLD_OBJECT_OVERLAY` (USE-WIRE-V1, #162 5868482467), for
    /// the join/resync snapshot. Channel-global, unlike `observe`: no actor is involved.
    async fn observe_world_object_overlay(
        &self,
    ) -> Option<oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry> {
        self.observe_object_overlay().await
    }

    /// One Channel-owner work item for one actor (`MOVE-RL-02` = 1): the pinned context, one
    /// direct lookup in the active generation's qualified cells (`MOVE-RL-03` = 1) and the
    /// owner's compare-commit. A blocked or out-of-room destination is `Blocked`; any stale,
    /// unpositioned or mismatched binding is `Rejected`. Nothing moves unless the step commits.
    ///
    /// SPEED-1: the step is paced by the step duration onto its destination (CONDITIONS-0 §4.2):
    /// the player's effective speed, with the actor's active `SPEED` condition delta for
    /// `session` at the owner's time, and the destination's ground speed from the §1.11 seam.
    /// A destination whose duration cannot be computed is refused before anything moves.
    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<Duration>) {
        self.tutorial_paced_step(actor, session, direction).await
    }

    async fn item_walk_route(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        target: AutoWalkTarget,
    ) -> Result<Option<AutoWalkRoute>, ()> {
        ComposedFreshAdmission::item_walk_route(self, actor, session, target).await
    }

    async fn item_walk_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        expected: crate::foundation::MovementPositionSnapshot,
        direction: StepDirection,
    ) -> (
        StepOutcome,
        Option<Duration>,
        Option<crate::foundation::MovementPositionSnapshot>,
    ) {
        ComposedFreshAdmission::item_walk_step(self, actor, session, expected, direction).await
    }

    async fn item_walk_valid(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        expected: crate::foundation::MovementPositionSnapshot,
    ) -> bool {
        let mut runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        !states.is_dead(actor)
            && !self.states.newhaven_travel.held(session)
            && runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            && runtime.borrow_movement_position().read(actor) == Ok(expected)
    }

    /// CHASE-1 (RANGED-0 §8): the owner's chase step (`attack::chase_step_now`).
    async fn chase_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(StepOutcome, Option<Duration>)> {
        self.newhaven_chase_step(actor, session).await
    }

    async fn yield_chase_to_client(&self, actor: ExactActorRef, session: GameSessionId) {
        self.chase_yield_to_client(actor, session).await;
    }

    async fn chase_search_waiting(&self, actor: ExactActorRef, session: GameSessionId) -> bool {
        self.chase_search_waiting_now(actor, session).await
    }

    async fn actor_step_deadline(&self, actor: ExactActorRef) -> Option<tokio::time::Instant> {
        self.actor_step_deadline_now(actor).await
    }

    async fn begin_actor_step(
        &self,
        actor: ExactActorRef,
        holder: &std::sync::Arc<()>,
    ) -> Option<tokio::time::Instant> {
        self.begin_actor_step_now(actor, holder).await
    }

    async fn record_actor_step_deadline(
        &self,
        actor: ExactActorRef,
        holder: &std::sync::Arc<()>,
        due: Option<tokio::time::Instant>,
    ) {
        self.record_actor_step_deadline_now(actor, holder, due)
            .await;
    }

    /// ITEM-USE-1c: one `USE` of a main backpack entry under capability 15.
    async fn use_item(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        item: item_view::ItemKey,
    ) -> UseOutcome {
        self.use_backpack_item(actor, command, item).await
    }

    async fn use_pick(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::world_object::UseIntent,
        source: Option<item_view::ItemKey>,
        target: Option<item_view::ItemKey>,
    ) -> Option<UseOutcome> {
        self.use_pick_impl(actor, command, intent, source, target)
            .await
    }

    async fn use_item_completion(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        item: item_view::ItemKey,
    ) -> Option<UseOutcome> {
        if let Some(outcome) = self.use_timed_item(actor, command, item).await {
            return outcome;
        }
        Some(self.use_item(actor, command, item).await)
    }

    async fn use_item_by_definition(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        definition: std::num::NonZeroU32,
    ) -> UseOutcome {
        self.use_item_definition(actor, command, definition).await
    }

    async fn use_craft(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::world_object::UseIntent,
        source: Option<item_view::ItemKey>,
        target: Option<item_view::ItemKey>,
    ) -> Option<UseOutcome> {
        self.use_cooking_conversion(actor, command, intent, source, target)
            .await
    }

    async fn use_rune(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::world_object::UseIntent,
        key: Option<item_view::ItemKey>,
        visible: bool,
    ) -> Option<UseOutcome> {
        self.use_legacy_rune(actor, command, intent, key, visible)
            .await
    }

    /// One `USE_INTENT` for the admitted actor against the one entry-room door (#162
    /// 5868482467, USE-WIRE-V1). The client names only the target placement and its expected
    /// revision; the server selects the unique bound transition
    /// (`LocalObjectRuntime::attempt_use`). Reach is same floor, Chebyshev distance <=1 from the
    /// door's own collision cell, no line-of-sight check. `runtime` is locked first, then
    /// `door`, the same fixed order `step` uses, so the two can never deadlock against each
    /// other; both stay locked for this whole decision so nothing else can move the acting actor
    /// or the door state in between.
    ///
    /// C2: a `USE` naming the injected entry chest is routed to [`Self::use_chest`] instead;
    /// every other placement takes the unchanged door path.
    async fn use_object(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> UseOutcome {
        self.use_world_object(actor, command, target).await
    }

    async fn observe_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        self.observe_actor_vitals(actor, game_session_id).await
    }

    /// One cast as one Channel-owner work item (`SPELL-RL-01` = 1), under the same runtime lock
    /// `step` and `use_object` take.
    async fn cast_spell(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
    ) -> actor_spell::SpellCastOutcome {
        self.cast_actor_spell(actor, game_session_id, command_id, intent)
            .await
    }

    async fn cast_spell_completion(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        intent: actor_spell::SpellCastIntent,
    ) -> Option<actor_spell::SpellCastOutcome> {
        self.complete_actor_spell(actor, game_session_id, command_id, intent)
            .await
    }

    /// Display contract §4: a read-only query of the facts, no Character fence and no Channel
    /// owner lock. More facts than catalogue keys prove an unknown key, so one more than the
    /// catalogue's size is read and the page build refuses it.
    async fn observe_achievement_notices(
        &self,
        account_id: [u8; 16],
    ) -> Option<oteryn_protocol_oteryn::achievement_notices::AchievementWatermark> {
        self.observe_account_achievement_notices(account_id).await
    }

    async fn account_achievements(
        &self,
        request: AccountAchievementsRequest,
    ) -> AccountAchievementsReply {
        self.query_account_achievements(request).await
    }

    async fn world_event(&self, command: connection::UseCommand) -> world_event::Reply {
        world_event::query(self, command).await
    }

    async fn highscores(
        &self,
        world: WorldId,
        session: GameSessionId,
        query: oteryn_protocol_oteryn::highscores::HighscoresQuery,
    ) -> highscores::HighscoresReply {
        highscores::answer_session_highscores(self.root, self.world_id, world, session, query).await
    }

    /// The periodic 1000 ms Serene evaluation of the admitted actor (SPELL-D8 §8.2), under the
    /// same runtime lock as a cast. DEATH-2: a dead actor is respawned on this cadence instead
    /// ([`Self::respawn_after_death`]), and its respawn vitals are the published delta.
    async fn tick_vitals(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<(u64, actor_spell::ActorVitals)> {
        self.tick_actor_vitals(actor, game_session_id).await
    }

    /// ATTACK-1b: one `ATTACK_TARGET_INTENT` as one Channel-owner work item, under the runtime
    /// lock; a dead or unbound actor takes no target.
    async fn attack_target(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        command_id: u64,
        target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    ) -> oteryn_protocol_oteryn::attack::AttackIntentDisposition {
        self.select_attack_target(actor, game_session_id, command_id, target)
            .await
    }

    async fn fight_modes(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        modes: oteryn_protocol_oteryn::attack::FightModes,
    ) -> oteryn_protocol_oteryn::attack::AttackIntentDisposition {
        self.set_fight_modes(actor, game_session_id, modes).await
    }

    async fn observe_combat_state(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::attack::ActorCombatState> {
        self.observe_actor_combat_state(actor, game_session_id)
            .await
    }

    /// ATTACK-1b: every 250 ms while a session has domain 10, so a 2000 ms swing is late by at
    /// most one refresh; the Serene cadence drains as well.
    async fn tick_combat(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::attack::ActorCombatState> {
        self.tick_actor_combat(actor, game_session_id).await
    }

    async fn lose_control(
        &self,
        admitted: AdmittedSession,
        wait: Duration,
        reset: Option<&crate::durability::world_reset::RetiringWorldReset>,
    ) -> ControlLossResult {
        self.lose_control_for_cause(admitted, wait, reset).await
    }

    async fn enter_input(&self) -> Option<input_gate::InputPermit<'_>> {
        self.states.input_gate.enter().await
    }

    async fn resume(
        &self,
        attempt: connection::ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        self.resume_session(attempt).await
    }

    /// The abandoned-session release follows the same fence, commit and settle steps as grace
    /// expiry (CHARM-DESC-FENCE-LEASE §3 item 1), under one transition token minted for it.
    async fn release_abandoned(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        self.release_abandoned_session(admitted).await
    }

    async fn reconcile_terminal(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        self.reconcile_terminal_session(admitted).await
    }

    async fn offline_training(
        &self,
        admitted: AdmittedSession,
        command: connection::UseCommand,
        target: item_view::ItemKey,
    ) -> Option<oteryn_protocol_oteryn::loop_training::TrainingResult> {
        self.offline_training(admitted, command, target).await
    }
    async fn exercise_training(
        &self,
        admitted: AdmittedSession,
        command: connection::UseCommand,
        intent: oteryn_protocol_oteryn::loop_training::ExerciseIntent,
        weapon: Option<item_view::ItemKey>,
        dummy: Option<item_view::ItemKey>,
    ) -> Option<oteryn_protocol_oteryn::loop_training::TrainingResult> {
        self.exercise_training(admitted, command, intent, weapon, dummy)
            .await
    }
    async fn training_activity(&self, admitted: AdmittedSession, kind: u32) -> bool {
        self.loop_training_activity(admitted, kind).await
    }
    async fn logout(&self, admitted: AdmittedSession) -> logout::LogoutReply {
        self.logout_session(admitted).await
    }

    async fn expire_control_loss(&self, admitted: AdmittedSession) -> GraceExpiryResult {
        self.expire_session_control_loss(admitted).await
    }

    async fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        self.admit_fresh_session(attempt).await
    }

    /// ITEM-MOVE-1 replay first: the receipt of this command 9's CommandRef in
    /// `game_item_transfer_receipts`, read under the cause lock. Without an item fence nothing can
    /// have committed for the session.
    async fn committed_item_move(
        &self,
        _actor: ExactActorRef,
        command: connection::UseCommand,
    ) -> Result<
        Option<crate::durability::item_transfer::CommittedItemTransfer>,
        crate::durability::item_transfer::ItemTransferError,
    > {
        self.read_committed_item_move(_actor, command).await
    }

    /// LOOK-2: examine a freshly observed item admitted by the session's handle table.
    async fn look_item(
        &self,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        fence: Option<CurrentCharacterItemFence>,
        views: &item_view::SessionItemView,
        target: ItemHandle,
    ) -> ItemLookResult {
        item_look::look_item(self, actor, game_session_id, fence, views, target).await
    }

    /// LOOK-2: examine a current visible actor incarnation.
    async fn look_creature(
        &self,
        actor: ExactActorRef,
        _game_session_id: GameSessionId,
        target: EntityRef,
    ) -> ItemLookResult {
        item_look::look_creature(self, actor, target).await
    }

    /// MAP-ITEM-REF-1 Part B: a bound corpse's durable contents for `USE` (domain 11).
    async fn observe_item_target(
        &self,
        actor: ExactActorRef,
        target: item_view::ItemKey,
    ) -> Option<item_view::ItemTargetObservation> {
        item_ref_admission::observe_item_target(self, actor, target).await
    }

    /// MAP-ITEM-REF-1 Part B: command 9's corpse-entry TRANSFER into the main backpack.
    async fn take_corpse_entry(
        &self,
        actor: ExactActorRef,
        command: connection::UseCommand,
        corpse: item_view::ItemKey,
        entry: item_view::ItemKey,
    ) -> Result<
        crate::durability::item_transfer::ItemTransferOutcome,
        crate::combat_pickup::GroundPickupError,
    > {
        item_ref_admission::take_corpse_entry(self, actor, command, corpse, entry).await
    }
}
