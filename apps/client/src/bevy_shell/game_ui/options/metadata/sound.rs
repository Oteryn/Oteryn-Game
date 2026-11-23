use super::*;
pub(super) const MAIN: [Row; 8] = [
    check!("sound.anthem", "Play the Anthem", audio.anthem),
    Row {
        id: "sound.enabled",
        label: "Enable Sound",
        editor: Editor::Check {
            read: |p| !p.audio.muted,
            write: |p, v| p.audio.muted = !v,
        },
    },
    number!(
        "sound.master",
        "Master Volume (%)",
        0,
        100,
        u8,
        audio.master_volume
    ),
    number!(
        "sound.ambient",
        "Ambient Volume (%)",
        0,
        100,
        u8,
        audio.ambient_volume
    ),
    number!(
        "sound.music",
        "Music Volume (%)",
        0,
        100,
        u8,
        audio.channel_volumes[0]
    ),
    number!(
        "sound.item",
        "Item Volume (%)",
        0,
        100,
        u8,
        audio.channel_volumes[1]
    ),
    number!(
        "sound.event",
        "Event Volume (%)",
        0,
        100,
        u8,
        audio.channel_volumes[2]
    ),
    number!(
        "sound.ui",
        "UI Volume (%)",
        0,
        100,
        u8,
        audio.channel_volumes[3]
    ),
];
pub(super) const BATTLE: [Row; 18] = [
    check!(
        "battle_sound.creature_death",
        "Creature Death Sounds",
        audio.creature_death
    ),
    check!(
        "battle_sound.creature_noises",
        "Creature Noises",
        audio.creature_noises
    ),
    check!(
        "battle_sound.other_spells",
        "Other Players’ Spell Sounds",
        audio.other_spells
    ),
    check!(
        "battle_sound.other_weapons",
        "Other Players’ Weapon Sounds",
        audio.other_weapons
    ),
    check!(
        "battle_sound.own_spells",
        "Own Spell Sounds",
        audio.own_spells
    ),
    check!(
        "battle_sound.own_weapons",
        "Own Weapon Sounds",
        audio.own_weapons
    ),
    number!(
        "battle_sound.own_volume",
        "Own Sounds Volume (%)",
        0,
        100,
        u8,
        audio.battle_volumes[0]
    ),
    number!(
        "battle_sound.other_volume",
        "Other Players' Sounds Volume (%)",
        0,
        100,
        u8,
        audio.battle_volumes[1]
    ),
    number!(
        "battle_sound.creature_volume",
        "Creature Sounds Volume (%)",
        0,
        100,
        u8,
        audio.battle_volumes[2]
    ),
    check!(
        "battle_sound.own_attacks",
        "Own Attack Sounds",
        audio.battle_filters[0][0]
    ),
    check!(
        "battle_sound.own_healing",
        "Own Healing Sounds",
        audio.battle_filters[0][1]
    ),
    check!(
        "battle_sound.own_support",
        "Own Support Sounds",
        audio.battle_filters[0][2]
    ),
    check!(
        "battle_sound.other_attacks",
        "Other Players' Attack Sounds",
        audio.battle_filters[1][0]
    ),
    check!(
        "battle_sound.other_healing",
        "Other Players' Healing Sounds",
        audio.battle_filters[1][1]
    ),
    check!(
        "battle_sound.other_support",
        "Other Players' Support Sounds",
        audio.battle_filters[1][2]
    ),
    check!(
        "battle_sound.creature_attacks",
        "Creature Attack Sounds",
        audio.battle_filters[2][0]
    ),
    check!(
        "battle_sound.creature_healing",
        "Creature Healing Sounds",
        audio.battle_filters[2][1]
    ),
    check!(
        "battle_sound.creature_support",
        "Creature Support Sounds",
        audio.battle_filters[2][2]
    ),
];
pub(super) const UI: [Row; 14] = [
    check!("ui_sound.eating", "Eating Sounds", audio.eating),
    check!("ui_sound.global", "Global Messages", audio.global),
    check!("ui_sound.guild", "Guild Messages", audio.guild),
    check!(
        "ui_sound.move_item",
        "Item Movement Sounds",
        audio.move_item
    ),
    check!("ui_sound.npc", "NPC Messages", audio.npc),
    check!(
        "ui_sound.party_messages",
        "Party Messages",
        audio.party_messages
    ),
    check!("ui_sound.private", "Private Messages", audio.private),
    check!(
        "ui_sound.private_without_tab",
        "Private Messages Without a Tab",
        audio.private_without_tab
    ),
    check!("ui_sound.raid", "Raid Announcements", audio.raid),
    check!("ui_sound.system", "System Messages", audio.system),
    check!(
        "ui_sound.interaction",
        "Interaction Sounds",
        audio.ui_sounds[0]
    ),
    check!(
        "ui_sound.party",
        "Join/Leave Party Sounds",
        audio.ui_sounds[1]
    ),
    check!("ui_sound.vip", "VIP Sounds", audio.ui_sounds[2]),
    check!("ui_sound.chat", "Chat Sounds", audio.ui_sounds[3]),
];
