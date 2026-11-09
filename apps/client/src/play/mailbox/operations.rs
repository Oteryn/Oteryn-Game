//! Session outbox delivery and command lifecycle; state remains owned by the mailbox.
use super::*;
impl Outbox {
    pub(in crate::play) fn stepped(&self, outcome: StepOutcome) {
        let mut mailbox = lock(&self.0);
        mailbox.movement_turn = None;
        let older = std::mem::take(&mut mailbox.pushed);
        mailbox.before.absorb(older);
        mailbox.check_server_messages();
        mailbox.stepped = Some(Box::new(outcome));
    }

    /// Delivers validated session events to the native view (also used by the headless harness).
    pub fn push(&self, events: Vec<SessionEvent>) {
        self.console_events(&events);
        self.activity_scope_events(&events);
        self.channel_list_events(&events);
        self.appearance_events(&events);
        self.pvp_expert_events(&events);
        self.inventory_events(&events);
        self.container_events(&events);
        let mut mailbox = lock(&self.0);
        super::super::feature_0060_player_trade::protocol::reduce(
            &mut mailbox.player_trade,
            &events,
        );
        super::super::feature_local_spell_action_bar::protocol::vitals_events(
            &mut mailbox,
            &events,
        );
        mailbox.death_protection_events(&events);
        mailbox.dialogue_events(&events);
        mailbox.npc_shop_events(&events);
        for event in &events {
            if let SessionEvent::ContainerViews(update) = event {
                super::super::feature_0230_browse_field::protocol::replace_views(
                    &mut mailbox.browse_field,
                    update.value.clone(),
                );
            }
            if let SessionEvent::PlayerTyping(update) = event {
                oteryn_client_ui::player_typing::protocol::reduce(
                    &mut mailbox.player_typing,
                    update.value,
                );
            }
        }
        super::super::feature_0010_analyser::protocol::reduce(&mut mailbox, &events);
        super::super::feature_0134_party_skull::protocol::reduce(
            &mut mailbox.properties.party_skull,
            &events,
        );
        super::super::feature_0287_actor_spell_catalog::protocol::reduce(&mut mailbox, &events);
        super::super::feature_0291_cast_chat::protocol::reduce(&mut mailbox, &events);
        mailbox.coalesce_events(events);
        mailbox.check_server_messages();
        let notices = std::mem::take(&mut mailbox.pushed.achievements);
        mailbox.reduce_achievement_notices(notices);
        for text in std::mem::take(&mut mailbox.pushed.notices.tutorial) {
            mailbox
                .notices
                .apply(oteryn_client_runtime::tutorial_notices::Notice::Tutorial(
                    text,
                ));
        }
        if let Some(minutes) = mailbox.pushed.notices.save_minutes.take() {
            mailbox
                .notices
                .apply(oteryn_client_runtime::tutorial_notices::Notice::ServerSave { minutes });
        }
        let deltas = std::mem::take(&mut mailbox.pushed.quests);
        super::super::feature_0016_quest_log::protocol::reduce(&mut mailbox.quest, deltas);
        super::super::feature_0240_chat_party_room::protocol::reset_if_replaced(&mut mailbox);
        for line in std::mem::take(&mut mailbox.pushed.chat_lines) {
            oteryn_client_ui::party::push_feedback(&mut mailbox.party, &line);
            oteryn_client_ui::chat_party_room::push_line(&mut mailbox.chat_party_room, line);
        }
        if let Some(rooms) = mailbox.pushed.chat_rooms.take() {
            oteryn_client_ui::chat_party_room::replace_rooms(&mut mailbox.chat_party_room, rooms);
        }
        if let Some(presence) = mailbox.pushed.vip_presence.take() {
            oteryn_client_ui::vip_presence::replace(&mut mailbox.vip_presence, presence);
        }
        if let Some(position) = mailbox.pushed.own
            && let (Ok(x), Ok(y), Ok(floor)) = (
                u16::try_from(position.x),
                u16::try_from(position.y),
                i8::try_from(position.floor),
            )
        {
            mailbox.hireling.target = Some((x, y, floor));
        }
    }

    pub(in crate::play) fn entities(&self, entities: Vec<WorldSpatialEntity>) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::player_typing::protocol::retain_visible(
            &mut mailbox.player_typing,
            &entities,
        );
        mailbox.pushed.entities = Some(entities);
    }

    pub(in crate::play) fn logout_refused(&self, result: Option<LogoutResult>) {
        lock(&self.0).logout_refused = Some(result);
    }

    /// The first end wins.
    pub(in crate::play) fn end(&self, class: PublicClass) {
        self.inventory_end();
        self.container_end();
        let mut mailbox = lock(&self.0);
        mailbox.end_custom_hotkeys();
        if let Some(launcher) = &mailbox.coin_shop {
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
        mailbox.achievement_notices = Default::default();
        mailbox.achievements = Default::default();
        mailbox.highscores = Default::default();
        mailbox.highscores_shutdown = true;
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
        self.inventory_end();
        self.container_end();
        let mut mailbox = lock(&self.0);
        mailbox.end_custom_hotkeys();
        if let Some(launcher) = &mailbox.coin_shop {
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
        mailbox.achievement_notices = Default::default();
        mailbox.achievements = Default::default();
        mailbox.highscores = Default::default();
        mailbox.highscores_shutdown = true;
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
    pub(in crate::play) commands: UnboundedSender<PlayCommand>,
    pub(in crate::play) mailbox: SharedMailbox,
}

impl PlayLink {
    pub fn request(&self, direction: StepDir) {
        let mailbox = lock(&self.mailbox);
        if mailbox.world_input_blocked() {
            return;
        }
        // A closed task has already reported or will report `Ended`.
        let _ = self.commands.send(PlayCommand::Step(direction));
    }

    pub fn request_combat(&self, command: CombatCommand) {
        let mailbox = lock(&self.mailbox);
        if !mailbox.world_input_blocked() {
            let _ = self.commands.send(PlayCommand::Combat(command));
        }
    }

    pub fn request_logout(&self) {
        let _ = self.commands.send(PlayCommand::Logout);
    }

    /// The next event: pushes older than the step outcome, the outcome, coalesced pushes, then end.
    pub fn poll(&self) -> Option<PlayEvent> {
        #[cfg(windows)]
        super::super::feature_0148_wheel_allocation::view::poll_gems(&self.mailbox, &self.commands);
        #[cfg(windows)]
        super::super::feature_0152_item_forge::view::poll_resource_dialog(
            &self.mailbox,
            &self.commands,
        );
        #[cfg(windows)]
        super::super::feature_0081_npc_shop::view::open_window(&self.mailbox, &self.commands);
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
    (
        PlayLink {
            commands,
            mailbox: Arc::clone(&mailbox),
        },
        command_rx,
        Outbox(mailbox),
    )
}

impl Drop for PlayLink {
    fn drop(&mut self) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.end_custom_hotkeys();
        if let Some(launcher) = &mailbox.coin_shop {
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
