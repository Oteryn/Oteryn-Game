# ACCEPT-BATCH-2026-10-15 Owner acceptance batch

- Decision: `ACCEPT-BATCH-2026-10-15`
- Status: **ACCEPTED BY THE OWNER 2026-10-15 VIA THE CONTROL PLANE**. Owner `blakinio` answered the architect's consolidated decision list ("masz zgody na wszystko", owner accepted via control plane). The acceptance takes effect only when this PR passes exact-head validation, independent review and CI on its frozen head and is integrated through the protected merge path. It adds no rule text and changes no code, wire, migration or content, except the plan edits of §4, which record owner and architect rulings.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Source of authority: the owner decisions recorded at https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6187301378 (control plane, #1622).
- Precedent: `OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md`.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Runtime, migration, wire and production authority: NONE. Each child still needs its own #162 allocation.

## 1. Accepted briefs (owner items 1 to 6)

All paths are under `docs/architecture/reviews/`. The gate is not waived: where a brief names a prerequisite, the prerequisite still binds the child packets.

| Item | Brief | File |
|---|---|---|
| 1 | BAGS-0 | `OTERYN_GAME_BAGS0_CONTAINERS_WITH_CONTENTS_DECISION_2026-09-30.md` |
| 1 | STACK-0 | `OTERYN_GAME_STACK0_PARTIAL_COUNTS_AND_STACK_MERGES_DECISION_2026-10-01.md` |
| 1 | B3 | `OTERYN_GAME_B3_INVENTORY_DESTINATION_CAPACITY_AND_STACKS_DECISION_2026-09-28.md` |
| 1 | ITEM-MOVE-WIRE-1 | `OTERYN_GAME_ITEM_MOVE_WIRE1_EQUIP_AND_DROP_DECISION_2026-09-30.md` |
| 1 | STARTER-BACKPACK-0 | `OTERYN_GAME_STARTER_BACKPACK0_STARTER_GRANT_DECISION_2026-09-30.md` |
| 2 | A13 | `OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md` |
| 3 | CREATURE-AI-0 | `OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md` |
| 3 | RANGED-0 | `OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md` |
| 4 | NPC-0 | `OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md` |
| 4 | BANK-FEE-0 | `OTERYN_GAME_BANK_FEE0_FEES_FROM_THE_BANK_DECISION_2026-09-30.md` |
| 5 | CHARACTER-APPEARANCE | `OTERYN_GAME_CHARACTER_APPEARANCE_OWNER_DECISION_2026-09-28.md` |
| 5 | BOSS-RAID-0 | `OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md` |
| 5 | MONSTER-D15 (with BOSS-RAID-0) | `OTERYN_GAME_MONSTER_D15_BESTIARY_DECISION_2026-10-03.md` |
| 5 | ADMIT-0 | `OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md` |
| 5 | DEATH-COMPLETE-0 | `OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md` |
| 5 | OFFLINE-0 | `OTERYN_GAME_OFFLINE0_STAMINA_AND_OFFLINE_TRAINING_DECISION_2026-10-01.md` |
| 5 | VIP-0 | `OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md` |
| 5 | IMBUE-FORGE-0 | `OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md` |
| 5 | BED-0 | `OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md` |
| 5 | SEC-CLIENT-01 | `OTERYN_GAME_SEC_CLIENT01_CLIENT_INTEGRITY_AND_ANTI_BOT_DECISION_2026-10-01.md` |
| 5 | CHARM-4 | `OTERYN_GAME_CHARM4_INCOMING_EFFECT_ORDER_DECISION_2026-10-03.md` |
| 5 | FAMILIARS-0 | `OTERYN_GAME_FAMILIARS0_FAMILIARS_DECISION_2026-10-03.md` |
| 5 | QUEST-GATE-0 | `OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md` |
| 5 | QUEST-STATE-0 (with QUEST-GATE-0) | `OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md` |
| 5 | PLAYER-TRADE-0 | `OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md` |
| 5 | MAIL-0 | `OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md` |
| 5 | MARKET-0 | `OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md` |
| 5 | PREY-0 | `OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md` |
| 5 | DAILY-REWARD-0 | `OTERYN_GAME_DAILY_REWARD0_DAILY_REWARD_SYSTEM_DECISION_2026-10-03.md` |
| 5 | ACHIEVEMENT-0 | `OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md` |
| 5 | HIGHSCORES-0 | `OTERYN_GAME_HIGHSCORES0_WORLD_HIGHSCORES_DECISION_2026-10-03.md` |
| 5 | ANALYSERS-0 (with HIGHSCORES-0) | `OTERYN_GAME_ANALYSERS0_HUNTING_ANALYSERS_DECISION_2026-10-03.md` |
| 5 | TIMED-ITEM-0 | `OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md` |
| 5 | TIMED-ITEM-0B (with TIMED-ITEM-0) | `OTERYN_GAME_TIMED_ITEM0B_RUNTIME_CHARGES_AND_DURATION_DECISION_2026-10-03.md` |
| 5 | DEPOT-0 | `OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md` |
| 5 | SUPPLY-STASH-0 (STASH-0, with DEPOT-0) | `OTERYN_GAME_SUPPLY_STASH0_SUPPLY_STASH_DECISION_2026-10-03.md` |
| 5 | WRITE-0 (with DEPOT-0) | `OTERYN_GAME_WRITE0_BOOKS_SCROLLS_AND_BLACKBOARDS_DECISION_2026-10-01.md` |
| 5 | TRAVEL-0 | `OTERYN_GAME_TRAVEL0_NPC_TRAVEL_DECISION_2026-09-30.md` |
| 5 | CYCLOPEDIA-0 | `OTERYN_GAME_CYCLOPEDIA0_MAP_DISCOVERY_AND_AREA_DONATIONS_DECISION_2026-10-03.md` |
| 5 | EXERCISE-0 | `OTERYN_GAME_EXERCISE0_EXERCISE_WEAPONS_AND_DUMMIES_DECISION_2026-10-03.md` |
| 6 | ECON-RET-0 | `OTERYN_GAME_ECON_RET0_GUILD_AND_MARKET_EVENT_RETENTION_DECISION_2026-10-04.md` |

Every one of these files has its Status line set to ACCEPTED and cites this record. Amendments they wrote as "pending on acceptance of themselves" become effective in their targets when the target is itself accepted; an amended document keeps its wording and no contract text changes here.

Gate notes:
- CREATURE-AI-0 and RANGED-0 are accepted **without waiving the gate** (owner item 3).
- CHASE-1 (#2113) merged **before** the RANGED-0 acceptance. This record notes that retroactively: RANGED-0 was cited while CANDIDATE, and CHASE-1 was merged on that basis. The acceptance covers it; no re-merge is needed.
- BED-0 and ECON-RET-0 were left CANDIDATE by ACCEPT-SOCIAL-MAP-0 (D545); this record releases them. The HOUSE-RUNTIME-0 amendments to ADMIT-0 and BED-0 can take effect with this acceptance.

## 2. Owner rulings (items 7 to 12)

| Item | Ruling |
|---|---|
| 7 | **HIRE-1** goes ahead and is not deferred. A brief is needed: `OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md` (CANDIDATE). Delivery follows Tibia: the hireling is a Store product, wired once the Store contract lands (Platform #1625). |
| 8 | **SOCIAL-LATE-1:** as in Tibia. No marriage. The PvP arena moves to ARENA-1. |
| 9 | **LOOP-GROUND-1:** no periodic ground cleanup between resets. Revisit only if the server is heavily loaded in practice. |
| 10 | **WEATHER-1** is visual only, with no gameplay effect. |
| 11 | **Platform #1627:** the 4 h provisional acknowledgement bound (RTO) is ratified as provisional, to be reviewed after the first restore drill. A Platform matter; recorded here only. |
| 12 | **Store S-1 to S-9 (Platform #1625):** still open. The owner asked for plain-language recommendations first. Nothing is accepted. |

## 3. Architect rulings (A)

Executed by the control plane within architect scope:
- **Status-line reconciliation.** GUILD-0, PARTY-PVP-0, HOUSE-OWN-0, HOUSE-CUSTODY-0, HOUSE-RUNTIME-0 and ADR-0021 now read ACCEPTED, citing ACCEPT-SOCIAL-MAP-0 (they were accepted on 2026-10-04 and their status lines were left behind). The plan's Appendix 7 status list is refreshed.
- **CRAFT-1 brief** written: `OTERYN_GAME_CRAFT1_DECISION_2026-10-15.md` (CANDIDATE, architect-authored).
- **PREM-2b+PREM-5 hold released.** The promotion source is the PREM-5 NPC promotion: 20 000 gp, level 20 or higher, Premium (PREMIUM_ACTIVATION §4.2, D70), paid in coins via GOLD-FEE-1b. "Promotion purchase" is removed from NPC-ECON-1.
- **Registry order:** ITEM-LOOK-1, then WEATHER-1, then SEC-1.
- **LOOP-GROUND per-tile limit:** 64 items, "not enough room" when the tile is full. ADR-0021 D191 keeps the no-timed-decay rule.
- **TUTORIAL-1** brief comes after Wave 4.

## 4. Changes in this PR

- the Status line of each brief in §1 (ACCEPTED, citing this record);
- the Status lines of GUILD-0, PARTY-PVP-0, HOUSE-OWN-0, HOUSE-CUSTODY-0, HOUSE-RUNTIME-0 and `ADR-0021-world-map-runtime-loading.md` (ACCEPTED, citing ACCEPT-SOCIAL-MAP-0);
- `OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md`: Appendix 7 status list; PREM-2b+PREM-5; NPC-ECON-1; the WEATHER-1, SEC-1 and ITEM-LOOK-1 order; LOOP-GROUND-1; HIRE-1; SOCIAL-LATE-1; TUTORIAL-1; CRAFT-1 rows. No file under `docs/contracts/` encodes this order or these statuses, so no registry changed;
- the two new CANDIDATE briefs CRAFT-1 and HIRE-1.

## 5. Unblocked

Each child still needs its own #162 allocation and any other accepted base that its packet names. The plan's Appendix 7 graphs list the children of each accepted brief; the "no child of a CANDIDATE brief" gate no longer applies to them. The briefs still CANDIDATE are listed in the plan's Appendix 7 status paragraph.
