# Staging qualification audit

**Staging SHA `43a8b036a0e3d9e05f130b6932b533b4cf7c7573`.** `git fetch origin staging` populated `FETCH_HEAD`. `git ls-remote origin refs/heads/staging` returned the same SHA, and the checkout was already at that commit. The worktree was clean before and after the audit. I made no worktree changes, commits, or pushes.

The same-head PostgreSQL workflow completed, and its node-boot plan, builds, shards, and aggregate job were skipped. The findings below distinguish source coverage, retained qualification evidence, and the 45 expected failures still registered on this head. I did not rerun the qualification suites. GitHub denied access to the individual PostgreSQL job log I requested, so the quest count uses the checked-in claim evidence. [Same-head PostgreSQL workflow](https://github.com/Oteryn/Oteryn-Game/actions/runs/46078143692) · [Skipped node-boot job](https://github.com/Oteryn/Oteryn-Game/actions/runs/46078143692/job/140770623613)

## Gap list

Path prefixes in the tables are `phase/` for `apps/game-server/src/qualification/client_phases/`, `dispatch-pg/` for `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/`, `authority/` for `apps/game-server/tests/support/authority_cases/`, and `native/` for `apps/client/src/features/`. `PG` identifies the proposed test’s need for PostgreSQL. For transient state, the reconnect assertion should observe the correct new state or absence of stale state. For client-local settings, the persistence boundary is reopening the client.

### Expected-fail and blocked node-boot rows

These 45 IDs are present in `known_fail_rows.txt:5`. The first observations come from that ledger. Several rows have had source fixes since the ledger was seeded, so their next run may clear the expected failure. The lane identifies the first actionable qualification work on this tree. [Expected-failure ledger](/workspace/Oteryn-Game/tools/qualification/node_boot/known_fail_rows.txt:5)

| id | name | missing step | nearest test | lane | PG | size | batch |
|---|---|---|---|---|---|---|---|
| C01 | Melee, defence, modes and chase | Admission failed before melee, block and chase observations and saved combat progress | `journey`, `phase/full_row_c01.rs:187` | E2E | yes | M | B-C01 |
| C02 | Ranged weapons, ammunition and rods | Walk to the target stalled before bow/ammo and rod/mana settlement and relog | `journey`, `phase/full_row_c02.rs:570` | E2E | yes | M | B-C02 |
| C03 | Critical, leech, reflection and modifiers | Baseline combat stalled before controlled modifier comparisons and durable readback | `journey`, `phase/full_row_c03.rs:564` | E2E | yes | M | B-C03 |
| C04 | Spell cast and gates | PZ rejection did not satisfy the expected reply and no-delta assertion | `journey`, `phase/full_row_c04.rs:220` | E2E | yes | M | B-C04 |
| C05 | Parameter spells and player lookup | Row failed before proving successful named lookup and refusal cases on the live route | `journey`, `phase/full_row_c05.rs:84` | E2E | yes | M | B-C05 |
| C06 | Combat runes, healing and conjuring | Admission/acquisition failed before charged use, healing and conjuring with inventory readback | `journey`, `phase/full_row_c06.rs:47` | E2E | yes | L | B-C06 |
| C09 | Food and HP/MP regeneration | Admission failed before obtaining and eating food and observing regeneration | `journey`, `phase/full_row_c09.rs:511` | E2E | yes | M | B-C09 |
| C11 | Monk Harmony, Serene and Battle Instinct | Reach Oressa, complete Monk selection, then observe class-specific outcomes | `journey`, `phase/full_row_c11.rs:101` | E2E | yes | L | B-C11 |
| C12 | Equipment spell augments | Baseline cast lost transport before acquisition, equip/unequip comparison and relog | `journey`, `phase/full_row_c12.rs:587` | E2E | yes | M | B-C12 |
| C13 | Summons and familiars | Summon payment and spawn failed before owner logout/death cleanup | `summons`, `phase/full_e2ef5.rs:3556` | E2E | yes | L | B-SPELL-1 |
| C16a | Magic Wall and Wild Growth | Acquire Magic Wall and then prove both barriers, custody and expiry | `barriers`, `phase/full_e2ef5.rs:1811` | E2E | yes | M | B-SPELL-1 |
| C16b | Paralyze rune | Acquire a rune and observe target slowdown, expiry and the spent charge | `paralyze`, `phase/full_e2ef5.rs:1657` | E2E | yes | M | B-SPELL-1 |
| C16c | Convince Creature and Animate Dead | Buy Convince Creature, then prove creature and corpse conversion and cleanup | `acquired_companions`, `phase/full_e2ef5.rs:4100` | E2E | yes | L | B-SPELL-1 |
| C16d | Chameleon and Disintegrate | Buy Chameleon, then observe appearance and tile-item outcomes with durable charge readback | `tile_items`, `phase/full_e2ef5.rs:4519` | E2E | yes | M | B-SPELL-2 |
| C17a | Haste and Strong Haste | Get the first successful haste cast before cadence and expiry checks | `haste`, `phase/full_e2ef5.rs:783` | E2E | yes | M | B-SPELL-2 |
| C17c | Creature illusion | Cast the rat illusion and observe exact appearance, expiry and reconnect state | `illusion`, `phase/full_e2ef5.rs:343` | E2E | yes | M | B-SPELL-2 |
| C17d | Challenge | Establish player spell access, then prove the monster actually switches targets | `challenge`, `phase/full_e2ef5.rs:2644` | E2E | yes | M | B-SPELL-2 |
| C18a | Directional spell targeting | Earn usable spell access, then prove the aimed target takes the effect | `directional_aim`, `phase/full_e2ef5.rs:2449` | E2E | yes | M | B-SPELL-2 |
| C20a | Death XP loss | S16a blocked the vocation prerequisite before loss with and without protection | `experience_loss`, `phase/full_e2ef6.rs:1771` | E2E | yes | L | B-DEATH |
| C20b | Death item loss and AoL | Earn a loss-eligible level before comparing dropped and protected custody after relog | `item_loss`, `phase/full_e2ef6.rs:1963` | E2E | yes | L | B-DEATH |
| C20c | Death blessings and Twist of Fate | Earn Adventurer prerequisites before blessing consumption and saved readback | `blessing_death`, `phase/full_e2ef6.rs:2595` | E2E | yes | L | B-DEATH |
| C20d | Temple respawn | Complete death, respawn and fresh admission with the temple and restored vitals | `temple_respawn`, `phase/full_e2ef6.rs:857` | E2E | yes | M | B-DEATH |
| E01 | NPC shop and bank | NPC greeting failed before buy/sell and balance persistence | `shop_row`, `phase/full_e2ef4.rs:307` | E2E | yes | M | B-ECON-1 |
| E03 | Coin conversions | First upward denomination conversion failed before the reverse and relog checks | `coins`, `phase/full_e2ef4.rs:627` | E2E | yes | M | B-ECON-1 |
| E04 | Player trade | Trade look failed before reciprocal settlement and both inventories after relog | `trade`, `phase/full_e2ef4.rs:694` | E2E | yes | M | B-ECON-1 |
| E05 | Market offers and history | Browse failed before matched settlement, history and relog custody | `market`, `phase/full_e2ef4.rs:1029` | E2E | yes | M | B-ECON-1 |
| E07 | Depot, inbox and stash | Depot open failed before storage, retrieval and custody readback | `storage`, `phase/full_e2ef4.rs:1240` | E2E | yes | M | B-ECON-1 |
| P10 | Streak rest regeneration | Open the daily shrine, claim the streak and compare rest effects after fresh admission | `resting_bonuses`, `phase/full_e2ef6.rs:495` | E2E | yes | M | B-P10 |
| P16 | Blessing services | Earn Adventurer prerequisites, use the service and read back the saved blessing set | `blessing_services`, `phase/full_e2ef6.rs:2648` | E2E | yes | L | B-P16 |
| P19 | Influenced and fiendish monsters | Influenced selection/combat stalled before the marked kill, reward and durable credit | `forge_journey`, `phase/full_e2ef7.rs:6841` | E2E | yes | L | B-P19 |
| P21 | Imbuement, etcher and scrolls | Obtain materials through a supported player route and prove apply, custody and relog | `imbuement_journey`, `phase/full_e2ef7.rs:4877` | E2E | yes | L | B-P21 |
| S01 | Party and shared XP | Party join failed before shared kill XP and both characters’ durable progression | `party`, `phase/full_e2ef4.rs:1437` | E2E | yes | M | B-SOCIAL-1 |
| S08 | PvP skulls, frags and modes | Earn PvP-eligible levels, then prove the mode/skull/frag transitions and saved history | `pvp_journey`, `phase/full_e2ef7.rs:4198` | E2E | yes | L | B-S08 |
| S09 | Logout, PZ and idle | In-combat refusal failed before ordinary logout and re-entry from the saved tile | `logout`, `phase/full_e2ef4.rs:1605` | E2E | yes | M | B-SOCIAL-1 |
| S11 | Highscores | Category query stalled and training-session holder was busy before ladder and re-admission checks | `journey`, `phase/full_e2ef7.rs:386` | E2E | yes | M | B-S11 |
| S13a | Party spell range and refusal | Solo refusal failed before solo, out-of-range and in-range comparison | `party_spell_journey`, `phase/full_e2ef7.rs:2812` | E2E | yes | M | B-PARTY-SPELL |
| S13b | Party spell mana cost | Solo zero-charge check failed before exact paid group cost and durable resource readback | `party_spell_journey`, `phase/full_e2ef7.rs:2812` | E2E | yes | M | B-PARTY-SPELL |
| S16a | Confirmed vocation choice | Reach Oressa and complete the confirmed choice and relog projection | `first_vocation`, `phase/full_e2ef6.rs:1256` | E2E | yes | M | B-S16a |
| S18 | Say, whisper and yell | Yell leg failed before proving audience/range and clean new-session chat state | `speech`, `phase/full_e2ef4.rs:1726` | E2E | yes | M | B-SOCIAL-1 |
| W01b | Spawn and respawn | Admission failed before live spawn, kill, respawn and saved credit | `journey`, `phase/full_row_w01b.rs:156` | E2E | yes | M | B-W01b |
| W18 | Podia and house décor | Prove owned-house entry, then rotate the three unresolved targets and observe persisted decorations | `journey`, `phase/full_row_w18.rs:174` | SERVER | yes | L | B-W18 |
| W20a | Day and night light | Sunrise timed out before the rest of the epoch transitions and fresh view | `clock_journey`, `phase/full_e2ef7.rs:1235` | E2E | yes | M | B-LIGHT |
| W20b | Light spells | Cast/mana assertion failed before observer light, expiry and resource readback | `light_journey`, `phase/full_e2ef7.rs:2417` | E2E | yes | M | B-LIGHT |
| W22b | Loot modifiers | Fixture release/prey slot and rat reachability failed before controlled bonus loot and retained inventory | `journey`, `phase/full_row_w22b.rs:122` | E2E | yes | L | B-W22b |
| W23 | Citizenship and temple | No walking route to the citizenship tile, so citizenship and saved temple were unproved | `journey`, `phase/full_row_w23.rs:160` | E2E | yes | M | B-W23 |

### Other mechanics gaps

| id | name | missing step | nearest test | lane | PG | size | batch |
|---|---|---|---|---|---|---|---|
| C15 | Hunting stamina and XP boosts | Assert the XP multiplier across stamina brackets and the saved award. The journey asserts an XP increase and stamina debit/recovery | `journey`, `phase/full_row_c15.rs:131` | E2E | yes | M | B-C15 |
| C19b | Starting loadout | Prove the supported first-entry loadout and its once-only readback. The row ends on the excluded Dawnport branch even after its vocationless check | `journey`, `phase/full_row_c19b.rs:69` | E2E | yes | M | B-C19b |
| M06 | Keys and level/quest doors | Earn level and quest predicates, then assert successful USE on the same doors and after relog | `journey`, `phase/full_row_m06.rs:161` | E2E | yes | L | B-M06 |
| M11b | Magic Rope | Exercise the blocked landing branch with a real usable spot and assert committed ascent, mana payment and saved landing | `journey`, `phase/full_row_m11b.rs:122` | E2E | yes | M | B-M11b |
| ADD-0012 | Equip, unequip and drop | Drive all three native actions on the same item and verify the final custody/equipment snapshot after relog | `crates/client-ui/src/features/0012-item-equip-drop/tests/panel.rs:41` | E2E | yes | M | B-EQUIP |
| ADD-0146 | Container sort | Sort a populated nested container in a live session and read the authoritative order after relog | `container_sort_postgres_dispatch_replay_stale_generation_and_conservation`, `dispatch-pg/container_sort.rs:14` | E2E | yes | M | B-SORT |
| I06 | Corpse and remaining-loot decay | Correlate materialization and retirement to the same corpse, verify residual loot removal and fresh view | `decay`, `phase/full_e2ef3_e2ei-c.rs:465` | E2E | yes | M | B-DECAY |
| I08 | Primal Bag | Acquire the bag from an eligible encounter before the existing consume/mint/relog leg | `reward_bag`, `phase/full_e2ef3.rs:159` | E2E | yes | L | B-BAGS |
| I10 | Wrap, kits, destroy and fluids | Replace placeholder wrap and self-USE targets with real owned-house objects and fluid pairs, then verify transformed state at relog | `journey`, `phase/full_row_i10.rs:121` | E2E | yes | L | B-I10 |
| I13 | Random reward bags | Obtain Bag You Covet through supported acquisition before the consume/reward/relog leg | `run`, `phase/full_e2ef3_e2ei-d.rs:54` | E2E | yes | L | B-BAGS |
| I15 | Energy Ring | Observe shield expiry and retirement of the same charged ring after resume using an authoritative clock | `energy_ring`, `phase/full_e2ef3_e2ei-d.rs:544` | E2E | yes | M | B-RING |
| I17d | Hotkey jewellery delays | Exercise the feared refusal on a real equipped item and confirm custody after relog | `journey`, `phase/full_row_i17d.rs:291` | E2E | yes | M | B-I17d |
| I18a | Browse field walk-to | Prove a far native Browse drives approach and opens the intended tile. Current test walks back explicitly after `TooFar` | `journey`, `phase/full_row_i18a.rs:229` | E2E | yes | M | B-I18a |
| I18c | Browse parent from ground bag | Verify the retrieved bag in the fresh inventory and that the old browse context is cleared | `journey`, `phase/full_row_i18c.rs:374` | E2E | yes | S | B-I18c |
| I19 | Elemental enchanting | Reach an altar with mana/soul, enchant the gem and apply it to a weapon, then verify the persisted result | `journey`, `phase/full_row_i19.rs:210` | E2E | yes | M | B-I19 |
| E06 | Market expiry and refund | Let an uncancelled offer expire through the scheduler and assert the refund after relog | `journey`, `phase/full_row_e06.rs:79` | E2E | yes | M | B-E06 |
| E10 | Store purchase and history | Run the implemented positive purchase and history readback against a qualification Platform that serves an affordable sandbox offer | `journey`, `phase/full_row_e10.rs:170` | E2E | yes | M | B-STORE |
| E12 | Hireling and house services | Retrieve and use the delivered food and read back the house and outfit state after fresh admission | `journey`, `phase/full_row_e12.rs:140` | E2E | yes | M | B-E12 |
| P13 | Loyalty and skills | Exercise the Platform-sourced loyalty update before verifying the new points, bracket and bonus after relog | `journey`, `phase/full_row_p13.rs:126` | SERVER | yes | M | B-P13 |
| P22d | Tracker live kill refresh | Assert the domain-98 push against the tracked race before relog. The scenario drains the events and checks only the later query | `journey`, `phase/full_row_p22d.rs:85` | E2E | yes | M | B-P22d |
| ADD-0031 | Tutorial and hints | Trigger a real tutorial step and assert the native notice and next-session treatment of the saved step | `native/0031-tutorial-notices/tests/play.rs:5` | E2E | yes | M | B-TUTORIAL |
| W15c | Grizzly Adams | Continue past task start into kill credit, required boss/turn-in, reward and relog | `journey`, `phase/full_row_w15c.rs:199` | E2E | yes | L | B-W15c |
| W15d | Executable quests | Start from native entry and drive missions, finale, reward and relog. The node row covers a single Monk shrine callback | `journey`, `phase/full_row_w15d.rs:123` | E2E | yes | L | B-W15d |
| ADD-0240 | Party chat | Drive native composer → peer room → leave/rejoin, then verify old room membership and messages do not leak | `party_chat_postgres_real_wire_members_only_departure_refusal_and_replay`, `dispatch-pg/party_chat.rs:6` | E2E | yes | M | B-PARTY-CHAT |
| S03 | Guilds, bank and wars | Complete guild lifecycle, bank and war and verify durable state. An answered own-guild query reaches the explicit missing MOTD-write boundary | `guild`, `phase/full_e2ef4.rs:1588` | SERVER | yes | L | B-GUILD |
| S06 | Outfits, addons and mounts | Cover remaining world acquisition sources and verify their usable, saved unlocks through the native appearance path | `journey`, `phase/full_row_s06.rs:664` | E2E | yes | L | B-S06 |
| S10 | NPC voices, travel and Rashid | Verify the remaining authored voice/routes and mission/service reachability, including arrival and fresh state | `journey`, `phase/full_row_s10.rs:399` | E2E | yes | L | B-S10 |
| S14b | Team Finder browse and requests | Read pending and cancelled request state after member re-admission | `journey`, `phase/full_row_s14b.rs:183` | E2E | yes | S | B-TEAM |
| S14c | Team Finder leader decisions | Read back accepted/denied member state and the resulting invitation after the parties reconnect | `journey`, `phase/full_row_s14c.rs:242` | E2E | yes | M | B-TEAM |
| S19 | Typing indicator | Observe the correct sender on a second client and verify no stale indicator on reconnect | `journey`, `phase/full_row_s19.rs:95` | E2E | yes | S | B-S19 |
| ADD-0026 | Regional weather | Traverse a weather boundary, verify the native view, then establish the correct fresh-admission weather | `the_composed_server_offers_26_and_serves_the_actors_domain_22_snapshot`, `dispatch-pg/weather_wire.rs:65` | E2E | yes | M | B-WEATHER |
| W05b | Daily world changes | Observe a player-visible transformation or service under the persisted daily decision. The row compares SQL flags and always fails | `run_packet`, `phase/full_row_w05b.rs:24` | E2E | yes | M | B-W05b |
| W05d | Seasonal calendar and events | Supply the missing Cake bridge item placement, then cross/use it and exercise the Hare raid under controlled calendar dates | `journey`, `phase/full_row_w05d.rs:109` | SERVER | yes | L | B-SEASON |
| W12 | Map discovery and donations | Verify the donated-area speed rule from authoritative movement observations after respawn/relog. The full row ends at its missing speed observation | `journey`, `phase/full_row_w12.rs:141` | SERVER | yes | M | B-W12 |
| W13 | Daily boosted creature and boss | Assert the selected creature’s XP/extra roll and the selected boss’s bonus/unique roll and respawn, then read back the award | `journey`, `phase/full_row_w13.rs:85` | E2E | yes | L | B-W13 |
| W16 | Summer population | Supply reachable encounter entry, then drive the Summer bosses and quest hand-ins to reward and relog | `journey`, `phase/full_row_w16.rs:60` | SERVER | yes | L | B-W16 |
| W03 | Boss reward chest | Earn a separate uncollected group, advance it past expiry and assert refused collection and fresh inventory | `journey`, `phase/full_row_w03.rs:138` | E2E | yes | M | B-W03 |
| W06 | Echo Warden | Observe portal trigger, enter, clear the raid, claim first-species charm and relog. The row checks a short portal search | `journey`, `phase/full_row_w06.rs:69` | E2E | yes | L | B-W06 |
| W07 | Soulpit | Link native Core/prism/seal acquisition and obelisk entry to waves, survival reward and durable return | `journey`, `phase/full_row_w07.rs:89`; `dispatch-pg/soulpit_shared_entry/native.rs:6` | E2E | yes | L | B-SOULPIT |
| W09 | Drome concoctions and Cube | Run positive USE for both items, assert exact cooldown and reward custody, then relog | `journey`, `phase/full_row_w09.rs:92`; `dispatch-pg/drome_cube.rs:58` | E2E | yes | M | B-W09 |
| W11 | Soul War and Rotten Blood | Complete both chains from accepted entry through boss kills, taint changes, reward and relog | `journey`, `phase/full_row_w11.rs:73` | E2E | yes | L | B-CHAINS |
| W17 | Arenas and Tibiadrome | Serve and fight the five native profiles with wave scaling and verify the resulting rank and rewards after relog | `journey`, `phase/full_row_w17.rs:98` | SERVER | yes | L | B-W17 |
| W19b | Planting | Harvest seeds on the applicable day, plant, observe growth/decay and verify custody after relog | `journey`, `phase/full_row_w19b.rs:91` | E2E | yes | M | B-W19b |
| W19c | Fireworks and toys | Drive a firework use and assert the native visual/audio cue and consume result. The positive scenario exercises trumpet decay | `journey`, `phase/full_row_w19c.rs:240` | E2E | yes | S | B-W19c |
| W19d | Appearance-unlock items | Consume a real unlock item, select the unlocked appearance and verify it after relog | `journey`, `phase/full_row_w19d.rs:91` | E2E | yes | M | B-W19d |
| W19e | Rust remover, lottery and anniversary items | Use obtainable inputs and assert the exact transformed or awarded item and saved custody | `journey`, `phase/full_row_w19e.rs:91` | E2E | yes | L | B-W19e |
| W21 | Boss difficulty | Start an accepted difficulty profile, fight and verify its durable result. The scenario terminates on `NoAcceptedProfile` | `journey`, `phase/full_row_w21.rs:99`; `dispatch-pg/boss_difficulty.rs:52` | SERVER | yes | L | B-W21 |
| ADD-0023 | Effects, damage numbers and sound | Correlate a live action’s committed damage with native effect, number and sound, then verify clean presentation after reconnect | `native/0023-presentation/tests/play.rs:49` | E2E | yes | M | B-PRESENTATION |
| ADD-0066 | Detailed refusal reasons | Drive native STEP/USE/MOVE refusals and assert reason, unchanged authoritative state and absence of stale errors on reconnect | `crates/session/src/features/0004-item-view-move/tests.rs:122` | E2E | yes | M | B-REFUSAL |
| ADD-0083 | Current World and Channel | Select/admit a World/Channel in the native client and verify the displayed identity after the next admission | `gap_world_channel_list`, `native/0083-channel-list/tests/play.rs:13` | E2E | yes | M | B-ENTRY |
| ADD-0286 | Visible entity names | Drive visibility leave/re-enter from live actors and verify exact native labels and no stale handles after reconnect | `dispatch-pg/entity_names.rs:12` | E2E | yes | M | B-NAMES |
| ADD-0287 | Learned and available spell catalogue | Learn a spell via a player action and assert the fresh native catalogue, including the distinction between learned and available | `catalog_replacement_reaches_panel_and_disconnect_clears_it`, `native/0287-actor-spell-catalog/tests/play.rs:5` | E2E | yes | M | B-CATALOGUE |
| S12 | Reports, GM tools and analysers | Drive native report submission and staff readback and relate an analyser result to a committed player event across reconnect | `dispatch-pg/player_report.rs:25`; `dispatch-pg/staff/nx1_native.rs:5` | E2E | yes | L | B-S12 |
| UI-CHARACTERS | Character and Channel selection | Native picker → selected character/Channel admission → same durable character on re-entry | `native/local-character-selection/mod.rs:118` | E2E | yes | M | B-ENTRY |
| UI-LOGIN | Native login | Native form → real Platform admission → world entry → reconnect to the same saved character | `native_login_round_trip_pending_edit_and_retry`, `apps/client/tests/native_login.rs:31` | E2E | yes | L | B-ENTRY |
| UI-MINIMAP | Minimap navigation | Drive click-to-walk from actual map frames, then render the saved position and current floor after relog | `minimap_projects_live_tiles_creatures_floor_changes_and_reset`, `native/local-minimap/tests/play.rs:39` | E2E | yes | M | B-MINIMAP |
| UI-OBJECT-OVERLAY | Interactive object markers | Click a live placement, assert the resulting overlay revision/native marker and fresh replacement | `overlay_delta_adds_a_marker`, `native/local-world-object-overlay/tests/play.rs:4` | E2E | yes | M | B-OVERLAY |
| UI-PREFERENCES | Local settings and shortcuts | Exercise settings changes through the native window and verify application after client reopen | `native/local-reference-preferences/tests/play.rs:263` | E2E | no | M | B-LOCAL |
| UI-SPELL-BAR | Spell bar, cooldown and resources | Native hotbar cast → real resource debit/cooldown → correct fresh actor state | `full_spell_row_dispatches_last_slot_and_projects_server_cooldown`, `native/local-spell-action-bar/tests/play.rs:203` | E2E | yes | M | B-SPELL-BAR |
| UI-WORLD-VIEW | Live map and world view | Native movement through a real floor change, then render fresh saved position and replacement geometry | `live_map_push_reaches_the_rendered_scene`, `native/local-world-view/tests/play.rs:39` | E2E | yes | M | B-WORLD-VIEW |

### Supplemental node boot, seam and wire

The S3-B composition and entry-room seam assert behaviour beyond start-up. The entry-room sequence includes admission negatives, concurrent control, recovery, USE and readmission. Their explicit Cargo `--ignored --exact` selection is how the disposable topology runs them. The spell seam has a real Exura payment/cooldown assertion and a probe-only strict branch. [Seam entry point](/workspace/Oteryn-Game/apps/game-server/src/qualification.rs:1612) · [Entry-room assertions](/workspace/Oteryn-Game/apps/game-server/src/qualification.rs:2991) · [S3-B composition](/workspace/Oteryn-Game/apps/game-server/tests/wp5_s3b_composition.rs:720)

| id | name | missing step | nearest test | lane | PG | size | batch |
|---|---|---|---|---|---|---|---|
| G05 | Settings import/export | Native save, export, restore/default, then reopen and assert the resulting settings | `run_packet`, `phase/full_row_g05.rs:16` | E2E | no | M | B-LOCAL |
| G06 | Native login challenge | Pinned Platform must issue a challenge so the client can complete it and enter the World | `journey`, `phase/full_row_g06.rs:35` | SERVER | yes | L | B-CHALLENGE |
| G08 | Mount taming | Tame a live source creature and verify unlock and custody after relog. Existing positive result is construction self-use | `journey`, `phase/full_row_g08.rs:96` | E2E | yes | M | B-S06 |
| G10 | Titles, analyser, boss tracker and markers | Drive native minimap marker add/remove and persistence alongside the server-backed views | `journey`, `phase/full_row_g10.rs:97` | E2E | no | M | B-MINIMAP |
| G22 | Store browse and popularity | Pinned Platform catalogue/balance response and sandbox receipt-backed popularity update | `journey`, `phase/full_row_g22.rs:128` | SERVER | yes | L | B-STORE |
| G24 | Market details and statistics | Generate a settled trade, then query its correlated statistics and order book again after relog | `journey`, `phase/full_row_g24.rs:186` | E2E | yes | M | B-G24 |
| G25 | Coin transfer and coin market | Exercise an accepted gift quote and coin listing with the Platform coin API on the qualification topology | `journey`, `phase/full_row_g25.rs:83` | SERVER | yes | L | B-STORE |
| G26 | House sale escrow | Offer from an owner, accept/settle as recipient, and verify funds and title after both re-admit | `journey`, `phase/full_row_g26.rs:65` | E2E | yes | L | B-G26 |
| G27 | Account friends and badge visibility | Exercise a real earned badge with a second account and read back friend/badge visibility after relog | `journey`, `phase/full_row_g27.rs:105` | E2E | yes | M | B-G27 |
| G28 | Bazaar and World transfer | Start from the supported client entry point, consume the Platform-authorized intent, then re-admit on the destination. The registered row asserts admission/logout only | `journey`, `phase/full_row_g28.rs:30` | CLIENT | yes | L | B-G28 |
| G29 | Quest tracker, filters and titles | Start a quest, earn/select a title, then exercise filters and verify the saved native views | `journey`, `phase/full_row_g29.rs:61` | E2E | yes | M | B-G29 |
| G31 | Native options, resource arcs and action bar | Drive native settings and new-session presentation. The registered row has no driver | `run_packet`, `phase/full_row_g31.rs:11` | E2E | no | M | B-LOCAL |
| G32 | Shrine quote and bug report | Exercise a positive shrine quote for a valid carried item. The bug-report receipt leg already exists | `journey`, `phase/full_row_g32.rs:168` | E2E | yes | M | B-G32 |
| G33 | Cake and Santa seasonal rows | Run at open epochs and exercise Cake extraction and Santa service instead of the closed-window skip | `journey`, `phase/full_row_g33.rs:46` | E2E | yes | M | B-SEASON |
| G34 | NPC/quest receipts | On fresh admission, read the committed progress back through a client journal/NPC response. Current final check rereads PostgreSQL | `journey`, `phase/full_row_g34.rs:163` | E2E | yes | M | B-G34 |
| G35 | Endgame entry, private loot and arena | Accepted boss entry, fight, private loot and a Svargrond arena clear and relog | `journey`, `phase/full_row_g35.rs:67` | E2E | yes | L | B-CHAINS |
| G36 | Soulseal fight entry | Use a carried Core on the real obelisk, assert the committed handoff and read durable return state | `journey`, `phase/full_row_g36.rs:87` | E2E | yes | L | B-SOULPIT |
| G37 | Native parity windows | Drive cooldown, cosmetics, imbuement, market and chat windows. The row reports FAIL even when its supported probes pass | `run_packet`, `phase/full_row_g37.rs:33` | E2E | yes | L | B-G37 |
| G38 | Windows touch controls | Drive actual native joystick and hotbar input across the session and prove the effect. The product input path already exists | `touch_joystick_uses_paced_keyboard_steps_and_release_stops`, `native/0233-touch-controls/tests/play.rs:36`; `phase/full_row_g38.rs:33` | E2E | yes | M | B-G38 |
| I13.client | Bag You Covet capture | Replace the unconditional SKIPPED with a captured real USE and native inventory replay | `run_packet`, `phase/client_row_i13.rs:10` | E2E | yes | M | B-BAGS |
| NB-CI | Same-head node boot | Run the selected node-boot shards and retain per-row verdicts for this staging SHA | `.github/workflows/pr-postgres-harness.yml:306` | E2E | yes | M | B-NB-CI |
| NB-VERDICT | Node-boot result gate | Require a verdict for each selected row and surface SKIPPED/MISSING/absent rows. The shell gate chiefly checks emitted FAIL, with stricter W19a handling | `stage`, `tools/qualification/node_boot/run.sh:1180` | E2E | no | M | B-NB-VERDICT |
| SEAM-STRICT | Strict spell seam | Run targeted, rune, high-level and Premium checks under the strict content policy. The branch logs probe dispositions and returns | `seam_spell_clients`, `apps/game-server/src/qualification.rs:2169`, branch at `:2328` | E2E | yes | L | B-SEAM |
| SEAM-SPELL | Wounded healing | Wound the caster, assert exact Exura healing and payment, then assert fresh resources | `seam_spell_clients`, `apps/game-server/src/qualification.rs:2169`, evidence at `:2322` | E2E | yes | M | B-SEAM |
| MAP-ROUTE | Unreachable sampled map objects | Find a legitimate approach to retained samples currently skipped by the 160-tile/two-floor budget | `check`, `phase/map_journey.rs:84`, skip at `:142` | E2E | yes | M | B-MAP-E2E |
| MAP-LADDER | Unresolved ladders | Supply authoritative traversal for catalogue ladders with neither a valid floor transition nor USE owner, then verify landing | `check`, `phase/map_journey.rs:84`, skip at `:268` | SERVER | yes | M | B-MAP-SERVER |
| MAP-SPAWN | Exact spawn identity | Expose or correlate enough authoritative identity to distinguish the sampled creature/NPC from another same-kind actor | `check`, `phase/map_journey.rs:84`, skip at `:265` | SERVER | yes | M | B-MAP-SERVER |
| MAP-FIELD | Field sample damage | Correlate stepping onto the sampled field with the exact effect and vitals delta. This map row checks movement | `check`, `phase/map_journey.rs:84`, movement assertion at `:275` | E2E | yes | M | B-MAP-E2E |
| WIRE-0055 | Guild bank | Negotiated deposit/withdraw command and reply through `Session`; the native scenario uses a scripted `Stepper` | `cp01_guild_bank_scenario`, `native/0055-guild-bank/tests/play.rs:110`; `dispatch-pg/guild_bank.rs:24` | E2E | no | S | B-GUILD |
| WIRE-0103 | Guild war | Negotiated war request and state reply through `Session` into native state | `pc02_guild_war_scenario`, `native/0103-guild-war/tests/play.rs:117` | E2E | no | S | B-GUILD |
| WIRE-0451 | Personal map marks | Negotiated query and encoded marks reply through `Session` into the native minimap | `personal_map_marks_native_query_publishes_only_confirmed_sets`, `native/0451-personal-map-marks/tests/play.rs:4`; `authority/questpc5_map_marks.rs:27` | E2E | no | S | B-MINIMAP |
| WIRE-0497 | Sanction record | Send the sanction query through `Session` and apply the returned own-character result | `sanction_window_requests_only_when_selected_and_shows_the_reply`, `native/0199-cyclopedia-character/tests/sanctions.rs:8`; `dispatch-pg/sanction_record.rs:54` | E2E | no | S | B-WIRE-0497 |
| WIRE-0500 | Server info | Send the quest and spell info requests through `Session` and project encoded replies into the journal | `server_info_quest_reply_reaches_bevy_rows_and_search`, `apps/client/src/bevy_shell/game_ui/journal/server_info_tests.rs:15`; `dispatch-pg/server_info_dispatch.rs:12` | E2E | no | S | B-WIRE-0500 |
| WIRE-0507 | Highscores catalogue and typed query | Test both negotiated commands through `Session`. The PostgreSQL scenario constructs the client result directly after dispatch | `highscores_catalogue_and_type_predicate_reach_dispatch_client_state_and_selectors`, `dispatch-pg/highscores_catalogue.rs:13` | E2E | no | S | B-S11 |
| WIRE-0596 | Item inscription attribution | Send a negotiated attributed read through `Session` and verify author/text projection in the editor | `inscription_read_waits_for_the_mail_capability`, `native/0115-mail/tests/play.rs:486`; handler `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/item_inscription.rs:8` | E2E | no | S | B-WIRE-0596 |

The wire entries are confirmed findings from the handler, consumer and test cross-check. I could not establish an exhaustive negative for the remaining capabilities. Shared raw-feature helpers and numeric registrations can carry a capability without naming it in the test. Two examples found during the cross-check are the numeric container-sort exchange at `crates/session/src/session_tests_capabilities.rs:282` and the status exchange macro at `crates/session/src/tests/status_projection_tests.rs:54`. The full wire denominator is therefore unverified.

Other source-declared skips are bounded setup branches, with related qualification work already in the table. Fixture-only skips cover browse-field, daily boost, death, depot search, hotkey equip, hunt and forge where the entry room lacks content. Conditional world probes skip hireling, imbuement, market, NPC shop, player trade, prey, stash, Store, task board and VIP when admission, fixture items, NPCs or an upstream service are absent. These skips establish no successful positive path for the affected run. The shell also reports `SKIP(needs …)` for dependent stages after a failed prerequisite. WP5 returns `BLOCKED` for absent/mismatched Platform checkout, missing PG service or spell manifest, and invalid prebuilt selection. Those are topology prerequisites. [Node-boot stage dependencies](/workspace/Oteryn-Game/tools/qualification/node_boot/run.sh:1180) · [WP5 preflights](/workspace/Oteryn-Game/tools/qualification/wp5_s3b/run.sh:53) · [Prebuilt validation](/workspace/Oteryn-Game/tools/qualification/wp5_s3b/run.sh:124)

### DROP

| id | name | missing step | nearest test | lane | PG | size | batch |
|---|---|---|---|---|---|---|---|
| W14a | Configurable XP stages | Owner exclusion | Mechanics ledger, W14a | DROP | no | — | — |
| W14b | Autoloot/autobank toggles | Owner exclusion | `native/0098-quick-loot/tests/play.rs:1`, ordinary quick loot | DROP | no | — | — |
| W14c | Paid OT VIP | Owner exclusion | `native/0043-vip/tests/play.rs:1`, ordinary VIP list | DROP | no | — | — |
| W14d | OT convenience commands | Owner exclusion | `native/0007-chat/tests/play.rs:1`, ordinary chat | DROP | no | — | — |
| W14e | OTC auras, shaders and wings | Owner exclusion | `native/0025-appearance/tests/play.rs:1`, ordinary appearance | DROP | no | — | — |
| S17a | Cast configuration | Owner exclusion | `journey`, `phase/full_row_s17a.rs:23` | DROP | no | — | — |
| S17b | Cast viewer and fan-out | Owner exclusion | `journey`, `phase/full_row_s17b.rs:61` | DROP | no | — | — |
| S17c | Cast chat and moderation | Owner exclusion | `journey`, `phase/full_row_s17c.rs:62` | DROP | no | — | — |
| S16c | Dawnport vocation trials | Owner exclusion | `journey`, `phase/full_row_s16c.rs:161` | DROP | no | — | — |
| S16d | Dawnport rooms, doors and departure kit | Owner exclusion | `journey`, `phase/full_row_s16d.rs:129` | DROP | no | — | — |
| Q128 | Dawnport Quest | Owner exclusion | `content/quests/dawnport-starts.json:1` | DROP | no | — | — |
| X01 | Skill/magic-level loss on death | Owner exclusion | `crates/gameplay/durability/src/durability/character_death/settlement.rs:97`, rule | DROP | no | — | — |
| X02 | Donor melee formula | Owner exclusion | `crates/gameplay/combat-rules/src/attack/formulas.rs:31`, rule | DROP | no | — | — |
| X03 | Donor spell formula | Owner exclusion | `crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs:70`, rule | DROP | no | — | — |
| X04 | AoE damage to own summons | Owner exclusion | `crates/gameplay/spell-rules/src/ordinary_combat.rs:35`, rule | DROP | no | — | — |
| X05 | Universal ground-item decay | Owner exclusion | `crates/gameplay/dispatch/src/gameplay_transport/corpse_decay.rs:61`, corpse owner | DROP | no | — | — |
| X06 | Donor Lua and admin API | Owner exclusion | `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs:1024`, native owner | DROP | no | — | — |
| P23a | Prey Hunting Tasks selection | Ledger owner exclusion | `native/0140-prey/tests/play.rs:101`, Prey bonuses | DROP | no | — | — |
| P23b | Prey Hunting Tasks kills | Ledger owner exclusion | `native/0140-prey/tests/play.rs:101`, Prey bonuses | DROP | no | — | — |
| P23c | Prey Hunting Tasks rewards | Ledger owner exclusion | `journey`, `phase/full_row_p23c.rs:49` | DROP | no | — | — |
| E11 | Transfer coins and Bazaar | Ledger marks the Game mechanic N/A under Platform ownership | `journey`, `phase/full_row_g28.rs:30`, related supplemental boundary | DROP | no | — | — |

### Remaining mechanics ledger rows with journey implementations

These are the other **79 of 204** ledger IDs. Each has a scenario implementation for the row or its applicable transient checks. This is source evidence. Its execution on this staging SHA remains unverified. The other 125 are accounted for in the gap and DROP tables.

| area | row and test entry point |
|---|---|
| Combat and spells | C07 `journey`, `phase/full_row_c07.rs:47` · C08 `journey`, `phase/full_row_c08.rs:181` · C10 `journey`, `phase/full_row_c10.rs:128` · C14 `journey`, `phase/full_e2ef18.rs:293` · C17b `journey`, `phase/full_row_c17b.rs:180` · C18b `journey`, `phase/full_row_c18b.rs:202` · C19a `starter_backpack`, `phase/full_e2ef6.rs:786` |
| Movement | M01 `m01`, `phase/full_e2ef1.rs:399` · M02 `m02`, same file `:558` · M03 `m03`, `:725` · M04 `m04`, `:866` · M05 `m05`, `:1141` · M07 `m07`, `:1209` · M08 `journey`, `phase/full_row_m08.rs:428` · M09 `journey`, `phase/full_row_m09.rs:342` · M10 `journey`, `phase/full_row_m10.rs:296` · M11a `journey`, `phase/full_row_m11a.rs:70` |
| Items | I01 `inventory_journey`, `phase/full_e2ef3_e2ei-a.rs:344` · I03 `journey`, `phase/full_row_i03.rs:42` · I04 `documents`, `phase/full_e2ef3_e2ei-b.rs:565` · I05 `journey`, `phase/full_row_i05.rs:249` · I07 `loot_policy`, `phase/full_e2ef3_e2ei-c.rs:752` · I09 `furniture`, `phase/full_e2ef3_e2ei-b.rs:180` · I11 `journey`, `phase/full_row_i11.rs:244` · I12 `refine`, `phase/full_e2ef3_e2ei-d.rs:401` · I14 `journey`, `phase/full_row_i14.rs:154` · I16 `imbuement_journey`, `phase/full_e2ef3_e2ei-a.rs:540` · I17a/b/c `journey`, `phase/full_row_i17a.rs:290`, `phase/full_row_i17b.rs:288`, `phase/full_row_i17c.rs:301` · I18b `journey`, `phase/full_row_i18b.rs:243` |
| Economy | E02 `journey`, `phase/full_e2ef20.rs:280` · E08 `run`, `phase/full_row_e08.rs:178` · E09 `journey`, `phase/full_row_e09.rs:443` |
| Progression | P01 `journey`, `phase/full_row_p01.rs:169` · P02 `journey`, `phase/full_row_p02.rs:181` · P03 `run_packet`, `phase/full_row_p03.rs:88` · P04 `run_packet`, `phase/full_row_p04.rs:135` · P05 `journey`, `phase/full_row_p05.rs:619` · P06 `journey`, `phase/full_row_p06.rs:199` · P07 `journey`, `phase/full_row_p07.rs:137` · P08 `journey`, `phase/full_row_p08.rs:387` · P09 `journey`, `phase/full_row_p09.rs:186` · P11 `journey`, `phase/full_row_p11.rs:110` · P12a/b/c `journey`, `phase/full_row_p12a.rs:137`, `phase/full_row_p12b.rs:117`, `phase/full_row_p12c.rs:393` · P14 `journey`, `phase/full_row_p14.rs:59` · P15 `journey`, `phase/full_row_p15.rs:265` · P17 `journey`, `phase/full_row_p17.rs:225` · P18 `journey`, `phase/full_row_p18.rs:180` · P20 `run_packet`, `phase/full_row_p20.rs:397` · P22a/b/c `journey`, `phase/full_row_p22a.rs:71`, `phase/full_row_p22b.rs:83`, `phase/full_row_p22c.rs:82` |
| Social and housing | S02 `journey`, `phase/full_row_s02.rs:302` · S04a `journey`, `phase/full_row_s04a.rs:224` · S04b `journey`, `phase/full_e2ef19.rs:168` · S04c `journey`, `phase/full_row_s04c.rs:98` · S05 `journey`, `phase/full_row_s05.rs:231` · S07 `journey`, `phase/full_row_s07.rs:54` · S13c `journey`, `phase/full_row_s13c.rs:211` · S14a `journey`, `phase/full_row_s14a.rs:95` · S15a/b `journey`, `phase/full_row_s15a.rs:15`, `phase/full_row_s15b.rs:75` · S16b `journey`, `phase/full_row_s16b.rs:255` |
| World, encounters and journal | W01a `journey`, `phase/full_row_w01a.rs:163` · W01c `journey`, `phase/full_row_w01c.rs:89` · W02 `journey`, `phase/full_row_w02.rs:120` · W04 `journey`, `phase/full_row_w04.rs:129` · W05a `journey`, `phase/full_row_w05a.rs:86` · W05c `journey`, `phase/full_row_w05c.rs:468` · W08 `journey`, `phase/full_row_w08.rs:136` · W10 `journey`, `phase/full_row_w10.rs:112` · W15a `journey`, `phase/full_row_w15a.rs:170` · W15b `journey`, `phase/full_row_w15b.rs:158` · W19a `journey`, `phase/full_row_w19a.rs:332` · W22a `journey`, `phase/full_row_w22a.rs:301` · W24 `journey`, `phase/full_row_w24.rs:186` |

## Batches

The batch column gives the assignment for every row. **A batch owns only its proposed edit files.** Existing tests cited in the tables can be read by any batch. The following assignments handle the scenario files shared by multiple gaps.

| batches | proposed file ownership |
|---|---|
| B-SPELL-1, B-SPELL-2 | Separate summon/barrier/paralyze/companion and utility/aim test modules. Read `full_e2ef5.rs` from both. |
| B-DEATH, B-P10, B-P16, B-S16a | Death owns the four death cases. Rest, blessing services and vocation each own a separate test module. All read the existing F6 helpers. |
| B-ECON-1, B-SOCIAL-1, B-GUILD | Separate economy, party/logout/speech and guild scenario modules. Read `full_e2ef4.rs` from each. B-GUILD also owns the guild bank and war wire test cases. |
| B-P19, B-P21, B-S08, B-S11, B-PARTY-SPELL, B-LIGHT | Separate scenario modules for influenced loot, imbuement, PvP, ranking, party spells and light. Read `full_e2ef7.rs` from each. B-S11 owns the highscores-catalogue wire case. |
| B-DECAY, B-BAGS, B-RING | Separate corpse, bag and ring scenario modules. B-BAGS owns the I13 capture/replay case too. Keep the shared F3 packet files as reference. |
| B-MAP-E2E, B-MAP-SERVER | Separate map observation test module and ladder/spawn owner files. The observation module can consume the production owners when ready. |
| B-NB-CI, B-NB-VERDICT, B-SEAM | Workflow selection files, node-boot result gate and its tests, and WP5 spell seam tests respectively. B-NB-VERDICT owns changes to the expected-failure ledger after row evidence is available. |
| All other `B-*` | Own the corresponding feature’s dedicated scenario or native-wire test files. Multi-row assignments are B-TEAM, B-S06, B-SEASON, B-STORE, B-SOULPIT, B-CHAINS, B-LOCAL, B-ENTRY and B-MINIMAP. |
| `Q000`–`Q351` where listed below | One quest per batch. Each owns a quest-specific authority/journey test file. Reuse the mounted authority directory for new PG cases. Shared content shards and qualification registries are read inputs to these evidence batches. |

The largest batches have **five** gaps. The table uses dedicated files for batches that reference the same older packet, so those batches have disjoint proposed edits. `B-W15d` owns the generic node journey check. The `Q` batches own their individual quest cases.

## Quest counts and exact missing lists

I used the **352 canonical quest definitions** as the denominator. The 373 wiki titles include aliases and do not represent 373 separate content entries. `Qnnn` is the zero-based position in the committed definition shards, using each shard’s declared start offset. These IDs specify the exact missing entries even for similar quest titles. [Canonical quest index](/workspace/Oteryn-Game/content/quests/definitions/index.json:1) · [Wiki-title crosswalk](/workspace/Oteryn-Game/tools/content-schema/quest-authoring/samples/completion-matrix/all373.json:1)

| measure | count | evidence |
|---|---:|---|
| Canonical quests with all registered source-placement reward claims qualified on PostgreSQL | **133 / 352** | The coverage ledger has 333 `dispatch_pg_qualified` claim keys. Joining fully qualified, nonempty reward rows back to canonical quest identities yields 133 quests. The unrelated `unlinked` row has no canonical quest identity. The 35 in the completion matrix count a reward-only subset from a separate proof registry. |
| Full quest journey with fresh relog after the finale and reward | **2 / 352** | **Q200 Serpentine Tower** and **Q241 The Shattered Isles**. These are focused production dispatch/PG journey cases. |
| Additional full quest route with real recovery of the same session | **2** | **Q052 Koshei the Deathless** and **Q213 Thais Lighthouse**. Both exercise server recovery and check the retained reward. They still lack the fresh relog specified for the main count. |
| Active quests lacking full-journey evidence | **349** | All canonical IDs except `Q128` (DROP), `Q200` and `Q241`. |
| Active quests lacking complete canonical PG reward-claim qualification | **218** | Exact ID list below. This includes quests with no registered claim proof. A quest whose reward is access, XP or a service should qualify that outcome through its actual owner. |

Sources for the counts and the four positive route witnesses follow.

- `quest_catalogue_chests_slices_cover_every_supported_source_placement` enumerates the catalogue cases. The PostgreSQL execution loop requires the PG route and emits a marker per executed placement. The status used in the count is `dispatch_pg_qualified`. [Claim ledger](/workspace/Oteryn-Game/docs/repository/QUEST_CALLBACK_COVERAGE.json:349) · [Catalogue case enumeration](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs:66) · [PG execution](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/support/quest_catalogue_chests_fixtures.rs:765)
- **Q200 Serpentine Tower** uses the tower route, claims the pearl, reconnects under a fresh session, rejects a second use and verifies completed progress, one pearl and one claim. [Test](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/serpentine_tower_journey.rs:4) · [Fresh session and repeat refusal](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/support/serpentine_tower_journey_fixtures.rs:382) · [Durable reward oracle](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/support/serpentine_tower_journey_fixtures.rs:491)
- **Q241 The Shattered Isles** exercises the four passages, terminal hand-in and side chest, then checks receipts, access and journal after relog. [Test](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/shattered_isles_journey.rs:69) · [Terminal and relog](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/shattered_isles_journey.rs:447) · [Reward/access oracle](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/shattered_isles_journey.rs:927)
- **Q052 Koshei** checks the final reward under the next connection generation, and **Q213 Thais Lighthouse** resumes after collecting both rewards. [Koshei test](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/koshei_the_deathless_journey.rs:7) · [Koshei resume](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/support/koshei_the_deathless_journey_fixtures.rs:662) · [Thais test](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/thais_lighthouse_journey.rs:63) · [Thais resume](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/thais_lighthouse_journey.rs:330)

**Missing full journey**, inclusive ranges: `Q000–Q127`, `Q129–Q199`, `Q201–Q240`, `Q242–Q351`.

**Missing complete PG reward claims**, all active IDs, inclusive ranges:

`Q105–Q111`, `Q113–Q120`, `Q122–Q125`, `Q127`, `Q129–Q132`, `Q134`, `Q136–Q144`, `Q146–Q166`, `Q168–Q179`, `Q181–Q198`, `Q202–Q212`, `Q214–Q215`, `Q217–Q221`, `Q223–Q224`, `Q226`, `Q229–Q231`, `Q233`, `Q236`, `Q238–Q239`, `Q242–Q243`, `Q245–Q247`, `Q249–Q250`, `Q252–Q262`, `Q264–Q278`, `Q280–Q351`.

The definitions and names for every ID above are pinned in these four shards. [Q000–Q099](/workspace/Oteryn-Game/content/quests/definitions/quests-00000-00099.json:1) · [Q100–Q199](/workspace/Oteryn-Game/content/quests/definitions/quests-00100-00199.json:1) · [Q200–Q299](/workspace/Oteryn-Game/content/quests/definitions/quests-00200-00299.json:1) · [Q300–Q351](/workspace/Oteryn-Game/content/quests/definitions/quests-00300-00351.json:1)

### Quest routing

Each ID in the missing-journey list is one gap and one same-ID batch. The following exceptions replace the default evidence row where I found a more specific boundary. A missing canonical PG reward claim is included in the same quest gap. For the remaining quests, E2E is a provisional intake lane because an absent qualification record alone cannot establish that the server behaviour is missing. The exact first missing product step was unresolved for those quests.

| id | name | missing step | nearest test | lane | PG | size | batch |
|---|---|---|---|---|---|---|---|
| Q001 | Alawar’s Vault | Add a fresh relog after the final return teleport. Its existing relog occurs after chest reward and before the finale | `alawars_vault_full_journey`, `dispatch-pg/alawars_vault_journey.rs:8`; finale at `dispatch-pg/support/alawars_vault_journey_fixtures.rs:701` | E2E | yes | S | Q001 |
| Q002 | An Interest in Botany | Verify terminal XP/receipt after a new admission fence. The fixture rebuilds projections on retained ingress | `an_interest_in_botany_full_journey`, `dispatch-pg/an_interest_in_botany_journey.rs:83`, retained ingress at `:230` | E2E | yes | S | Q002 |
| Q007 | Behemoth | Drive entry and actual world chest use through the fourth claim, then verify the completion and custody after relog | `qn2_behemoth_claims_complete_only_after_the_fourth_chest`, `authority/qn2_claim_sets.rs:20` | E2E | yes | M | Q007 |
| Q012 | Circle Room | Drive both chests from player entry and read back the final completion through a fresh session | `qn2_circle_room_claims_complete_only_after_both_chests`, `authority/qn2_claim_sets.rs:34` | E2E | yes | M | Q012 |
| Q052 | Koshei the Deathless | Fresh relog after the recovered-session reward check | `koshei_the_deathless_full_journey`, `dispatch-pg/koshei_the_deathless_journey.rs:7` | E2E | yes | S | Q052 |
| Q068 | Panpipe | Join the numbered-key and bag routes, then verify the terminal reward and completion from the fresh client | `quest_reward_claims_panpipe_locked_door_to_reward_and_replay`, `dispatch-pg/quest_reward_claims.rs:365`; key relog at `:1192` | E2E | yes | M | Q068 |
| Q213 | Thais Lighthouse | Fresh relog after the two chest rewards and RESUME replay | `thais_lighthouse_full_journey`, `dispatch-pg/thais_lighthouse_journey.rs:63` | E2E | yes | S | Q213 |
| Q219 | Djinn War, Efreet | Fresh relog and readback of the terminal unlocked services and reward | `questl2_djinn_efreet_full_player_journey`, `dispatch-pg/questl2_djinn.rs:36` | E2E | yes | M | Q219 |
| Q220 | Djinn War, Marid | Fresh relog and readback of the terminal unlocked services and reward | `questl2_djinn_marid_full_player_journey`, `dispatch-pg/questl2_djinn.rs:48` | E2E | yes | M | Q220 |
| Q234 | Outlaw Camp | Fresh admission after the terminal chest. The current reconnect reconstructs an owner under retained admission | `outlaw_camp_full_journey`, `dispatch-pg/outlaw_camp_journey.rs:65`, retained admission at `:324` | E2E | yes | S | Q234 |
| Q235 | Paradox Tower | Fresh admission after the riddle/reward. The test replays on retained duplex continuity and reads the journal there | `paradox_tower_full_journey`, `dispatch-pg/paradox_tower_journey.rs:7`; reward/replay at `dispatch-pg/support/paradox_tower_journey_fixtures.rs:860` | E2E | yes | S | Q235 |
| Q247 | The Way of the Monk | Earn the shrine prerequisites from entry, continue past the guidance callback to finale/reward and relog | `journey`, `phase/full_row_w15d.rs:123` | E2E | yes | L | Q247 |
| Q251 | Threatened Dreams | Continue beyond the poacher-book mission through the remaining story and terminal reward, then relog | `threatened_dreams_poacher_book_journey`, `dispatch-pg/threatened_dreams_journey.rs:32` | E2E | yes | L | Q251 |
| Q254 | Too Hot to Handle | Continue beyond the research repairs/white gem, then exercise the finale and reward and relog | `too_hot_research_native_coffin_repairs_and_white_gem`, `dispatch-pg/quest_native_journeys.rs:7` | E2E | yes | L | Q254 |
| Q255 | Tower Defence | Fresh relog and terminal completion/reward readback after the contact/password/arrow flow | `tower_defence_native_contacts_earn_password_and_four_arrow_stacks`, `dispatch-pg/quest_native_journeys.rs:17` | E2E | yes | M | Q255 |
| Q265 | Wrath of the Emperor | Bind the outstanding `chest_items` selector on the production World, prove mint and once-only replay, then drive the whole quest through relog | `quest_catalogue_chests_qchk6_wrath_of_the_emperor_use_replay_and_refusal`, `dispatch-pg/quest_catalogue_chests_qchk6.rs:44`; blocker `docs/agents/evidence/quest-claim-qualification.json:1` | SERVER | yes | L | Q265 |
| Q320 | Nomads Land | Reach the cabinet through the authored entry/missions, complete the quest, then read back reward and completion after relog | `questpc5_nomads_cabinet_mints_bound_child_text_atomically_and_replays`, `authority/questpc5_reward_contents.rs:41` | E2E | yes | M | Q320 |
| Each remaining ID in the full-journey missing list | Canonical name at that position in the linked shard | Supply executable evidence from entry through unproved missions, finale and the authored reward, followed by fresh relog. Include exact PG reward proof for IDs in the reward-missing list | `quest_log_recorded_domain_16_frames_reach_the_play_panel`, `native/0016-quest-log/tests/recorded.rs:41`; qualified chest subset `dispatch-pg/quest_catalogue_chests.rs:113` | E2E | yes | L | Same as ID |

The reward count is an all-registered-source-claims count. Nearby authority tests can prove a narrower result even for a quest in the missing list. Examples include the guarded Forgemaster chest, the Nomads cabinet’s bound text and the M34 tests for one chosen Magic Sword vault and one Mastermind cache. [Forgemaster tests](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_container_chests.rs:28) · [Nomads cabinet authority](/workspace/Oteryn-Game/apps/game-server/tests/support/authority_cases/questpc5_reward_contents.rs:41) · [M34 tests](/workspace/Oteryn-Game/crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/engclaim_m34.rs:537)

## Weak or timing-sensitive tests

| file:line | weakness |
|---|---|
| `phase/full_row_c15.rs:165`, `:183` and `phase/client_row_c15.rs:140`, `:160` | Fixed two-second setup and thirteen-minute offline waits. The full row expects exactly one minute of recovered stamina, which is sensitive to the clock and scheduling boundary. |
| `phase/client_row_e01.rs:21`, `phase/client_row_e02.rs:82`, `phase/client_row_p20.rs:58`, `phase/client_row_s10.rs:29`, `phase/client_row_s16a.rs:53`, `phase/client_row_w05c.rs:22`, `phase/client_row_w16.rs:32`, `phase/full_e2ef20.rs:117` | Fixed 260 ms settling before a follow-up action. This assumes the preceding owner processed its command/cooldown by then. |
| `phase/client_row_s18.rs:60` | Fixed 1.1-second wait at the speech timing boundary. |
| `authority/engitem_tools/corpse.rs:106`, `:184`, `:336` | Sleep-based corpse timing. The two 10.05-second waits leave a narrow margin around the target boundary. |
| `authority/pvp_state_refresh.rs:86` | Uses a fixed 200 ms window while testing lock/sampling order. Explicit synchronization would make the order observable. |
| `dispatch-pg/quest_reward_claims.rs:1173`, `:1280`, `:1285`, `:1405` | Fixed 210 ms waits between USE operations. |
| `phase/full_row_p22d.rs:144` | Drops the pushed event stream while trying to establish live tracker refresh. `crates/session/src/domain_dispatch.rs:28` already routes the tracker domain, and `native/0197-cyclopedia-tracker/protocol.rs:72` consumes it. |
| `phase/full_row_g28.rs:30` | The scenario asserts admission and logout. It never sends a Bazaar or transfer intent. |
| `phase/full_row_g38.rs:36` | A non-moving synthetic STEP can leave the actor at the expected position and reach the closing assertion. No native touch input is exercised. |
| `phase/full_row_g34.rs:252` | Opens a fresh session but reads the final quest state from SQL only. |
| `phase/full_row_w13.rs:155` | A generic hunt and inventory delta do not identify boosted XP, bonus/unique loot or the boss respawn. The scenario explicitly fails those unproved steps. |
| `apps/game-server/src/qualification.rs:2328` | The strict spell loop records dispositions for untargeted catalog probes and can finish without positive targeted-spell assertions. |
| `tools/qualification/node_boot/run.sh:1214` | Stage success can coexist with unreported, skipped or missing mechanic rows. The separate E2E report has stricter completeness rules. |

## Counts per lane

| counted work | E2E | SERVER | CLIENT | DROP |
|---|---:|---:|---:|---:|
| Mechanics, node boot, seam, confirmed wire gaps and exclusions | 126 | 13 | 1 | 21 |
| Quest entries, with reward and full-journey deficits combined per quest | 348 | 1 | 0 | 0 |
| **Total routed or dropped** | **474** | **14** | **1** | **21** |

Dawnport Quest is counted once under DROP. The totals cover the gaps listed here, including provisional quest intake. The repo-wide wire total and same-head node-boot outcomes remain unverified.