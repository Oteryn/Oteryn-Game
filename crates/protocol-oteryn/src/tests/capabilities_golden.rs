// Explicit accepted facts. Update only for accepted capability-set changes.
[
    r#"{
  "id": 1,
  "name": "BESTIARY_CHARMS_V1",
  "constant": "charm::CAPABILITY_BESTIARY_CHARMS_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    4,
    5
  ],
  "domains": [
    4,
    5
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 3,
  "name": "NPC_SERVICE_V1",
  "constant": "npc_service::CAPABILITY_NPC_SERVICE_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    7,
    8
  ],
  "domains": [
    7,
    8
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 4,
  "name": "ITEM_VIEW_MOVE_V1",
  "constant": "item_view::CAPABILITY_ITEM_VIEW_MOVE_V1",
  "requires": [
    6
  ],
  "acceptance_requires": true,
  "commands": [
    9
  ],
  "domains": [
    9,
    11
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 6,
  "name": "WORLD_SPATIAL_ENTITIES",
  "constant": "world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES",
  "requires": [],
  "acceptance_requires": false,
  "commands": [],
  "domains": null,
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 7,
  "name": "CHAT_V1",
  "constant": "chat::CAPABILITY_CHAT_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    13
  ],
  "domains": [
    12
  ],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 8,
  "name": "ACHIEVEMENT_NOTICES_V1",
  "constant": "achievement_notices::CAPABILITY_ACHIEVEMENT_NOTICES_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [],
  "domains": [
    13
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 9,
  "name": "HIGHSCORES_V1",
  "constant": "highscores::CAPABILITY_HIGHSCORES_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    14
  ],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 10,
  "name": "ANALYSER_V1",
  "constant": "analyser::CAPABILITY_ANALYSER_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [],
  "domains": [
    15
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 11,
  "name": "WORLD_INTERACTION_V1",
  "constant": "world_interaction::CAPABILITY_WORLD_INTERACTION_V1",
  "requires": [
    15,
    18
  ],
  "acceptance_requires": true,
  "commands": [
    15,
    31
  ],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 12,
  "name": "ITEM_EQUIP_DROP_V1",
  "constant": "item_view::CAPABILITY_ITEM_EQUIP_DROP_V1",
  "requires": [
    4
  ],
  "acceptance_requires": true,
  "commands": [],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 13,
  "name": "PACED_MOVEMENT_V1",
  "constant": "world_spatial::CAPABILITY_PACED_MOVEMENT_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 14,
  "name": "CONTAINER_TREE_V1",
  "constant": "container_tree::CAPABILITY_CONTAINER_TREE_V1",
  "requires": [
    4,
    12
  ],
  "acceptance_requires": true,
  "commands": [
    21
  ],
  "domains": [
    14
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 15,
  "name": "ITEM_USE_V1",
  "constant": "world_object::CAPABILITY_ITEM_USE_V1",
  "requires": [
    4
  ],
  "acceptance_requires": true,
  "commands": [],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 16,
  "name": "QUEST_LOG_V1",
  "constant": "quest_log::CAPABILITY_QUEST_LOG_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    22
  ],
  "domains": [
    16
  ],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 17,
  "name": "ATTACK_V1",
  "constant": "attack::CAPABILITY_ATTACK_V1",
  "requires": [
    6
  ],
  "acceptance_requires": true,
  "commands": [
    11,
    12
  ],
  "domains": [
    10
  ],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 18,
  "name": "WORLD_MAP_VIEW_V1",
  "constant": "world_map::CAPABILITY_WORLD_MAP_VIEW_V1",
  "requires": [
    4,
    6
  ],
  "acceptance_requires": false,
  "commands": [],
  "domains": [
    17
  ],
  "supersedes": [
    2
  ],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 19,
  "name": "WORLD_LIGHT_V1",
  "constant": "world_clock::CAPABILITY_WORLD_LIGHT_V1",
  "requires": [
    6
  ],
  "acceptance_requires": true,
  "commands": [],
  "domains": [
    18
  ],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 21,
  "name": "LOGOUT_V1",
  "constant": "logout::CAPABILITY_LOGOUT_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    23
  ],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 22,
  "name": "RUNE_USE_V1",
  "constant": "world_object::CAPABILITY_RUNE_USE_V1",
  "requires": [
    15
  ],
  "acceptance_requires": true,
  "commands": [],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 23,
  "name": "PRESENTATION_V1",
  "constant": "presentation::CAPABILITY_PRESENTATION_V1",
  "requires": [
    6
  ],
  "acceptance_requires": true,
  "commands": [],
  "domains": [
    19,
    20
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 25,
  "name": "APPEARANCE_V1",
  "constant": "appearance::CAPABILITY_APPEARANCE_V1",
  "requires": [
    6
  ],
  "acceptance_requires": false,
  "commands": [
    25
  ],
  "domains": [
    21
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 26,
  "name": "WORLD_WEATHER_V1",
  "constant": "world_weather::CAPABILITY_WORLD_WEATHER_V1",
  "requires": [
    6
  ],
  "acceptance_requires": true,
  "commands": [],
  "domains": [
    22
  ],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 28,
  "name": "ITEM_LOOK_V1",
  "constant": "capability_tables::CAPABILITY_ITEM_LOOK_V1",
  "requires": [],
  "acceptance_requires": true,
  "commands": [
    30
  ],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 29,
  "name": "MAP_STATE_V1",
  "constant": "capability_tables::CAPABILITY_MAP_STATE_V1",
  "requires": [
    6
  ],
  "acceptance_requires": true,
  "commands": [],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 30,
  "name": "WORLD_SPATIAL_FIELDS",
  "constant": "capability_tables::CAPABILITY_WORLD_SPATIAL_FIELDS",
  "requires": [
    6
  ],
  "acceptance_requires": true,
  "commands": [],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 31,
  "name": "TUTORIAL_NOTICES_V1",
  "constant": "tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [],
  "domains": [
    23
  ],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 32,
  "name": "DAILY_REWARD_V1",
  "constant": "daily_reward::CAPABILITY_DAILY_REWARD_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    32,
    33
  ],
  "domains": [
    24
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 33,
  "name": "SPECTATE_V1",
  "constant": "spectate::CAPABILITY_SPECTATE_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    34,
    35
  ],
  "domains": [
    25
  ],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": false
  }
}"#,
    r#"{
  "id": 36,
  "name": "CHAT_GUILD_ROOM_V1",
  "constant": "chat::guild_room::CAPABILITY_CHAT_GUILD_ROOM_V1",
  "requires": [
    7
  ],
  "acceptance_requires": false,
  "commands": [],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 37,
  "name": "DEPOT_V1",
  "constant": "depot::CAPABILITY_DEPOT_V1",
  "requires": [
    4
  ],
  "acceptance_requires": false,
  "commands": [
    36
  ],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 40,
  "name": "GUILD_V1",
  "constant": "guild::CAPABILITY_GUILD_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    40,
    41
  ],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 43,
  "name": "VIP_V1",
  "constant": "vip::CAPABILITY_VIP_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    48,
    49,
    50
  ],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": false,
    "item_view": false,
    "bundle_world": true
  }
}"#,
    r#"{
  "id": 49,
  "name": "HOUSE_V1",
  "constant": "house::CAPABILITY_HOUSE_V1",
  "requires": [],
  "acceptance_requires": false,
  "commands": [
    56,
    57
  ],
  "domains": [],
  "supersedes": [],
  "offers": {
    "production": true,
    "item_view": true,
    "bundle_world": true
  }
}"#,
]
