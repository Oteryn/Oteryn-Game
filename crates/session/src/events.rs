//! Validated pushed domain updates exposed by the client session.
use super::*;

/// A server-sequenced delta that no command outcome claimed, already validated and applied to the
/// session's domain store (FND-02 §14, §15), in the order it was applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// Domain 16: the requested view and tracked lines, replaced whole.
    QuestLog(AppliedDelta<quest_log::QuestLog>),
    Appearance(AppliedDelta<AppearanceUpdate>),
    TutorialNotice(AppliedDelta<oteryn_protocol_oteryn::tutorial_notices::TutorialNotice>),
    /// Domain 57: one server status, warning or advance message, retained in wire order.
    ServerMessage(AppliedDelta<ServerMessage>),
    /// Domain 1 `WORLD_SPATIAL_VISIBILITY`.
    WorldSpatial(AppliedDelta<WorldSpatialObservation>),
    /// Domain 1 with capability 6 selected: the entities that entered, updated and left, and the
    /// own-actor position. [`Session::world_entities`] already holds the result.
    WorldSpatialEntities(AppliedDelta<WorldSpatialEntitiesDelta>),
    /// Domain 2 `WORLD_OBJECT_OVERLAY`.
    WorldObjectOverlay(AppliedDelta<WorldObjectOverlayEntry>),
    /// Domain 17: validated bundle map changes, bound to the admitted content and reset epoch.
    WorldMap(AppliedDelta<oteryn_protocol_oteryn::world_map::WorldMapViewDelta>),
    /// Domain 3 `ACTOR_VITALS`.
    ActorVitals(AppliedDelta<ActorVitals>),
    /// Domain 10 `ACTOR_COMBAT_STATE`: the own actor's attack target, modes and in-fight flag,
    /// replaced whole. [`Session::combat_state`] already holds it.
    ActorCombatState(AppliedDelta<ActorCombatState>),
    /// Domain 12 `CHAT`, delta type 1: one line. [`Session::chat_log`] already holds it.
    ChatLine(AppliedDelta<ChatLine>),
    /// Domain 12 `CHAT`, delta type 2: the open-room set, replaced whole.
    ChatRooms(AppliedDelta<ChatRoomSet>),
    /// Domain 12, capability-80 delta 3: available rooms and private-channel metadata.
    ChatChannelList(AppliedDelta<ChatChannelList>),
    /// Domain 12, negotiated delta 5: expiring coarse VIP statuses.
    VipPresence(AppliedDelta<oteryn_protocol_oteryn::vip_presence::PresenceBatch>),
    /// Domain 9 `CHARACTER_INVENTORY`: the inventory, replaced whole.
    /// [`Session::inventory`] already holds it.
    Inventory(AppliedDelta<CharacterInventory>),
    /// Domain 11 `OPEN_CONTAINER`: the open corpse, replaced whole (the default means closed).
    /// [`Session::open_container`] already holds it.
    OpenContainer(AppliedDelta<OpenContainer>),
}

impl SessionEvent {
    #[must_use]
    pub const fn domain_id(&self) -> u32 {
        match self {
            Self::QuestLog(_) => quest_log::STATE_DOMAIN_QUEST_LOG,
            Self::Appearance(_) => oteryn_protocol_oteryn::appearance::STATE_DOMAIN_APPEARANCE,
            Self::TutorialNotice(_) => 23,
            Self::ServerMessage(_) => {
                oteryn_protocol_oteryn::zones_nologout::STATE_DOMAIN_SERVER_MESSAGES
            }
            Self::WorldSpatial(_) | Self::WorldSpatialEntities(_) => {
                world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
            }
            Self::WorldObjectOverlay(_) => world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            Self::WorldMap(_) => oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
            Self::ActorVitals(_) => actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            Self::ActorCombatState(_) => attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            Self::ChatLine(_) | Self::ChatRooms(_) | Self::VipPresence(_) => STATE_DOMAIN_CHAT,
            Self::ChatChannelList(_) => STATE_DOMAIN_CHAT,
            Self::Inventory(_) => STATE_DOMAIN_CHARACTER_INVENTORY,
            Self::OpenContainer(_) => STATE_DOMAIN_OPEN_CONTAINER,
        }
    }
}
