//! Validated pushed domain updates exposed by the client session.
use super::*;

/// A server-sequenced delta that no command outcome claimed, already validated and applied to the
/// session's domain store (FND-02 §14, §15), in the order it was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    CastCasterLine(AppliedDelta<cast_chat::CasterLine>),
    CastRoster(AppliedDelta<cast_chat::Roster>),
    /// A fully validated handoff and destination snapshot replaced this logical session.
    ActivityScope(AppliedDelta<activity_scope::ActivityScopeSnapshot>),
    AnalyserFacts(AppliedDelta<Vec<analyser_transport::AnalyserFact>>),
    /// Domain 15 delta 1 of a session with capability 486: each fact with its optional attacker.
    AnalyserAttributedFacts(AppliedDelta<Vec<analyser_transport::AttributedFact>>),
    WorldWeather(AppliedDelta<world_weather::WorldWeather>),
    /// Every replaced domain, including those with a typed projection event.
    GatedSnapshotReplaced(AppliedDelta<GatedSnapshot>),
    /// Domain 2's whole placement set, including an empty set that removes old markers.
    WorldObjectOverlaySnapshot(AppliedDelta<Vec<WorldObjectOverlayEntry>>),
    /// Domain 12's complete room and retained-line state.
    ChatSnapshot(AppliedDelta<ChatLog>),
    WorldClock(AppliedDelta<world_clock::WorldClock>),
    Presentation(AppliedDelta<Vec<PresentationEvent>>),
    ItemEffects(AppliedDelta<Vec<PresentationEvent>>),
    ActorCooldowns(AppliedDelta<oteryn_protocol_oteryn::presentation::ActorCooldowns>),
    PlayerTrade(AppliedDelta<player_trade::State>),
    NpcConversation(AppliedDelta<npc_service::NpcConversation>),
    NpcTradeWindow(AppliedDelta<npc_service::NpcTradeWindow>),
    /// Domain 16: the requested view and tracked lines, replaced whole.
    QuestLog(AppliedDelta<quest_log::QuestLog>),
    Appearance(AppliedDelta<AppearanceUpdate>),
    ExpertFightModes(AppliedDelta<oteryn_protocol_oteryn::pvp_expert::ExpertFightModes>),
    ServerSaveCountdown(AppliedDelta<ServerSaveCountdown>),
    TutorialNotice(AppliedDelta<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice>),
    /// Domain 13: a newly earned account achievement; a duplicate grant produces no delta.
    AchievementEarned(AppliedDelta<AchievementEarned>),
    /// Domain 57: one server status, warning or advance message, retained in wire order.
    ServerMessage(AppliedDelta<ServerMessage>),
    /// Domain 1 `WORLD_SPATIAL_VISIBILITY`.
    WorldSpatial(AppliedDelta<WorldSpatialObservation>),
    /// Domain 1 with capability 6 selected: the entities that entered, updated and left, and the
    /// own-actor position. [`Session::world_entities`] already holds the result.
    WorldSpatialEntities(AppliedDelta<WorldSpatialEntitiesDelta>),
    /// Domain 1 icons-only replacement for the currently visible players.
    PartyIcons(AppliedDelta<Vec<oteryn_protocol_oteryn::party_skull::PartyIcon>>),
    /// Domain 2 `WORLD_OBJECT_OVERLAY`.
    WorldObjectOverlay(AppliedDelta<WorldObjectOverlayEntry>),
    /// Domain 17: validated bundle map changes, bound to the admitted content and reset epoch.
    /// A complete replacement window, applied only after its matching commit.
    WorldMapSnapshot(AppliedDelta<oteryn_protocol_oteryn::world_map::WorldMapViewSnapshot>),
    /// A revisioned incremental change to the current map window.
    WorldMap(AppliedDelta<oteryn_protocol_oteryn::world_map::WorldMapViewDelta>),
    /// Domain 3 `ACTOR_VITALS`.
    ActorVitals(AppliedDelta<ActorVitals>),
    /// Domain 10 `ACTOR_COMBAT_STATE`: the own actor's attack target, modes and in-fight flag,
    /// replaced whole. [`Session::combat_state`] already holds it.
    ActorCombatState(AppliedDelta<ActorCombatState>),
    /// Domain 12 `CHAT`, delta type 1: one line. [`Session::chat_log`] already holds it.
    ChatLine(AppliedDelta<ChatLine>),
    /// Domain 12 `CHAT`, delta type 1 under capability 495: a member joined, left or was
    /// excluded from a private channel. [`ChatLog::channel_notices`] already holds it.
    ChatNotice(AppliedDelta<chat_extended::ChannelNotice>),
    /// Domain 12 `CHAT`, delta type 2: the open-room set, replaced whole.
    ChatRooms(AppliedDelta<ChatRoomSet>),
    /// Domain 12, capability-80 delta 3: available rooms and private-channel metadata.
    ChatChannelList(AppliedDelta<ChatChannelList>),
    /// Domain 12, capability-233 delta 4: a visible actor's transient typing state.
    PlayerTyping(AppliedDelta<player_typing::TypingUpdate>),
    /// Domain 12, negotiated delta 5: expiring coarse VIP statuses.
    VipPresence(AppliedDelta<oteryn_protocol_oteryn::vip_presence::PresenceBatch>),
    /// Domain 9 `CHARACTER_INVENTORY`: the inventory, replaced whole.
    /// [`Session::inventory`] already holds it.
    Inventory(AppliedDelta<CharacterInventory>),
    /// Live bags shared by container, depot, trade and NPC shop projections.
    ContainerViews(AppliedDelta<container_tree::ContainerViews>),
    /// Domain 11 `OPEN_CONTAINER`: the open corpse, replaced whole (the default means closed).
    /// [`Session::open_container`] already holds it.
    OpenContainer(AppliedDelta<OpenContainer>),
}

impl SessionEvent {
    #[must_use]
    pub const fn domain_id(&self) -> u32 {
        match self {
            Self::CastCasterLine(_) | Self::CastRoster(_) => 103,
            Self::ActivityScope(_) => 99,
            Self::AnalyserFacts(_) | Self::AnalyserAttributedFacts(_) => 15,
            Self::PlayerTrade(_) => player_trade::STATE_DOMAIN_PLAYER_TRADE,
            Self::GatedSnapshotReplaced(snapshot) => snapshot.value.domain_id,
            Self::WorldObjectOverlaySnapshot(_) => 2,
            Self::ChatSnapshot(_) => STATE_DOMAIN_CHAT,
            Self::NpcConversation(_) => npc_service::STATE_DOMAIN_NPC_CONVERSATION,
            Self::NpcTradeWindow(_) => npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW,
            Self::QuestLog(_) => quest_log::STATE_DOMAIN_QUEST_LOG,
            Self::ExpertFightModes(_) => oteryn_protocol_oteryn::pvp_expert::STATE_DOMAIN_EXPERT_FIGHT_MODES,
            Self::Appearance(_) => oteryn_protocol_oteryn::appearance::STATE_DOMAIN_APPEARANCE,
            Self::TutorialNotice(_) => 23,
            Self::WorldClock(_) => 18,
            Self::WorldWeather(_) => 22,
            Self::Presentation(_) => 19,
            Self::ItemEffects(_) => oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS,
            Self::ActorCooldowns(_) => 20,
            Self::ServerSaveCountdown(_) => 40,
            Self::AchievementEarned(_) => {
                oteryn_protocol_oteryn::achievement_notices::STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES
            }
            Self::ServerMessage(_) => {
                oteryn_protocol_oteryn::zones_nologout::STATE_DOMAIN_SERVER_MESSAGES
            }
            Self::WorldSpatial(_) | Self::WorldSpatialEntities(_) | Self::PartyIcons(_) => {
                world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
            }
            Self::WorldObjectOverlay(_) => world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            Self::WorldMap(_) | Self::WorldMapSnapshot(_) => {
                world_map::STATE_DOMAIN_WORLD_MAP_VIEW
            }
            Self::ActorVitals(_) => actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            Self::ActorCombatState(_) => attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            Self::ChatLine(_)
            | Self::ChatNotice(_)
            | Self::ChatRooms(_)
            | Self::VipPresence(_)
            | Self::PlayerTyping(_) => STATE_DOMAIN_CHAT,
            Self::ChatChannelList(_) => STATE_DOMAIN_CHAT,
            Self::Inventory(_) => STATE_DOMAIN_CHARACTER_INVENTORY,
            Self::ContainerViews(_) => container_tree::STATE_DOMAIN_CONTAINER_VIEWS,
            Self::OpenContainer(_) => STATE_DOMAIN_OPEN_CONTAINER,
        }
    }
}
