//! Session outbox delivery and command lifecycle; state remains owned by the mailbox.
use super::*;
impl Outbox {
    pub(in crate::play) fn session_cancelled(&self) -> bool {
        // Only owner disposal sets this permanent dialog-lifetime flag. A transport end
        // can reconnect on the same mailbox, so ordinary end state is not cancellation.
        lock(&self.0).player_typing_shutdown
    }

    pub(in crate::play) fn stepped(&self, mut outcome: StepOutcome) {
        let mut mailbox = lock(&self.0);
        if let Some(delta) = &mut outcome.world_spatial_delta {
            mailbox.wire.observe_wire(delta.value.actor_position);
            delta.value.actor_position = mailbox.wire_frame().to_native(delta.value.actor_position);
        }
        mailbox.movement_turn = None;
        let older = std::mem::take(&mut mailbox.pushed);
        mailbox.before.absorb(older);
        mailbox.check_server_messages();
        mailbox.stepped = Some(Box::new(outcome));
    }

    /// Delivers validated session events to the native view (also used by the headless harness).
    pub fn push(&self, events: Vec<SessionEvent>) {
        lock(&self.0).push_events(events);
    }
}

impl Mailbox {
    /// `entities` are as the server sent them; the shell gets them in the native frame.
    pub(in crate::play) fn entities(&mut self, entities: Vec<WorldSpatialEntity>) {
        oteryn_client_ui::player_typing::protocol::retain_visible(
            &mut self.player_typing,
            &entities,
        );
        let native = self.native_entities(&entities);
        self.pushed.entities = Some(native);
        self.entities_wire = entities;
    }

    pub(in crate::play) fn push_events(&mut self, mut events: Vec<SessionEvent>) {
        let received_at = tokio::time::Instant::now();
        if let Some(index) = events
            .iter()
            .rposition(|event| matches!(event, SessionEvent::ActivityScope(_)))
        {
            events.drain(..index);
        }
        // A full replacement puts its snapshot events before the domain records.
        // The join MOTD shares the replacement sequence and does not advance a revision.
        let replacement = events.iter().enumerate().rev().find_map(|(index, event)| {
            if let SessionEvent::GatedSnapshotReplaced(delta) = event
                && delta.value.domain_id
                    == oteryn_session::activity_scope::STATE_DOMAIN_ACTIVITY_SCOPE
            {
                Some((index, delta.server_sequence))
            } else {
                None
            }
        });
        if let Some((index, sequence)) = replacement {
            events
                .extract_if(..index, |event| {
                    if let SessionEvent::ServerMessage(delta) = event {
                        delta.server_sequence != sequence
                            || delta.base_revision != delta.new_revision
                    } else {
                        // Presentation cues are transient and never part of a join snapshot.
                        matches!(event, SessionEvent::Presentation(_))
                    }
                })
                .for_each(drop);
        }
        self.console_events(&events);
        self.activity_scope_events(&events);
        self.channel_list_events(&events);
        self.appearance_events(&events);
        self.pvp_expert_events(&events);
        self.cyclopedia_discovery_events(&events);
        self.inventory_events(&events);
        self.container_events(&events);
        super::super::feature_0060_player_trade::protocol::reduce(&mut self.player_trade, &events);
        super::super::feature_local_spell_action_bar::protocol::vitals_events(self, &events);
        self.death_protection_events(&events);
        self.dialogue_events(&events);
        self.npc_shop_events(&events);
        for event in &events {
            if let SessionEvent::GatedSnapshotReplaced(delta) = event
                && delta.value.domain_id == 1
            {
                self.pushed.motion_reset = true;
            }
            if let SessionEvent::ContainerViews(update) = event {
                super::super::feature_0230_browse_field::protocol::replace_views(
                    &mut self.browse_field,
                    update.value.clone(),
                );
            }
            if let SessionEvent::PlayerTyping(update) = event {
                oteryn_client_ui::player_typing::protocol::reduce(
                    &mut self.player_typing,
                    update.value,
                );
            }
        }
        super::super::feature_0010_analyser::protocol::reduce(self, &events);
        super::super::feature_0134_party_skull::protocol::reduce(
            &mut self.properties.party_skull,
            &events,
        );
        super::super::feature_0287_actor_spell_catalog::protocol::reduce(self, &events);
        super::super::feature_0291_cast_chat::protocol::reduce(self, &events);
        self.mount_actor_published(&events);
        self.coalesce_events(events);
        self.check_server_messages();
        let notices = std::mem::take(&mut self.pushed.achievements);
        self.reduce_achievement_notices(notices);
        for text in std::mem::take(&mut self.pushed.notices.tutorial) {
            // The server's walk refusal (USE-AUTOWALK-1 canonical text).
            if text == "There is no way." {
                oteryn_client_ui::target_move::no_way(&mut self.target_walk);
            }
            self.notices
                .apply(oteryn_client_runtime::tutorial_notices::Notice::Tutorial(
                    text,
                ));
        }
        if let Some(minutes) = self.pushed.notices.save_minutes.take() {
            self.notices
                .apply(oteryn_client_runtime::tutorial_notices::Notice::ServerSave { minutes });
        }
        let deltas = std::mem::take(&mut self.pushed.quests);
        super::super::feature_0016_quest_log::protocol::reduce(&mut self.quest, deltas);
        super::super::feature_0240_chat_party_room::protocol::reset_if_replaced(self);
        for line in std::mem::take(&mut self.pushed.chat_lines) {
            oteryn_client_ui::party::push_feedback(&mut self.party, &line);
            oteryn_client_ui::chat_party_room::push_line(&mut self.chat_party_room, line);
        }
        if let Some(rooms) = self.pushed.chat_rooms.take() {
            oteryn_client_ui::chat_party_room::replace_rooms(&mut self.chat_party_room, rooms);
        }
        if let Some(presence) = self.pushed.vip_presence.take() {
            if self.social_channel_selected {
                let notices = oteryn_client_ui::social_channel::presence_notices(
                    &self.party.channel_names,
                    &self.vip_presence.presence,
                    &presence,
                );
                for notice in notices {
                    self.vip_channel_notice_serial += 1;
                    let serial = self.vip_channel_notice_serial;
                    self.vip_channel_notices.push((serial, notice));
                }
            }
            self.vip_presence_confirmed_at = Some(received_at);
            oteryn_client_ui::vip_presence::replace(&mut self.vip_presence, presence);
        }
        if let Some(position) = self.pushed.own
            && let (Ok(x), Ok(y), Ok(floor)) = (
                u16::try_from(position.x),
                u16::try_from(position.y),
                i8::try_from(position.floor),
            )
        {
            self.hireling.target = Some((x, y, floor));
        }
    }
}

impl Outbox {
    pub(in crate::play) fn entities(&self, entities: Vec<WorldSpatialEntity>) {
        lock(&self.0).entities(entities);
    }

    pub(in crate::play) fn logout_refused(&self, result: Option<LogoutResult>) {
        lock(&self.0).logout_refused = Some(result);
    }

    /// The first end wins.
    pub(in crate::play) fn end(&self, class: PublicClass) {
        self.market_details_end();
        self.performance_reset();
        self.inventory_end();
        self.container_end();
        let delivery = lock(&self.0).connection.browser_delivery.clone();
        let _delivery = delivery.lock().unwrap_or_else(PoisonError::into_inner);
        let mut mailbox = lock(&self.0);
        mailbox.end_custom_hotkeys();
        if let Some(launcher) = &mailbox.connection.coin_shop {
            launcher.end();
        }
        mailbox.arena.end();
        mailbox.discovery.end();
        mailbox.properties.end();
        mailbox.task_board = Default::default();
        mailbox.premium_game = Default::default();
        mailbox.end_wheel_visuals();
        mailbox.item_forge.resource_dialog.reset();
        mailbox.weapon_proficiency = Default::default();
        mailbox.imbue = Default::default();
        mailbox.analyser = Default::default();
        mailbox.cast_chat = Default::default();
        mailbox.cast_control = Default::default();
        mailbox.spellbook_details = Default::default();
        mailbox.server_info = Default::default();
        mailbox.quest_info_revision = None;
        mailbox.spell_catalog = Default::default();
        mailbox.hud_resource = Default::default();
        mailbox.spell_bar = Default::default();
        mailbox.bar_actions = Default::default();
        mailbox.boss_encounter_end();
        mailbox.player_trade = Default::default();
        mailbox.battle_end();
        mailbox.stash_end();
        mailbox.depot = Default::default();
        mailbox.depot_shutdown = true;
        mailbox.imbue_shutdown = true;
        #[cfg(bevy_shell)]
        mailbox.achievement_banners.clear();
        mailbox.achievement_notices = Default::default();
        mailbox.achievements = Default::default();
        mailbox.highscores = Default::default();
        mailbox.highscores_shutdown = true;
        mailbox.sanctions = Default::default();
        mailbox.tutorial_vocation = Default::default();
        mailbox.chat.guild = Default::default();
        mailbox.chat.guild_selected = false;
        mailbox.chat.guild_open = false;
        mailbox.npc_shop = Default::default();
        mailbox.dialogue = Default::default();
        mailbox.dialogue_shutdown = true;
        mailbox.chat_public.speech_selected = false;
        mailbox.chat_public.pending = false;
        mailbox.animus = Default::default();
        mailbox.loyalty_skills = Default::default();
        mailbox.condition_status = Default::default();
        mailbox.mechanics_status = Default::default();
        mailbox.rest_status = Default::default();
        mailbox.battle_instinct_status = Default::default();
        mailbox.skill_progress = Default::default();
        mailbox.party_effects = Default::default();
        mailbox.potion_durations = Default::default();
        mailbox.ended.get_or_insert(PlayEvent::Ended(class));
    }

    pub(in crate::play) fn logged_out(&self) {
        self.market_details_end();
        self.performance_reset();
        self.inventory_end();
        self.container_end();
        let delivery = lock(&self.0).connection.browser_delivery.clone();
        let _delivery = delivery.lock().unwrap_or_else(PoisonError::into_inner);
        let mut mailbox = lock(&self.0);
        mailbox.end_custom_hotkeys();
        if let Some(launcher) = &mailbox.connection.coin_shop {
            launcher.end();
        }
        mailbox.arena.end();
        mailbox.properties.end();
        mailbox.task_board = Default::default();
        mailbox.premium_game = Default::default();
        mailbox.end_wheel_visuals();
        mailbox.item_forge.resource_dialog.reset();
        mailbox.weapon_proficiency = Default::default();
        mailbox.imbue = Default::default();
        mailbox.analyser = Default::default();
        mailbox.cast_chat = Default::default();
        mailbox.cast_control = Default::default();
        mailbox.spellbook_details = Default::default();
        mailbox.server_info = Default::default();
        mailbox.quest_info_revision = None;
        mailbox.spell_catalog = Default::default();
        mailbox.hud_resource = Default::default();
        mailbox.spell_bar = Default::default();
        mailbox.bar_actions = Default::default();
        mailbox.boss_encounter_end();
        mailbox.player_trade = Default::default();
        mailbox.battle_end();
        mailbox.stash_end();
        mailbox.depot = Default::default();
        mailbox.depot_shutdown = true;
        mailbox.imbue_shutdown = true;
        #[cfg(bevy_shell)]
        mailbox.achievement_banners.clear();
        mailbox.achievement_notices = Default::default();
        mailbox.achievements = Default::default();
        mailbox.highscores = Default::default();
        mailbox.highscores_shutdown = true;
        mailbox.sanctions = Default::default();
        mailbox.tutorial_vocation = Default::default();
        mailbox.chat.guild = Default::default();
        mailbox.chat.guild_selected = false;
        mailbox.chat.guild_open = false;
        mailbox.npc_shop = Default::default();
        mailbox.dialogue = Default::default();
        mailbox.dialogue_shutdown = true;
        mailbox.chat_public.speech_selected = false;
        mailbox.chat_public.pending = false;
        mailbox.animus = Default::default();
        mailbox.loyalty_skills = Default::default();
        mailbox.condition_status = Default::default();
        mailbox.mechanics_status = Default::default();
        mailbox.rest_status = Default::default();
        mailbox.battle_instinct_status = Default::default();
        mailbox.skill_progress = Default::default();
        mailbox.party_effects = Default::default();
        mailbox.potion_durations = Default::default();
        mailbox.ended.get_or_insert(PlayEvent::LoggedOut);
    }
}

/// The shell's end of the session task: steps out, a bounded mailbox in, neither ever blocking.
#[derive(Debug)]
pub struct PlayLink {
    pub(in crate::play) owns_session: bool,
    pub(in crate::play) commands: UnboundedSender<PlayCommand>,
    pub(in crate::play) mailbox: SharedMailbox,
}

impl PlayLink {
    /// Select a shared desktop frontend before presenting any admitted game UI.
    /// Stored in the connection mailbox so all non-owning links use the same presenter.
    pub fn use_shared_frontend(&self) {
        lock(&self.mailbox).shared_frontend = true;
    }

    #[must_use]
    pub fn uses_shared_frontend(&self) -> bool {
        lock(&self.mailbox).shared_frontend
    }

    /// Shares the admitted session's byte counters with every link of this connection.
    pub fn set_traffic(&self, traffic: Arc<crate::traffic::Traffic>) {
        lock(&self.mailbox).traffic = Some(traffic);
    }

    /// `(sent, received)` bytes of the admitted session, for diagnostics.
    #[must_use]
    pub fn traffic_bytes(&self) -> Option<(u64, u64)> {
        lock(&self.mailbox)
            .traffic
            .as_ref()
            .map(|traffic| traffic.bytes())
    }

    pub fn request(&self, direction: StepDir) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.world_input_blocked() {
            return;
        }
        // A manual step ends a click or minimap walk.
        oteryn_client_ui::target_move::cancel(&mut mailbox.target_walk);
        // A closed task has already reported or will report `Ended`.
        let _ = self.commands.send(PlayCommand::Step(direction));
    }

    pub fn request_combat(&self, command: CombatCommand) {
        let mut mailbox = lock(&self.mailbox);
        if !mailbox.world_input_blocked() {
            mailbox.cancel_walk_for_combat(command);
            let _ = self.commands.send(PlayCommand::Combat(command));
        }
    }

    pub fn request_logout(&self) {
        let mut mailbox = lock(&self.mailbox);
        oteryn_client_ui::target_move::cancel(&mut mailbox.target_walk);
        let _ = self.commands.send(PlayCommand::Logout);
    }

    /// The next event: pushes older than the step outcome, the outcome, coalesced pushes, then end.
    pub fn poll(&self) -> Option<PlayEvent> {
        self.poll_weekly_results();
        let mut mailbox = lock(&self.mailbox);
        if mailbox.before != Pushed::default() {
            return Some(PlayEvent::Pushed(Box::new(std::mem::take(
                &mut mailbox.before,
            ))));
        }
        if let Some(outcome) = mailbox.stepped.take() {
            return Some(PlayEvent::Stepped(outcome));
        }
        if mailbox.pushed != Pushed::default() {
            return Some(PlayEvent::Pushed(Box::new(std::mem::take(
                &mut mailbox.pushed,
            ))));
        }
        if let Some(feedback) = mailbox.action_feedback.take() {
            return Some(PlayEvent::ActionFeedback(feedback));
        }
        if let Some(report) = mailbox.combat.pop_front() {
            return Some(PlayEvent::Combat(report));
        }
        if let Some(result) = mailbox.logout_refused.take() {
            return Some(PlayEvent::LogoutRefused(result));
        }
        mailbox.ended.take()
    }
}

/// The channel pair between the shell and `run_session`.
pub fn play_channel() -> (PlayLink, UnboundedReceiver<PlayCommand>, Outbox) {
    let (commands, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let mailbox = SharedMailbox::default();
    let link = PlayLink {
        owns_session: true,
        commands,
        mailbox: Arc::clone(&mailbox),
    };
    if let Some(directory) = crate::support_delivery::directory() {
        link.queue_support_at(&directory);
    }
    (link, command_rx, Outbox(mailbox))
}

impl Drop for PlayLink {
    fn drop(&mut self) {
        if !self.owns_session {
            return;
        }
        self.channel_switch_close();
        let delivery = lock(&self.mailbox).connection.browser_delivery.clone();
        let _delivery = delivery.lock().unwrap_or_else(PoisonError::into_inner);
        let mut mailbox = lock(&self.mailbox);
        mailbox.end_custom_hotkeys();
        if let Some(launcher) = &mailbox.connection.coin_shop {
            launcher.end();
        }
        mailbox.properties.end();
        mailbox.task_board = Default::default();
        mailbox.premium_game = Default::default();
        mailbox.end_wheel_visuals();
        mailbox.item_forge.resource_dialog.reset();
        mailbox.weapon_proficiency = Default::default();
        mailbox.imbue = Default::default();
        mailbox.analyser = Default::default();
        mailbox.cast_chat = Default::default();
        mailbox.cast_control = Default::default();
        mailbox.spellbook_details = Default::default();
        mailbox.server_info = Default::default();
        mailbox.quest_info_revision = None;
        mailbox.spell_catalog = Default::default();
        mailbox.hud_resource = Default::default();
        mailbox.spell_bar = Default::default();
        mailbox.bar_actions = Default::default();
        mailbox.boss_encounter_end();
        mailbox.player_trade = Default::default();
        mailbox.world_event_shutdown = true;
        mailbox.highscores_shutdown = true;
        mailbox.npc_shop = Default::default();
        mailbox.dialogue = Default::default();
        mailbox.dialogue_shutdown = true;
        mailbox.vip.open = false;
        mailbox.inventory_shutdown = true;
        mailbox.containers.end();
        mailbox.browse_field.open = false;
        mailbox.item_forge.open = false;
        mailbox.quest_shutdown = true;
        mailbox.wheel_shutdown = true;
        mailbox.prey_shutdown = true;
        mailbox.depot_shutdown = true;
        mailbox.imbue_shutdown = true;
        mailbox.market_shutdown = true;
        mailbox.hireling_shutdown = true;
        // An armed summon-location pick and its target end with the session, so the next
        // map click is not taken as a summon location.
        mailbox.hireling_picking = false;
        mailbox.hireling.feedback = None;
        mailbox.hireling.target = None;
        mailbox.cyclopedia_character_shutdown = true;
        // CL26s1
        mailbox.chat_public_shutdown = true;
        // CL26s2: stop the player-typing native panel with the session.
        mailbox.player_typing_shutdown = true;
        // CL26s3
        mailbox.daily_reward_shutdown = true;
        // CL26s4
        mailbox.boss_encounter_shutdown = true;
        mailbox.boss_rewards_shutdown = true;
    }
}

#[cfg(test)]
#[path = "window_lifetime_tests.rs"]
mod window_lifetime_tests;
