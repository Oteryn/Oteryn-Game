//! Existing session support allowlist and registry-derived raw routes.
use super::*;

/// The capabilities this client implements and advertises by default: 4 `ITEM_VIEW_MOVE_V1`
/// (domains 9 and 11, command 9, the USE item target and the item handle on domain-1 entities),
/// 6 `WORLD_SPATIAL_ENTITIES` (the domain-1 type 2 snapshot and delta with every visible entity), 7 `CHAT_V1` (command 13 and
/// domain 12), 13 `PACED_MOVEMENT_V1` (the step result `TOO_EARLY`),
/// 16 `QUEST_LOG_V1` (domain 16 and command 22), and 17 `ATTACK_V1` (command
/// types 11 and 12, domain 10; requires 6) and 21 `LOGOUT_V1` (command type 23). Add an ID here only
/// together with its routing, and keep the set closed under the registry's `requires`.
/// 147 edits Exiva settings through command 170. Field runes (91) reuse USE command 2,
/// existing map/entity updates and the rune refusal decoder; 11, 15, 18, 22 and 30
/// complete their prerequisite closure on shipped bundle Worlds.
/// 272 adds live Atelier resource queries to the existing 148 panel. The mechanic panels added
/// by CLIENT-E2E-1 advertise their typed command routes so the shipped session loop and the
/// qualification client negotiate the same feature set.
pub const CLIENT_SUPPORTED_CAPABILITIES: &[u32] = &[
    CAPABILITY_ITEM_VIEW_MOVE_V1,
    CAPABILITY_WORLD_SPATIAL_ENTITIES,
    CAPABILITY_CHAT_V1,
    oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
    CAPABILITY_PACED_MOVEMENT_V1,
    oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1,
    quest_log::CAPABILITY_QUEST_LOG_V1,
    CAPABILITY_ATTACK_V1,
    oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    logout::CAPABILITY_LOGOUT_V1,
    oteryn_protocol_oteryn::world_object::CAPABILITY_RUNE_USE_V1,
    oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1,
    oteryn_protocol_oteryn::capability_tables::CAPABILITY_WORLD_SPATIAL_FIELDS,
    oteryn_protocol_oteryn::tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
    chat::guild_room::CAPABILITY_CHAT_GUILD_ROOM_V1,
    oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1,
    oteryn_protocol_oteryn::world_object::CAPABILITY_FIELD_RUNES_V1,
    oteryn_protocol_oteryn::exiva::CAPABILITY_EXIVA_SETTINGS_V1,
    wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
    atelier::resources::CAPABILITY_ATELIER_RESOURCES_V1,
    guild::CAPABILITY_GUILD_V1,
    party::CAPABILITY_PARTY_V1,
    vip::CAPABILITY_VIP_V1,
    player_trade::CAPABILITY_PLAYER_TRADE_V1,
    stash::CAPABILITY_STASH_V1,
    market::CAPABILITY_MARKET_V1,
    imbuement::CAPABILITY_IMBUE_V1,
    task_board::CAPABILITY_TASK_BOARD_V1,
    npc_shop::CAPABILITY_NPC_SHOP_V1,
    prey::CAPABILITY_PREY_V1,
    hireling::CAPABILITY_HIRELING_V1,
];

/// Capabilities supported by the raw command/snapshot/delta routes, including opt-in
/// capabilities absent from `CLIENT_SUPPORTED_CAPABILITIES`. Typed routes stay separate.
const CLIENT_GATED_CAPABILITIES: &[u32] = &[
    wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
    atelier::resources::CAPABILITY_ATELIER_RESOURCES_V1,
    oteryn_protocol_oteryn::exiva::CAPABILITY_EXIVA_SETTINGS_V1,
    oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_V1,
    oteryn_protocol_oteryn::hotkey_equip::CAPABILITY_HOTKEY_EQUIP_CARRIED_V1,
    quest_log::CAPABILITY_QUEST_LOG_V1,
    oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1,
    oteryn_protocol_oteryn::tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
    oteryn_protocol_oteryn::appearance::CAPABILITY_APPEARANCE_V1,
    CAPABILITY_BESTIARY_CHARMS_V1,
    CAPABILITY_ACHIEVEMENT_NOTICES_V1,
    CAPABILITY_ANALYSER_V1,
    CAPABILITY_CONTAINER_TREE_V1,
];

pub(super) fn gated_routes() -> impl Iterator<Item = (u32, &'static [u32], &'static [u32])> {
    oteryn_protocol_oteryn::capability_tables::GATED
        .iter()
        .copied()
        .filter(|(capability, _, _)| CLIENT_GATED_CAPABILITIES.contains(capability))
}

pub(super) fn capability_of_domain(domain_id: u32) -> Option<u32> {
    gated_routes()
        .find(|(_, _, domains)| domains.contains(&domain_id))
        .map(|(capability, _, _)| capability)
}
