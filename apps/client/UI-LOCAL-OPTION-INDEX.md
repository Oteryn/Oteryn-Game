# Local client option index — source inventory, not completion

Base source: f8c68f183c51b6059935f916efda38fff30cb21c, apps/client/src/settings_catalog.rs.
One-time balanced macro extraction; 393 unique catalogue keys across27 sections.
The first20 are navigation families;7 others are compatibility/scaffold catalogue sections.
393 is not the count of visible original controls, nor a completed-function count.
Dynamic hotkey tables, reference-only/hidden rows and native special renderers do not
have one-to-one cardinality with this catalogue. No original settings values are included.

`implemented` and `stored` below describe source declarations ONLY. A serializer or
macro does not prove a working consumer. `pending` is not automatically a disabled
native widget: special hotkey/profile/help pages override generic catalogue rendering.
See UI-FIDELITY.md and OTV2-20261014-client-six-local-steps.md for routed consumer
proof, remaining gaps, source exceptions and native qualification. HUD/graphics
implementation is outside this owner's current scope; it is indexed to avoid omissions.

| Catalogue key | Declared kind | Declared consumer/field | Reference key |
| --- | --- | --- | --- |
| basic.basic.advanced | stored | settings discovery | — |
| basic.basic.click_walk | implemented | click_to_walk | — |
| basic.basic.ui_scale | implemented | ui_scale | — |
| controls.controls.turn_ctrl | pending | movement actions | — |
| controls.controls.turn_shift | pending | movement actions | — |
| controls.controls.turn_alt | pending | movement actions | — |
| controls.controls.complete_stack_ctrl | pending | item-move amount intent | — |
| controls.controls.mouse_preset | pending | input router | — |
| controls.controls.loot_button | pending | input router | — |
| controls.controls.movement | implemented | movement_keys | — |
| controls.controls.rotation | pending | movement actions | — |
| controls.controls.held_keys | pending | input router | — |
| controls.controls.drag_modifier | pending | inventory commands | — |
| controls.controls.face_movement | pending | movement presentation | alwaysTurnTowardsMoveDirection |
| controls.controls.default_key_delay | stored | movement repeat scheduler | keyboardDelayUseDefault |
| controls.controls.key_delay_ms | stored | movement repeat scheduler | keyboardDelayMs |
| controls.controls.drag_all | pending | item-move amount intent | dragAndDropDefaultActionIsMoveAll |
| general_hotkeys.hotkeys.profile | pending | binding profiles | — |
| general_hotkeys.hotkeys.add_profile | pending | binding profiles | — |
| general_hotkeys.hotkeys.copy_profile | pending | binding profiles | — |
| general_hotkeys.hotkeys.rename_profile | pending | binding profiles | — |
| general_hotkeys.hotkeys.remove_profile | pending | binding profiles | — |
| general_hotkeys.hotkeys.search | pending | action catalogue | — |
| general_hotkeys.hotkeys.primary | pending | binding profiles | — |
| general_hotkeys.hotkeys.secondary | pending | binding profiles | — |
| general_hotkeys.hotkeys.chat_on | pending | text/input routing | — |
| general_hotkeys.hotkeys.chat_off | pending | text/input routing | — |
| action_hotkeys.hotkeys.bottom_slots | pending | action bar | — |
| action_hotkeys.hotkeys.left_slots | pending | action bar | — |
| action_hotkeys.hotkeys.right_slots | pending | action bar | — |
| custom_hotkeys.hotkeys.custom_action | pending | action catalogue | — |
| custom_hotkeys.hotkeys.action_text | pending | supported action commands | — |
| custom_hotkeys.hotkeys.custom_binding | pending | supported action commands | — |
| interface.interface.highlight_mouse | stored | scene hover projection | — |
| interface.interface.big_cursor | stored | native/custom pointer renderer | — |
| interface.interface.link_copy_warning | pending | safe clipboard UI | — |
| interface.interface.expiry_inventory | pending | item expiry projection | — |
| interface.interface.expiry_containers | pending | item expiry projection | — |
| interface.interface.expiry_unused | pending | item expiry projection | — |
| interface.interface.colourise_loot_value | pending | item value presentation | — |
| interface.interface.english | implemented | english | — |
| interface.interface.contrast | implemented | high_contrast | — |
| interface.interface.reduced_motion | implemented | reduced_motion | — |
| interface.interface.cursor_size | pending | cursor integration | — |
| interface.interface.cursor_animation | pending | contextual interaction cursor | — |
| interface.interface.link_confirmation | pending | safe link routing | — |
| interface.interface.system_cursor | stored | native/custom pointer renderer | mouseSystemCursor |
| hud.arc_size_preset | stored | default | Rozmiar domyślny |
| hud.harmony_position | stored | health | Obok łuku zdrowia |
| hud.show_harmony | stored | own-resource HUD | — |
| hud.other_marks | pending | actor/condition projection | — |
| hud.status_bars | pending | actor/condition projection | — |
| hud.custom_status_bars | pending | actor/condition projection | — |
| hud.condition_poison_hud | pending | actor/condition projection | — |
| hud.condition_poison_bar | pending | actor/condition projection | — |
| hud.condition_burning_hud | pending | actor/condition projection | — |
| hud.condition_burning_bar | pending | actor/condition projection | — |
| hud.condition_electric_hud | pending | actor/condition projection | — |
| hud.condition_electric_bar | pending | actor/condition projection | — |
| hud.condition_bleeding_hud | pending | actor/condition projection | — |
| hud.condition_bleeding_bar | pending | actor/condition projection | — |
| hud.condition_agony_hud | pending | actor/condition projection | — |
| hud.condition_agony_bar | pending | actor/condition projection | — |
| hud.condition_powerless_hud | pending | actor/condition projection | — |
| hud.condition_powerless_bar | pending | actor/condition projection | — |
| hud.condition_rooted_hud | pending | actor/condition projection | — |
| hud.condition_rooted_bar | pending | actor/condition projection | — |
| hud.condition_feared_hud | pending | actor/condition projection | — |
| hud.condition_feared_bar | pending | actor/condition projection | — |
| hud.condition_drunk_hud | pending | actor/condition projection | — |
| hud.condition_drunk_bar | pending | actor/condition projection | — |
| hud.condition_magic_shield_hud | pending | actor/condition projection | — |
| hud.condition_magic_shield_bar | pending | actor/condition projection | — |
| hud.condition_monk_virtue_hud | pending | actor/condition projection | — |
| hud.condition_monk_virtue_bar | pending | actor/condition projection | — |
| hud.condition_slowed_hud | pending | actor/condition projection | — |
| hud.condition_slowed_bar | pending | actor/condition projection | — |
| hud.condition_haste_hud | pending | actor/condition projection | — |
| hud.condition_haste_bar | pending | actor/condition projection | — |
| hud.condition_logout_block_hud | pending | actor/condition projection | — |
| hud.condition_logout_block_bar | pending | actor/condition projection | — |
| hud.condition_drowning_hud | pending | actor/condition projection | — |
| hud.condition_drowning_bar | pending | actor/condition projection | — |
| hud.condition_freezing_hud | pending | actor/condition projection | — |
| hud.condition_freezing_bar | pending | actor/condition projection | — |
| hud.condition_dazzled_hud | pending | actor/condition projection | — |
| hud.condition_dazzled_bar | pending | actor/condition projection | — |
| hud.condition_cursed_hud | pending | actor/condition projection | — |
| hud.condition_cursed_bar | pending | actor/condition projection | — |
| hud.condition_strengthened_hud | pending | actor/condition projection | — |
| hud.condition_strengthened_bar | pending | actor/condition projection | — |
| hud.condition_pz_block_hud | pending | actor/condition projection | — |
| hud.condition_pz_block_bar | pending | actor/condition projection | — |
| hud.condition_in_pz_hud | pending | actor/condition projection | — |
| hud.condition_in_pz_bar | pending | actor/condition projection | — |
| hud.condition_resting_hud | pending | actor/condition projection | — |
| hud.condition_resting_bar | pending | actor/condition projection | — |
| hud.condition_lesser_hex_hud | pending | actor/condition projection | — |
| hud.condition_lesser_hex_bar | pending | actor/condition projection | — |
| hud.condition_intense_hex_hud | pending | actor/condition projection | — |
| hud.condition_intense_hex_bar | pending | actor/condition projection | — |
| hud.condition_greater_hex_hud | pending | actor/condition projection | — |
| hud.condition_greater_hex_bar | pending | actor/condition projection | — |
| hud.condition_goshnar_taint_hud | pending | actor/condition projection | — |
| hud.condition_goshnar_taint_bar | pending | actor/condition projection | — |
| hud.condition_bakragore_taint_hud | pending | actor/condition projection | — |
| hud.condition_bakragore_taint_bar | pending | actor/condition projection | — |
| hud.condition_yellow_skull_hud | pending | actor/condition projection | — |
| hud.condition_yellow_skull_bar | pending | actor/condition projection | — |
| hud.condition_party_mode_hud | pending | actor/condition projection | — |
| hud.condition_party_mode_bar | pending | actor/condition projection | — |
| hud.condition_white_skull_hud | pending | actor/condition projection | — |
| hud.condition_white_skull_bar | pending | actor/condition projection | — |
| hud.condition_red_skull_hud | pending | actor/condition projection | — |
| hud.condition_red_skull_bar | pending | actor/condition projection | — |
| hud.condition_black_skull_hud | pending | actor/condition projection | — |
| hud.condition_black_skull_bar | pending | actor/condition projection | — |
| hud.condition_orange_skull_hud | pending | actor/condition projection | — |
| hud.condition_orange_skull_bar | pending | actor/condition projection | — |
| hud.condition_guild_war_hud | pending | actor/condition projection | — |
| hud.condition_guild_war_bar | pending | actor/condition projection | — |
| hud.condition_hungry_hud | pending | actor/condition projection | — |
| hud.condition_hungry_bar | pending | actor/condition projection | — |
| hud.conditions_hud_enabled | pending | actor/condition projection | — |
| hud.conditions_bar_enabled | pending | actor/condition projection | — |
| hud.condition_order | pending | actor/condition projection | — |
| hud.hud.owner_name | stored | own-character HUD | — |
| hud.hud.other_names | pending | actor HUD | — |
| hud.hud.owner_health | stored | own-resource HUD | — |
| hud.hud.other_health | stored | visible-actor HUD | — |
| hud.hud.owner_mana | stored | own-resource HUD | — |
| hud.hud.other_mana | pending | supported resource projection | — |
| hud.hud.marks | pending | actor status projection | — |
| hud.hud.npc_icons | pending | NPC presentation | — |
| hud.hud.arcs | stored | own-resource HUD | — |
| hud.hud.arc_size | pending | actor HUD | — |
| hud.hud.arc_distance | stored | own-resource HUD | — |
| hud.hud.arc_opacity | stored | own-resource HUD | — |
| hud.hud.conditions_bar | pending | condition projection | — |
| hud.hud.conditions_world | pending | condition projection | — |
| hud.hud.conditions_order | pending | condition layout | — |
| hud.hud.status_placement | pending | HUD layout | — |
| hud.hud.status_customization | pending | HUD layout | — |
| hud.hud.player_enabled | stored | own-resource HUD | playerHudEnabled |
| hud.hud.creatures_enabled | stored | visible-actor HUD | creatureHudEnabled |
| hud.hud.resource_bars | stored | own-resource HUD | playerHudShowBars |
| hud.hud.cooldown_bar | pending | cooldown projection | cooldownBarEnabled |
| hud.hud.harmony_left | pending | vocation HUD | playerHudShowHarmonyLeft |
| hud.hud.serene_harmony | pending | vocation condition projection | playerShowHarmonySerene |
| console.console.visible | implemented | show_chat | — |
| console.console.font_size | stored | chat text renderer | — |
| console.console.timestamps | stored | received chat clock | — |
| console.console.seconds | stored | received chat clock | — |
| console.console.levels | pending | chat projection | — |
| console.console.own_status | pending | session events | — |
| console.console.other_status | pending | session events | — |
| console.console.events | pending | session events | — |
| console.console.information | pending | session events | — |
| console.console.private_tab | pending | chat presentation | — |
| console.console.join_leave | pending | chat membership projection | consoleShowJoinLeaveMessages |
| game_window.window.target_marking | stored | scene target projection | — |
| game_window.window.textual_effects | pending | effect projection | — |
| game_window.window.potion_sounds | pending | effect projection | — |
| game_window.window.fit | pending | scene layout | — |
| game_window.window.top_pane | pending | scene layout | — |
| game_window.window.bottom_pane | pending | scene layout | — |
| game_window.window.private_messages | pending | chat overlays | — |
| game_window.window.general_messages | pending | session overlays | — |
| game_window.window.own_spells | pending | effect projection | — |
| game_window.window.other_spells | pending | effect projection | — |
| game_window.window.hotkey_notices | pending | action feedback | — |
| game_window.window.loot_notices | pending | loot events | — |
| game_window.window.loot_highlight | pending | loot projection | — |
| game_window.window.training | pending | training events | — |
| game_window.window.combat_frame | pending | combat projection | — |
| game_window.window.pvp_frame | pending | PvP projection | — |
| game_window.window.attack_animation | pending | combat presentation | — |
| game_window.window.banner | pending | scene presentation | — |
| game_window.window.target_frame | pending | target projection | — |
| game_window.window.target_highlight | pending | target projection | — |
| game_window.window.potion_notices | pending | item-use notices | gameWindowShowPotionMessages |
| game_window.window.store_notices | pending | commercial notices | gameWindowShowStoreMessages |
| game_window.window.boosted_creatures | pending | world notices | gameWindowShowBoostedCreatureMessages |
| action_bars.bars.bottom_rows | implemented | action_bottom_rows | — |
| action_bars.bars.left_rows | implemented | action_left_rows | — |
| action_bars.bars.right_rows | implemented | action_right_rows | — |
| action_bars.bars.locked | pending | action bar layout | — |
| action_bars.bars.labels | stored | action-bar renderer | — |
| action_bars.bars.item_amounts | pending | action-bar assignment source | — |
| action_bars.bars.spell_parameters | pending | spell actions | — |
| action_bars.bars.graphic_cooldown | pending | cooldown projection | — |
| action_bars.bars.numeric_cooldown | pending | cooldown projection | — |
| action_bars.bars.tooltips | pending | action-bar assignment source | — |
| action_bars.bars.auto_spells | pending | spell catalogue | — |
| action_bars.bars.clear_row | pending | action bar layout | — |
| action_bars.bars.bottom_visible | implemented | action_bottom_enabled | — |
| action_bars.bars.left_visible | implemented | action_left_enabled | — |
| action_bars.bars.right_visible | implemented | action_right_enabled | — |
| action_bars.bars.bottom_locked | pending | action bar layout | actionBarBottomLocked |
| action_bars.bars.left_locked | pending | action bar layout | actionBarLeftLocked |
| action_bars.bars.right_locked | pending | action bar layout | actionBarRightLocked |
| action_bars.bars.bottom_first | pending | action bar layout | actionBarShowBottom1 |
| action_bars.bars.bottom_second | pending | action bar layout | actionBarShowBottom2 |
| action_bars.bars.bottom_third | pending | action bar layout | actionBarShowBottom3 |
| action_bars.bars.left_first | pending | action bar layout | actionBarShowLeft1 |
| action_bars.bars.left_second | pending | action bar layout | actionBarShowLeft2 |
| action_bars.bars.left_third | pending | action bar layout | actionBarShowLeft3 |
| action_bars.bars.right_first | pending | action bar layout | actionBarShowRight1 |
| action_bars.bars.right_second | pending | action bar layout | actionBarShowRight2 |
| action_bars.bars.right_third | pending | action bar layout | actionBarShowRight3 |
| shortcuts.shortcuts.visible | pending | panel registry | — |
| shortcuts.shortcuts.available | pending | panel registry | — |
| shortcuts.shortcuts.add | pending | panel registry | — |
| shortcuts.shortcuts.remove | pending | panel registry | — |
| shortcuts.shortcuts.order | pending | panel layout | — |
| graphics.graphics.fullscreen | implemented | fullscreen | — |
| graphics.graphics.width | implemented | window_width | — |
| graphics.graphics.height | implemented | window_height | — |
| graphics.graphics.vsync | implemented | vsync | — |
| graphics.graphics.fps | implemented | fps | — |
| graphics.graphics.background_fps | implemented | background_fps | — |
| graphics.graphics.engine | pending | renderer capabilities | — |
| graphics.graphics.antialiasing | pending | whole-scene resampler | — |
| graphics.graphics.integer_scale | stored | scene viewport | — |
| graphics.graphics.monitor | pending | window integration | — |
| graphics.graphics.fps_indicator | stored | frame diagnostics overlay | — |
| graphics.graphics.latency_indicator | pending | session diagnostics | — |
| effects.effects.ambient_light | stored | scene lighting overlay | — |
| effects.effects.level_separator | pending | floor projection | — |
| effects.effects.cloud_light | pending | lighting renderer | — |
| effects.effects.own_opacity | pending | effect renderer | — |
| effects.effects.other_opacity | pending | effect renderer | — |
| effects.effects.creature_opacity | pending | effect renderer | — |
| effects.effects.boss_opacity | pending | effect renderer | — |
| effects.effects.lighting | stored | scene lighting overlay | lightEffectsEnabled |
| sound.sound.device | pending | audio backend | — |
| sound.sound.master | pending | audio mixer | — |
| sound.sound.music | pending | audio mixer | — |
| sound.sound.ambience | pending | audio mixer | — |
| sound.sound.items | pending | audio mixer | — |
| sound.sound.eating | pending | item-use audio | — |
| sound.sound.item_movement | pending | inventory audio | — |
| sound.sound.events | pending | audio mixer | — |
| sound.sound.background_mute | pending | audio mixer | — |
| sound.sound.anthem | pending | audio event policy | soundAnthemEnabled |
| battle_sounds.sound.own_combat | pending | combat audio | — |
| battle_sounds.sound.other_combat | pending | combat audio | — |
| battle_sounds.sound.creature_combat | pending | combat audio | — |
| battle_sounds.sound.attack_spells | pending | spell audio | — |
| battle_sounds.sound.healing_spells | pending | spell audio | — |
| battle_sounds.sound.support_spells | pending | spell audio | — |
| battle_sounds.sound.weapons | pending | combat audio | — |
| battle_sounds.sound.other_attack_spells | pending | spell audio | — |
| battle_sounds.sound.other_healing_spells | pending | spell audio | — |
| battle_sounds.sound.other_support_spells | pending | spell audio | — |
| battle_sounds.sound.other_weapons | pending | combat audio | — |
| battle_sounds.sound.creature_attacks | pending | creature audio | — |
| battle_sounds.sound.creature_noise | pending | creature audio | — |
| battle_sounds.sound.creature_death | pending | creature audio | — |
| battle_sounds.sound.own_spells | pending | combat audio | soundBattleOwnSpellsEnabled |
| battle_sounds.sound.other_spells | pending | combat audio | soundBattleOthersSpellsEnabled |
| ui_sounds.sound.ui | pending | UI audio | — |
| ui_sounds.sound.interactions | pending | UI audio | — |
| ui_sounds.sound.party_notice | pending | social audio | — |
| ui_sounds.sound.vip_notice | pending | social audio | — |
| ui_sounds.sound.chat_channels | pending | chat audio filters | — |
| ui_sounds.sound.chat_enabled | pending | chat audio | ChatEnabled |
| ui_sounds.sound.global | pending | chat audio | GlobalMessagesEnabled |
| ui_sounds.sound.guild | pending | chat audio | GuildMessagesEnabled |
| ui_sounds.sound.npc | pending | chat audio | NpcMessageseEnabled |
| ui_sounds.sound.party_chat | pending | chat audio | PartyMessagesEnabled |
| ui_sounds.sound.private | pending | chat audio | PrivateMessagesEnabled |
| ui_sounds.sound.private_without_tab | pending | chat audio | PrivateMessagesWithoutTabEnabled |
| ui_sounds.sound.raids | pending | world-event audio | RaidMessagesEnabled |
| ui_sounds.sound.system | pending | session audio | SystemMessagesEnabled |
| ui_sounds.sound.team_finder | pending | social audio | TeamFinderMessagesEnabled |
| miscellaneous.buy_confirmation | pending | accepted command/session policy | — |
| miscellaneous.stow_confirmation | pending | accepted command/session policy | — |
| miscellaneous.sort_confirmation | pending | accepted command/session policy | — |
| miscellaneous.move_confirmation | pending | accepted command/session policy | — |
| miscellaneous.stay_logged_in | pending | accepted command/session policy | — |
| miscellaneous.optimise_connection | pending | accepted command/session policy | — |
| miscellaneous.misc.confirmations | pending | safe command UI | — |
| miscellaneous.misc.session_retention | pending | accepted session policy | — |
| miscellaneous.misc.connection_stability | pending | accepted transport policy | — |
| miscellaneous.misc.quick_login | pending | approved device-session contract | — |
| gameplay.gameplay.inspect_permission | pending | inspection authority | — |
| gameplay.gameplay.cancel_chase | pending | combat commands | — |
| gameplay.gameplay.nearby_corpses | pending | accepted loot commands | — |
| screenshots.auto | pending | capture event pipeline | — |
| screenshots.level | pending | capture event pipeline | — |
| screenshots.skill | pending | capture event pipeline | — |
| screenshots.bestiary_unlocked | pending | capture event pipeline | — |
| screenshots.bestiary_completed | pending | capture event pipeline | — |
| screenshots.death_pve | pending | capture event pipeline | — |
| screenshots.death_pvp | pending | capture event pipeline | — |
| screenshots.player_kill | pending | capture event pipeline | — |
| screenshots.player_assist | pending | capture event pipeline | — |
| screenshots.player_attack | pending | capture event pipeline | — |
| screenshots.gift_of_life | pending | capture event pipeline | — |
| screenshots.capture.manual | pending | renderer capture | — |
| screenshots.capture.folder | pending | capture storage | — |
| screenshots.capture.game_only | stored | native renderer viewport crop | — |
| screenshots.capture.backlog | pending | bounded capture buffer | — |
| screenshots.capture.progression | pending | progression events | — |
| screenshots.capture.achievements | pending | achievement events | — |
| screenshots.capture.bestiary | pending | bestiary events | — |
| screenshots.capture.treasure | pending | treasure events | — |
| screenshots.capture.loot | pending | loot events | — |
| screenshots.capture.boss | pending | boss events | — |
| screenshots.capture.death | pending | death events | — |
| screenshots.capture.pvp | pending | PvP events | — |
| screenshots.capture.damage | pending | combat events | — |
| screenshots.capture.healing | pending | combat events | — |
| screenshots.capture.low_health | pending | vitals projection | — |
| screenshots.capture.gift_of_life | pending | accepted resurrection mechanic | — |
| screenshots.capture.auto_enabled | pending | capture events | screenshotsAutoScreenshotsEnabled |
| screenshots.capture.skill_up | pending | progression events | screenshotsSkillUpEnabled |
| screenshots.capture.bestiary_partial | pending | bestiary events | screenshotsBestiaryPartlyEnabled |
| screenshots.capture.bestiary_full | pending | bestiary events | screenshotsBestiaryFullEnabled |
| screenshots.capture.death_pve | pending | death events | screenshotsDeathPvEEnabled |
| screenshots.capture.death_pvp | pending | death events | screenshotsDeathPvPEnabled |
| screenshots.capture.pvp_attack | pending | PvP events | screenshotsPvPAttackEnabled |
| screenshots.capture.player_kill | pending | PvP events | screenshotsPlayerKillFullEnabled |
| screenshots.capture.player_kill_assist | pending | PvP events | screenshotsPlayerKillAssistEnabled |
| screenshots.capture.reward_chest | pending | reward events | screenshotsRewardChestEnabled |
| help.help.client_help | pending | safe link routing | — |
| help.help.compendium | pending | safe link routing | — |
| help.help.rule_violations | pending | safe link routing | — |
| help.help.manual | pending | safe link routing | — |
| help.help.faq | pending | safe link routing | — |
| help.help.info | pending | local client information | — |
| help.help.export_all_options | pending | credential-free preference export | — |
| help.help.documentation | pending | safe link routing | — |
| help.help.import_options | pending | validated preference import | — |
| help.help.export_options | pending | credential-free preference export | — |
| help.help.import_minimap | pending | validated minimap import | — |
| help.help.export_minimap | pending | minimap export | — |
| help.help.reset_options | pending | explicit local confirmation | — |
| help.help.reset_minimap | pending | explicit local confirmation | — |
| panels.panels.inventory | implemented | show_inventory | — |
| panels.panels.battle | implemented | show_battle | — |
| panels.panels.minimap | implemented | show_minimap | — |
| panels.panels.skills | pending | skills projection | — |
| panels.panels.quest_tracker | pending | quest projection | — |
| panels.panels.layout | pending | panel layout | — |
| panels.panels.battle_sort | pending | battle presentation | — |
| panels.panels.battle_filters | pending | battle presentation | — |
| panels.panels.minimap_layers | pending | minimap projection | — |
| panels.panels.minimap_filters | pending | minimap projection | — |
| containers.containers.loot_assignment | pending | accepted inventory/loot commands | — |
| containers.containers.obtain_assignment | pending | accepted inventory commands | — |
| containers.containers.main_fallback | pending | accepted inventory commands | — |
| containers.containers.accepted_loot | pending | accepted loot preferences | — |
| containers.containers.skipped_loot | pending | accepted loot preferences | — |
| containers.containers.loot_search | pending | item catalogue | — |
| containers.containers.loot_valuation | pending | accepted item valuation | — |
| containers.containers.track_drop | pending | drop tracker | — |
| containers.containers.skip_item | pending | accepted loot preferences | — |
| containers.containers.backpacks_first | pending | container presentation | containerSortBackpacksFirst |
| containers.containers.sort_recursive | pending | container presentation | containerSortRecursive |
| containers.containers.confirm_recursive_sort | pending | container confirmation | containerSortRecursiveShowWarningAgain |
| containers.containers.managed_recursive | pending | accepted managed-container intents | containerMoveToManagedContainerRecursive |
| containers.containers.confirm_managed_move | pending | container confirmation | containerMoveToManagedContainerRecursiveShowWarningAgain |
| containers.containers.expiry | pending | item expiry projection | showExpireInContainers |
| containers.containers.inventory_expiry | pending | item expiry projection | showExpireInInventory |
| containers.containers.unused_expiry | pending | item expiry projection | showExpireWhenUnused |
| containers.containers.confirm_stash | pending | stash confirmation | stashAskBeforeStowContainerContent |
| containers.containers.sort_order | pending | container presentation | — |
| character_selection.characters.hidden | pending | character-list presentation | characterSelectionShowHidden |
| character_selection.characters.outfits | pending | character appearance projection | characterSelectionShowOutfits |
| character_selection.characters.sort_column | pending | character-list presentation | characterSelectionSortColumn |
| character_selection.characters.sort_ascending | pending | character-list presentation | characterSelectionSortAscendingOrder |
| market.market.locker_only | pending | market item eligibility | marketShowLockerOnly |
| floating_messages.floating.enabled | pending | scene text renderer | — |
| floating_messages.floating.own_messages | pending | message projection | — |
| floating_messages.floating.other_messages | pending | message projection | — |
| analyzers.analyzers.session_values | pending | session analyzers | — |
| analyzers.analyzers.per_hour | pending | session analyzers | — |
| analyzers.analyzers.damage_types | pending | combat analyzers | — |
| analyzers.analyzers.damage_sources | pending | combat analyzers | — |
| analyzers.analyzers.damage_graph | pending | combat analyzers | — |
| analyzers.analyzers.gauges | pending | analyzer presentation | — |
| analyzers.analyzers.graphs | pending | analyzer presentation | — |
| analyzers.analyzers.raw_xp | pending | XP analyzer | — |
| analyzers.analyzers.party_valuation | pending | party analyzer | — |
| analyzers.analyzers.boss_sort | pending | boss cooldown projection | — |
| analyzers.analyzers.reset | pending | explicit local confirmation | — |
| analyzers.analyzers.reset_high | pending | explicit local confirmation | — |
| analyzers.analyzers.copy | pending | clipboard privacy | — |
| notifications.notifications.private | pending | chat notification events | — |
| notifications.notifications.party | pending | party notification events | — |
| notifications.notifications.vip | pending | social notification events | — |
