use super::*;
pub(super) const MAIN: [Row; 7] = [
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
pub(super) const BATTLE: [Row; 12] = [
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
pub(super) const UI: [Row; 4] = [
    check!(
        "ui_sound.interaction",
        "Interaction Sounds",
        audio.ui_sounds[0]
    ),
    check!("ui_sound.party", "Party Sounds", audio.ui_sounds[1]),
    check!("ui_sound.vip", "VIP Sounds", audio.ui_sounds[2]),
    check!("ui_sound.chat", "Chat Sounds", audio.ui_sounds[3]),
];
