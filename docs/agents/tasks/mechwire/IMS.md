# Mechwire inventory: I, M, S (main 60e40aae)

Path roots, used in place of the full paths:
- `B` = apps/client/src/bevy_shell
- `F` = apps/client/src/features (shared native code, reached from B)
- `CR` = crates/client-runtime/src/features
- `CU` = crates/client-ui/src/features
- `P` = crates/protocol-oteryn/src
- `GT` = crates/gameplay/dispatch/src/gameplay_transport
- `R` = GT/connection/routing
- `SES` = crates/session/src

Reducers are the ones the live loop `apps/client/src/play/session_loop.rs` uses. `session_loop_run_session_1.rs` is not mounted. Only Bevy reachability counts; windows_shell-only callers count as "not sent".

| row | mechanic | status | client action | protocol | server handler | client reducer | note |
|---|---|---|---|---|---|---|---|
| M01 | Paced step, autowalk, stop, turn, follow | BOTH-WIRED | B/controls.rs:151 → F/0013-paced-movement/input.rs:7; B/controls.rs:68 → F/0064-target-move/input.rs:11 | P/world_spatial.rs:195 (cmd 1); P/target_move.rs:18 (cap 64 / cmd 81) | GT/connection/serve/command.rs:272; GT/connection/serve/target_move.rs:35 | apps/client/src/play/tick.rs:27; F/0064-target-move/protocol.rs:9; CR/0064-target-move/protocol.rs:18 | |
| M02 | Far use/move walks into range | BOTH-WIRED | B/gameplay_dialogs.rs:133 → F/0011-world-interaction/input.rs:68; F/0004-item-view-move/input.rs:36 | CR/0011-world-interaction/protocol.rs:112 (cmd 2); P/item_view.rs:723 (cmd 9) | GT/connection/serve/command.rs:313 → GT/connection/serve/autowalk.rs:15 | CR/0011-world-interaction/protocol.rs:166 | |
| M03 | Stairs/teleport floor change, exhaust | BOTH-WIRED | B/controls.rs:151 (step) | P/world_spatial.rs:195 (cmd 1) | GT/actor_movement.rs:261 | crates/session/src/domain_dispatch.rs:153 | effect of a step |
| M04 | Push creatures/players | BOTH-WIRED | B/controls.rs:166 → F/0082-creature-push/input.rs:9; CU/0082-creature-push/mod.rs:7 | P/creature_push.rs:62 (cap 82 / cmd 98) | GT/connection/serve/creature_push.rs:20 → GT/creature_push.rs:16 | F/0082-creature-push/protocol.rs:7 | |
| M05 | Doors, unlocked key doors | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (USE) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | R/use_item.rs:13 → GT/composed_admission/objects.rs:101 → GT/map_door.rs:177 | SES/domain_dispatch.rs:149 | |
| M06 | Keys, level/quest doors | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (use-with) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/map_door.rs:231 | SES/domain_dispatch.rs:149 | |
| M07 | Rope-spot ascent | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (use-with) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/item_craft.rs:116 → GT/item_craft/rope.rs:9 | SES/domain_dispatch.rs:153 | |
| M08 | Shovel, machete and other tools | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (use-with) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/item_craft.rs:108 | SES/domain_dispatch.rs:149 | |
| M09 | Fishing, skinning | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (use-with) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/item_craft.rs:271; GT/item_craft.rs:70 | SES/domain_dispatch.rs:386 | |
| M10 | Swimming, drowning, traps | BOTH-WIRED | B/controls.rs:151 (step) | P/world_spatial.rs:195 (cmd 1) | GT/paced_movement/underwater_drown.rs:92 | SES/domain_dispatch.rs:245 | server effect of a step |
| M11a | Levitate | BOTH-WIRED | B/game_ui/chat/commands.rs:20 → F/0007-chat/input.rs:60 | P/chat.rs:309 (cmd 13, Say) | GT/composed_admission/spells.rs:251 → GT/parameter_cast.rs:566 | SES/domain_dispatch.rs:153 | spoken only; the spell-control input is cfg(any(windows,test)) |
| M11b | Magic Rope | BOTH-WIRED | B/game_ui/chat/commands.rs:20 → F/0007-chat/input.rs:60 | P/chat.rs:309 (cmd 13) | GT/composed_admission/spells.rs:251 → crates/gameplay/spell/src/spell/world_execution.rs:405 | SES/domain_dispatch.rs:153 | spoken only |
| I01 | Containers, nesting, capacity, stacks | BOTH-WIRED | B/game_ui/inventory/actions.rs:169,196,214 → F/0014-container-tree/input.rs:44; F/0004-item-view-move/input.rs:45 | P/container_tree.rs:466 (cap 14 / cmd 21); P/item_view.rs:723 (cmd 9) | R/container_view.rs:10; R/item_move.rs:13 | SES/domain_dispatch.rs:139,386,407 | |
| I03 | Look, signs/books, spellbook | BOTH-WIRED | CR/0011-world-interaction/protocol.rs:106 | P/capability_tables.rs:7 (cap 28 / cmd 30); spellbook cap 449 / cmd 733 | R/item_look.rs:11 → GT/item_look.rs:190 | CR/0011-world-interaction/protocol.rs:151 | |
| I04 | Writing text, write-once | SERVER-ONLY | | F/0115-mail/protocol.rs:274 (cap 115 / cmd 124 WriteText) | R/mail.rs:6 | | only apps/client/src/windows_shell/input.rs:151 sends it |
| I05 | Timed items, charges | SERVER-ONLY | | P/timed_item.rs:10 (cap 39 / cmd 38) | R/timed_item.rs:10; GT/timed_item_clock.rs:47 | SES/domain_dispatch.rs:386 | passive ticking reaches the client; the Inspect/Use/Repair request is windows-only |
| I06 | Corpse decay | BOTH-WIRED | | P/world_map.rs:749 (domain 17) | GT/corpse_decay.rs:9 | SES/domain_dispatch.rs:149 | server push; links 2-4 only |
| I07 | Quick loot, filters, pouch | BOTH-WIRED | B/game_ui/quick_loot.rs:124 → F/0098-quick-loot/view.rs:49 | cap/cmd 114 | R/quick_loot.rs:10 → GT/quick_loot.rs:46 | F/0098-quick-loot/protocol.rs:61 | |
| I08 | Primal Bag | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (USE) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/reward_bag_use.rs:17 | SES/domain_dispatch.rs:386 | |
| I09 | Furniture rotate | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (USE) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/composed_admission/objects.rs:192 → GT/item_transform.rs:16 | SES/domain_dispatch.rs:149 | the explicit rotate (cap 445 / cmd 730, R/house_rotation.rs:5) is not sent from Bevy |
| I10 | Wrap/unwrap, kits, fluids | SERVER-ONLY | | cap 207 / cmd 402 | R/house_wrap.rs:5 | | kits and fluids work via USE (BOTH-WIRED); wrap/unwrap is not sent from Bevy |
| I11 | Food crafting, stations | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (use-with) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/item_craft.rs:47 | SES/domain_dispatch.rs:386 | |
| I12 | Pick refining | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (use-with) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/mining/mod.rs:26 | SES/domain_dispatch.rs:386 | |
| I13 | Reward bag | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (USE) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/reward_bag_use.rs:88 | SES/domain_dispatch.rs:386 | |
| I14 | Potions, foods, casks | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (USE); action-bar Use | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | R/use_item.rs:13 | SES/domain_dispatch.rs:245 | |
| I15 | Energy Ring mana shield | BOTH-WIRED | F/0004-item-view-move/input.rs:45 (equip via move) | P/item_view.rs:723 (cmd 9) | GT/equipment_effects_owner.rs:68 | SES/domain_dispatch.rs:245 | |
| I16 | Item inspection | BOTH-WIRED | B/game_ui/item_details.rs:211 | cmd 153 | R/inspection.rs:6 → GT/inspection.rs:53 | F/0098-quick-loot/protocol.rs:61 | |
| I17a | Hotkey equip by id (re-audit :99) | BOTH-WIRED | B/game_ui/actions/editor.rs:337 → F/local-spell-action-bar/assignments.rs:371 | CU/0179-hotkey-equip/protocol.rs:8 (cap 179 / cmd 348) | R/hotkey_equip.rs:16 → GT/hotkey_equip.rs:172 | F/0179-hotkey-equip/protocol.rs:17 | re-audit MISSING is stale |
| I17b | Hotkey toggle unequip (:100) | BOTH-WIRED | F/local-spell-action-bar/assignments.rs:371 | CU/0179-hotkey-equip/protocol.rs:8 | GT/hotkey_equip.rs:470 | F/0179-hotkey-equip/protocol.rs:17 | stale |
| I17c | Two-hand/shield swap, ammo to quiver (:101) | BOTH-WIRED | F/local-spell-action-bar/assignments.rs:371 | CU/0179-hotkey-equip/protocol.rs:8 | GT/hotkey_equip.rs:341 | F/0179-hotkey-equip/protocol.rs:17 | quiver is present; no two-hand/shield auto-swap found (server gap) |
| I17d | Ring/necklace delay, feared gate (:102) | BOTH-WIRED | F/local-spell-action-bar/assignments.rs:371 | CU/0179-hotkey-equip/protocol.rs:8 | GT/hotkey_equip.rs:190,485,517 | F/0179-hotkey-equip/protocol.rs:17 | stale |
| I18a | Browse field open (:103) | SERVER-ONLY | | cap 230 / cmd 440 | R/browse_field.rs:11 | | F/0230-browse-field/input.rs:4 has no Bevy caller |
| I18b | Browse field move in/out (:104) | SERVER-ONLY | | cmd 440 / cmd 9 | R/browse_field.rs:11 | | no browse-field window in Bevy |
| I18c | Up from ground container (:105) | SERVER-ONLY | | cmd 440 | R/browse_field.rs:11 | | F/0014-container-tree/input.rs:57 needs a parent_handle; F/0230-browse-field/up.rs:13 is not reached |
| I19 | Shrine enchanting (:106) | BOTH-WIRED | F/0011-world-interaction/input.rs:68 (use-with) | CR/0011-world-interaction/protocol.rs:112 (cmd 2) | GT/item_craft.rs:86 → GT/item_craft/enchanting.rs:58 | SES/domain_dispatch.rs:386 | stale |
| S01 | Party invite, leader, shared XP | BOTH-WIRED | B/game_ui/party.rs:185; B/controls.rs:134 | cap 42, cmds 45/46; CR/0042-party/protocol.rs:17 | R/party.rs:8; GT/party_xp_command.rs:18 | F/0042-party/protocol.rs:42 | |
| S02 | Party shields, skull exemption | BOTH-WIRED | apps/client/src/play/tick.rs:71 → F/0134-party-skull/input.rs:43 | cap 134 / cmd 155 | R/party_skull.rs:5 | apps/client/src/play/session_loop.rs:82 (properties_result) | periodic query, not a player action |
| S03 | Guild, ranks, MOTD, bank, wars | SERVER-ONLY | B/game_ui/chat/widgets.rs:219 (guild chat only) | cmds 40/41/68/69/119 | R/guild.rs:8 → GT/guild_intent.rs:24 | | F/0040-guild/input.rs:5,13 is reached only from windows_shell |
| S04a | VIP list, presence | BOTH-WIRED | B/controls.rs:107; B/ui/toolbar.rs:43 → F/0043-vip/input.rs:9 | cap 43, cmds 48-50 | R/vip.rs:8 | SES/domain_dispatch.rs:310 | |
| S04b | PMs, private channels | BOTH-WIRED | CU/0007-chat/protocol.rs:62 | P/chat.rs:309 (cmd 13) | R/chat.rs:8 | SES/domain_dispatch.rs:288 | |
| S04c | Public channels, mute | BOTH-WIRED | CU/0007-chat/protocol.rs:61 | P/chat.rs:309 (cmd 13) | R/chat.rs:8 | SES/domain_dispatch.rs:340,351 | |
| S05 | House auction/rent/ACL, move-out, guildhall | BOTH-WIRED | B/game_ui/feature_dialogs/text_window.rs:179 → CR/0049-house/protocol.rs:81,106 | cap 49, cmds 56/57 | R/house.rs:8 → GT/house_intent.rs:12 | CR/0049-house/protocol.rs:106 | move-out (cmds 76/77), transfer (cmd 102) and guildhall (cmd 115) are not sent from Bevy |
| S06 | Outfit/addons/mounts | BOTH-WIRED | B/game_ui/outfit.rs:125 | cap 25 / cmd 25 | R/appearance.rs:9 → GT/appearance_change.rs:98 | SES/domain_dispatch.rs:150 | |
| S07 | Beds, sleep/wake | CLIENT-ONLY | CR/0011-world-interaction/protocol.rs:126 (Action::Bed) | cap 11 / cmd 31 | R/bed.rs:11 → GT/bed_sleep.rs:83 | | production prepare_bed_sleep returns None (GT/connection/authority.rs:773) |
| S08 | PvP skulls, secure and expert modes | BOTH-WIRED | B/game_ui/hud.rs:229 | P/attack.rs:254 (cmd 11); P/attack.rs:277 (cmd 12, secure :98) | R/combat_intent.rs:10,32 | SES/domain_dispatch.rs:261,278 | expert modes (cap 87 / cmd 103) are not sent from Bevy |
| S09 | Logout, PZ, NOLOGOUT | BOTH-WIRED | B/controls.rs:160; B/gameplay_dialogs.rs:123 → F/0021-logout/input.rs:5 | P/logout.rs:68 (cmd 23) | GT/connection/serve/route.rs:81 → GT/logout.rs:99 | apps/client/src/play/tick.rs:46 | |
| S10 | NPC speech, travel, Rashid | BOTH-WIRED | B/controls.rs:99 → F/0003-npc-service/input.rs:67 | P/npc_service.rs:272 (cmd 7) | R/npc.rs:11 → GT/paid_travel.rs:93 | F/0003-npc-service/protocol.rs:33 | |
| S11 | Highscores | BOTH-WIRED | B/controls.rs:115 → F/0009-highscores/input.rs:5 | P/highscores.rs:158 (cmd 14) | R/highscores.rs:12 | F/0009-highscores/protocol.rs:8 | |
| S13a | Party spells: gate and range | BOTH-WIRED | B/game_ui/chat/commands.rs:20 (spoken) | P/chat.rs:309 (cmd 13); cmd 3 | R/spell.rs:10; GT/ordinary_combat/prepare.rs:113 | SES/domain_dispatch.rs:245 | |
| S13b | Party spells: mana cost | BOTH-WIRED | B/game_ui/chat/commands.rs:20 | P/chat.rs:309 | crates/gameplay/spell/src/spell/party.rs:76 | SES/domain_dispatch.rs:245 | |
| S13c | Party spells: member buffs | BOTH-WIRED | B/game_ui/chat/commands.rs:20 | P/chat.rs:309 | GT/ordinary_combat.rs:335 | SES/domain_dispatch.rs:245 | |
| S15a | Fresh-login protection 10 s (:169) | BOTH-WIRED | | | GT/composed_admission/admission.rs:638 → crates/gameplay/entities/src/carrier/runtime_actor_reentry.rs:21; GT/attack.rs:864 | | server-internal, no wire message; re-audit MISSING is stale |
| S15b | Reconnect protection | BOTH-WIRED | | | crates/gameplay/entities/src/carrier/runtime_actor_reentry.rs:43; GT/attack.rs:864 | | server-internal, no wire message |
| S16a | Oressa first vocation choice | BOTH-WIRED | B/controls.rs:99 → F/0003-npc-service/input.rs:67 | P/npc_service.rs:272 (cmd 7) | R/npc.rs:11 → GT/npc_command.rs:151 → GT/vocation_choice.rs:282 | F/0003-npc-service/protocol.rs:33 | |
| S16b | Skill/ML conversion on change | BOTH-WIRED | F/0003-npc-service/input.rs:67 | P/npc_service.rs:272 | crates/gameplay/durability/src/durability/character_build.rs:316 | SES/domain_dispatch.rs:245 | |
| S18 | Say/whisper/yell, range, yell gate, mute | BOTH-WIRED | B/game_ui/chat/commands.rs:20 → F/0007-chat/input.rs:60 → CU/0007-chat/protocol.rs:5 | P/chat.rs:309 (cmd 13; ChatSpeechMode P/chat.rs:68) | R/chat.rs:8 | SES/domain_dispatch.rs:281 | |
| S16c | Dawnport trial tiles (:173) | SKIPPED | | | | | Dawnport, owner-excluded |
| S16d | Dawnport vocation room (:174) | SKIPPED | | | | | Dawnport |
| S17a | Livestream caster (:175) | SKIPPED | | | | | S17 Canary-only, excluded |
| S17b | Livestream viewer (:176) | SKIPPED | | | | | S17 |
| S17c | Livestream chat (:177) | SKIPPED | | | | | S17 |
| S14a | Team Finder leader listing (:166) | SERVER-ONLY | | cap 200 / cmds 390/391 | R/team_finder_leader.rs:8 | | Bevy opens panel 232 (F/0098-quick-loot/input.rs:8) but never sends |
| S14b | Team Finder browse/join (:167) | SERVER-ONLY | | cmds 444-446 | R/team_finder_members.rs:7 | | team_finder_control has no Bevy caller |
| S14c | Team Finder accept/deny (:168) | SERVER-ONLY | | cmds 444-446 | R/team_finder_members.rs:7 | | as S14b |
| S19 | Typing indicator (:179) | BOTH-WIRED | F/0007-chat/input.rs:67 (Bevy composer) | cap 233 / cmd 447 | R/player_typing.rs:6 | SES/domain_dispatch.rs:327 | re-audit MISSING is stale |

## Client gaps
- I04: add a Bevy text/edit window that sends WriteText (cmd 124). Today only apps/client/src/windows_shell/input.rs:151 reaches F/0115-mail/protocol.rs:274.
- I05: add a Bevy caller for timed_item_request (cap 39 / cmd 38).
- I09: add the explicit rotate command (cap 445 / cmd 730). USE already covers rotation.
- I10: add wrap/unwrap (cap 207 / cmd 402) to the Bevy context menu.
- I18a-c: add a Bevy browse-field window and "up" from a ground container. Wire F/0230-browse-field/input.rs:4,18,37 and up.rs:13 into Bevy; F/0014-container-tree/input.rs:57 needs a parent_handle.
- S03: add a guild panel, bank and war controls in Bevy. F/0040-guild/input.rs:5,13 is windows-only.
- S05: add Bevy controls for house move-out (cmds 76/77), transfer (cmd 102) and guildhall (cmd 115).
- S08: add a Bevy control for PvP expert modes (cap 87 / cmd 103).
- S14a-c: make the Bevy Team Finder panel send the leader commands (390/391) and the member commands (444-446).

## Server gaps
- S07: production prepare_bed_sleep is a closed default returning None (GT/connection/authority.rs:773-781), so the client's bed use (cmd 31) is always refused. Only test implementations exist.
- I17c: no two-hand/shield auto-swap found in GT/hotkey_equip.rs. Only ammo-to-quiver (:341-424) exists.
