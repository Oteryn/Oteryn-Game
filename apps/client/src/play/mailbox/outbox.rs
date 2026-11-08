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
        self.activity_scope_events(&events);
        self.channel_list_events(&events);
        self.appearance_events(&events);
        self.pvp_expert_events(&events);
        self.inventory_events(&events);
        self.container_events(&events);
        let mut mailbox = lock(&self.0);
        crate::play::feature_0060_player_trade::protocol::reduce(
            &mut mailbox.player_trade,
            &events,
        );
        crate::play::feature_local_spell_action_bar::protocol::vitals_events(&mut mailbox, &events);
        mailbox.death_protection_events(&events);
        mailbox.dialogue_events(&events);
        mailbox.npc_shop_events(&events);
        for event in &events {
            if let SessionEvent::ContainerViews(update) = event {
                crate::play::feature_0230_browse_field::protocol::replace_views(
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
        crate::play::feature_0010_analyser::protocol::reduce(&mut mailbox, &events);
        crate::play::feature_0134_party_skull::protocol::reduce(
            &mut mailbox.properties.party_skull,
            &events,
        );
        crate::play::feature_0287_actor_spell_catalog::protocol::reduce(&mut mailbox, &events);
        crate::play::feature_0291_cast_chat::protocol::reduce(&mut mailbox, &events);
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
        crate::play::feature_0016_quest_log::protocol::reduce(&mut mailbox.quest, deltas);
        crate::play::feature_0240_chat_party_room::protocol::reset_if_replaced(&mut mailbox);
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
        if let Some(launcher) = &mailbox.coin_shop {
            launcher.end();
        }
        mailbox.arena.end();
        mailbox.discovery.end();
        mailbox.properties.end();
        mailbox.weapon_proficiency = Default::default();
        mailbox.imbue = Default::default();
        mailbox.analyser = Default::default();
        mailbox.cast_chat = Default::default();
        mailbox.cast_control = Default::default();
        mailbox.spell_catalog = Default::default();
        mailbox.spell_bar = Default::default();
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
        if let Some(launcher) = &mailbox.coin_shop {
            launcher.end();
        }
        mailbox.arena.end();
        mailbox.properties.end();
        mailbox.weapon_proficiency = Default::default();
        mailbox.imbue = Default::default();
        mailbox.analyser = Default::default();
        mailbox.cast_chat = Default::default();
        mailbox.cast_control = Default::default();
        mailbox.spell_catalog = Default::default();
        mailbox.spell_bar = Default::default();
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
