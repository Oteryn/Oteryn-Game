//! Registered capability command/domain ownership and superseded core domains.

use crate::achievement_notices::{
    CAPABILITY_ACHIEVEMENT_NOTICES_V1, STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
};
use crate::analyser::{CAPABILITY_ANALYSER_V1, STATE_DOMAIN_ACTOR_ANALYSER};
use crate::attack::{
    CAPABILITY_ATTACK_V1, COMMAND_TYPE_ATTACK_TARGET_INTENT, COMMAND_TYPE_FIGHT_MODES_INTENT,
    STATE_DOMAIN_ACTOR_COMBAT_STATE,
};
use crate::bed::COMMAND_TYPE_BED_SLEEP_INTENT;
use crate::bestiary::STATE_DOMAIN_CHARACTER_BESTIARY;
use crate::charm::{
    CAPABILITY_BESTIARY_CHARMS_V1, COMMAND_TYPE_CHARM_ASSIGN_INTENT,
    COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT, STATE_DOMAIN_CHARACTER_CHARMS,
};
use crate::chat::{CAPABILITY_CHAT_V1, COMMAND_TYPE_CHAT_INTENT, STATE_DOMAIN_CHAT};
use crate::container_tree::{
    CAPABILITY_CONTAINER_TREE_V1, COMMAND_TYPE_CONTAINER_VIEW_INTENT, STATE_DOMAIN_CONTAINER_VIEWS,
};
use crate::daily_reward::{
    CAPABILITY_DAILY_REWARD_V1, COMMAND_TYPE_DAILY_REWARD_CLAIM, COMMAND_TYPE_DAILY_REWARD_QUERY,
    STATE_DOMAIN_DAILY_REWARD,
};
use crate::highscores::{CAPABILITY_HIGHSCORES_V1, COMMAND_TYPE_HIGHSCORES_QUERY_V1};
use crate::item_view::{
    CAPABILITY_ITEM_EQUIP_DROP_V1, CAPABILITY_ITEM_VIEW_MOVE_V1, COMMAND_TYPE_ITEM_MOVE_INTENT,
    STATE_DOMAIN_CHARACTER_INVENTORY, STATE_DOMAIN_OPEN_CONTAINER,
};
use crate::logout::{CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT};
use crate::npc_service::{
    CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, COMMAND_TYPE_NPC_TRADE_INTENT,
    STATE_DOMAIN_NPC_CONVERSATION, STATE_DOMAIN_NPC_TRADE_WINDOW,
};
use crate::presentation::{
    CAPABILITY_PRESENTATION_V1, STATE_DOMAIN_ACTOR_COOLDOWNS, STATE_DOMAIN_WORLD_PRESENTATION,
};
use crate::quest_log::{
    CAPABILITY_QUEST_LOG_V1, COMMAND_TYPE_QUEST_LOG_QUERY, STATE_DOMAIN_QUEST_LOG,
};
use crate::world_clock::{CAPABILITY_WORLD_LIGHT_V1, STATE_DOMAIN_WORLD_CLOCK};
use crate::world_interaction::{CAPABILITY_WORLD_INTERACTION_V1, COMMAND_TYPE_PUSH_INTENT};
use crate::world_map::{CAPABILITY_WORLD_MAP_VIEW_V1, STATE_DOMAIN_WORLD_MAP_VIEW};
use crate::world_object::{
    CAPABILITY_ITEM_USE_V1, CAPABILITY_RUNE_USE_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
};
use crate::world_spatial::CAPABILITY_PACED_MOVEMENT_V1;
use crate::world_weather::{CAPABILITY_WORLD_WEATHER_V1, STATE_DOMAIN_WORLD_WEATHER};

/// `ITEM_LOOK_V1`, registry capability 28.
pub const CAPABILITY_ITEM_LOOK_V1: u32 = 28;
/// `ITEM_LOOK_INTENT`, registry command type 30, gated by capability 28.
pub const COMMAND_TYPE_ITEM_LOOK_INTENT: u32 = 30;
/// `MAP_STATE_V1`, registry capability 29 (CP D1442): never offered, superseded by 18.
const CAPABILITY_MAP_STATE_V1: u32 = 29;
/// `WORLD_SPATIAL_FIELDS`, registry capability 30 (CP D1442): not offered before FIELD-WIRE-1.
const CAPABILITY_WORLD_SPATIAL_FIELDS: u32 = 30;

/// The command types and state domains each registered capability owns
/// (`PROTOCOL_OTERYN_V1_REGISTRY.json`; a test keeps them equal). Capability 6
/// `WORLD_SPATIAL_ENTITIES` is not listed: it extends the core domain 1 with payload type 2, which
/// the visibility encoders gate on the selected set themselves. Capability 12
/// `ITEM_EQUIP_DROP_V1` owns none either: it extends command type 9 and domain 9 of capability 4,
/// whose codecs gate the extension on the selected set. Capability 13 `PACED_MOVEMENT_V1` owns
/// none: it extends the command type 1 result, whose encoder gates `TOO_EARLY` on the selection.
/// Capability 15 `ITEM_USE_V1` owns none: it extends command type 2, whose decoder gates fields 4
/// and 5 on the selection, and the use result, whose dispositions 7 to 10 only it receives.
pub const GATED: &[(u32, &[u32], &[u32])] = &[
    (
        CAPABILITY_BESTIARY_CHARMS_V1,
        &[
            COMMAND_TYPE_CHARM_UNLOCK_STAGE_INTENT,
            COMMAND_TYPE_CHARM_ASSIGN_INTENT,
        ],
        &[
            STATE_DOMAIN_CHARACTER_BESTIARY,
            STATE_DOMAIN_CHARACTER_CHARMS,
        ],
    ),
    (
        CAPABILITY_NPC_SERVICE_V1,
        &[COMMAND_TYPE_NPC_TALK_INTENT, COMMAND_TYPE_NPC_TRADE_INTENT],
        &[STATE_DOMAIN_NPC_CONVERSATION, STATE_DOMAIN_NPC_TRADE_WINDOW],
    ),
    (
        CAPABILITY_ITEM_VIEW_MOVE_V1,
        &[COMMAND_TYPE_ITEM_MOVE_INTENT],
        &[
            STATE_DOMAIN_CHARACTER_INVENTORY,
            STATE_DOMAIN_OPEN_CONTAINER,
        ],
    ),
    (
        CAPABILITY_CHAT_V1,
        &[COMMAND_TYPE_CHAT_INTENT],
        &[STATE_DOMAIN_CHAT],
    ),
    (
        CAPABILITY_ACHIEVEMENT_NOTICES_V1,
        &[],
        &[STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES],
    ),
    // RANK-1 (HS-WIRE-1): capability 9 gates the highscore query and owns no domain.
    (
        CAPABILITY_HIGHSCORES_V1,
        &[COMMAND_TYPE_HIGHSCORES_QUERY_V1],
        &[],
    ),
    (CAPABILITY_ANALYSER_V1, &[], &[STATE_DOMAIN_ACTOR_ANALYSER]),
    (
        CAPABILITY_WORLD_INTERACTION_V1,
        &[COMMAND_TYPE_PUSH_INTENT, COMMAND_TYPE_BED_SLEEP_INTENT],
        &[],
    ),
    (CAPABILITY_ITEM_EQUIP_DROP_V1, &[], &[]),
    (CAPABILITY_PACED_MOVEMENT_V1, &[], &[]),
    (
        CAPABILITY_CONTAINER_TREE_V1,
        &[COMMAND_TYPE_CONTAINER_VIEW_INTENT],
        &[STATE_DOMAIN_CONTAINER_VIEWS],
    ),
    (CAPABILITY_ITEM_USE_V1, &[], &[]),
    (
        CAPABILITY_QUEST_LOG_V1,
        &[COMMAND_TYPE_QUEST_LOG_QUERY],
        &[STATE_DOMAIN_QUEST_LOG],
    ),
    (
        CAPABILITY_ATTACK_V1,
        &[
            COMMAND_TYPE_ATTACK_TARGET_INTENT,
            COMMAND_TYPE_FIGHT_MODES_INTENT,
        ],
        &[STATE_DOMAIN_ACTOR_COMBAT_STATE],
    ),
    (
        CAPABILITY_WORLD_MAP_VIEW_V1,
        &[],
        &[STATE_DOMAIN_WORLD_MAP_VIEW],
    ),
    (CAPABILITY_WORLD_LIGHT_V1, &[], &[STATE_DOMAIN_WORLD_CLOCK]),
    (CAPABILITY_LOGOUT_V1, &[COMMAND_TYPE_LOGOUT_INTENT], &[]),
    // RUNE-USE-0 §4: owns no command type or domain; it gates the USE position arm and
    // dispositions 18 and 19, and is not offered before RUNE-CAST-1.
    (CAPABILITY_RUNE_USE_V1, &[], &[]),
    (
        CAPABILITY_PRESENTATION_V1,
        &[],
        &[
            STATE_DOMAIN_WORLD_PRESENTATION,
            STATE_DOMAIN_ACTOR_COOLDOWNS,
        ],
    ),
    (
        crate::appearance::CAPABILITY_APPEARANCE_V1,
        &[crate::appearance::COMMAND_TYPE_APPEARANCE_CHANGE],
        &[crate::appearance::STATE_DOMAIN_APPEARANCE],
    ),
    // WEATHER-1: capability 26 owns domain 22 and no command type; it is offered per connection
    // only with a weather source (`with_weather_offer`).
    (
        CAPABILITY_WORLD_WEATHER_V1,
        &[],
        &[STATE_DOMAIN_WORLD_WEATHER],
    ),
    // ITEM-LOOK-1: capability 28 `ITEM_LOOK_V1` gates command type 30 `ITEM_LOOK_INTENT` (CP
    // D1268, D1292) and owns no domain. It is not offered and command 30 has no handler yet, so a
    // session never selects 28 and command 30 is never dispatched.
    (
        CAPABILITY_ITEM_LOOK_V1,
        &[COMMAND_TYPE_ITEM_LOOK_INTENT],
        &[],
    ),
    // CP D1442: registered for the ADMIT0 floor names; neither owns a command type or domain
    // and neither is offered, so no session selects them.
    (CAPABILITY_MAP_STATE_V1, &[], &[]),
    (CAPABILITY_WORLD_SPATIAL_FIELDS, &[], &[]),
    (
        crate::tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
        &[],
        &[crate::tutorial_notices::STATE_DOMAIN_TUTORIAL_NOTICES],
    ),
    (
        CAPABILITY_DAILY_REWARD_V1,
        &[
            COMMAND_TYPE_DAILY_REWARD_CLAIM,
            COMMAND_TYPE_DAILY_REWARD_QUERY,
        ],
        &[STATE_DOMAIN_DAILY_REWARD],
    ),
    (
        crate::spectate::CAPABILITY_SPECTATE_V1,
        &[
            crate::spectate::COMMAND_TYPE_OBSERVE_GRANT_SET,
            crate::spectate::COMMAND_TYPE_OBSERVE_GRANT_REVOKE,
        ],
        &[crate::spectate::STATE_DOMAIN_OBSERVE_TARGET],
    ),
    (
        crate::chat::guild_room::CAPABILITY_CHAT_GUILD_ROOM_V1,
        &[],
        &[],
    ),
    (
        crate::guild::CAPABILITY_GUILD_V1,
        &[
            crate::guild::COMMAND_TYPE_GUILD_INTENT,
            crate::guild::COMMAND_TYPE_GUILD_QUERY,
        ],
        &[],
    ),
];

/// Core domains a selected capability replaces, as (capability, domains). MAP-WIRE-1 contract §3
/// "One object domain per World" (`MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md`, accepted) with
/// ARCH-MAP-WIRE-1 §1.5: a World booted from a bundle serves domain 17 and does not serve
/// `WORLD_OBJECT_OVERLAY` (domain 2); its doors, levers and other object states reach the client
/// as tile changes of domain 17. Capability 18 is selected exactly on a bundle World (only it
/// offers 18, and it requires 18), so a session that selected 18 is never sent domain 2 on join,
/// resume or delta, and a client that selected 18 refuses domain 2. A fixture World never offers
/// 18 and keeps domain 2.
pub const SUPERSEDED: &[(u32, &[u32])] = &[(
    CAPABILITY_WORLD_MAP_VIEW_V1,
    &[STATE_DOMAIN_WORLD_OBJECT_OVERLAY],
)];

#[cfg(test)]
#[path = "capability_tables_tests.rs"]
mod tests;
