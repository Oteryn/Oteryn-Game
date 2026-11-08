//! Bounded, non-blocking mailbox between the session task and the shell.

use super::{ServerMessageInbox, StepDir};
use crate::combat_input::{CombatCommand, CombatReport, PlayCommand};
use oteryn_platform_client::native_login::PublicClass;
use oteryn_session::{
    ActorPosition, LogoutResult, SessionEvent, StepOutcome, WorldObjectOverlayEntry,
    WorldSpatialEntity,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

mod activity_scope;

/// What the session task reports to the shell.
#[derive(Debug)]
pub enum PlayEvent {
    ActionFeedback(Option<&'static str>),
    Stepped(Box<StepOutcome>),
    /// Deltas pushed outside any step, coalesced since the last poll.
    Pushed(Box<Pushed>),
    /// The server's answer to a combat command.
    Combat(CombatReport),
    /// The server refused a logout (`None`: before its logout authority); play goes on.
    LogoutRefused(Option<LogoutResult>),
    /// The server committed the logout and closed the session.
    LoggedOut,
    /// The session ended; return to login with this public class.
    Ended(PublicClass),
}

/// Pushed deltas coalesced to latest state, so an undrained shell holds a bounded amount.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Pushed {
    /// A committed activity switch invalidates the source view's queued actions.
    pub scope_changed: bool,
    pub served_channel: Option<Option<oteryn_client_runtime::channel_list::ServedChannel>>,
    pub world_weather: Option<oteryn_session::world_weather::WorldWeather>,
    pub world_clock: Option<oteryn_session::world_clock::WorldClock>,
    pub achievements: Vec<oteryn_client_ui::achievement_notices::AchievementEarned>,
    pub sounds: Vec<super::SoundCue>,
    pub visuals: Vec<oteryn_session::PresentationEventKind>,
    pub notices: Box<oteryn_client_ui::tutorial_notices::NoticePanel>,
    /// Latest complete live map window, reduced before publication.
    pub map: Option<Box<oteryn_session::world_map::WorldMapViewSnapshot>>,
    /// Transient domain-16 updates, reduced before publication to the shell.
    pub quests: Vec<oteryn_client_runtime::quest_log::QuestUpdate>,
    /// Transient domain-57 messages retained in order; they are never coalesced to one value.
    pub server_messages: ServerMessageInbox,
    /// The latest own-actor position pushed by domain 1.
    pub own: Option<ActorPosition>,
    /// The latest full list of the other visible entities, when it changed.
    pub entities: Option<Vec<WorldSpatialEntity>>,
    pub appearances: Option<Vec<oteryn_session::VisibleAppearance>>,
    /// The latest domain-2 entry per placement, in first-pushed order of placement.
    pub markers: Vec<WorldObjectOverlayEntry>,
    pub markers_reset: bool,
    pub chat_reset: bool,
    pub chat_lines: Vec<oteryn_session::ChatLine>,
    pub chat_rooms: Option<oteryn_session::ChatRoomSet>,
    pub vip_presence: Option<oteryn_session::VipPresenceBatch>,
}

impl Pushed {
    /// Folds `newer` over `self`: its position and entries win.
    fn absorb(&mut self, newer: Self) {
        self.scope_changed |= newer.scope_changed;
        super::feature_0023_presentation::protocol::absorb_visuals(
            &mut self.visuals,
            newer.visuals,
        );
        super::feature_0023_presentation::protocol::absorb_sounds(&mut self.sounds, newer.sounds);
        for text in newer.notices.tutorial {
            self.notices
                .apply(oteryn_client_runtime::tutorial_notices::Notice::Tutorial(
                    text,
                ));
        }
        self.notices.save_minutes = newer.notices.save_minutes.or(self.notices.save_minutes);
        self.served_channel = newer.served_channel.or(self.served_channel);
        self.world_clock = newer.world_clock.or(self.world_clock);
        self.world_weather = newer.world_weather.or(self.world_weather);
        self.map = newer.map.or(self.map.take());
        self.own = newer.own.or(self.own);
        self.entities = newer.entities.or(self.entities.take());
        self.appearances = newer.appearances.or(self.appearances.take());
        self.server_messages.append(newer.server_messages);
        self.achievements.extend(newer.achievements);
        if newer.chat_reset {
            self.chat_lines.clear();
            self.chat_reset = true;
        }
        self.chat_lines.extend(newer.chat_lines);
        self.chat_rooms = newer.chat_rooms.or(self.chat_rooms);
        self.vip_presence = newer.vip_presence.or(self.vip_presence.take());
        if newer.markers_reset {
            self.markers.clear();
            self.markers_reset = true;
        }
        self.quests.extend(newer.quests);
        for entry in newer.markers {
            self.set_marker(entry);
        }
    }
}

/// Most distinct placements held while the shell is not draining; a later new placement is
/// dropped until the next poll, an update of a held one is still kept.
pub(super) const MAX_PUSHED_MARKERS: usize = 1024;

#[derive(Debug, Default)]
pub(super) struct Mailbox {
    pub(super) battle_lists: super::feature_0017_attack::view::BattleListsState,
    pub(super) target_move_selected: bool,
    pub(super) movement_turn: Option<StepDir>,
    pub(super) creature_push_selected: bool,
    pub(super) movement_feedback: Option<&'static str>,
    pub(super) analyser: oteryn_client_ui::analyser::Panel,
    pub(super) arena: oteryn_client_ui::arena::ArenaPanel,
    pub(super) cast_chat: oteryn_client_ui::cast_chat::Panel,
    pub(super) cast_control: oteryn_client_ui::cast_control::Panel,
    pub(super) spellbook_details: super::feature_0287_actor_spell_catalog::view::SpellbookState,
    pub(super) spell_catalog: oteryn_client_ui::spell_catalog::Panel,
    pub(super) spell_bar: oteryn_client_ui::spell_action_bar::ActionBarPanel,
    pub(super) served_channel: Option<oteryn_client_runtime::channel_list::ServedChannel>,
    pub(super) discovery: oteryn_client_ui::quick_loot::Discovery,
    pub(super) properties: super::feature_0115_mail::protocol::Panels,
    pub(super) containers: oteryn_client_ui::container_tree::ContainerPanel,
    pub(super) world_context: oteryn_client_ui::world_interaction::menu::State,
    pub(super) inventory: oteryn_client_ui::inventory::InventoryPanel,
    pub(super) inventory_shutdown: bool,
    pub(super) item_movement: oteryn_client_ui::inventory::movement::Movement,
    pub(super) item_count_selected: bool,
    pub(super) equipment_selected: bool,
    #[cfg(windows)]
    pub(super) item_drag_native: super::feature_0004_item_view_move::view::DragNative,
    pub(super) battle: oteryn_client_ui::battle::BattlePanel,
    pub(super) death: super::feature_local_death_respawn::protocol::DeathState,
    pub(super) death_reset_commands: bool,
    pub(super) activity_reset_commands: bool,
    pub(super) depot: oteryn_client_ui::depot::DepotPanel,
    pub(super) depot_shutdown: bool,
    pub(super) target_walk: oteryn_client_ui::target_move::WalkState,
    pub(super) achievement_notices: oteryn_client_ui::achievement_notices::AchievementNoticesPanel,
    pub(super) vitals: Option<oteryn_session::ActorVitals>,
    pub(super) notices: oteryn_client_ui::tutorial_notices::NoticePanel,
    pub(super) boss_encounter: oteryn_client_ui::boss_encounter::BossEncounterPanel,
    pub(super) boss_encounter_shutdown: bool,
    pub(super) map: Option<Box<oteryn_session::world_map::WorldMapViewSnapshot>>,
    pub(super) boss_difficulty: oteryn_client_ui::boss_difficulty::BossDifficultyPanel,
    pub(super) browse_field: oteryn_client_ui::browse_field::BrowseFieldPanel,
    pub(super) chat_party_room: oteryn_client_ui::chat_party_room::ChatPartyRoomPanel,
    pub(super) cyclopedia_character:
        oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel,
    pub(super) cyclopedia_character_shutdown: bool,
    pub(super) cyclopedia_tracker: oteryn_client_ui::cyclopedia_tracker::CyclopediaTrackerPanel,
    pub(super) world_event: oteryn_client_ui::world_event::WorldEventPanel,
    pub(super) world_event_shutdown: bool,
    pub(super) daily_boost: oteryn_client_ui::daily_boost::DailyBoostPanel,
    pub(super) highscores: oteryn_client_ui::highscores::HighscoresPanel,
    pub(super) highscores_shutdown: bool,
    pub(super) depot_search: oteryn_client_ui::depot_search::DepotSearchPanel,
    pub(super) hotkey_equip: oteryn_client_ui::hotkey_equip::HotkeyEquipPanel,
    pub(super) potion_durations: oteryn_client_ui::potion_durations::PotionDurationsPanel,
    pub(super) item_forge: oteryn_client_ui::item_forge::ItemForgePanel,
    pub(super) store: oteryn_client_ui::store::StorePanel,
    pub(super) coin_shop: Option<Arc<crate::coin_shop::launcher::Launcher>>,
    pub(super) vip_presence: oteryn_client_ui::vip_presence::VipPresencePanel,
    pub(super) weapon_proficiency: oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel,

    pub(super) achievements: oteryn_client_ui::account_achievements::AchievementsPanel,
    pub(super) animus: oteryn_client_ui::animus_mastery::AnimusPanel,
    pub(super) wheel: oteryn_client_ui::wheel_allocation::WheelPanel,
    pub(super) wheel_shutdown: bool,
    pub(super) exiva: oteryn_client_ui::exiva_settings::ExivaPanel,
    pub(super) mechanics_status: oteryn_client_ui::mechanics_status::MechanicsStatusPanel,
    pub(super) condition_status: oteryn_client_ui::condition_status::ConditionStatusPanel,
    pub(super) rest_status: oteryn_client_ui::rest_status::RestStatusPanel,
    pub(super) battle_instinct_status:
        oteryn_client_ui::battle_instinct_status::BattleInstinctStatusPanel,
    pub(super) skill_progress: oteryn_client_ui::skill_progress::SkillProgressPanel,
    pub(super) party_effects: oteryn_client_ui::party_effects::PartyEffectsPanel,
    pub(super) loyalty_skills: oteryn_client_ui::loyalty_skills::LoyaltySkillsPanel,
    pub(super) skills_layout: super::feature_0274_loyalty_skills::view::SkillsLayoutState,
    pub(super) quest: oteryn_client_ui::quest_log::QuestPanel,
    pub(super) quest_pending: bool,
    pub(super) quest_shutdown: bool,
    pub(super) stash: oteryn_client_ui::stash::StashPanel,
    pub(super) guild: oteryn_client_ui::guild::GuildPanel,
    /// Last published connection-local presentation, including appearance-only updates.
    pub(super) appearance_state: Vec<oteryn_session::VisibleAppearance>,
    /// Pushes that arrived before `stepped` was stored: older than its outcome.
    before: Pushed,
    stepped: Option<Box<StepOutcome>>,
    pub(super) action_feedback: Option<Option<&'static str>>,
    pub(super) pushed: Pushed,
    /// Combat answers not yet polled; the oldest is dropped past [`MAX_COMBAT_REPORTS`].
    combat: VecDeque<CombatReport>,
    /// The latest refused logout not yet polled.
    logout_refused: Option<Option<LogoutResult>>,
    ended: Option<PlayEvent>,
    pub(super) task_board: oteryn_client_ui::task_board::TaskBoardPanel,
    pub(super) dialogue: oteryn_client_ui::npc_dialogue::DialoguePanel,
    #[cfg(any(windows, test))]
    pub(super) dialogue_pointer: Option<(f64, f64)>,
    #[cfg(windows)]
    pub(super) dialogue_window: bool,
    pub(super) dialogue_shutdown: bool,
    #[cfg(windows)]
    pub(super) npc_shop_window: bool,
    pub(super) npc_shop: oteryn_client_ui::npc_shop::NpcShopPanel,
    pub(super) prey: oteryn_client_ui::prey::PreyPanel,
    pub(super) prey_shutdown: bool,
    pub(super) imbue: oteryn_client_ui::imbue::ImbuePanel,
    pub(super) imbue_shutdown: bool,
    pub(super) player_trade: oteryn_client_ui::player_trade::PlayerTradePanel,
    pub(super) vip: oteryn_client_ui::vip::VipPanel,
    pub(super) party: oteryn_client_ui::party::PartyPanel,
    pub(super) hireling: oteryn_client_ui::hireling::HirelingPanel,
    pub(super) hireling_shutdown: bool,
    pub(super) hireling_management: oteryn_client_ui::hireling_management::Panel,
    pub(super) market: oteryn_client_ui::market::MarketPanel,
    pub(super) market_shutdown: bool,
    // CL26s1
    pub(super) chat: oteryn_client_ui::chat::ChatPanel,
    pub(super) chat_public: oteryn_client_ui::chat_public_channels::ChatPanel,
    pub(super) chat_private: oteryn_client_ui::chat_private_channel::ChatPanel,
    pub(super) chat_public_shutdown: bool,
    // CL26s2: player-typing panel state and native-window shutdown.
    pub(super) player_typing: oteryn_client_ui::player_typing::PlayerTypingPanel,
    pub(super) player_typing_shutdown: bool,
    // CL26s3
    pub(super) daily_reward: oteryn_client_ui::daily_reward::DailyRewardPanel,
    pub(super) daily_reward_shutdown: bool,
    // CL26s4
    pub(super) boss_rewards: oteryn_client_ui::boss_rewards::BossRewardsPanel,
    pub(super) boss_rewards_shutdown: bool,
}

const MAX_COMBAT_REPORTS: usize = 8;
pub(super) type SharedMailbox = Arc<Mutex<Mailbox>>;

impl Mailbox {
    pub(super) fn combat_report(&mut self, report: CombatReport) {
        if let CombatReport::Cast(disposition) = &report {
            self.spell_bar.feedback =
                oteryn_client_runtime::spell::feedback_text(*disposition).into();
        }
        if self.combat.len() == MAX_COMBAT_REPORTS {
            self.combat.pop_front();
        }
        self.combat.push_back(report);
    }

    /// A slow shell may not silently lose an announcement or grow an unbounded event queue.
    fn check_server_messages(&mut self) {
        if self.before.server_messages.overflowed()
            || self.pushed.server_messages.overflowed()
            || self.before.server_messages.len() + self.pushed.server_messages.len()
                > oteryn_session::MAX_QUEUED_EVENTS
        {
            self.before.server_messages = ServerMessageInbox::default();
            self.pushed.server_messages = ServerMessageInbox::default();
            self.ended
                .get_or_insert(PlayEvent::Ended(PublicClass::SessionUnavailable));
        }
    }
}

pub(super) fn lock(mailbox: &SharedMailbox) -> MutexGuard<'_, Mailbox> {
    mailbox.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The task's end of the mailbox.
#[derive(Debug, Clone)]
pub struct Outbox(pub(super) SharedMailbox);

impl Outbox {
    pub(super) fn stepped(&self, outcome: StepOutcome) {
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
        super::feature_0060_player_trade::protocol::reduce(&mut mailbox.player_trade, &events);
        super::feature_local_spell_action_bar::protocol::vitals_events(&mut mailbox, &events);
        mailbox.death_protection_events(&events);
        mailbox.dialogue_events(&events);
        mailbox.npc_shop_events(&events);
        for event in &events {
            if let SessionEvent::ContainerViews(update) = event {
                super::feature_0230_browse_field::protocol::replace_views(
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
        super::feature_0010_analyser::protocol::reduce(&mut mailbox, &events);
        super::feature_0134_party_skull::protocol::reduce(
            &mut mailbox.properties.party_skull,
            &events,
        );
        super::feature_0287_actor_spell_catalog::protocol::reduce(&mut mailbox, &events);
        super::feature_0291_cast_chat::protocol::reduce(&mut mailbox, &events);
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
        super::feature_0016_quest_log::protocol::reduce(&mut mailbox.quest, deltas);
        super::feature_0240_chat_party_room::protocol::reset_if_replaced(&mut mailbox);
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

    pub(super) fn entities(&self, entities: Vec<WorldSpatialEntity>) {
        let mut mailbox = lock(&self.0);
        oteryn_client_ui::player_typing::protocol::retain_visible(
            &mut mailbox.player_typing,
            &entities,
        );
        mailbox.pushed.entities = Some(entities);
    }

    pub(super) fn logout_refused(&self, result: Option<LogoutResult>) {
        lock(&self.0).logout_refused = Some(result);
    }

    /// The first end wins.
    pub(super) fn end(&self, class: PublicClass) {
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
        mailbox.spellbook_details = Default::default();
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

    pub(super) fn logged_out(&self) {
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
        mailbox.spellbook_details = Default::default();
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

/// The shell's end of the session task: steps out, a bounded mailbox in, neither ever blocking.
#[derive(Debug)]
pub struct PlayLink {
    pub(super) commands: UnboundedSender<PlayCommand>,
    pub(super) mailbox: SharedMailbox,
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
        super::feature_0081_npc_shop::view::open_window(&self.mailbox, &self.commands);
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
        if let Some(launcher) = &mailbox.coin_shop {
            launcher.end();
        }
        mailbox.properties.end();
        mailbox.weapon_proficiency = Default::default();
        mailbox.imbue = Default::default();
        mailbox.analyser = Default::default();
        mailbox.cast_chat = Default::default();
        mailbox.cast_control = Default::default();
        mailbox.spellbook_details = Default::default();
        mailbox.spell_catalog = Default::default();
        mailbox.spell_bar = Default::default();
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
