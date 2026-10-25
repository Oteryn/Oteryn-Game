# Oteryn Game server mechanic parity audit — 2026-10-25

Status: research audit (`PARITY-AUDIT-1`). This is not architecture acceptance, not implementation authority and not an allocation. Scope is server only; client rendering is excluded. Oteryn snapshot: `main` at `d7dddb38` (after #2457). Primary reference: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`. Secondary reference: Crystal Server `ff7ede593c69d4c658b382c97443e8155926924a`. Reference repositories were read as data only, and none of their scripts were run.

This audit refreshes [the parity inventory](OTERYN_GAME_CANARY_PARITY_INVENTORY_2026-10-25.md) §7 against what has merged since it was taken at `865c9231`. It reads the "Excluded scope" sections of #2436–#2457 and the open PRs #2456, #2458 and #2459, and it applies [the NIGHT-W1 owner decisions](OTERYN_GAME_NIGHT_W1_OWNER_DECISIONS_2026-10-25.md). An owner decision overrides Canary. Soul War (#2269, #1867, `codex/soul-war-*`) belongs to its own lane and is excluded here.

## Column meanings

- **Rules**: the pure crate or module that owns the rule.
- **Storage**: the `apps/game-server/migrations` numbers. "mem" means in-memory only.
- **Caller**: the production command id and mechanic fragment (`crates/gameplay/dispatch/mechanics/`), or the native cycle that calls it.
- **Wire**: the capability id and name, with `+` for offered and `−` for not offered. Then `c` lists command ids and `d` lists state domains (`docs/contracts/protocol-oteryn/v1/registry/`).
- **E2E**: `R` is a test that drives the mechanic through the production dispatch or router with PostgreSQL (`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/` or an explicit mount in `monk_save_postgres_tests.rs`). `S` is a PostgreSQL test that calls the store or SQL directly. `—` means none was found.
- **Gaps**: the IDs of rows in the [gap register](#gap-register).

Paths are shortened as follows: `pt/` is `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/`, `gt/` is `crates/gameplay/dispatch/src/gameplay_transport/`, `cpp/pg` is Canary `src/server/network/protocol/protocolgame.cpp`, and `obr/` is Canary `data-otservbr-global/`.

## 1. Mechanic table

### 1.1 Movement and items

| Mechanic | Rules | Storage | Caller | Wire | E2E | Gaps |
|---|---|---|---|---|---|---|
| Step / paced movement | world, `gt/` movement | 0080 | c1 (native router) | 6 WORLD_SPATIAL_ENTITIES+, 13 PACED_MOVEMENT+ | R `pt/floor_exhaust.rs` | — |
| Floor change exhaust (2 s) | dispatch (#2433) | mem | step/teleport | as above | R `pt/floor_exhaust.rs::floor_exhaust_postgres_real_session_step_then_aggressive_cast` | G21 |
| Push creature/item | protocol `world_interaction.rs:48` | — | **none** (no fragment routes c15) | 11 WORLD_INTERACTION− c15,31 | — | G13 |
| Target move: autowalk, stop, turn, follow | `gt/` A* radius 12 (#2448) | mem | c81 `0310-target_move` | 64 TARGET_MOVE+ c81 | R `pt/autowalk.rs::target_move_real_dispatch_walk_turn_follow_and_cancel_on_postgres` | G22 |
| Use or move from afar (walk, then retry) | `connection/serve/autowalk.rs` (#2434) | mem | c2, c9 retry | via 15/4 | R `pt/autowalk.rs::autowalk_real_session_use_move_cancel_and_stale_on_postgres` | — |
| Creature (monster) movement | monster-ai | mem | native think cycle | 6+ | R `pt/creature_move.rs` | — |
| Item move / map pickup | items, durability | 0010, 0011, 0128, 0130 | c9 `0080-item_move` | 4 ITEM_VIEW_MOVE+ c9 d9,11 | R `loop_map_pickup_tests.rs`, `pt/stack_quantity.rs` | — |
| Stack quantity (split, merge, drop) | items (#2441) | 0128 | c9 field 6 | 61 ITEM_STACK_COUNT− | R `pt/stack_quantity.rs::stack_quantity_dispatch_split_merge_drop_replay_and_atomic_refusals` | G12 |
| Item look and text read | items (#2431, #2444) | — | c30 `0090-item_look` | 28 ITEM_LOOK− c30 | R `gt/item_look_postgres_tests.rs` | G3 |
| Item use (incl. nested) | items, durability | 0083, 0131 | c2 `0030-use_item` | 15 ITEM_USE− | R `pt/autowalk.rs`, `pt/refusal_detail.rs` | G2, G3, G11 |
| Key doors | `world/map/facts.rs:590` (#2440) | 0086 | c2 | 15− | R `pt/refusal_detail.rs` | G2 |
| Tools: rope spot | `gt/item_craft/rope.rs` (#2439) | — | c2 | 15− | — (unit only) | G1 |
| Tools: mining pick refining | `MINING_PICK_USE_V1` | 0167 | c2 | 15− | R `pt/mining.rs::mining_pick_refining_commits_and_replays_through_real_wire_on_postgres` | G1 |
| Item craft (cooking) | items | 0165 | c2 | 15− | R `pt/item_craft.rs` | — |
| Rotate furniture | `gt/item_transform.rs` (#2445) | — | c2 | 15− | R (ITEM-TRANSFORMS-1 test) | G11 |
| Equip / drop and equipment effects | items, durability | 0035, 0113 | c9 | 12 ITEM_EQUIP_DROP− | R `pt/equipped_offence.rs` | — |
| Container tree / view | items | 0011 | c21 `0050-container_view` | 14 CONTAINER_TREE− c21 d14 | R `pt/stack_quantity.rs` | — |
| Timed items / charges / expiry | items | 0054, 0058, 0179 | c38 `0070-timed_item` | 39 TIMED_ITEM− c38 | R `pt/timed_item.rs::timed_item_postgres_wire_charges_replay_repair_and_owner_expiry` | — |
| Corpse decay / ground items | combat, durability (#2437) | 0013, 0014, 0015 | native clock | — | R `pt/corpse_decay.rs` | — (owner decision 11) |
| Map item initialization / materialization | durability | 0049, 0077 | world boot | — | S `apps/game-server/tests/map_item_mint_postgres.rs` | — |
| Documents (books, letters, write) | — | — | **none** | none | — | G3 |

### 1.2 Combat

| Mechanic | Rules | Storage | Caller | Wire | E2E | Gaps |
|---|---|---|---|---|---|---|
| Attack target, melee, chase | combat-rules `attack/`; chase `gt/attack/attack_chase_search.rs:55` | 0017 | c11 `0200-attack` | 17 ATTACK+ c11,12 d10 | R `gt/kill_reward_postgres_e2e_tests.rs::auto_attack_kill_settles_xp_and_a_linked_corpse_with_loot_on_postgres` | — (owner decision 7: formula kept) |
| Fight modes / secure | combat-rules `attack/constants.rs:195` | 0017 | c12 `0210-fight_modes` | 17+ | R `pt/pvp_live.rs` | G23 |
| Ranged: bows, throwing, wands, rods | combat-rules (#2436, #2429 content) | 0184 | c11 swing | 17+ | R `pt/ranged_attack.rs` | G9 |
| Spell cast (instant, area, heal, support) | spell-rules; catalogue 246 bundles | 0034–0052 | c3 `0040-spell` | base command (no capability) | R `pt/floor_exhaust.rs`, `pt/parameter_spell.rs` | — (owner decisions 5 and 8) |
| Parameter spells (exiva and others) | spell-rules (#2430, #2417) | 0044 | c13 say → c3 dispatcher | 7+ | R `pt/parameter_spell.rs` | — |
| Feared gate on spells and runes | spell-rules (#2420) | — | c3, c2 | — | unit | — |
| Runes | spell-rules `dispatch_plans/rune_use.rs:10`; `gt/rune_use_bridge.rs:215` | 0042 | c2 legacy bridge | 22 RUNE_USE− (position arm REJECTED) | R `gt/rune_use_bridge_postgres_tests.rs` | G10 |
| Conditions and DoT in a PZ | domain `condition.rs:23-33` (#2426) | mem | native tick `gt/actor_spell.rs` | — | unit | — |
| Energy Ring / mana shield | domain `mana_shield.rs` (#2427, 2:1) | — | equip and damage path | — | R `pt/energy_ring.rs` | — |
| Equipment crit and leech | combat-rules `resolution/equipped_offence.rs` (#2452) | — | damage path | — | R `pt/equipped_offence.rs::equipped_offence_postgres_real_swings_critical_hit_and_leech` | G8 |
| PvP: skulls, harm, death consequences | combat `pvp/mod.rs` | 0081, 0142, 0164 | c11 and death | 17+ | R `pt/pvp_live.rs`, `pt/pvp_death.rs` | G19 |
| Death, death loss, blessings | domain `death.rs`; durability `character_death.rs` | 0016, 0018 | `gt/composed_admission/death.rs` | — | R `gt/tests/death_ml_loss_postgres.rs`, `pt/pvp_death.rs` | G19, Q1 |
| Adventurer's Blessing (through level 20) | durability (#2438) | — | death | — | R `pt/adventurer_blessing.rs` | — |
| Kill reward and loot | combat `death_reward.rs` | 0010, 0012, 0013 | native kill | — | R `gt/kill_reward_postgres_e2e_tests.rs` | — |
| Monster AI, targeting, spells | monster-ai, encounters `native_combat/` | mem | native think | 6+ | R `pt/loop_monster.rs`, `pt/creature_move.rs` | not compared |
| Summons (monster and player) | monster-ai `monster_summon/` (#2415) | mem | native | — | R `pt/loop_monster/summon_transaction.rs` | — |
| Familiars / monk harmony | spell-rules `familiar_cast.rs` | 0026, 0033, 0041, 0043, 0107 | c3 `gt/familiar_cast_dispatch.rs` | base | — | — |
| Bosses / encounters (Soul War excluded) | encounters `bosses/*` | mem | native | — | per-boss tests in encounters (not PostgreSQL routed) | G17 |
| Raids | encounter-rules `raid` | 0153 (metadata) | **none** | — | S `lifted_tests/auto/raid_schedule_postgres_cases.rs` | G7 |
| Protection zone and logout block | `gt/logout.rs:30-50` (#2413, #2421) | — | c23 `0220-logout` | 21 LOGOUT+ c23 | S `gt/logout_postgres_tests.rs` (calls the owner directly) | G20 |

### 1.3 Progression

| Mechanic | Rules | Storage | Caller | Wire | E2E | Gaps |
|---|---|---|---|---|---|---|
| XP, levels, skills, magic level | progression | 0009, 0068, 0073 | kill and spell paths | — | R `gt/kill_reward_postgres_e2e_tests.rs` | Q1 |
| Regeneration, food, soul (PZ rules) | progression `regeneration.rs` (#2409, #2410, #2412) | 0084 | native tick | 23 (codec)− | S last-vitals cases | — |
| Hunting stamina | characters `hunting_experience.rs`, `stamina.rs` (#2432) | 0181 | attack path | — | R `pt/hunting_stamina.rs::hunting_stamina_postgres_real_attacks_idle_checkpoint_and_reload` | — (owner decision 9) |
| Training: offline, exercise, loop, ranged | dispatch `loop_training` | 0180 | c66, c67 `0010-loop_training` | 54 TRAINING− (provisional flag) | R `pt/loop_training.rs`, `pt/ranged_training.rs` | — |
| Bestiary and charms | durability `bestiary_progress.rs`, charms | 0019, 0020 (+0195 in #2458) | kill credit only; c4 and c5 unrouted on `main` | 1 BESTIARY_CHARMS− c4,5 d4,5; #2458 adds 67 c84 | S `apps/game-server/tests/charm_state_postgres.rs`, `bestiary_progress_postgres.rs` | G5 |
| Bosstiary | content only (`content/project/v2.rs:659`) | — | none | none | — | G5 |
| Prey | progression `prey.rs` | 0152 | hunting XP percentage only | none | S `lifted_tests/auto/prey_postgres_cases.rs` | G6 |
| Wheel of Destiny | durability `character_wheel.rs` (premium gate #2428) | 0070, 0073 | store only | none | S `apps/game-server/tests/support/character_wheel_promotion_tests.rs` | G6 |
| Weapon proficiency | progression `proficiency.rs`, `proficiency_modification.rs` | 0032, 0055, 0060 | **none** | none | S `character_proficiency_modification_postgres.rs` | G16 |
| Forge, exaltation, dust | items `forge.rs` | 0059, 0156 | store only | none | S `lifted_tests/auto/forge_postgres_cases.rs`, `character_forge_dust_postgres.rs` | G6 |
| Imbuements | items `imbuement.rs` | 0150 | store only | none | S `lifted_tests/auto/imbuement_postgres_cases.rs` | G6 |
| Promotion purchase | progression `promotion.rs:5-14` | 0133 | c7 NPC `npc_command/premium_purchase.rs:165` | 3 NPC_SERVICE− | R `pt/promotion_npc.rs` | — |
| Premium entitlement | durability | 0029, 0057, 0085 | admission | — | R `pt/promotion_npc.rs` | — |
| Achievements | social / durability | 0021 | c10 `0230-achievements` | 8 ACHIEVEMENT_NOTICES− d13 | S `apps/game-server/tests/account_achievement_postgres.rs` | not compared |
| Quest state, log, chests | quests | 0056, 0069 | c22 `0140-quest_log`; c2 chest (#2446) | 16 QUEST_LOG+ c22 d16 | R `gt/quest_log_postgres_tests.rs`, `gt/quest_chest_postgres_tests.rs` | G4 |
| Task board (weekly kills) | dispatch (#2449) | mem (0200 reserved) | c89 `0290-task_board` | 72 TASK_BOARD+ c89 | R `pt/task_board.rs::task_board_real_dispatch_selects_credits_a_real_kill_and_claims_once` | G25 |
| Highscores | durability | 0129 | c14 `0280-highscores` | 9 HIGHSCORES+ c14 | S `apps/game-server/tests/support/highscores_session_postgres_cases.rs` (session handler, not router) | — |
| Cyclopedia map | durability | 0176 | c72, c73 `0110-cyclopedia` | 57 CYCLOPEDIA_MAP− | R `pt/cyclopedia.rs::cyclopedia_postgres_real_dispatch` | — |
| Daily reward | durability | 0140, 0159 | c32, c33 `0260-daily_reward` | 32 DAILY_REWARD− d24 | R `pt/daily_reward.rs::daily_reward_real_dispatch_delivers_replays_reloads_and_orders_seven_days` | — |
| Analysers | dispatch | mem | native | 10 ANALYSER− d15 | — | G24 |
| Tutorial notices | dispatch `tutorial_*` | 0136 | native | 31 TUTORIAL_NOTICES+ d23 | S `authority_cases/tutorial_progress_postgres_cases.rs` | — |
| Reward bags (#2456, open) | dispatch | — | c2 | 15− | R in #2456 | G18 |

### 1.4 Economy and social

| Mechanic | Rules | Storage | Caller | Wire | E2E | Gaps |
|---|---|---|---|---|---|---|
| NPC talk | npc `npc_talk.rs` (#2416) | 0114 | c7 `0180-npc` | 3 NPC_SERVICE− c7,8 d7,8 | R `pt/promotion_npc.rs` | — |
| NPC trade / shop | economy `npc_trade.rs` | 0082 | c8 `0190-npc_trade` | 3− | S `authority_cases/npc_trade_postgres_cases.rs` | G15 |
| NPC bank, account bank, gold fees | economy `bank.rs`; npc `bank_npc.rs` | 0023, 0031, 0071, 0072, 0114 | c7 keywords | 3− | R `pt/promotion_npc.rs`; S `npc_econ_bank_postgres.rs` | — |
| Depot | items `depot.rs` | 0151, 0157 | c36 `0060-depot` | 37 DEPOT− c36 | R `pt/depot_dispatch.rs::depot_session_dispatch_opens_deposits_browses_withdraws_and_replays` | G14 |
| Supply stash | durability `character_stash.rs` (#2447) | 0189 | c79 `0065-stash` | 62 STASH− c79 d27 | R `pt/depot_dispatch.rs::stash_session_dispatch_conserves_and_replays` | G14 |
| Inbox and mail | items `mail.rs`, durability `mail/` | 0076, 0101 | native deliveries only | none | S `mail_postgres_cases.rs`, `character_inbox_postgres_cases.rs` | G14 |
| Market (#2459, open) | economy `market.rs` | 0155 (+0191 in #2459) | none on `main`; #2459 adds 63 c80 d28 | none on `main` | S `lifted_tests/auto/market_postgres_cases/` | G4m |
| Player trade | economy `player_trade.rs` (#2442, #2455) | 0147 | c78 `0195-player_trade` | 60 PLAYER_TRADE+ c78 d26 | R `pt/player_trade.rs::player_trade_postgres_two_real_sessions_atomic_replay_and_cancellation` | G15 |
| House ownership / auction | housing `house_auction/` | 0047, 0143, 0074 | c56, c57 `0250-house` | 49 HOUSE+ c56,57 | R `pt/house_wire/bid.rs` | G7h |
| House ACL / management | housing `house_management.rs` | 0038, 0168 | c56 | 49+ | R `pt/house_wire.rs` | — |
| House move-out and disposition | housing `house_move_out.rs` | 0178 (no writer) | c76, c77 `0100-house_move_out` | 59 HOUSE_MOVE_OUT+ | R `pt/house_move_out.rs` (notice only) | G7h |
| House item custody | — | 0025, 0095 | **none** | none | S `house_custody_postgres.rs`, `house_interior_postgres.rs` | G7h |
| Beds: sleep, regen | housing `house_bed_regen.rs` | 0149, 0166 | c31 `0020-bed` | under 11 WORLD_INTERACTION− | R `pt/bed_sleep_dispatch.rs`, `pt/bed_regen_sleep.rs` | G13 |
| Guilds | social-rules `guild.rs` | 0132, 0161 | c40, c41 `0120-guild` | 40 GUILD+ | R `pt/guild_wire.rs` | G15 |
| Guild bank / guild chat | economy `guild_bank.rs`; social `chat/guild_room.rs` | 0174 | c68, c69; c13 | 55 GUILD_BANK+, 36 CHAT_GUILD_ROOM+ | R `pt/guild_bank.rs`, `pt/guild_chat.rs` | — |
| Party and shared XP | social-rules `party.rs`, `shared_experience.rs` (#2414) | 0046, 0141 | c45, c46 `0160-party` | 42 PARTY+ | R `pt/party_membership.rs`, `pt/party_xp.rs` | G24 |
| VIP | durability `vip.rs` | 0163 | c48–50 `0150-vip` | 43 VIP− | R `pt/vip.rs` | — |
| Chat: say, whisper, yell, rooms | social `chat/` | 0122 | c13 `0170-chat` | 7 CHAT+ c13 d12 | R `pt/guild_chat/wire_tests.rs` | G15 |
| Player report (#2453) | durability `player_report.rs` | 0204 | c93 `0285-player_report` | 76 PLAYER_REPORT+ c93 | R `pt/player_report.rs` | — (moderation belongs to Platform) |
| Staff / GM | social-rules `staff.rs` | 0170 | c60, c61 `0130-staff` | 51 STAFF+ | R `pt/staff.rs` | — (signed operators diverge on purpose) |
| Spectate / observe | social `spectate/` | — | **none** | 33 SPECTATE− c34,35 d25 | — | not a Canary surface |

### 1.5 World, session and presentation

| Mechanic | Rules | Storage | Caller | Wire | E2E | Gaps |
|---|---|---|---|---|---|---|
| Admission, reconnect, scope | session, admission | 0001–0008, 0144 | session ingress | Hello | R `pt/session_ingress.rs`, `pt/admit_cap_2.rs` | G20 |
| Character name, sex, restore | characters | 0022, 0080, 0084, 0134 | admission | — | S `durability_postgres.rs`, last-position cases | — |
| World light and clock | `world/world_clock.rs:153` | mem | native | 19 WORLD_LIGHT+ d18 | unit | — (matches Canary `game.cpp:9447`) |
| Weather | weather | mem | native | 26 WORLD_WEATHER+ d22 | R `pt/weather_wire.rs` | — (Oteryn-native) |
| World events | durability | 0172 | c64 `0270-world_event` | 53 WORLD_EVENT+ | R `pt/world_event.rs` | — |
| Server-save countdown (#2457) | dispatch | mem (0203 reserved) | native | 75 SERVER_SAVE_COUNTDOWN+ d40 | R `pt/server_save_countdown.rs` | G17 |
| Planned world reset | durability | 0148 | native | — | R `pt/planned_world_reset.rs`, `pt/world_reset_activation.rs` | G17 |
| Appearance / outfits | durability | 0093 | c25 `0240-appearance` | 25 APPEARANCE− (offer hook) | R `pt/appearance_change_postgres.rs` | G24 |
| Presentation effects and cooldowns | protocol codecs | — | — | 23 PRESENTATION− d19,20 | — | G24 |
| Refusal detail (#2451) | dispatch | — | c2, c9, c1 results | 66 REFUSAL_DETAIL− | R `pt/refusal_detail.rs` | G2 |
| Spatial fields (magic fields) | spell | 0048 | spells | 30 WORLD_SPATIAL_FIELDS− | — | not compared |
| Multichannel WorldId / ChannelId | runtime scope | 0003 | admission | — | S `runtime_scope_assignment_postgres.rs` | — (invariant) |
| Soul War | — | — | — | — | — | **owner: separate lane, excluded** |

## 2. Gap register

Size: **S** is one function or one file. **M** is a coherent mechanic batch of several files, possibly with a migration. **L** is a new subsystem, protocol surface and storage. Canary refs were opened at the pin. Rows marked *in flight* are covered by an open PR.

| ID | Mechanic | Gap still open | Size | Canary ref (Crystal) |
|---|---|---|---|---|
| G1 | Tools | Shovel, pick, machete, scythe and sickle terrain transitions and reverts; rope on an open hole (lifting creatures and items). Only rope-spot ascent (#2439) and mining refining exist | M | `obr/scripts/lib/register_actions.lua:332` (rope hole), `:351` shovel, `:515` pick, `:838` machete, `:1007` scythe; `data/scripts/actions/tools/rope.lua:1` (crystal same file) |
| G2 | Doors | Key USE-WITH with key-number match and lock/unlock; level doors; quest doors; custom doors. Only direct key-door toggle and Locked refusal (#2440) | M | `data/scripts/actions/doors/key_door.lua:56`, `level_door.lua:16`, `quest_door.lua:16`, `custom_door.lua:12` |
| G3 | Documents | Text window and write-once write; books, letters, spellbook; Use to open. Look reads signs within 4 cells (#2444) | M | `cpp/pg:1845` (0x89) → `:2793` parseTextWindow; `data/scripts/actions/items/spellbook.lua:23` |
| G4 | Quest callbacks | Levers, action/unique-id step-in scripts (Annihilator), container and multi-item rewards, cooldown/random/written rewards. One map chest pays (#2446); 96 claims are unplayable | L | `data-canary/scripts/actions/quests/annihilator.lua:14`; `quest_reward_common.lua:241` (crystal `:316`) |
| G4m | Market | Browse, create, cancel and accept over the wire. *In flight #2459*: buy-side acceptance, crossing matches, held-credit recovery and concurrent settlement still lack PostgreSQL coverage | L | `cpp/pg:4411`, `:4435`, `:4455`, `:4469` |
| G5 | Bestiary, charms, bosstiary | Query and mutation wire. *In flight #2458* (cap 67, c84). Bosstiary and charm reset stay open; cap 1 stays unoffered | L | `cpp/pg:3472` parseBestiarySendRaces, `:4164`; bosstiary in `src/creatures/monsters/monsters.cpp` |
| G6 | Prey, Wheel, forge, imbuement | Storage and rules exist with no command route or capability | L | `cpp/pg:4263` parsePreyAction, `:11643` parseWheelGemAction, `:7398` parseForgeEnter, `:7429`, `:3065` parseImbuementAction |
| G7 | Raids | Runtime raid activation. 0153 holds metadata only | L | `src/lua/creature/raids.cpp:227` Raid::startRaid |
| G7h | Houses | Recurring rent collection; ownership transfer; disposition execution (0178 has no writer); runtime house-item placement and removal (0025) | L | `src/map/house/house.cpp:953` Houses::payHouses, `:94` House::setOwner |
| G8 | Equipment modifiers | Reflect, cleave, perfect shot, elemental and spell-magnitude modifiers (promotion data only); crit and leech on spells and PvP; no crit/leech item admitted to the native pin yet | L | `src/creatures/creature.cpp:1928` getReflectPercent; `src/creatures/players/player.cpp:7564` getPerfectShotDamage |
| G9 | Ranged | Quiver ammunition selection; scripted weapon conditions and area weapons | M | `src/creatures/players/player.cpp:426` hasQuiverEquipped |
| G10 | Runes | Position-target rune arm and offering cap 22. The registry offer gate still names RUNE-CAST-1 as unshipped, but the RUNE-CAST-1 task record is archived as complete. Reconcile the two, then offer | M | `src/creatures/combat/spells.cpp:1501` RuneSpell::executeUse |
| G11 | Item transforms | transformOnUse and script toggles; wrap and unwrap; destroy; fluids fill and pour (needs typed durable state; 0188 reserved); rotating house furniture | M | `src/game/game.cpp:5436` (wrap); `data-canary/scripts/actions/other/fluids.lua:35`; `data-canary/scripts/lib/register_actions.lua:14` destroyItem |
| G12 | Stack quantity | Partial destination fit with a PARTIAL result; ground merge, partial ground pickup, ground-to-ground; counted moves of equipment and corpse loot (currently NOT_SUPPORTED) | M | `src/game/game.cpp:2553` (room check), `:2576` (partial merge count) |
| G13 | World interaction | c15 push has no route and cap 11 is unoffered, so c31 bed sleep cannot be reached by an offered client | M | `cpp/pg` parseThrow/push path via `src/game/game.cpp` playerMoveCreature |
| G14 | Depot, stash, inbox | Depot search and retrieve; stow from depot; stow-all; partial stow; player inbox read; NPC sale letters | M | `cpp/pg:11022` parseDepotSearchItemRequest, `:10873` parseStashWithdraw; `src/creatures/npcs/npc.cpp:390` |
| G15 | Trade, chat, guild, NPC shop | Player-trade offers from ground, container trees and equipment; private messages and channel rooms (CHAT-2); guild MOTD and rank create/rename; NPC shop look and sell-all-loot | M | `src/game/game.cpp:5797` playerRequestTrade; `cpp/pg:2434` parseOpenChannel; `src/creatures/players/grouping/guild.hpp:78`, `:83`; `src/creatures/npcs/npc.cpp:798`, `:1019` |
| G16 | Proficiency | Accrual from combat, perk effects (PROF-2) and the wire (PROF-WIRE-1) | M | `src/creatures/combat/combat.cpp:870` |
| G17 | Event callbacks | Map clean at save, periodic save, encounter/boss/creature callbacks, guild-war status, startup cleanups | M | `data/scripts/globalevents/global_server_save.lua:3` cleanMap; `save_interval.lua`; `encounters.lua:1`; crystal `globalevents/clean.lua:8` |
| G18 | Reward chest | Boss reward chest collect with 7-day expiry; store-inbox fallback and reward message. *In flight #2456* (Bag You Covet only) | M | `cpp/pg:4491` parseRewardChestCollect; `data/scripts/actions/items/reward_bags.lua:108`, `:178` |
| G19 | Death | A PvP death without a skull removes only Twist of Fate. No Twist-specific branch was found in `character_death.rs`; verify, then fix. General item and container loss on death (DEATH-3) is not wired | S | `src/creatures/players/player.cpp:4208` |
| G20 | Logout and login | NOLOGOUT tile flag (read and refuse); login refusals for premium-only worlds and the per-account outside-PZ cap (owner question) | M | `src/io/iomap.cpp:175`, `cpp/pg:1154`; `cpp/pg:973`, `:1036` (crystal `iomap.cpp:266`) |
| G21 | Floor change | Mount exhaust (3 s, owner decision 12) waits on mounts | S | owner decision 12 |
| G22 | Target move | Diagonal steps, cross-floor routes, legacy direction-list packet | S | `cpp/pg:2449` parseAutoWalk |
| G23 | Fight modes | Secure mode has no PvP effect yet | S | `src/game/game.cpp:7050`; `player.cpp:6196` setChaseMode |
| G24 | Presentation and analysers | Magic effects and cooldown presentation (cap 23 is codecs only); party hunt analyser; inspection; modal windows; outfit window | L | `cpp/pg:3900` parsePartyAnalyzerAction, `:2913` parseInspectionObject; `sendMagicEffect`, `sendModalWindow` in `cpp/pg` |
| G25 | Task board | Delivery and bounty tasks, difficulty, rerolls, task shop, talismans, Soulpit | M | `cpp/pg:3204` parseTaskHuntingAction |

## 3. Corrections to inventory §7

The following §7 rows are stale at `d7dddb38`.

- Row 12, PARITY-TARGET-MOVE-1: now present (#2448, #2434). The remaining work is G22.
- Row 3, PARITY-STACK-QUANTITY-1: carried-tree moves are present (#2441). The remaining work is G12.
- Row 7, PARITY-TRADE-PLAYER-1: basic trade is present (cap 60 offered, routed test). The remaining work is in G15.
- Rows 1, 2, 5, 6, 8, 10, 13, 16, 17, 18 and 19 are partly landed. Their residues are G1, G3, G2, G4, G14, G8, G11, G17, G2, none and G25.
- NIGHT-W1 decision 6 says "Oteryn has no Energy Ring mechanic yet". That is stale because #2427 landed. Decisions 3, 9, 10, 15 and 18 also have their follow-ups landed (#2428, #2432, #2429, #2434, #2430).

## 4. Owner questions found

- **Q1.** `crates/gameplay/durability/src/durability/character_death.rs:738` reduces skills and magic level on death (SKILLS-0 §3.6). The D58 text says they never drop. One of the two needs a ruling.
- **Q2.** Should G20's premium-only login refusal and per-account outside-PZ cap exist in Oteryn at all?
- **Q3.** NIGHT-W1 decision 3 cites Canary `modules.lua:106` for the premium promotion rule. The verified price source is `data-canary/npc/canary.lua:130`.

## 5. Mechanics with rules and storage but no routed e2e

Each of these has a store-only PostgreSQL test (S) or no test at all (—).

- NPC trade / shop (0082): S
- Bestiary and charms (0019, 0020): S. #2458 would add routed cases.
- Prey (0152): S
- Wheel of Destiny (0070, 0073): S
- Weapon proficiency (0032, 0055, 0060): S
- Forge and dust (0059, 0156): S
- Imbuements (0150): S
- Market (0155): S. #2459 would add a partly routed test.
- Inbox and mail (0076, 0101): S
- House item custody and interior (0025, 0095): S
- House disposition (0178): —
- Raids (0153): S
- Achievements (0021): S
- Highscores (0129): S, through the session handler only
- Familiars (0033, 0041, 0043, 0107): —
- Character restore of position and vitals (0080, 0084): S
- Logout (no migration; the owner-direct test is store-only): S

## Method and limits

The evidence is static: registry fragments, mechanic fragments, migrations, test names and bodies, PR bodies (read-only), and Canary/Crystal source at the pins. No gameplay session and no donor runtime were run. "Routed" means that the test sends commands through the dispatch or serve path. It was judged from the test body and its command constants, with no test executed. Monster AI, spells per family, loot tables and the shared-XP formula were not compared line by line against Canary. Crystal was consulted only where it is cited. Sizes are planning estimates and must be checked against the owning contract before any allocation. Only this file changed.
