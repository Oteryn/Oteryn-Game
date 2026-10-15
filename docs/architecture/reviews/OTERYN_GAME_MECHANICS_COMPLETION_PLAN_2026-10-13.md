# Mechanics completion plan (server track)

- Plan: `MECHANICS-COMPLETION-PLAN-V1`
- Status: **CANDIDATE** (plan, not a contract). Allocation of each batch needs the control plane; batches that
  change a contract still need their own decision.
- Role: client-track architect, delegated by the control plane (#1622), at the owner's direction of 2026-10-13
  ("all non-DONE mechanics go through the architect")
- Input: control-plane audit of 115 of 139 mechanics rows that are not DONE (row numbers below are the audit's)
- Admission baseline: `main@4ac13390`; merged-work check against `main@ab7d51ae6` on 2026-10-13 (rows marked MERGED below)
- Runtime, registry, migration, protocol and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What this plan does

It puts every non-DONE row in exactly one place: a batch (a coherent unit of work), an existing in-flight PR, a
client-owned item (#1942), a Platform-owned item, or out of scope. It orders the batches in waves and says which
need a migration, a protocol-registry PR, and a hard worker. Profile: Global Tibia at 2026-09-27 (D33); anything
else is a decision first.

## 2. Unknowns resolved from code (main)

| Row | Fact | Result |
|---|---|---|
| 32 Soul | Soul is in actor vitals and paid on a cast (`spell/cast.rs`, `actor_spell_v1.proto`), cost in spell content, max 100 (`domain/premium.rs`). No regeneration and no soul-orb item exist. | PARTIAL: regen and item go to LOOP-VITALS-1 |
| 75 Look | No look/examine message in the protocol registry and no server handler. | ABSENT: ITEM-LOOK-1 |
| 94 Monster respawn | `drive_spawn_respawns` runs only inside `drain_monster_melee`, which runs while a session is admitted and ticking; no standalone scheduler. A comment in `lib.rs:34` ("uncalled") is stale. | PARTIAL: world-owned scheduler in LOOP-MONSTER-1 |
| 139 Tutorial | No Rookgaard/Newhaven map, quest or runtime content (reference and tooling only). | ABSENT: content batch, owner question Q1 |
| 66 Cap and weight | No weight or capacity check on pickup, move or equip; only backpack entry count. | ABSENT: ITEM-LOOP-1 |
| 20 Fight modes | Offensive/Balanced/Defensive are wired and change the defence factor after a swing. Chase is treated as Stand ("until CHASE-1"). Secure mode is not found. Target-selection effect not verified. | PARTIAL: LOOP-COMBAT-MODES-1 |
| 37 Level-up stats | The level is computed from experience only; no per-vocation max HP, mana or capacity change on level exists at runtime (vocation vitals are a tooling candidate). | ABSENT at runtime: LOOP-PROGRESS-1. Bundle node not checked. |

## 3. Not Game-owned (owner direction 2026-10-13: weather and server save/restart are IN scope, see batches WEATHER-1, OPS-SAVE-1; Store entitlements are in scope as a mechanic, but who delivers them across the Game/Platform boundary is OPEN under gap register §32, see STORE-ENT-1)

| Row | Mechanic | Why |
|---|---|---|
| 131 | Create/delete/rename character | Platform owns web identity and the account flow (ADR-0012); Game keeps only the character command contract (GAME-CHAR-CMD-0). Re-evaluate if a Game-side gap appears. |
| 3, 4, 14 | STEP-PACE-WIRE-1 (#2041), auto-walk, minimap | client-owned (#1942). Server dependency: none new for 3 and 4 (the server receives paced steps); 14 needs LOOP-MAP-1 |
| 16 | Light (utevo lux, torches) | client rendering is client-owned; the server light state comes from LOOP-CLOCK-1 |

## 4. Batches

Class: `hard` = persistence, fencing, wire format, authority or durable value (hard worker and independent
review); `impl` = ordinary worker. `Mig` = needs a migration (numbers after 0083 leased by the control plane);
`Reg` = changes the protocol registry (one registry PR at a time).

Source of truth: this plan is not the source of truth for dependencies, classes or registry/migration flags.
The brief (decision, packets) takes precedence over any row here, and a conflict is a defect of this
plan. A collective row is not complete until every child its brief defines is listed as its own batch, here or
in the child-graph appendix (section 7), with the brief's class and prerequisites. Workers allocate from the
brief's child graph; the row's `Depends on` is only the entry gate of the family.

Family-entry semantics: the part of a row's `Depends on` before `external prerequisites of the brief children:` is the
entry gate of the family, and every child of the family inherits it. The external list names prerequisites of single
children, which the appendix lists on those children; they do not gate the other children. A row may name its own
children (they complete before the row does) but never a child of a family whose entry gate needs this row: such a
shared child is delivered first and named by the consumer child, not by the row (LOOP-WORLD-1, LOOP-MONSTER-1,
MAGIC-SUMMON-1 and ITEM-STORAGE-1 were narrowed this way). An integration child that needs a downstream family's child is released by that child and is outside its own family's completion set (stated under the graph); a family gate names the narrowest child or the entry gate, not an aggregate whose completion set contains a consumer. Packets that the accepted brief releases together are one
allocation named with `+` (PREM-2b+PREM-5). Dependency check: a script over this section and section 7 finds
cycles (strongly connected components of entry-gate and child edges) and dangling names; it reports none.

### Epic 1. Game loop

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| LOOP-WORLD-1 | 6, 8, 9, 10 | confirm teleports, keyed/level/quest doors, levers and tile damage on the bundle node | #2018, ENCOUNTER-RT-0, BUNDLE-COMBAT-ENABLE-1 (the `qualified_room` gate), CHAT-1; CHAT-1b-2b (ENC-WORLD-1 child only); external prerequisites of the brief children: MAP-WIRE-2, ITEM-USE-WIRE-1, ITEM-EQUIP-WIRE-1, MAP-OVERLAY-1, WO-3, QUEST-GATE-1, QUEST-TRIGGER-1, MAP-LOAD-1, VSL-MOVE-01, ATTACK-1b, ITEM-MOVE-2b, BAGS-GROUND-1, FIELD-WIRE-1, FIELD-1, PVP-RT-1, COND-1, VIS-2, WO-2, QUEST-CONTENT-2, CREATURE-AI-1 (merged), ITEM-USE-1c, BOSS-REWARD-1, QUEST-STATE-1, SCOPE-HANDOFF-1, BOSS-ROOM-1; ENC-COMBAT-1 names MONSTER-SUMMON-1 itself, a shared child of the LOOP-MONSTER-1 graph that is delivered first and is not an entry gate here | hard (child graph, App. 7) | maybe | maybe |
| LOOP-MAP-1 | 13 | close the cap 18 offer decision: MAP-VIEWPORT-MEASURE-1 is merged (#1873; 0.41-0.44 core per 500 players against the 20% default, so it does not fit), what remains is MAP-VIEWPORT-OPT-1 and MAP-VIEWPORT-LIMIT-1 (Appendix 7) | #1942 client proof; MAP-VIEWPORT-OPT-1; MAP-VIEWPORT-LIMIT-1; external prerequisites of the brief children: MAP-LOAD-1, MAP-OVERLAY-1, VIS-2, CHAR-REV-SEQ-1, BANK-FEE-0, SPAWN-1a (merged), SPAWN-1b | hard (child graph, App. 7) | maybe | maybe |
| LOOP-COMBAT-MODES-1 | 20 | chase mode (CHASE-1), secure mode, fight-mode effect on targeting | none; external prerequisites of the brief children: VIS-2, MOVE-RL-11, GAME-AI-01, ITEM-SEM-2b, BAGS-1, ITEM-MOVE-2a, BAGS-WIRE-1, ITEM-MOVE-2b, DUR-03, SPEED-1 (merged) (CHASE-1 supplies the shared path search CREATURE-MOVE-1 reuses, so CREATURE-MOVE-1 is not a prerequisite) | hard (child graph, App. 7) | maybe | maybe |
| LOOP-DAMAGE-1 | 23, 24, 25, 26 | enable cap 12 (EQUIP-0), resistances, crits/leech, area and chain attacks on the bundle, mana shield | EQUIP-0, BUNDLE-COMBAT-ENABLE-1; external prerequisites of the brief children: ITEM-SEM-2b, ITEM-MOVE-2a, CHAR-BUILD-1 (merged), SKILLS-0, ATTACK-1b, GAME-AI-01, VIS-2, FOOD-REGEN-1 | hard (wire) | maybe | yes |
| LOOP-RANGED-1 | 21, 22, 51 | projectile profiles, distance weapons and ammo (a DUR-03 burn), wands/rods, attack spells | BUNDLE-TILES-PROJECTILE-1, LOOP-DAMAGE-1, ITEM-LOOP-1; external prerequisites of the brief children: VIS-2, MOVE-RL-11, GAME-AI-01, ITEM-SEM-2b, BAGS-1, ITEM-MOVE-2a, BAGS-WIRE-1, ITEM-MOVE-2b, SPEED-1 (merged), CREATURE-MOVE-1 | hard (ammo burn) | yes | maybe |
| CHAR-BUILD-1 | 38, 40, 41 | **MERGED (#1393 (1a, migration 0030) and the 1b writer, archive records OTV2-20260930-char-build-1 and -1b; the residues are W2b, DAWNPORT-1, DEATH-ML-LOSS-1 and LOOP-PROGRESS-1).** character build state: table, receipt kind, nullable death-receipt build columns, admission load, guards, verifier (A13 §5); takes the skill columns of SKILLS-0 in the same migration | A13 accepted, migrations 0018-0020 | hard (persistence review) | yes | no |
| W2b | 38, 40, 41 | `CasterState` from build state, `Vocation::None`, training accumulation, Reference formula; trains skills with magic level | CHAR-BUILD-1 (merged) | hard | maybe | no |
| DAWNPORT-1 | 38 | Dawnport content and the vocation-choice interaction (vocation choice keeps cumulative progress, SKILLS-0 §3.3) | CHAR-BUILD-1 (merged) | impl (content) | maybe | no |
| DEATH-ML-LOSS-1 | 40, 41 | flush before a death and the death receipt's build fields; production training stays disabled until it merges | CHAR-BUILD-1 (merged), DEATH-1 | hard (persistence) | yes | no |
| LOOP-PROGRESS-1 | 37, 38, 40, 41 | level-up vitals/capacity per vocation, vocations end to end, skill gain on hits, magic level; entry gate of the four children above | GAME-CHAR-CMD-0, CHAR-BUILD-1 (merged), W2b, DAWNPORT-1 (vocation choice); production training enabled only after DEATH-ML-LOSS-1 | hard (Character writes) | yes | no |
| LOOP-VITALS-1 | 30, 32, 33 | durable regeneration accumulator, food-time ingress, soul regen and soul orb, stamina | ITEM-USE-1c (in flight; #2039 merged only the 1a and 1b slices), LOOP-PROGRESS-1; external prerequisites of the brief children: CHAR-BUILD-1 (merged), SKILLS-0, CHAR-POSITION-1, PREY-0, WORLDINT-USE-1, WO-2, TIMED-CONTENT-1, TIMED-ITEM-0B, ITEM-SEM-2b, HOUSE-RUNTIME-1a, HOUSE-VIEW-1, WORLDINT-WIRE-1, DUR-02 | hard (Character state) | yes | no |
| LOOP-TRAIN-1 | 34, 35 | offline training (statue) and exercise weapons; beds are BED-1 | PREMIUM-1 (the statue is Premium), LOOP-VITALS-1, LOOP-PROGRESS-1 (skill build state, A13 magic-level training), CHAR-POSITION-1, a slot in the shared guard-rewrite order, WORLDINT-USE-1, OFFLINE-1, STATUE-1, EXERCISE-1 children (Appendix 7), ITEM-TIMED-1 (`charges`), the training statue and dummy content (WO-2, OFFLINE-CONTENT-1, EXERCISE-CONTENT-1; OFFLINE-0 §30-34, EXERCISE-0 §33-35); external prerequisites of the brief children: CHAR-BUILD-1 (merged), SKILLS-0, PREY-0, TIMED-CONTENT-1, TIMED-ITEM-0B, ITEM-SEM-2b, HOUSE-RUNTIME-1a, HOUSE-VIEW-1, WORLDINT-WIRE-1, DUR-02 | hard | yes | no |
| LOOP-MONSTER-1 | 94, 96 | world-owned respawn scheduler independent of sessions; monster ranged, spell and summon AI | LOOP-RANGED-1, MAGIC-CAST-1 (MAGIC-SUMMON-1 is not an entry gate: the two rows share one child graph and its shared summon children are delivered first); external prerequisites of the brief children: MAP-OVERLAY-1, MAP-CUTOVER-1b, VIS-1 (merged), ATTACK-1b, SPEED-1 (merged), VIS-2 | hard (child graph, App. 7) | maybe | maybe |
| LOOP-GROUND-1 | 18 | ground item limit shared with the death drop: 64 items per tile, "not enough room" when the tile is full; no periodic ground cleanup between resets (owner 2026-10-15; revisit only under real server load); ADR-0021 D191 keeps the no-timed-decay rule | DEATH-DUR-1 | hard (retire) | maybe | no |
| LOOP-CLOCK-1 | 15 | **Slice MERGED (#2071: capability 19, domain 18, the pure World clock). Row 15 is not complete without the residual CLOCK-1 batch (Appendix 7): the VIS-2 `light` field, which WORLD-INTERACTION-0 §11 assigns to CLOCK-1 as an amendment of VIS-2 after VIS-2 and COND-1.** World clock and day phase, server light state | CLOCK-1 (residue; Wave 1, behind VIS-2 and COND-1) | impl | no | yes |
| WEATHER-1 | 17 | weather state and change events; visual only, no gameplay effect (owner 2026-10-15) | LOOP-CLOCK-1 (slice merged); CLOCK-1 (residue); ITEM-LOOK-1 (registry order) | impl | no | yes |
| LOGOUT-WIRE-1 | 109 | the logout command: `LOGOUT_INTENT`, capability `LOGOUT_V1`, `LOGOUT-RL-01`, `TerminalRelease::Logout` (ARCH-KILL-REWARD-LOGOUT-1 §2.2) | ATTACK-1b (#1798, merged); LOGOUT_V1 is not on main; not KILL-REWARD-COMP-1 | hard (fencing; protocol review) | no (`migration_lease: none`) | yes (protocol and resource-limit registries) |
| KILL-REWARD-COMP-1 | 107 | **MERGED on main@ab7d51ae6 (`gameplay_transport/kill_reward.rs`, `content/creature_reward.rs`, `combat/death_reward.rs`, e2e #2075); a worker confirms only the §2.1 acceptance tests at allocation.** Live loot and XP settlement for a creature kill (ARCH-KILL-REWARD-LOGOUT-1 §2.1) | ATTACK-1b (#1798, merged), D3-7 (merged) | hard | no (`migration_lease: none`) | yes (`KILLRW-RL-01`) |

### Epic 2. Economy and items

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| ITEM-LOOP-1 | 62, 63, 64, 65, 66, 68 (pickup rows 62 and 63 from a base-map entry are gated on MAP-PICKUP-1) | equip/unequip, drop/pickup, nested containers, stacks, cap and weight enforcement, starter pack on the bundle | EQUIP-0, #2039; external prerequisites of the brief children: CAP-NEG-1, CAP-NEG-RESUME-FALLBACK-1, VIS-2, MOVE-RL-11, ITEM-SEM-2b, MAP-OVERLAY-1, DEPOT-1, INBOX-1a (merged), TRADE-1, GROUND-MOVE-1, RANGED-1, GAME-ABILITY-01 | hard (durable item) | yes | maybe |
| ITEM-TIMED-1 | 69 | charges, duration and repair; children TIMED-RT-1a, TIMED-RT-1b, TIMED-RT-1c (hard), TIMED-HOUSE-1, TIMED-FX-1 (hard, combat), TIMED-WIRE-1 (Reg), TIMED-PARITY-1 (TIMED_ITEM0B; Appendix 7) | ITEM-LOOP-1, TIMED-CONTENT-1, ITEM-MOVE-2a/2b, EQUIP-RT-1, COND-1 (per child); external prerequisites of the brief children: ITEM-SEM-2b, NPC-TRADE-1, ITEM-MOVE-2b, ITEM-USE-1c, HOUSE-CUSTODY-0, ITEM-SEM-USE, USE-WIRE-V1, ITEM-MOVE-WIRE-1, GOLD-FEE-2, ATTACK-1b, SPEED-1 (merged), GAME-AI-01, SPELL-D3, MARKET-1 | hard | yes | yes |
| ITEM-IMBUE-1 | 72 | imbuing; children IMBUE-1 (hard), IMBUE-RT-1 (hard, combat), IMBUE-SCROLL-1 (hard), IMBUE-WIRE-1 (Reg), IMBUE-CONTENT-1 (IMBUE_FORGE0; Appendix 7) | ITEM-LOOP-1, GOLD-FEE-2, ITEM-MOVE-2a, ATTACK-1b, SPEED-1 (merged), NPC-ECON-1 (per child), PREMIUM-1 | hard | yes | yes |
| ITEM-FORGE-1 | 73 | exaltation forge; children FORGE-1a, FORGE-1b (hard), FORGE-WIRE-1 (Reg), FORGE-CREATURE-1 (hard, combat), TIER-EFFECT-1 (hard, combat), MARKET-TIER-1 (hard, after MARKET-1), FORGE-CONTENT-1 (IMBUE_FORGE0; Appendix 7) | ITEM-LOOP-1, GOLD-FEE-2, ITEM-MOVE-2a, GAME-AI-01 runtime, ATTACK-1b (per child), PREMIUM-1 | hard | yes | yes |
| ITEM-STORAGE-1 | 70, 71, 74 | depot, supply stash, books and papers | ITEM-LOOP-1; external prerequisites of the brief children: ITEM-VIEW-1b, MAP-LOAD-1, MAP-WIRE-1, HOUSE-CUSTODY-1, ITEM-MOVE-2a, WO-1 (merged), BANK-RET-0 (merged, #1748), MARKET-CONTENT-1, ITEM-SEM-2b, MAIL-TEXT-1, ITEM-MOVE-2b, BAGS-1, HOUSE-RUNTIME-1a, LEVER-1; WRITE-WIRE-1 names MAIL-WIRE-1 itself and is delivered after MAIL-1 | hard | yes | yes |
| ITEM-LOOK-1 | 75 | look/examine message | none | impl | no | yes |
| BANK-RET-0 | 39 | **MERGED (#1748).** bank retention profile (BANK-0 §2.3); releases BANK-1 | none (control plane routes; privacy review) | decision | no | maybe |
| ECON-RET-0 | 122, 113 | **MERGED (#1748).** guild and market event retention profiles (GUILD-RET-0, MARKET-RET-0); releases GUILD-1 and MARKET-1 | none (control plane routes; privacy review) | decision | no | maybe |
| NPC-ECON-1 | 39 | bank (mechanics row 81, the promotion purchase, is not here: it belongs to PREM-2b+PREM-5, the PREM-5 NPC promotion paid in coins via GOLD-FEE-1b) | #2040 (merged), BANK-FEE-0, BANK-RET-0 (merged, #1748) (BANK-1 waits on it); external prerequisites of the brief children: MAP-BUNDLE-1, MAP-CUTOVER-1b, GOLD-FEE-1b, DEATH-1 | hard (value) | yes | maybe |
| TRADE-PLAYER-1 | 82 | player-to-player trade | ITEM-LOOP-1; external prerequisites of the brief children: ITEM-VIEW-1b, VIS-2, ITEM-MOVE-2a, WO-1 (merged), MAP-LOAD-1, ITEM-MOVE-1, MAP-WIRE-2, BAGS-1, BAGS-DEPOT-1, HOUSE-CUSTODY-1, DEPOT-1, BANK-1, GOLD-FEE-2, DEPOT-WIRE-1 | hard | yes | yes |
| MAIL-1 | 88 | mail and parcels | ITEM-LOOP-1 (ITEM-STORAGE-1 as a whole is not an entry gate: the parcel children name DEPOT-1 through BAGS-DEPOT-1 themselves, and MAIL-TEXT-1 and MAIL-CONTENT-1 run before WRITE-1); external prerequisites of the brief children: ITEM-VIEW-1b, VIS-2, ITEM-MOVE-2a, WO-1 (merged), MAP-LOAD-1, ITEM-MOVE-1, MAP-WIRE-2, BAGS-1, BAGS-DEPOT-1, HOUSE-CUSTODY-1, DEPOT-1, BANK-1, GOLD-FEE-2, DEPOT-WIRE-1 | hard | yes | yes |
| MARKET-1 | 122 | market | ITEM-LOOP-1, NPC-ECON-1 (ITEM-STORAGE-1 as a whole is not an entry gate: MARKET-CONTENT-1 runs before STASH-CONTENT-1), ECON-RET-0 (merged, #1748; MARKET-RET-0), PREMIUM-1; external prerequisites of the brief children: ITEM-VIEW-1b, VIS-2, ITEM-MOVE-2a, WO-1 (merged), MAP-LOAD-1, ITEM-MOVE-1, MAP-WIRE-2, BAGS-1, BAGS-DEPOT-1, HOUSE-CUSTODY-1, DEPOT-1, GOLD-FEE-2, DEPOT-WIRE-1 | hard | yes | yes |
| CRAFT-1 | 89, 91 | crafting, fishing, digging, cooking, NPC exchanges; brief `OTERYN_GAME_CRAFT1_DECISION_2026-10-15.md` (CANDIDATE, revision 2 per the owner direction of 2026-10-15: in full, nothing deferred; the used item alone, use-with on a map item or object, carried item onto carried item (UseWithV1 arm 6, CRAFT1 §4.1), NPC exchanges (C8, §4.2), reference fishing values (C4a, C4b) and the TIMED-ITEM-0B and dish-effect hook (C7); children in Appendix 7, CRAFT-1 graph) | #2039 (merged), ITEM-LOOP-1 | hard (item conversions; child graph, App. 7) | yes | yes (CRAFT1 C3a and C8 DUR-03 rows at acceptance; CRAFT1-RL-01..03 by CRAFT-NPC-WIRE-1) |
| HIRE-1 | 90 | hirelings (children in Appendix 7, HIRE-1 graph): a Store-delivered, house-bound NPC product; goes ahead, not deferred (owner 2026-10-15); brief `OTERYN_GAME_HIRE1_HIRELING_DECISION_2026-10-15.md` (CANDIDATE); delivery waits for the Platform Store contract (Platform #1625) | HOUSE-1b, NPC-ECON-1, PREMIUM-1 | hard (persistence, economy; child graph, App. 7) | yes | no |
| QUEST-RT-1 | 86, 87 | quest runtime on the bundle, chest rewards, cap 16 offer | QUEST-LOWER-1, ITEM-LOOP-1, MAP-PICKUP-1 (chest and ground pickup), QUEST-XP-ADMISSION-1 (pays quest XP); external prerequisites of the brief children: MAP-LOAD-1, MAP-WIRE-2, NPC-TALK-1, GOLD-FEE-2, MAP-BUNDLE-1, NPC-CONTENT-1 | hard (rewards are mints) | maybe | yes |

### Epic 3. Magic and conditions

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| MAGIC-CAST-1 | 49, 50, 52, 53, 54 | close SPELL-STARTER-1 RUNTIME_GAP and ADAPTER_MISSING: spellbook, casting, healing, support, parametric spells, cooldown groups | PREMIUM-1 (Premium spell check, PREMIUM-ACTIVATION-0 §4.6); external prerequisites of the brief children: VIS-2, SPELL-D4, ATTACK-1b, COND-1, STANCE-1, GAME-AI-01, PARTY-1, SKILLS-0, ITEM-SEM-2b; complete only when every child in App. 7 is complete | hard (child graph, App. 7) | maybe | maybe |
| MAGIC-COND-1 | 27, 28, 29 | DoT, haste/paralyze, muted/exhaust/infight/pacified/powerless/agony (ACCEPT-CONDITIONS-0 scope); invisibility/outfit and rooted/feared policy are future scope, each gated on its own accepted decision | BUNDLE-COMBAT-ENABLE-1; external prerequisites of the brief children: ITEM-SEM-2b, ITEM-MOVE-2a, CHAR-BUILD-1 (merged), SKILLS-0, ATTACK-1b, GAME-AI-01, VIS-2, FOOD-REGEN-1 | hard (child graph, App. 7) | maybe | maybe |
| MAGIC-ITEMS-1 | 55, 56 | rune use with target and conjure | ITEM-USE-1c (#2039 merged only 1a and 1b); external prerequisites of the brief children: ITEM-USE-WIRE-1, ITEM-SEM-USE, ITEM-USE-1c, SPELL-TARGET-1, ATTACK-0, VIS-2, CHAR-BUILD-1 (merged), SPELL-D4, COND-1 | hard (DUR-03) | maybe | maybe |
| MAGIC-SUMMON-1 | 57, 58 | summons, familiars, magic fields; complete only when every child in App. 7 is complete | MAGIC-CAST-1 (LOOP-WORLD-1 field ingress gates only the field children, which name it themselves, not the shared summon children); external prerequisites of the brief children: MAP-OVERLAY-1, MAP-CUTOVER-1b, VIS-1 (merged), ATTACK-1b, SPEED-1 (merged), VIS-2 | hard (child graph, App. 7) | maybe | maybe |
| PROG-ADV-1 | 42, 43, 44 | weapon proficiency (PROF-WIRE-0), wheel of destiny, gem atelier | LOOP-PROGRESS-1, PREMIUM-1; external prerequisites of the brief children: PROF-CONTENT-1, PROF-1, FORGE-1a, PROF-WIRE-1, ATTACK-1b, RANGED-1, SKILLS-0, EQUIP-RT-1, CHAR-REV-SEQ-1, GAME-ABILITY-01, CONDITIONS-0, PREM-1, GOLD-FEE-2, WHEEL-GEM-0A, ITEM-USE-0 | hard | yes | yes |

### Epic 4. World and NPC

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| NPC-WORLD-1 | 83, 84 | NPC travel, walking, voices, focus | #2040 (merged), CHAR-POSITION-1, PREMIUM-1, CHAT-1; CHAT-1b-2b (NPC-VOICE-1); external prerequisites of the brief children: NPC-CONTENT-1, QUEST-CONTENT-1, NPC-TALK-1, GOLD-FEE-1b, DEATH-1, QUEST-PRED-1, PREM-1, PREM-3, NPC-PLACE-1b, CREATURE-MOVE-1, SPEED-1 (merged), NPC-WIRE-1, VIS-2 | hard (travel moves a Character) | yes | maybe |
| MONSTER-META-1 | 98, 99, 100, 101 | bestiary (CHARM-6), charms, bosstiary, bosses | LOOP-MONSTER-1, CHAT-1; CHAT-1b-2b (RAID-1), PARTY-XP-1 (BOSS-REWARD-1); external prerequisites of the brief children: SCOPE-HANDOFF-1, D3-3, CHAR-REV-SEQ-1, GOLD-FEE-1b, GOLD-FEE-2 | hard (child graph, App. 7) | maybe | maybe |
| EVENTS-RAID-1 | 102 | raids | LOOP-MONSTER-1; external prerequisites of the brief children: SCOPE-HANDOFF-1, D3-3, CHAR-REV-SEQ-1, GOLD-FEE-1b, GOLD-FEE-2, CHAT-1; CHAT-1b-2b, PARTY-XP-1 | hard (child graph, App. 7) | maybe | maybe |
| EVENTS-WORLD-1 | 103, 104 | world changes, Fury Gates, Nightmare Isles, hazard | EVENTS-RAID-1; external prerequisites of the brief children: SCOPE-HANDOFF-1, D3-3, CHAR-REV-SEQ-1, GOLD-FEE-1b, GOLD-FEE-2, CHAT-1; CHAT-1b-2b, PARTY-XP-1 | hard (child graph, App. 7) | maybe | maybe |
| ARENA-1 | 106 | Tibiadrome and arenas | LOOP-MONSTER-1; external prerequisites of the brief children: SCOPE-HANDOFF-1, D3-3, CHAR-REV-SEQ-1, GOLD-FEE-1b, GOLD-FEE-2, CHAT-1; CHAT-1b-2b, PARTY-XP-1 | hard (child graph, App. 7) | maybe | maybe |
| TUTORIAL-1 | 139 | late test epic, not blocking: Newhaven/Rookgaard content after Thais start is tested; its brief is written after Wave 4 (owner/architect ruling 2026-10-15) | Wave 4 done | impl | no | no |

### Epic 5. Community

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| CHAT-2 | 116 (relay) | the World chat relay that PARTY-1 needs (CHAT-0 §26-32) | CHAT-1; CHAT-1b-2b (NOT merged: no task record and no resource row yet; Wave 1); acceptance gate: CHAT-0 is CANDIDATE (reopened by the 2026-10-12 security review: rate gate before the Channel lock, now #2026; cast-path mute, now #2003); CHAT-2 is not allocated until CHAT-0 is re-reviewed and ACCEPTED | hard (World relay) | maybe | maybe |
| PARTY-1 | 112 | party membership and state: invite/join/leave, succession, shared-XP toggle, `PartyView` cache (membership only) | CHAT-2 (which follows CHAT-1; CHAT-1b-2b) (PARTY-PVP-0 §30-33) | hard (shared state) | yes | yes |
| PARTY-XP-1 | 59 | shared-XP eligibility and split, party immunity in area effects, party loot right, `party_buff` query and party spells | PARTY-1, the D118 XP lane (LOOP-PROGRESS-1), D3-4 (LOOP-DAMAGE-1) (PARTY-PVP-0 §30-32); external prerequisites of the brief children: DEATH-1, ATTACK-1b, COND-1, DEATH-3, SPEED-1 (merged), VIS-2, ATTACK-WIRE-1, CHAT-2 | hard (combat, rewards) | maybe | maybe |
| GUILD-1 | 113 | guild, rank, member, invitation tables and transactions (GUILD0 §3, §4) | PARTY-1, ECON-RET-0 (merged, #1748; GUILD-RET-0), PREMIUM-1 (PREM-WIRE-1), NPC-ECON-1 (BANK-1) (PREMIUM-ACTIVATION-0 l.367-368; SOCIAL_MAP_PACKETS l.122-126) | hard (persistence, security) | yes | yes |
| GUILD-BANK-1 | 113 | guild balance and ledger, disband payout and custody claims | GUILD-1, NPC-ECON-1 (BANK-1) | hard (persistence, economy) | yes | maybe |
| GUILDHALL-1 | 113, 119 | guildhall bid with split escrow, guild rent, guild ban, `GUILD_DISBAND`, guild ACL | GUILD-1, GUILD-BANK-1, HOUSE-1b, HOUSE-ACL-1 | hard (persistence, economy, security) | yes | no |
| GUILD-WIRE-1 | 113 | capability `GUILD_V1`, commands, emblem, guildhall variants of `HOUSE_INTENT` | GUILD-1, GUILD-BANK-1, GUILDHALL-1 (guildhall variants), HOUSE-WIRE-1 | impl (protocol review) | no | yes |
| GUILD-CHAT-1 | 113, 116 | guild room on the World relay | GUILD-1, CHAT-2 | impl (security review) | maybe | maybe |
| VIP-1 | 114 | VIP store, receipts, consent lifecycle, caps, wire | VIP-0, PARTY-1, PREMIUM-1 | impl (persistence, protocol, privacy review) | yes | yes |
| VIP-2 | 114 | VIP presence on the World relay, gated by consent and `vip_status` | VIP-1, CHAT-2 | hard (security, privacy) | maybe | maybe |
| CHAT-1 | 116 | **CHAT-WIRE-1 MERGED on main@ab7d51ae6 (`gameplay_transport/chat_intent.rs` dispatches local speech, `CHAT_V1` offered; `chat/` has the NPC greeting, spam limiter and mute). The accepted CORE_LOOP amendment defines CHAT-1 as CHAT-1a, CHAT-1b-1, CHAT-1b-2a and CHAT-1b-2b; CHAT-1b-2b (resource row `CHAT0-RL-11`, absent from the registry, no task record) is the residue, so every dependent waits for CHAT-1 and CHAT-1b-2b. Wave 1.** Was: capability `CHAT_V1`, chat command and domain, local speech, NPC greeting, spam limiter, in-memory mute | none (CHAT-0 root; the yell gate calls PREM-WIRE-1 when it lands later) | impl (protocol review) | maybe | yes |
| HOUSE-RUNTIME-1a | 120 | house interior runtime, first child (HOUSE-RUNTIME-1a) | SCOPE-HANDOFF-1 (HOUSE-CUSTODY-1 merged) | hard (security, persistence review) | maybe | no |
| HOUSE-RUNTIME-1b | 120 | house interior runtime, map-load child (HOUSE-RUNTIME-1b) | HOUSE-RUNTIME-1a, MAP-LOAD-1 | hard (security, persistence review) | maybe | no |
| HOUSE-RUNTIME-1c | 120 | house interior runtime, position child (HOUSE-RUNTIME-1c) | HOUSE-RUNTIME-1b, CHAR-POSITION-1 | hard (security, persistence review) | maybe | no |
| HOUSE-1a | 119 | house ownership, auction, rent (HOUSE-OWN-0 §3-§9; SOCIAL_MAP_PACKETS HOUSE-1a) | BANK-1 (NPC-ECON-1), HOUSE-RUNTIME-1a, 1b and 1c, PREM-WIRE-1 (PREMIUM-1) | hard (persistence, economy, security) | yes | no |
| HOUSE-1b | 119 | house ownership, second half (SOCIAL_MAP_PACKETS HOUSE-1b) | HOUSE-1a, INBOX-1a (merged) (MAIL-1), MAP-OVERLAY-1c | hard (persistence, economy, security) | yes | no |
| MAP-OVERLAY-1a | 13 | **MERGED (#1800).** map overlay, first child | MAP-LOAD-1 | hard | maybe | maybe |
| MAP-OVERLAY-1b | 13 | **MERGED (#1809).** map overlay, second child | MAP-OVERLAY-1a | hard | maybe | maybe |
| MAP-OVERLAY-1c | 13 | map overlay, house child | MAP-OVERLAY-1b, HOUSE-RUNTIME-1a | hard | maybe | maybe |
| MAP-CUTOVER-1a | 13 | **MERGED (#1825).** map cutover, first child (MAP_TRACK_PACKETS) | none | hard | maybe | maybe |
| MAP-CUTOVER-1b | 13 | **MERGED (#1916).** map cutover, second child: the bundle World serves domain 17 under capability 18 (MAP_TRACK_PACKETS §2.4); it does not wait for the overlay | MAP-CUTOVER-1a, MAP-VIEWPORT-PERF-1, MAP-ITEM-REF-1, login N8-1 | hard | maybe | maybe |
| MAP-CUTOVER-1c | 13 | map cutover, third child (the old SOCIAL-MAP §2.8, renamed, MAP_TRACK_PACKETS §2.5): Ground persistence, the reset-driven first cutover and the position fallback on the bundle World of 1a and 1b | MAP-CUTOVER-1b, MAP-OVERLAY-1c | hard | maybe | maybe |
| HOUSE-ACL-1 | 121 | house ACL tables and revisioned edits (HOUSE-OWN-0 §10) | HOUSE-1a (SOCIAL_MAP_PACKETS §0.1) | hard (security review) | yes | maybe |
| HOUSE-WIRE-1 | 119, 121 | capability `HOUSE_V1`, house list, bids, move-out, House Management panel | HOUSE-1b, HOUSE-ACL-1 | impl (protocol review) | no | yes |
| BED-1 | 121 | house beds: sleeper table, bed USE and sleep, owner wake, settlement (BED0 §4-§8) | OFFLINE-1, STATUE-1, HOUSE-RUNTIME-1a, 1b and 1c, HOUSE-VIEW-1, WORLDINT-WIRE-1 | hard (persistence, protocol review) | yes | maybe |
| BED-REGEN-1 | 121 | bed vitals and food settlement | BED-1, DUR-02 durable vitals | impl (persistence review) | no | no |
| SOCIAL-LATE-1 | 118 | no marriage (as in Tibia, owner 2026-10-15); the PvP arena moves to ARENA-1, so this batch is retired into ARENA-1 | GUILD-1, ARENA-1 | impl | maybe | no |

### Epic 6. Meta

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| STORE-ENT-1 | 123 | Store delivery: inbox items and cosmetic unlocks (D47, D49). **Delivery owner unassigned (Game or Platform): blocked on the §32 delivery decision** (`MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md` row 'Store delivery'; ACCOUNT_PROGRESS_AND_QUEST_707 §4.5). Fixed already: scope and portability only; entitlement lifecycle (refund, revocation, expiry) is Platform's; a Game record is evidence about Platform authority, never a second commercial authority; an item claim is an idempotent DUR-03 MINT. Purchase and payment stay with Platform. | §32 delivery decision (owner of purchase delivery, entitlement identity and lifecycle, idempotent delivery across a boundary failure); then PREMIUM-1 and the Platform contract | blocked (decision) | unknown until the decision | unknown until the decision |
| ATTACK-2 | 21, 23, 26 | residue of ATTACK-1b (#1798 excluded scope): weapon melee, creature defence, armor and block (only fists melee is composed), VIS negative floors -8..-5 (a VIS-1 acceptance item) | EQUIP-0, ATTACK-1b (merged), VIS-1 (merged) | hard (combat) | no | maybe |
| BUNDLE-GATES-1 | 10, 49, 57, 94 | residue of BUNDLE-COMBAT-ENABLE-1 (#2025 excluded scope). **Inventory audited on main@75ec7985 and test-enforced (`gameplay_transport/bundle_gates_tests.rs`): the seam's two rooms are private to `gameplay_transport/seam_rooms.rs` and read only through its gate-named accessors; an accessor returning the combat room is open on a bundle World, one returning the qualified room is closed there.** Open: player attack target, auto-attack drain, native combat cast and its timer continuation (BUNDLE-COMBAT-ENABLE-1 #2025); monster AI (LOOP-MONSTER-1 child BUNDLE-MONSTER-AI-1, #2036); periodic player damage (MAGIC-COND-1 child COND-1c, #2086); the `locate_message` parameter cast (MAGIC-CAST-1, #2112, the parameter-cast bundle gate); the chase step (LOOP-COMBAT-MODES-1, #2113). Closed, with owner: `house_access` and `vertical_move` parameter casts (MAGIC-SUMMON-1, child SPELL-RT-WORLD-1); world item casts, `random_item_grant` (MAGIC-SUMMON-1, SPELL-RT-WORLD-1) and `conjure` (MAGIC-ITEMS-1, RUNE-CONJ-1); field step ingress (LOOP-WORLD-1, child WORLDINT-ADMIT-1); source map initialization, the durable custody of map Items, which on a bundle World is its Ground persistence (MAP-CUTOVER-1c). The four source-World branches (door overlay, step blocking, USE of `mod.rs`, and the chase step blocking of `attack.rs`) are not gates: a bundle World is served by `bundle_map` there. The guard fails when an accessor is added, removed or switched, or a call of one added, removed or moved, without updating the inventory, when a closed gate has no plan-batch owner, and when this row stops naming an opener or owner; this row stays open until no gate is closed | BUNDLE-COMBAT-ENABLE-1 (merged) | impl | no | no |
| STEP-PACE-ENABLE-1 | 3, 4 | residue of STEP-PACE-WIRE-1 (#2041): turn on `OTERYN_STEP_WAIT_RECEIPTS=1` only after the client with the capability 13 field 2 decoder ships (STEP-PACE-CLIENT-1, client-owned #1942) and the 9c and 6c3 builds are retired | STEP-PACE-WIRE-1 (merged); client release | ops (rollout switch) | no | no |
| OPS-SAVE-1 | 137 | **First slice MERGED (#2073: live monk state flushed at shutdown through the fenced writer); the residue is OPS-SAVE-2.** Server save and graceful restart |  none | hard (durability) | maybe | no |
| OPS-SAVE-2 | 137 | residue of OPS-SAVE-1 (#2073 excluded scope): quiesce admissions and input before the snapshot (shutdown ordering in `serve.rs` and `mod.rs`), flush all other session-generation fenced writes (familiar and spell-training saves, other lanes' writers), drain, resume; a worker confirms the residue against main | OPS-SAVE-1 (#2073, merged) | hard (durability) | maybe | no |
| PREMIUM-1 | 124 | Premium gating across features: the PREM-WIRE-1 gate (trusted clock, read seam, `PremiumStatus` in the legality checks), fail closed | none beyond merged PREM-1c-harden; acceptance gate: PREMIUM_ACTIVATION is CANDIDATE, so PREMIUM-1 is not allocated until it is ACCEPTED, and every Premium-gated batch below waits for that too; once accepted it is routed first and releases them | hard (authority, fail closed) | no | maybe |
| ACH-1 | 125, 129 | achievements, badges, titles | LOOP-PROGRESS-1; external prerequisites of the brief children: QUEST-STATE-1, ENC-OUTCOME-1, SKILLS-0, FND-02 | hard (child graph, App. 7) | yes | maybe |
| APP-1 | 126, 127 | durable selected appearance and the AccountUnlock set, validation, fence (CHARACTER_APPEARANCE §5) | Character authority, progression readiness (migration numbering) | hard (persistence) | yes | no |
| APP-2 | 126, 127 | outfit and mount definitions with look types, starter flags, palette | content pipeline, #1025 loader follow-up | impl (content) | no | no |
| APP-3 | 126, 127 | change command and actor appearance state | APP-1, combat child E protocol lane (DEATH-2/ATTACK), registry lease | hard (protocol) | no | yes |
| APP-4 | 126, 127 | client outfit window and rendering of other actors | APP-3, client owner (#1942) | impl (client) | no | no |
| HUNT-1 | 45, 46 | prey and hunting tasks | LOOP-MONSTER-1, PREMIUM-1; external prerequisites of the brief children: CHAR-REV-SEQ-1, GOLD-FEE-1b, SPELL-TARGET-1, ATTACK-0, ITEM-USE-1c, ATTACK-1b, CHAR-BUILD-1b (merged), CHARM-1 | hard (rewards) | yes | maybe |
| DAILY-1 | 47 | daily reward | NPC-ECON-1, MARKET-1 (CharacterInbox), HUNT-1 (PREY-1 wildcard balance), ADR-0021 World reset epoch, PREMIUM-1; external prerequisites of the brief children: WO-1 (merged), EXERCISE-CONTENT-1, RUNE-USE, WORLDINT-USE-1 | hard (rewards) | yes | maybe |
| RANK-1 | 48, 128 | highscores and analysers as normalized Game exports (Atlas consumes them) | LOOP-PROGRESS-1; external prerequisites of the brief children: CHAR-REV-SEQ-1, GOLD-FEE-1b, SPELL-TARGET-1, ATTACK-0, ITEM-USE-1c, ATTACK-1b, CHAR-BUILD-1b (merged), CHARM-1 | hard (child graph, App. 7) | maybe | maybe |
| SEC-1 | 135 | server-side integrity checks (bot controls: CLIENT_INTEGRITY, INPUT_TELEMETRY); pacing stays in LOOP-WORLD-1; the 9c/6c3 retirement is separate | LOOP-CLOCK-1 (slice merged); CLOCK-1 (residue); ITEM-LOOK-1 and WEATHER-1 (registry order ITEM-LOOK-1, WEATHER-1, SEC-1); external prerequisites of the brief children: PROD-COMPAT-01, ADMIT-CAP-1, ANL-01, DATA-PRIVACY-01, ANL-03 | hard (child graph, App. 7) | no | yes |
| ADMIT-1 | none (security prerequisite) | World capability admission (ADMIT0, Appendix 7): ADMIT-CAP-1 (the admission authority for `MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS`, `WORLD_INTERACTION_V1`) and ADMIT-CAP-2 (the field activation guard). SEC-CHAL-1 and SEC-CHAL-2 of SEC-1 wait for ADMIT-CAP-1 and are allocated after it | ADMIT0 acceptance (CANDIDATE, Appendix 7); the numbers of the three capabilities reserved on #162 by their wire children; ADMIT-CAP-2 names FIELD-2 (a LOOP-WORLD-1 child) itself | hard (authority, protocol) | no | yes |
| PREREQ-1 | none (supporting) | Scheduling aggregate for the supporting prerequisite briefs and archive follow-ups that no family graph holds (Appendix 7, last three blocks): CHARM-6, DEATH-3, DEATH-4, MAP-OVERLAY-1c (the HOUSE_CUSTODY0 reset check; the unsplit MAP-OVERLAY-1 is superseded), PREM-2b+PREM-5 (Wave 2, after NPC-TALK-1, PREM-WIRE-1 and a promotion source), MKTCOIN-1, MKTCOIN-CONTRACT-1, MKTCOIN-WIRE-1, MKTCOIN-E2E-1, MKTCOIN-P and PREM-P (Platform producer; not Game work), PREM-3, PREM-4, PROF-WIRE-1, WO-2, WO-3, HOUSE-ITEM-WIRE-1, HOUSE-VIEW-1, CHEST-APPEARANCE-ADMIT-1, GOLD-FEE-ACT-2, OPS-KEY-SEPARATION-2, PVP-CLIENT-1, SPELL-LOCK-2b, BOOT-RAT-POLICY-1, FLOOR-FACTS-BUNDLE-1, LOOT-ITEM-PROFILES-1, CONSUMABLE-CONDITIONS-1, SCOPE-HANDOFF-2, SEC-TRUST-1. Each is allocated in dependency order in the wave of its first consumer, never before its own `Depends on` and acceptance gate | each child's own Depends in Appendix 7 | per child | maybe | maybe |
| GM-1 | 136 | GM tools, mute, kick, staff audit and reports (Game side of OPS-GM-01, ARCH-LIVE-READINESS-0 §1). Account bans and sanction enforcement across the boundary are a Platform proposal (P1) that binds nothing in Game until Platform accepts; the owner of bans is unassigned until then. Complete only when every child in App. 7 is complete | LOGOUT-WIRE-1 (LOGOUT-WIRE-1), WRITE-MOD-1; external prerequisites of the brief children: see App. 7 | hard (child graph, App. 7) | yes | yes |
| SPECTATE-1 | 138 | spectating | LOOP-WORLD-1; external prerequisites of the brief children: MAP-WIRE-2, ITEM-USE-WIRE-1, ITEM-EQUIP-WIRE-1, MAP-OVERLAY-1, WO-3, QUEST-GATE-1, QUEST-TRIGGER-1, MAP-LOAD-1, VSL-MOVE-01, ATTACK-1b, ITEM-MOVE-2b, BAGS-GROUND-1, FIELD-WIRE-1, FIELD-1, PVP-RT-1, COND-1, VIS-2, WO-2, QUEST-CONTENT-2, CREATURE-AI-1 (merged), MONSTER-SUMMON-1, ITEM-USE-1c, BOSS-REWARD-1, QUEST-STATE-1, SCOPE-HANDOFF-1, BOSS-ROOM-1, CHAT-1; CHAT-1b-2b | hard (child graph, App. 7) | maybe | yes |

### Death module and in-flight work (not re-planned here)

Rows 107 and 108: DEATH-DUR-1 (B), DEATH-3 (C), PVP-DEATH-1 (D), DEATH-4 (E), optional wire F, per the
DEATH-COMPLETE-0 decision (#2043; fixes in #2044 must land before B, C, E). Row 85 (blessings) is DEATH-4. Rows
110 and 111: PVP-1 (#2032 merged only PVP-1a: migration 0081, the PvP state tables and the deadline write-ahead; the residue PVP-1b, then PVP-RT-1, still wait) and PVP-RT-1. Partly merged: 31 and 76 (#2039 is ITEM-USE-1a and 1b only; the runtime, ingress and capability 15 offer are ITEM-USE-1c, in flight) and 80 (#2040 is the NPC trade writer only; the residue is NPC-CMD-1b, NPC-TRADE-WIRE-1 and NPC-TRADE-CHARGED-1, Appendix 7). In-flight PR: 105 (#1867). Rows 6,
55 and 3 are split, never owned twice: #2018 owns the part of row 6 it implements and LOOP-WORLD-1 owns only the
residue after #2018 merges (keyed/level/quest doors, levers, tile damage, as listed in its scope); #2039 owns the
rune item-use part of row 55 and MAGIC-ITEMS-1 only rune use with a target and conjure; the server part of row 3
is #2041 (merged) and the rest of row 3 is client-owned (#1942), so no batch here takes row 3. Each worker confirms the
residue against the merged PR before it starts.

## 5. Waves and parallelism

Throttles: one migration PR at a time and one protocol-registry PR at a time, so wave width is set by `Mig` and
`Reg`, not by the number of workers.

1. **Wave 0 (in flight):** #2018, #1867, #2044, ITEM-USE-1c (USE-ITEM worker; #2039 merged only 1a and 1b), NPC-CMD-1b (own branch), ENCOUNTER-RT-0 (a CANDIDATE decision, see section 7 status list). Merged on main@ab7d51ae6 and no longer scheduled: #2039 (ITEM-USE-1a+1b only), #2040 (the NPC trade writer only), #2041 (STEP-PACE-WIRE-1), #2025 (BUNDLE-COMBAT-ENABLE-1), #2032 (PVP-1a only), #1798 (ATTACK-1b), #2071 (the LOOP-CLOCK-1 slice only; CLOCK-1 is the Wave 1 residue), #2073 (OPS-SAVE-1 monk-state flush). Status audit against main (archive records, migrations, `GAME_EVENT_FOUNDATION_REGISTRY.json`, symbols): every batch delivered there is marked MERGED in its row and is not scheduled; only the residue is. ECON-RET-0 and BANK-RET-0 (#1748: `GUILD_ACTIVITY_RETENTION_V1`, `MARKET_ECONOMY_LEDGER_RETENTION_V1`, `ECONOMY_LEDGER_RETENTION_V1`) are merged, so BANK-1 (merged), GUILD-1 and MARKET-1 have no retention gate.
2. **Wave 1 (no migration, parallel):** ADMIT-CAP-1 (the first part of ADMIT-1, after the capability numbers are reserved; before SEC-1, whose SEC-CHAL-1 and SEC-CHAL-2 wait for it), CHAT-1b-2b (residue of CHAT-1), PREMIUM-1 (PREM-WIRE-1; gated on the acceptance of PREMIUM_ACTIVATION; then first: releases every Premium-gated batch), CHAT-2 (only CHAT-1b-2b of CHAT-1 is open; CHAT-2 only after CHAT-1b-2b and the CHAT-0 re-review and owner acceptance; before PARTY-1), LOGOUT-WIRE-1 (KILL-REWARD-COMP-1 is merged on main; LOGOUT-WIRE-1 after ATTACK-1b #1798, merged), MAGIC-CAST-1, LOOP-COMBAT-MODES-1, LOOP-WORLD-1, MAGIC-COND-1,
   CLOCK-1 (residue of LOOP-CLOCK-1, which is only a merged slice; after VIS-2 and COND-1 from MAGIC-COND-1, before both SEC-1 and WEATHER-1), ITEM-LOOK-1, WEATHER-1 (after ITEM-LOOK-1, before SEC-1), then PREREQ-1 (each child in the wave of its first consumer; residue children BOOT-RAT-POLICY-1 and SCOPE-HANDOFF-2 in Wave 2), then SEC-1 (SEC-CHAL-1 and SEC-CHAL-2 wait for ADMIT-1; registry order ITEM-LOOK-1, WEATHER-1, SEC-1; WEATHER-1 is visual only and now Wave 1, before SEC-1). In sequence for migrations: ITEM-LOOP-1, then W2b, DAWNPORT-1 and LOOP-PROGRESS-1 (CHAR-BUILD-1 is merged: #1393 and the 1b writer; after the
   death module's B and C; DEATH-ML-LOSS-1 after DEATH-1 gates production training).
3. **Wave 2:** PREM-2b+PREM-5 (after NPC-TALK-1, PREM-WIRE-1 and GOLD-FEE-1b; the hold is released, the promotion source is the PREM-5 NPC promotion), OPS-SAVE-2, ATTACK-2, BUNDLE-GATES-1, LOOP-DAMAGE-1, LOOP-RANGED-1, LOOP-VITALS-1, NPC-ECON-1 (with NPC-CMD-1b, NPC-TRADE-WIRE-1), QUEST-RT-1, MAGIC-ITEMS-1,
   MAGIC-SUMMON-1, LOOP-MONSTER-1, CREATURE-MOVE-1 (residue of 1a and 1b), PARTY-1.
   PVP-1a (#2032) is merged; PVP-1b (residue of PVP-1), PVP-RT-1 and PVP-DEATH-1 follow PARTY-1 and DEATH-1.
4. **Wave 3:** ADMIT-1 residue (ADMIT-CAP-2 after FIELD-2; ADMIT-CAP-1 is in Wave 1), MAP-VIEWPORT-OPT-1, MAP-VIEWPORT-LIMIT-1, LOOP-MAP-1 (after the #1942 client proof), ITEM-TIMED-1 (NPC-TRADE-CHARGED-1 follows TIMED-RT-1c), ITEM-STORAGE-1, NPC-WORLD-1, LOOP-GROUND-1, CRAFT-1, PROG-ADV-1,
   MONSTER-META-1, HOUSE-RUNTIME-1a, HOUSE-RUNTIME-1b, HOUSE-RUNTIME-1c, MAP-OVERLAY-1c, MAP-CUTOVER-1c (in that order, after MAP-LOAD-1; SCOPE-HANDOFF-1, MAP-OVERLAY-1a and 1b and MAP-CUTOVER-1a and 1b are merged), PARTY-XP-1.
5. **Wave 4:** TRADE-PLAYER-1, MAIL-1, MARKET-1, GUILD-1, GUILD-BANK-1, GUILD-CHAT-1, VIP-1, ITEM-IMBUE-1, ITEM-FORGE-1,
   HUNT-1, DAILY-1, ACH-1, RANK-1, APP-1, APP-2, APP-3, APP-4 (in that order). STORE-ENT-1 is not scheduled: it is blocked on the §32 delivery decision and routes to Game or Platform only after that decision lands.
6. **Wave 5:** STEP-PACE-ENABLE-1 (when the client release gate is met), HOUSE-1a, then HOUSE-ACL-1, HOUSE-1b, HOUSE-WIRE-1, BED-1, BED-REGEN-1, GUILDHALL-1, GUILD-WIRE-1, VIP-2, HIRE-1 and LOOP-TRAIN-1, EVENTS-*, ARENA-1 (with the PvP arena of the retired SOCIAL-LATE-1), GM-1, SPECTATE-1, TUTORIAL-1.

Waves are an order of dependency, not a schedule; the control plane starts a batch when its dependencies have
merged. The death module runs alongside wave 1 and 2 on its own track.

## 6. Owner answers (2026-10-13, via the control plane)

1. Start stays in Thais; the tutorial island is a late, non-blocking test epic (TUTORIAL-1).
2. Weather and server save/restart are in scope (WEATHER-1, OPS-SAVE-1). The Store is in scope as a mechanic, but the owner direction did not decide the Game/Platform delivery boundary; that stays open under gap register §32 (see STORE-ENT-1 and owner question Q4 below).
3. Order confirmed.

Original questions, kept for the record:

1. **Tutorial.** Do we need a tutorial island (Rookgaard or the newer Newhaven) before the first public release?
   Recommendation: no; start in Thais as today and decide later.
2. **Out of scope.** Confirm that weather, the Store and server save/restart are not Game mechanics for now.
   Recommendation: confirm.
3. **Order.** The plan finishes the game loop first, then economy, magic and conditions, the world, then
   community and meta. Recommendation: confirm; community features (guilds, market, houses) come last.
4. **Store delivery owner.** Who delivers purchased Store items and cosmetic unlocks across the Game/Platform boundary (gap register §32)? Options: (a) Platform entitlement line that Game reads, (b) Game inbox record. Recommendation: decide in the §32 decision before STORE-ENT-1 is routed; until then it is unassigned and blocked.

## 7. Decision test

- **Minimum sufficient:** one batch per coherent unit, in-flight PRs untouched, no new infrastructure.
- **What becomes harder later:** the wave order fixes the migration and registry sequence; a late change of
  priority costs a re-lease of numbers.
- **Superseding evidence:** a measured gap on the bundle node (rows with "bundle not checked"), or a changed
  owner priority.
- **Deliberately not decided:** per-batch design (each batch brings its own decision when it changes a contract).

## 8. Handback

```yaml
result: CANDIDATE
owner_decisions: [Q1, Q2, Q3 answered 2026-10-13]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "document review (coverage of rows, dependencies, hard/impl classes)"
implementation_lanes: []   # allocated per batch by the control plane
remaining_unknowns:
  - "bundle-node state of level-up stats (row 37) and of the rows marked 'bundle'"
  - "target-selection effect of fight modes (row 20)"
next_action: "#1622: sub-issues for waves 1 and 2, then allocation by the control plane."
```

## 7. Appendix: child graphs of the briefs

**Status of the cited briefs (checked on main@ab7d51ae6, 2026-10-13, against each decision's Status line and the ACCEPT-* acceptance records; refreshed 2026-10-15 for the acceptances recorded since and for ACCEPT-BATCH-2026-10-15).** ACCEPTED: ARCH_LIVE_READINESS, ATTACK0, BANK0, GOLD_FEE_BOUNDARY, GUILD0, HOUSE_CUSTODY0, HOUSE_OWN0, HOUSE_RUNTIME0, ITEM_USE0, MAP_WIRE1, NPC_BEHAVIOUR0, PARTY_PVP0, PREMIUM_ACTIVATION0, WORLD_INTERACTION0 (GUILD0, HOUSE_CUSTODY0, HOUSE_OWN0, HOUSE_RUNTIME0, PARTY_PVP0 and ADR-0021 by ACCEPT-SOCIAL-MAP-0, whose status lines are reconciled 2026-10-15); accepted since the 2026-10-13 check: CHAT0, CONDITIONS0, ENCOUNTER_RT0, EQUIP0, RUNE_USE0, SPELL_NPC_MAP0, SPELL_PRESENT0 and PREMIUM_ACTIVATION (the 2026-09-28 decision, D69-D76; its acceptance releases PREMIUM-1 and the PREM-* children); accepted by ACCEPT-BATCH-2026-10-15 (`OTERYN_GAME_ACCEPT_BATCH_2026-10-15_ACCEPTANCE_DECISION.md`): A13, B3, BAGS0, STACK0, ITEM_MOVE_WIRE1, STARTER_BACKPACK0, CREATURE_AI0, RANGED0, NPC0, BANK_FEE0, CHARACTER_APPEARANCE, BOSS_RAID0, MONSTER_D15, ADMIT0, DEATH_COMPLETE0, OFFLINE0, VIP0, IMBUE_FORGE0, BED0, SEC_CLIENT01, CHARM4, FAMILIARS0, QUEST_GATE0, QUEST_STATE0, PLAYER_TRADE0, MAIL0, MARKET0, PREY0, DAILY_REWARD0, ACHIEVEMENT0, HIGHSCORES0, ANALYSERS0, TIMED_ITEM0, TIMED_ITEM0B, DEPOT0, SUPPLY_STASH0 (STASH0), WRITE0, TRAVEL0, CYCLOPEDIA0, EXERCISE0, ECON_RET0; accepted by the owner on 2026-10-15 ("3 tak", ACCEPT-BATCH-2026-10-15 §6): SKILLS0, PROFICIENCY0, PROFICIENCY1, PROFICIENCY1B, PROF_EFFECT0, PROF_WIRE0, WHEEL0, WHEEL_GEM0, WHEEL_GEM0A. Every other brief named in a Brief column is still CANDIDATE: ACTION_INTEGRATION, D3, DONOR_IDENTITY, FIRST_PLAYER_DEATH, ITEM_IDENTITY, ITEM_MOVE_WIRE0, MARKET_COINS0, PREMIUM_DELIVERY0, STANCE0, VIS, WORLD_OBJECT (not re-verified on 2026-10-15); the CRAFT1 and HIRE1 briefs are CANDIDATE (2026-10-15). The per-graph gate lines below keep their 2026-10-13 wording: a brief they list as CANDIDATE that is ACCEPTED here is no longer a gate. **Acceptance gate: no child of a CANDIDATE brief is allocated until the owning decision is ACCEPTED;** every graph below carries the gate line, and it also binds the section 4 rows that depend on those briefs. The packets documents (SOCIAL_MAP, CORE_LOOP, KILL_REWARD_LOGOUT, HOUSE_RT_INBOX, MAP_TRACK) are accepted on merge.

Each row below is its own batch, allocated when its listed dependencies have merged. An ID that appears both in section 4 and here is the same batch (section 4 schedules it, this table carries the brief's gate); an ID appears once per section, and two briefs that name the same child share one row. A collective batch in section 4 is complete only when all of its children are listed here or in section 4. `Class` is the brief's worker column (`hard` = hard worker with the named review); `Reg` is marked where the child builds a capability or wire contract. The brief is authoritative for Builds and Depends on; this table only mirrors it.

### LOOP-WORLD-1, SPECTATE-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (ENCOUNTER_RT0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| WORLDINT-WIRE-1 | WORLD_INTERACTION0 | impl, protocol review | yes | MAP-WIRE-2; ITEM-USE-WIRE-1; ITEM-EQUIP-WIRE-1 |
| WORLDINT-USE-1 | WORLD_INTERACTION0 | hard, security review |  | WORLDINT-WIRE-1; MAP-OVERLAY-1; WO-3 |
| DOOR-1 | WORLD_INTERACTION0 | **MERGED (#1966).** hard, security review |  | WORLDINT-USE-1; QUEST-GATE-1 |
| KEY-1 | WORLD_INTERACTION0 | hard, persistence review | yes | DOOR-1; CHEST-1 (merged) |
| LEVER-1 | WORLD_INTERACTION0 | hard |  | QUEST-TRIGGER-1; WORLDINT-USE-1 |
| FLOOR-1 | WORLD_INTERACTION0 | **MERGED (#1967).** impl, movement review |  | MAP-LOAD-1; WORLDINT-USE-1 |
| PUSH-1 | WORLD_INTERACTION0 | impl, movement and combat review |  | WORLDINT-WIRE-1; the VSL-MOVE-01 movement owner; ATTACK-1b |
| GROUND-MOVE-1 | WORLD_INTERACTION0 | hard, persistence review |  | ITEM-MOVE-2b; BAGS-GROUND-1 for trees |
| WORLDINT-ADMIT-1 | WORLD_INTERACTION0 | hard, protocol and security review with the FND-04 owner |  | FIELD-WIRE-1 |
| FIELD-2 | WORLD_INTERACTION0 | hard (combat), combat review |  | FIELD-1; WORLDINT-ADMIT-1; PVP-RT-1 |
| HAZARD-1 | WORLD_INTERACTION0 | impl, combat review |  | LEVER-1; COND-1 |
| CLOCK-1 | WORLD_INTERACTION0 | impl, determinism and protocol review. The clock epoch, the clock and light functions and capability 19 with domain 18 are MERGED (#2071); the residue is the `world_time` predicate and the VIS-2 `light` field (WORLD_INTERACTION0 §11) | yes | VIS-2; COND-1 |
| WORLDINT-CONTENT-1 | WORLD_INTERACTION0 | content |  | WO-2; WO-3; QUEST-CONTENT-2 |
| ENC-RT-1 | ENCOUNTER_RT0 | hard (determinism), determinism review |  | CREATURE-AI-1 (merged); the typed Encounter profile (E1) |
| ENC-COMBAT-1 | ENCOUNTER_RT0 | hard (combat), combat review |  | ENC-RT-1; COND-1; MONSTER-SUMMON-1 |
| ENC-WORLD-1 | ENCOUNTER_RT0 | impl, determinism review |  | ENC-RT-1; LEVER-1; FLOOR-1; CHAT-1; CHAT-1b-2b; ITEM-USE-1c |
| ENC-OUTCOME-1 | ENCOUNTER_RT0 | hard (persistence), persistence and security review |  | ENC-RT-1; BOSS-REWARD-1; QUEST-STATE-1 |
| INSTANCE-GROUND-1 | ENCOUNTER_RT0 | hard (persistence), persistence review |  | SCOPE-HANDOFF-1; GROUND-MOVE-1 |
| BOSS-LEVER-1 | ENCOUNTER_RT0 | impl, security review |  | LEVER-1; BOSS-ROOM-1 |
| ENC-PARITY-1 | ENCOUNTER_RT0 | impl |  | every child above |

### LOOP-MAP-1, map wire (APP-* children are in section 4)

Acceptance gate: no child in this graph whose Brief is CANDIDATE (CYCLOPEDIA0; ARCH_MAP_VIEWPORT_BUDGET is a decision of 2026-10-06 whose limits become production rows only through MAP-VIEWPORT-LIMIT-1) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| MAP-WIRE-2 | MAP_WIRE1 | **MERGED (#1810).** hard (protocol), protocol and security review | yes | MAP-LOAD-1; MAP-OVERLAY-1; VIS-2 |
| MAP-VIEWPORT-MEASURE-1 | ARCH_MAP_VIEWPORT_BUDGET | **MERGED (#1873, evidence `docs/agents/evidence/MAP-VIEWPORT-MEASURE-1-viewport.md`).** impl: real-map measurement of the domain-17 snapshot and delta; result: snapshot p99 646-791 us, delta p99 388-445 us, 163-174 us CPU per update, 0.41-0.44 core at 500 players and 5 steps per second, which does not fit the 20% (0.20) default |  | MAP-VIEWPORT-PERF-2 |
| MAP-VIEWPORT-OPT-1 | ARCH_MAP_VIEWPORT_BUDGET | impl, performance review: the bounded optimisation slice §1.2 names (the `crates/protocol-oteryn` encode, PERF-2 option A), or a budget question to the owner; never a wire change made only to pass. Wave 3, with LOOP-MAP-1 |  | MAP-VIEWPORT-MEASURE-1 (merged); MAP-CUTOVER-1b (merged) |
| MAP-VIEWPORT-LIMIT-1 | ARCH_MAP_VIEWPORT_BUDGET | decision (architect, control plane routes): registers the production snapshot and delta limits derived from the evidence and D128 (500 players per Channel, 20% of one core default); no production offer of capability 18 before it. Wave 3 |  | MAP-VIEWPORT-OPT-1 |
| MAP-PICKUP-1 | MAP_WIRE1 | hard, persistence review: command 9 from a movable base-map entry to a container (MAP_WIRE_PACKETS §2.4); on main it answers NOT_SUPPORTED, and it replaces the MAP-WIRE-2 refusal |  | MAP-WIRE-2; MAP-OVERLAY-1b; ITEM-MOVE-1 |
| MAP-CLIENT-1 | MAP_WIRE1 | client lane; the RE-ISSUED packet (OTV2-20261005-map-client-1, MAP_TRACK_PACKETS §2.6) supersedes the ARCH-MAP-WIRE §2.3 version, which was never allocated |  | MAP-CUTOVER-1b; login N2N3-1; login N8-1; MAP-ITEM-REF-1 |
| CYC-CONTENT-1 | CYCLOPEDIA0 | content |  | none |
| CYC-DISCOVERY-1 | CYCLOPEDIA0 | hard (persistence) |  | CYC-CONTENT-1; CHAR-REV-SEQ-1 |
| CYC-DONATE-1 | CYCLOPEDIA0 | hard (economy, persistence) |  | CYC-CONTENT-1; BANK-FEE-0 |
| CYC-RESPAWN-1 | CYCLOPEDIA0 | impl |  | CYC-DONATE-1; SPAWN-1b |
| CYC-WIRE-1 | CYCLOPEDIA0 | impl, protocol review | yes | CYC-DISCOVERY-1; CYC-DONATE-1 |
| CYC-PARITY-1 | CYCLOPEDIA0 | research |  | none |

### LOOP-COMBAT-MODES-1, LOOP-RANGED-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (RANGED0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| ATTACK-WIRE-1 | ATTACK0 | **MERGED (#1758).** impl, protocol review | yes | VIS-2 (MOVE-RL-11 is still a candidate); the protocol numbers re-checked at allocation |
| ATTACK-CLIENT-1 | ATTACK0 | client lane (CORE_CLIENT_PACKETS item 5) |  | ITEM-CLIENT-1 (merged); ATTACK-WIRE-1; ATTACK-1b offers capability 17 |
| ATTACK-1a | ATTACK0 | **MERGED (#1737).** impl |  | see CORE_LOOP_PACKETS |
| ATTACK-1b | ATTACK0 | hard (combat), combat review; MERGED (#1798) |  | ATTACK-1a; ATTACK-WIRE-1; GAME-AI-01 runtime wiring (#1798) |
| SPELL-TARGET-1 | ATTACK0 | **MERGED (#1808).** spell lane |  | ATTACK-1b |
| ATTACK-PARITY-1a | ATTACK0 | **MERGED (#1775).** impl: captures the fixtures; owns only new paths, so it starts in parallel with ATTACK-1b (ATTACK_PARITY1 packet §2.1) |  | none |
| ATTACK-PARITY-1b | ATTACK0 | impl, combat review: compares the fixtures with the formulas and corrects them; writes `combat/attack/**`, so it starts from main after ATTACK-1b merges (§2.2) |  | ATTACK-1b (merged); ATTACK-PARITY-1a |
| RANGED-CONTENT-1 | RANGED0 | content |  | ITEM-SEM-2b |
| QUIVER-1 | RANGED0 | impl, persistence review |  | BAGS-1; ITEM-MOVE-2a; BAGS-WIRE-1 |
| RANGED-1 | RANGED0 | hard (combat, persistence), combat and persistence review |  | ATTACK-1b; QUIVER-1; RANGED-CONTENT-1; ITEM-MOVE-2b; DUR-03 amendment accepted |
| CHASE-1 | RANGED0 | hard (movement), movement and determinism review |  | ATTACK-1b; SPEED-1 (merged); supplies the shared CREATURE-MOVE-1 §5.3 path search that CREATURE-MOVE-1 then reuses (no CREATURE-MOVE-1 prerequisite) |
| RANGED-PARITY-1 | RANGED0 | impl |  | RANGED-1 |

### LOOP-DAMAGE-1, MAGIC-COND-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (CHARM4, CONDITIONS0, EQUIP0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| EQUIP-CONTENT-1 | EQUIP0 | **MERGED (#1719).** content |  | ITEM-SEM-2b |
| EQUIP-RT-1 | EQUIP0 | hard (combat), combat and determinism review |  | ITEM-MOVE-2a; SPEED-1 (merged); COND-1; CHAR-BUILD-1 (merged) and SKILLS-0's live skill read (the base a `SKILL_BOOST` adds to) |
| EQUIP-PARITY-1 | EQUIP0 | impl |  | EQUIP-RT-1 |
| CHARM-DEF-1 | CHARM4 | combat lane, combat and determinism review |  | ATTACK-1b (creature attacks on characters) |
| COND-1 | CONDITIONS0 | **COND-1a/1b/1c MERGED (#1405, #2086).** Residual: the §3.5 keys, the runtime `COND0_RL_01_INSTANCES_PER_ACTOR` bound raised from 16 to 32, the `agony` tick and its §3.5 replace-on-equal-or-greater policy, the admission gates and the `PACIFIED` clearing of a running attack target. hard (combat), combat and determinism review |  | this decision |
| SPEED-1 | CONDITIONS0 | **MERGED (#1715).** No residual: the creature step timer is CREATURE-MOVE-1's (CONDITIONS-0 §4.3). impl, movement review |  | none (merged) |
| COND-WIRE-1 | CONDITIONS0 | impl, protocol review | yes | COND-1; SPEED-1 (merged); VIS-2 |
| COND-DUR-1 | CONDITIONS0 | hard (persistence), persistence review |  | COND-1; FOOD-REGEN-1 |
| COND-CONTENT-1 | CONDITIONS0 | **MERGED (#1400).** content |  | this decision |
| COND-CONTENT-2 | CONDITIONS0 | content |  | this decision; COND-CONTENT-1 (merged) |
| COND-WIRE-2 | CONDITIONS0 | impl, protocol review | yes | COND-1 (§3.5 residual); COND-WIRE-1; PRESENT-WIRE-1 |
| COND-CLIENT-2 | CONDITIONS0 | client lane (client owner) |  | COND-WIRE-2; PRESENT-CLIENT-1 |

### MAGIC-CAST-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (SPELL_NPC_MAP0, SPELL_PRESENT0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

SPELL-RT-FIELDBUFF-1 is not part of the MAGIC-CAST-1 completion set: its prerequisite is the field-ingress child WORLDINT-ADMIT-1 of LOOP-WORLD-1 (not the LOOP-WORLD-1 aggregate), and it completes with MAGIC-SUMMON-1, so the shared summon children (MONSTER-SUMMON-1, CREATURE-AI-1) that inherit the MAGIC-CAST-1 entry gate do not wait for LOOP-WORLD-1.

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| PRESENT-CONTENT-1 | SPELL_PRESENT0 | content. The 15.30 sound files and the sound-id check are PRESENT-CONTENT-1b |  | this decision |
| PRESENT-CONTENT-1b | SPELL_PRESENT0 | content. **BLOCKED_OWNER_INPUT:** the 15.30 sound files (the client `sounds/` folder) are in neither the repository nor the asset manifest; commit them under `content/assets/files/` with manifest entries and check every bound sound id against them |  | PRESENT-CONTENT-1; owner input |
| PRESENT-WIRE-1 | SPELL_PRESENT0 | impl, protocol review | yes | this decision; VIS-2 |
| SPELL-PRESENT-1 | SPELL_PRESENT0 | spell lane, combat review |  | PRESENT-WIRE-1; PRESENT-CONTENT-1; SPELL-D4 composition; PRESENT-CONTENT-1b before any sound id is emitted |
| COMBAT-PRESENT-1 | SPELL_PRESENT0 | combat lane, combat and determinism review |  | PRESENT-WIRE-1; PRESENT-CONTENT-1; ATTACK-1b; COND-1 for ticks; PRESENT-CONTENT-1b before any sound id is emitted |
| PRESENT-CLIENT-1 | SPELL_PRESENT0 | client lane (client owner) |  | PRESENT-WIRE-1; PRESENT-CONTENT-1; PRESENT-CONTENT-1b for sound |
| SPELL-RT-SCHED-1 | SPELL_NPC_MAP0 §3 | spell lane, combat and determinism review. Key `delayed_strike` (D.1.5): the spell scheduler, delayed and repeated strikes, cancellation (no scheduler exists on main; this child is the scheduler, never a second runtime) |  | SPELL_NPC_MAP0 decision; S7 reader; ATTACK-1b; COND-1 for ticks |
| SPELL-RT-FIELDBUFF-1 | SPELL_NPC_MAP0 §2, §3 | spell lane. Key `owned_field_buff` (Divine Empowerment, B.1) |  | SPELL-RT-SCHED-1; WORLDINT-ADMIT-1 (the LOOP-WORLD-1 field-ingress child) |
| SPELL-RT-MONK-1 | SPELL_NPC_MAP0 §2, §3, §5 | spell lane, combat review. Key `monk_focus` and the `harmony_role` field (A.2, S26), the 3 callbacks once (never per spell), the 6 Monk formulas with source types and rounding |  | SPELL_NPC_MAP0 decision; S7 reader; ATTACK-1b; COND-1 |
| SPELL-RT-MONKAI-1 | SPELL_NPC_MAP0 §2, §3 | spell lane. Key `monster_ai_override` (A.3, Balanced Brawl) |  | GAME-AI-01 runtime wiring (#1798); SPELL-RT-MONK-1 |
| SPELL-RT-STANCE-1 | SPELL_NPC_MAP0 §3 | spell lane. Key `stance_toggle` (C.4), the 27 stance spells |  | STANCE-0; STANCE-1 |
| SPELL-RT-GUARD-1 | SPELL_NPC_MAP0 §3 | spell lane. Key `cast_restriction` (B.5): the 14 cast guards, Sweeping Takedown, Crystal Heal Friend (`spell/cast.rs` admission) |  | SPELL_NPC_MAP0 decision; S7 reader |
| SPELL-RT-TARGET-1 | SPELL_NPC_MAP0 §3 | spell lane. Fields `targeting.allowed_targets` and `affects.top_creature` (D.3), the `targeting.parameter` extension (B.3) (`spell/target.rs`) |  | SPELL-TARGET-1; ATTACK-1b |
| SPELL-RT-CHAIN-1 | SPELL_NPC_MAP0 §3 | spell lane, combat review. S23 chain: several Combat objects in one cast, Forked Glacier and Forked Thorns, Executioner's Throw, Spiritual Outburst (`spell/chain.rs`, `spell/plan.rs`) |  | SPELL-RT-SCHED-1; ATTACK-1b |
| SPELL-RT-EQUIP-1 | SPELL_NPC_MAP0 §2, §3 | spell lane. D.4 reader additions: `player_expression` Formula with `inputs: skill` and `needs_weapon`, the three shield-spell Ability additions, `outfit_binding` for the Avatars |  | SKILLS-0; ITEM-SEM-2b |
| SPELL-RT-PARTY-1 | SPELL_NPC_MAP0 §3 | spell lane. Key `party_buff` (C.3) adapter and the 11 party spells (`spell/party.rs`) |  | PARTY-1 for membership reads |
| SPELL-RT-PREM-1 | PREMIUM_ACTIVATION0 §4.6 | spell lane. Premium spell check at admission through the merged Premium reader, no second Premium runtime |  | PREMIUM-1 |

MAGIC-CAST-1 is complete only when every child in this table except SPELL-RT-FIELDBUFF-1 is complete (that child completes with MAGIC-SUMMON-1, see the exclusion above). The wheel spells (36) belong to the WHEEL batches (SPELL-WHEEL-GATE-1 first) and the Paralyze caster effect to RUNE-CAST-1 under MAGIC-ITEMS-1; neither is a second copy here.

### LOOP-VITALS-1, LOOP-TRAIN-1, house beds

Acceptance gate: no child in this graph whose Brief is CANDIDATE (BED0, EXERCISE0, OFFLINE0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| OFFLINE-1 | OFFLINE0 | hard (persistence), persistence review |  | CHAR-BUILD-1 (merged) (`0030`, merged); SKILLS-0's build-state skills; CHAR-POSITION-1; a slot in the shared guard-rewrite order |
| STAMINA-1 | OFFLINE0 | impl, combat review |  | OFFLINE-1; PREY-0's hunting clock owner moves here |
| STATUE-1 | OFFLINE0 | impl, protocol review |  | OFFLINE-1; WORLDINT-USE-1 |
| OFFLINE-CONTENT-1 | OFFLINE0 | content |  | WO-2 |
| OFFLINE-PARITY-1 | OFFLINE0 | impl |  | OFFLINE-1 |
| EXERCISE-CONTENT-1 | EXERCISE0 | content |  | TIMED-CONTENT-1 (the `charges` fact shape); WO-2 |
| EXERCISE-1 | EXERCISE0 | hard (persistence), persistence and determinism review |  | TIMED-ITEM-0B; SKILLS-0's build-state skills; A13's magic-level training; WORLDINT-USE-1 |
| EXERCISE-PARITY-1 | EXERCISE0 | impl |  | EXERCISE-1 |
| BED-CONTENT-1 | BED0 | **MERGED (#1861).** content |  | ITEM-SEM-2b |
| BED-1 | BED0 | hard (persistence), persistence and protocol review |  | OFFLINE-1; STATUE-1 (the shared activation path); HOUSE-RUNTIME-1a, 1b and 1c; HOUSE-VIEW-1; WORLDINT-WIRE-1 |
| BED-REGEN-1 | BED0 | impl, persistence review |  | BED-1; DUR-02 durable vitals |

### LOOP-MONSTER-1, MAGIC-SUMMON-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (CREATURE_AI0, FAMILIARS0, SPELL_NPC_MAP0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| SPAWN-CONTENT-1 | CREATURE_AI0 | **MERGED (#1759).** content (amended: spawns are a base bundle family) |  | MAP-BUNDLE-1 (merged); admitted creature definitions; not MAP-OVERLAY-1 and not the MAP-CUTOVER-1a/1b pair (CREATURE_AI0 amendment, CORE_LOOP_PACKETS §1.3, §1.4) |
| SPAWN-1a | CREATURE_AI0 | **MERGED (#1969).** hard, performance review: realization and respawn of the D116 fixture spawn (2 rats, `oteryn:spawn/entry-rat`) through one spawn-source seam, before MAP-LOAD-1 |  | AI-2 (merged); SPAWN-CONTENT-1 (merged); VIS-1 (merged) |
| SPAWN-1b | CREATURE_AI0 | **MERGED (#1981).** hard, performance review: the bundle spawn family through MAP-LOAD-1's reader, and the `RL-15` and `RL-16` measurements |  | SPAWN-1a (merged); MAP-LOAD-1; SPAWN-CONTENT-1 (merged) |
| CREATURE-AI-1 | CREATURE_AI0 | **MERGED (#1813).** hard (AI), combat and determinism review |  | VIS-1 (merged); ATTACK-1b; AI-4 (merged) |
| CREATURE-MOVE-1 | CREATURE_AI0 | Slices MERGED: 1a (#1977: chase, flee and wander steps, one step per think) and 1b (#1987: steps admitted on the entry room). Residue, still this row: the 1a deferrals (speed-based step timer, A* paths, walk back, leash, dance, near-target, `relocate_creature`; Wave 2). hard (movement), movement and performance review |  | CREATURE-AI-1 (merged); SPEED-1 (merged) |
| MONSTER-SUMMON-1 | CREATURE_AI0 | impl, combat review |  | CREATURE-AI-1 (merged) |
| SUMMON-1 | CREATURE_AI0 | hard (combat), combat and security review |  | CREATURE-MOVE-1; ATTACK-1b |
| SUMMON-WIRE-1 | CREATURE_AI0 | impl, protocol review | yes | VIS-2; SUMMON-1 |
| FAMILIAR-CONTENT-1 | FAMILIARS0 | content |  | the spell and creature authoring schemas |
| FAMILIAR-1 | FAMILIARS0 | hard (persistence, combat), persistence and combat review |  | SUMMON-1; FAMILIAR-CONTENT-1 |
| SPELL-RT-SUMMON-1 | SPELL_NPC_MAP0 §3 | spell lane. Keys `acquire_summon` (C.2) and `summon_named_creature` (B.3); the 17 summon spells |  | SUMMON-1; SUMMON-WIRE-1 |
| SPELL-RT-FAMILIAR-1 | SPELL_NPC_MAP0 §3 | spell lane. Key `familiar_summon` (C.1), the 5 familiar spells |  | FAMILIAR-1; FAMILIAR-CONTENT-1 |
| SPELL-RT-WORLD-1 | SPELL_NPC_MAP0 §2, §3 | spell lane. World-control keys `vertical_move`, `locate_message`, `tile_item_operation` (B.1, B.3, D.2), `house_access` (B.2), `creature_appearance` (B.3), `random_item_grant` (B.4); Levitate and Magic Rope helpers belong to WORLD-INTERACTION-0 |  | SPELL_NPC_MAP0 decision; S7 reader; WORLDINT-USE-1; HOUSE-1b for `house_access` |

MAGIC-SUMMON-1 is complete only when every child above, the `owned_field_buff` child of MAGIC-CAST-1 and the SUMMON-1 and SUMMON-WIRE-1 children are complete.

### ITEM-LOOP-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (BAGS0, ITEM_MOVE_WIRE0, ITEM_MOVE_WIRE1, STACK0, STARTER_BACKPACK0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

Integration children BAGS-DEPOT-1, BAGS-TRADE-1, STACK-1, STACK-PARITY-1 and ITEM-USE-CLIENT-1 are not part of the ITEM-LOOP-1 completion set: each is allocated after the downstream child it names (DEPOT-1, TRADE-1, the RANGED-0 §6.1.2 Ground order (existing `stack_ordinal`), ATTACK-CLIENT-1) and completes with that downstream family, so ITEM-STORAGE-1, TRADE-PLAYER-1 and LOOP-RANGED-1 are not blocked by them. ITEM-LOOP-1 is complete only when B3-3 (item weights, character capacity source, overweight refusal) and every other child in this table, except those integration children, are complete; B3-3 keeps the acceptance gate of the B3 brief (B3 is CANDIDATE): it is allocated only after B3's acceptance and its weight and capacity evidence.

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| ITEM-VIEW-1a | ITEM_MOVE_WIRE0 | **MERGED (archive record OTV2-20261003-item-view-1a).** impl, protocol review | yes | none |
| ITEM-VIEW-1b | ITEM_MOVE_WIRE0 | **MERGED (archive record OTV2-20261003-item-view-1b).** hard (session resume state) |  | ITEM-VIEW-1a; CAP-NEG-1; CAP-NEG-RESUME-FALLBACK-1; VIS-2 |
| ITEM-MOVE-1 | ITEM_MOVE_WIRE0 | **MERGED (#1827).** hard, persistence review |  | ITEM-VIEW-1b; the combat loot MINT wired into the runtime (combat lane child, MOVE-RL-11 child E) |
| ITEM-EQUIP-WIRE-1 | ITEM_MOVE_WIRE1 | **MERGED (#1711).** impl, protocol review | yes | ITEM-VIEW-1b |
| ITEM-MOVE-2a | ITEM_MOVE_WIRE1 | hard, persistence review |  | ITEM-MOVE-1; ITEM-EQUIP-WIRE-1; ITEM-SEM-2b (slot, hands, requirements in content) |
| ITEM-MOVE-2b | ITEM_MOVE_WIRE1 | hard, persistence review |  | ITEM-MOVE-2a; VIS-2; MAP-OVERLAY-1 (Ground rebuild and `WorldReset` retirement) |
| BAGS-WIRE-1 | BAGS0 | **MERGED (archive record OTV2-20261003-bags-wire-1; 1b #1730).** impl, protocol review | yes | ITEM-VIEW-1b; ITEM-EQUIP-WIRE-1 |
| BAGS-1 | BAGS0 | hard, persistence and performance review |  | ITEM-MOVE-2a; BAGS-WIRE-1 |
| ITEM-CLIENT-2 | ITEM_MOVE_WIRE1 | client lane, one client owner at a time (ITEM_EQUIP_PACKETS items 15-17) |  | ITEM-CLIENT-1 (merged); ITEM-MOVE-2a |
| ITEM-CLIENT-3 | ITEM_MOVE_WIRE1 | client lane |  | ITEM-CLIENT-2; BAGS-1 |
| ITEM-CLIENT-4 | ITEM_MOVE_WIRE1 | client lane |  | ITEM-CLIENT-3; ITEM-MOVE-2b |
| BAGS-USE-1 | BAGS0 | impl, combat review |  | ITEM-USE-1c; BAGS-1 |
| BAGS-GROUND-1 | BAGS0 | hard, persistence review |  | ITEM-MOVE-2b; BAGS-1 |
| BAGS-DEPOT-1 | BAGS0 | hard, persistence review |  | DEPOT-1; INBOX-1a (merged); BAGS-1 |
| BAGS-TRADE-1 | BAGS0 | hard, persistence and security review |  | TRADE-1; BAGS-1 |
| STACK-WIRE-1 | STACK0 | impl, protocol review | yes | ITEM-EQUIP-WIRE-1; BAGS-WIRE-1 |
| STACK-1 | STACK0 | hard (persistence), persistence review |  | ITEM-MOVE-2a; ITEM-MOVE-2b; BAGS-1; GROUND-MOVE-1; the existing migration 0034 `stack_ordinal` (RANGED-0 §6.1.2; no RANGED-1 dependency) |
| STACK-PARITY-1 | STACK0 | impl |  | STACK-1 |
| STARTER-1 | STARTER_BACKPACK0 | hard, persistence and security review |  | this decision; the C2 fence plumbing (`CurrentCharacterItemFence` in `AdmittedSession`); STARTER-CONTENT-1 |
| STARTER-CONTENT-1 | STARTER_BACKPACK0 | **MERGED (#1424).** content |  | this decision |
| ITEM-USE-WIRE-1 | ITEM_USE0 | **MERGED (#1761).** impl, protocol review | yes | ITEM-VIEW-1b |
| ITEM-USE-CLIENT-1 | ITEM_USE0 | client lane (CORE_CLIENT_PACKETS item 6) |  | ATTACK-CLIENT-1; ITEM-USE-WIRE-1; ITEM-USE-1c (offers capability 15) |
| ITEM-SEM-USE | ITEM_USE0 | content |  | ITEM-SEM-2b |
| ITEM-USE-1c | ITEM_USE0 | hard, persistence review. 1a and 1b are MERGED (#2039: DUR-03 `item_use` audit tag 10, migration 0083, `commit_item_use`). 1c is the residue and is IN FLIGHT with the USE-ITEM worker: runtime (cooldowns, requirements, effects), ingress, capability 15 offer, ITEMUSE0-RL-01/-02/-04. Every consumer that needs the use runtime waits for it |  | ITEM-USE-WIRE-1; ITEM-SEM-USE |
| FOOD-REGEN-1 | ITEM_USE0 | spell lane (condition runtime), combat review |  | the GAME-ABILITY-01 condition runtime; ITEM-USE-1c |
| B3-3 | B3 inventory | hard (durable item), persistence review: weight capacity, item weights, the character capacity source, refusal on overweight (B3 is the authoritative brief; B3-1 and B3-2 are merged against it) |  | evidence for item weights and base capacity; ITEM-LOOP-1 entry gate |

### ITEM-TIMED-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (IMBUE_FORGE0, TIMED_ITEM0, TIMED_ITEM0B) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| TIMED-CONTENT-1 | TIMED_ITEM0 | **MERGED (#1699).** content |  | ITEM-SEM-2b |
| TIMED-REPAIR-1 | TIMED_ITEM0 | impl, persistence review |  | NPC-TRADE-1 (the gold fee path) |
| TIMED-RT-1a | TIMED_ITEM0B | **MERGED (#1681).** hard (persistence) |  | #1681 (migration 0054, merged) |
| TIMED-RT-1b | TIMED_ITEM0B | **MERGED (archive record OTV2-20261003-timed-rt-1b).** hard (persistence, fencing) |  | #1681; TIMED-CONTENT-1 |
| TIMED-RT-1c | TIMED_ITEM0B | hard (persistence) |  | TIMED-RT-1b; ITEM-MOVE-2a; ITEM-MOVE-2b; ITEM-USE-1c |
| TIMED-HOUSE-1 | TIMED_ITEM0B | impl, persistence review |  | TIMED-RT-1c; the house-lane item placement (HOUSE-CUSTODY-0) |
| TIMED-FX-1 | TIMED_ITEM0B | hard (combat), combat and determinism review |  | TIMED-RT-1c; COND-1 |
| TIMED-WIRE-1 | TIMED_ITEM0B | impl, protocol review | yes | TIMED-RT-1c |
| TIMED-PARITY-1 | TIMED_ITEM0B | impl |  | TIMED-FX-1 |
| IMBUE-CONTENT-1 | IMBUE_FORGE0 | content |  | ITEM-SEM-USE |
| IMBUE-WIRE-1 | IMBUE_FORGE0 | impl, protocol review | yes | USE-WIRE-V1; ITEM-MOVE-WIRE-1 |
| IMBUE-1 | IMBUE_FORGE0 | hard, persistence and economy review |  | GOLD-FEE-2; ITEM-MOVE-2a |
| IMBUE-RT-1 | IMBUE_FORGE0 | hard (combat), combat review |  | IMBUE-1; ATTACK-1b; SPEED-1 (merged) |
| IMBUE-SCROLL-1 | IMBUE_FORGE0 | hard, persistence review |  | IMBUE-1; NPC-TRADE-1 |
| FORGE-CONTENT-1 | IMBUE_FORGE0 | content |  | ITEM-SEM-USE |
| FORGE-1a | IMBUE_FORGE0 | **MERGED (#1691).** hard, persistence review (value asset) |  | none (migration 0059) |
| FORGE-1b | IMBUE_FORGE0 | hard, persistence and economy review |  | FORGE-1a; FORGE-CONTENT-1; GOLD-FEE-2; ITEM-MOVE-2a |
| FORGE-WIRE-1 | IMBUE_FORGE0 | impl, protocol review | yes | FORGE-1b |
| FORGE-CREATURE-1 | IMBUE_FORGE0 | hard (combat), combat review |  | GAME-AI-01 runtime; FORGE-1b |
| TIER-EFFECT-1 | IMBUE_FORGE0 | hard (combat), combat review |  | FORGE-1b; ATTACK-1b; SPELL-D3 |
| MARKET-TIER-1 | IMBUE_FORGE0 | hard, persistence and economy review |  | MARKET-1; FORGE-1b |

### ITEM-STORAGE-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (DEPOT0, SUPPLY_STASH0, WRITE0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

WRITE-WIRE-1 is not part of the ITEM-STORAGE-1 completion set: it is allocated after MAIL-WIRE-1 and completes with the MAIL-1 family, which keeps ITEM-LOOP-1 (not ITEM-STORAGE-1) as its entry gate.

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| DEPOT-WIRE-1 | DEPOT0 | impl, protocol review | yes | ITEM-VIEW-1b; MAP-LOAD-1; MAP-WIRE-1 |
| DEPOT-1 | DEPOT0 | hard, persistence review |  | HOUSE-CUSTODY-1 (the exclusivity guard, not yet built); ITEM-MOVE-2a (backpack entry deletion by TRANSFER) |
| DEPOT-CONTENT-1 | DEPOT0 | content |  | WO-1 (merged); MAP-LOAD-1 |
| STASH-1 | SUPPLY_STASH0 | hard (persistence), persistence and value review |  | DEPOT-1; BANK-RET-0 (merged, #1748) (the `ECONOMY_LEDGER` retention profile); ITEM-MOVE-2a |
| STASH-WIRE-1 | SUPPLY_STASH0 | impl, protocol review | yes | DEPOT-WIRE-1; STASH-1 |
| STASH-CONTENT-1 | SUPPLY_STASH0 | content |  | MARKET-CONTENT-1 |
| WRITE-CONTENT-1 | WRITE0 | content |  | ITEM-SEM-2b; WO-1 (merged) |
| WRITE-1 | WRITE0 | hard (persistence), persistence, security and privacy review |  | MAIL-TEXT-1; ITEM-MOVE-2b; BAGS-1; HOUSE-RUNTIME-1a, 1b and 1c for house items |
| WRITE-MAP-1 | WRITE0 | impl, determinism review |  | LEVER-1 (overlay state); WRITE-1 |
| WRITE-MOD-1 | WRITE0 | hard (security), security and privacy review |  | WRITE-1; the GM tools decision for its caller |
| WRITE-WIRE-1 | WRITE0 | impl, protocol review | yes | MAIL-WIRE-1 |

### NPC-ECON-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (BANK_FEE0, NPC0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| NPC-CONTENT-1 | NPC0 | **MERGED (#1858).** impl |  | this decision |
| NPC-PLACE-1a | NPC0 | **MERGED (#1940).** content lane: the family, the generator, the held list and the pin refresh a `content/world/` input requires (NPC_PLACE1 §4.4) |  | NPC-CONTENT-1; MAP-BUNDLE-1 |
| NPC-PLACE-1b | NPC0 | **MERGED (#1983).** impl, format and CI-routing review: v4, the compiler lowering, the reader, limits, manifest and parity output, input routing and re-pin; merges main after 1a and re-pins then |  | NPC-PLACE-1a; MAP-BUNDLE-1 |
| NPC-WIRE-1 | NPC0 | **MERGED (#1866).** impl, protocol review | yes | NPC-CONTENT-1 |
| NPC-TALK-1 | NPC0 | **MERGED (#1976).** impl |  | NPC-WIRE-1; NPC-PLACE-1b; MAP-CUTOVER-1b |
| NPC-TRADE-1 | NPC0 | hard, persistence review. The writer is MERGED (#2040); the residue is NPC-TRADE-WIRE-1 and NPC-TRADE-CHARGED-1 below, plus the bank part (BANK-FEE-0, GOLD-FEE-2) |  | NPC-TALK-1; GOLD-FEE-1a (merged); GOLD-FEE-1b |
| NPC-CMD-1b | NPC0 | impl; IN FLIGHT on its own branch: the read-only domain-8 window that supplies the bound catalogue revision |  | NPC-WIRE-1 |
| NPC-TRADE-WIRE-1 | NPC0 | impl, persistence review: wire the trade intent (command 8) to the merged writer |  | NPC-CMD-1b; NPC-TRADE-1 (writer merged, #2040) |
| NPC-TRADE-CHARGED-1 | NPC0 | hard, persistence review: trade charged and fluid items (held now as `ChargedOrFluidPending`, 865 offers) |  | NPC-TRADE-WIRE-1; TIMED-RT-1c |
| NPC-TRAVEL-1 | NPC0; TRAVEL0 (widened: "widened" describes the TRAVEL0 amendment of this one child, it is not a second batch) | hard, persistence review. Acceptance gates: NPC0 and TRAVEL0 are both CANDIDATE, so the child is not allocated until both are ACCEPTED |  | NPC-TALK-1 (merged); GOLD-FEE-1b (merged); DEATH-1 admission consumption (DEATH-1 merged); QUEST-PRED-1 (merged); CHAR-POSITION-1 |
| BANK-RET-0 | BANK0 | **MERGED (#1748).** control plane routes; privacy review | yes | this decision |
| BANK-1 | BANK0 | **MERGED (#1766).** hard, persistence review |  | GOLD-FEE-1b (three coins, change guards; BANK-1 rewrites the same guard functions after it); BANK-RET-0 (merged, #1748) |
| BANK-NPC-1 | BANK0 | impl, content review |  | BANK-1; NPC-CONTENT-1; NPC-TALK-1 |
| GOLD-FEE-2 | BANK_FEE0 | **MERGED (#1781).** hard, persistence review |  | BANK-1; BANK-RET-0 (merged, #1748); GOLD-FEE-1b; for BUY and travel, NPC-TRADE-1 and NPC-TRAVEL-1 (whichever lands later takes the bank part) |

### TRADE-PLAYER-1, MAIL-1, MARKET-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (MAIL0, MARKET0, PLAYER_TRADE0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| TRADE-WIRE-1 | PLAYER_TRADE0 | impl, protocol review | yes | ITEM-VIEW-1b; VIS-2 |
| TRADE-1 | PLAYER_TRADE0 | hard, persistence and security review |  | TRADE-WIRE-1; ITEM-MOVE-2a |
| MAIL-CONTENT-1 | MAIL0 | content |  | WO-1 (merged); MAP-LOAD-1 |
| MAIL-TEXT-1 | MAIL0 | hard, security and privacy review |  | ITEM-MOVE-1; MAIL-CONTENT-1 |
| MAIL-1 | MAIL0 | hard, persistence review |  | INBOX-1a (merged); ITEM-MOVE-2a; MAIL-TEXT-1; MAIL-CONTENT-1 |
| MAIL-WIRE-1 | MAIL0 | impl, protocol review | yes | ITEM-VIEW-1b; MAP-WIRE-2; MAIL-1 |
| MAIL-PARCEL-1 | MAIL0 | hard, persistence review |  | MAIL-1; BAGS-1; BAGS-DEPOT-1 (Inbox trees) |
| MAIL-SYSTEM-1 | MAIL0 | hard, persistence review |  | MAIL-1; HOUSE-1b |
| MARKET-CONTENT-1 | MARKET0 | content |  | none |
| MARKET-RET-0 | MARKET0 | **MERGED (#1748).** control plane routes; privacy review |  | this decision |
| INBOX-1a | MARKET0 | **MERGED (#1785).** hard, persistence review |  | none (HOUSE-CUSTODY-1 merged) |
| INBOX-1b | MARKET0 | hard, persistence review |  | INBOX-1a (merged); DEPOT-1 |
| MARKET-1 | MARKET0 | hard, persistence, economy and security review |  | BANK-1; GOLD-FEE-2; MARKET-RET-0; INBOX-1a (merged); MARKET-CONTENT-1 |
| MARKET-WIRE-1 | MARKET0 | impl, protocol review | yes | DEPOT-WIRE-1; MARKET-1 |

### QUEST-RT-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (QUEST_GATE0, QUEST_STATE0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| QUEST-GATE-1 | QUEST_GATE0 | hard, security review |  | QUEST-PRED-1; MAP-LOAD-1; MAP-WIRE-2; QUEST-CONTENT-2 |
| QUEST-TRIGGER-1 | QUEST_GATE0 | hard, persistence review |  | QUEST-STATE-1; QUEST-GATE-1 |
| NPC-QUEST-1 | QUEST_GATE0 | hard, persistence review |  | NPC-TALK-1; QUEST-STATE-1; QUEST-PRED-1; CHEST-1 (merged); GOLD-FEE-2 (the bank part) |
| QUEST-XP-1 | QUEST_GATE0 | **MERGED (#1724).** hard, persistence review |  | QUEST-STATE-1; CHAR-REV-SEQ-1 |
| QUEST-XP-ADMISSION-1 | QUEST_GATE0 | hard, persistence review: request pending quest XP with the session binding; today nothing calls `request_pending_quest_experience`, so the obligation is never paid (ARCH-PROGRESSION-SOURCE-0 §1.4, §1.5; QUEST-GATE-0 §5.5; PROGRESSION_SOURCE_PACKETS §2.3) |  | PROGRESSION-OWNER-1 (task record archived); QUEST-XP-1 |
| QUEST-LOG-WIRE-1 | QUEST_GATE0 | **MERGED (#1741).** impl, protocol review | yes | QUEST-PRED-1; QUEST-LOWER-1 |
| QUEST-CONTENT-2 | QUEST_GATE0 | **MERGED (#1764).** content |  | QUEST-LOWER-1; MAP-BUNDLE-1 |
| NPC-QUEST-CONTENT-1 | QUEST_GATE0 | NPC content lane |  | NPC-CONTENT-1; QUEST-LOWER-1 |
| CHAR-REV-SEQ-1 | QUEST_STATE0 | **MERGED (archive record OTV2-20261003-char-rev-seq1).** hard, persistence and concurrency review |  | this decision |
| QUEST-STATE-1 | QUEST_STATE0 | **MERGED (#1684).** hard, persistence review |  | this decision; CHAR-REV-SEQ-1 |
| QUEST-ACCOUNT-1 | QUEST_STATE0 | hard, persistence review |  | QUEST-STATE-1; a World product profile family source (ADR-0010) on `main` |
| QUEST-PRED-1 | QUEST_STATE0 | **MERGED (#1723).** impl |  | QUEST-STATE-1 |
| QUEST-CONTENT-1 | QUEST_STATE0 | content |  | this decision |
| QUEST-LOWER-1 | QUEST_STATE0 | **MERGED (#1727).** content |  | QUEST-STATE-1 (the catalogue type) |
| CHEST-RANDOM-1 | QUEST_STATE0 | CHEST lane, hard |  | this decision |

### MAGIC-ITEMS-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (RUNE_USE0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| RUNE-WIRE-1 | RUNE_USE0 | impl, protocol review | yes | ITEM-USE-WIRE-1 |
| RUNE-CONTENT-1 | RUNE_USE0 | content |  | ITEM-SEM-USE |
| RUNE-1 | RUNE_USE0 | hard, persistence review |  | ITEM-USE-1c; RUNE-WIRE-1 |
| RUNE-CAST-1 | RUNE_USE0 | spell lane, combat review |  | RUNE-1; SPELL-TARGET-1 (ATTACK-0); VIS-2; CHAR-BUILD-1 (merged); COND-1 (§3.5 residual) |
| RUNE-CONJ-1 | RUNE_USE0 | hard, persistence, combat and protocol review |  | RUNE-1; the instant cast composition (SPELL-D4) |
| FIELD-1 | RUNE_USE0 | hard (combat), combat review |  | COND-1; RUNE-CAST-1 |
| FIELD-WIRE-1 | RUNE_USE0 | impl, protocol review | yes | VIS-2; FIELD-1 |

### PROG-ADV-1

Acceptance gate: satisfied. PROFICIENCY1, PROFICIENCY1B, PROF_EFFECT0, WHEEL0 and WHEEL_GEM0 (with SKILLS0, PROFICIENCY0, PROF_WIRE0 and WHEEL_GEM0A) were ACCEPTED by the owner on 2026-10-15 ("3 tak", ACCEPT-BATCH-2026-10-15 §6), effective on protected integration of that acceptance; each child still needs its own #162 allocation and its named prerequisites (status of main@ab7d51ae6, 2026-10-13, refreshed 2026-10-15).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| PROF-SHAPE-CONTENT-1 | PROFICIENCY1, PROFICIENCY1B | **MERGED (#1670).** content (one batch for both briefs) |  | PROF-CONTENT-1 |
| PROF-SHAPE-1b | PROFICIENCY1B | **Slice MERGED (#1704: migration 0060, the perk modification dust SPEND shape).** Residue: the orb BURN shape, ORB_RANK and the DUR03-RL-01/02-PROF rows are PROF-ORB-BURN-1 (orb ranks stay NOT_ADMITTED until then). hard (persistence), persistence, economy and determinism rev |  | PROF-1; PROF-2; FORGE-1a (the dust balance and ledger; #1685 and FORGE-1a merged first, ARCH_BATCH_TIMED_FORGE_PROF_PACKETS §1.4); PROF-SHAPE-CONTENT-1 (merged) |
| PROF-ORB-BURN-1 | PROFICIENCY1B | hard (persistence), persistence, economy and determinism review: the orb BURN shape, ORB_RANK and the `DUR03-RL-01/02-PROF` rows (residue of PROF-SHAPE-1b; PROFICIENCY-1B §7.1, §7.2, §9) | yes | PROF-SHAPE-1b (slice merged); FORGE-1a (merged); acceptance of PROFICIENCY1B (ACCEPTED 2026-10-15, ACCEPT-BATCH-2026-10-15 §6) |
| PROF-SHAPE-WIRE-1 | PROFICIENCY1B | hard (protocol), protocol review | yes | PROF-WIRE-1; PROF-SHAPE-1b; PROF-ORB-BURN-1 |
| PROF-CONTENT-2 | PROF_EFFECT0 | content |  | PROF-CONTENT-1 |
| PROF-PARITY-1 | PROF_EFFECT0 | impl |  | PROF-2 |
| SPELL-WHEEL-GATE-1 | WHEEL0 | impl, spell review |  | this decision |
| W-R | WHEEL0 | **MERGED (migration 0070 (W-R carry)).** impl, content review |  | this decision |
| W-1 | WHEEL0 | **MERGED (archive record WHEEL-W1 and migration 0070).** hard, persistence review |  | W-R; CHAR-REV-SEQ-1 |
| W-2 | WHEEL0 | impl, protocol review | yes | W-1 |
| W-FX-1 | WHEEL0 | hard, combat review |  | W-1; the vitals owner; the damage-mitigation owner (the GAME-ABILITY-01 magnitude-side mitigation/resistance/absorb stage) for resistance; CONDITIONS-0 for timed effects |
| W-3 | WHEEL0 | client owner: Wheel window |  | W-2 |
| GEM-R | WHEEL_GEM0 | impl, content review |  | W-R |
| GEM-CONTENT-1 | WHEEL_GEM0 | content |  | none |
| GEM-1 | WHEEL_GEM0 | hard, persistence and economy review |  | W-1; GEM-R; GOLD-FEE-2; WHEEL-GEM-0A (its §5 ordering rule; the WHEEL_GEM0 brief is ACCEPTED 2026-10-15, ACCEPT-BATCH-2026-10-15 §6, so the acceptance gate is satisfied) |
| GEM-VESSEL-1 | WHEEL_GEM0 | hard, persistence review |  | W-1; GEM-1 |
| GEM-WIRE-1 | WHEEL_GEM0 | impl, protocol review | yes | W-2; GEM-1; GEM-VESSEL-1 |
| GEM-FX-1 | WHEEL_GEM0 | hard, combat review |  | W-FX-1; GEM-VESSEL-1 |
| GEM-CRUSH-1 | WHEEL_GEM0 | impl, persistence review |  | GEM-1; ITEM-USE-0's item-target arm |
| W-3-ATELIER | WHEEL_GEM0 | client owner: Atelier tabs (the WHEEL_GEM0 brief names it W-3; renamed here because WHEEL0 has its own W-3, the Wheel window) |  | GEM-WIRE-1 |

### NPC-WORLD-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (TRAVEL0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| TRAVEL-CONTENT-1 | TRAVEL0 | impl (NPC content lane), content review |  | NPC-CONTENT-1; QUEST-CONTENT-1 for gate and discount tracks |
| NPC-TRAVEL-2 | TRAVEL0 | impl, persistence review |  | NPC-TRAVEL-1; PREM-1; PREM-3 |
| NPC-ACTOR-1 | NPC_BEHAVIOUR0 | impl, determinism review |  | NPC-PLACE-1b; CREATURE-MOVE-1; SPEED-1 (merged); NPC-WIRE-1 |
| NPC-TALK-2 | NPC_BEHAVIOUR0 | impl |  | NPC-TALK-1 |
| NPC-VIS-1 | NPC_BEHAVIOUR0 | **MERGED (archive record OTV2-20261003-npc-vis-1).** impl, protocol review | yes | VIS-2 |
| NPC-VOICE-1 | NPC_BEHAVIOUR0 | impl |  | CHAT-1; CHAT-1b-2b; NPC-ACTOR-1 |
| NPC-CONTENT-2 | NPC_BEHAVIOUR0 | content |  | NPC-CONTENT-1 |

### CRAFT-1

Acceptance gate: no child in this graph is allocated until its owning decision (CRAFT1; CANDIDATE, 2026-10-15) is ACCEPTED.

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| CRAFT-CONTENT-1 | CRAFT1 | content: the `item_conversion` family, schema, generator and CI check |  | this decision |
| CRAFT-RT-1 | CRAFT1 | hard, determinism and persistence review: conversion engine with the USED, TARGET and INGREDIENT roles, `ConversionRngPurpose`, conversion receipt and guard arm, the C4b corpse target | yes | CRAFT-CONTENT-1; ITEM-LOOP-1; D3-4 (corpse Ground lock order for C4b); ITEM-USE-1c (ingress for the used item alone, UseIntentV1 field 2); WORLDINT-USE-1 (use-with ingress on a map item or map object, UseWithV1 arms 3 and 4); the CRAFT1 C3a admission (DUR-03 §39.3 pointer and the C3a CRAFT rows, including the RL-07 event, payload and envelope byte rows, added at acceptance) |
| CRAFT-FISH-1 | CRAFT1 | impl: fishing (C4, the reference values of C4a) with the SKILLS-0 hook |  | CRAFT-RT-1; SKILLS-0 (Fishing skill and try hook); W2b (live skill-try accumulator and training formula) |
| CRAFT-DIG-1 | CRAFT1 | impl: digging on the MAP-OVERLAY-1a volatile layer, `revert_ms` from the row |  | CRAFT-RT-1; WORLDINT-USE-1 (use-with ingress on a map item or map object, with its reach and security gates); MAP-OVERLAY-1a (merged, #1800; the per-channel volatile overlay; the unsplit MAP-OVERLAY-1 is superseded) |
| CRAFT-WIRE-1 | CRAFT1 | impl, protocol review: UseWithV1 arm 6 `carried_item`, capability `ITEM_USE_CARRIED_TARGET_V1` (number on #162), codec, boundary test under ITEMUSE0-RL-03, ingress into CRAFT-RT-1 (CRAFT1 §4.1) | yes | CRAFT-RT-1; ITEM-USE-1c (UseIntentV1 ingress the arm extends) |
| CRAFT-NPC-1 | CRAFT1 | hard, persistence and economy review: the `npc_exchange` family and compiler, the EXCHANGE side of the NPC-0 trade writer, the `0023` and side-check migration (CRAFT1 C8) | yes | CRAFT-CONTENT-1; NPC-TRADE-1 (the trade writer it extends); NPC-CONTENT-1 (NPC catalogue); the CRAFT1 C8 DUR-03 rows (added at acceptance); gated rows held until NPC-QUEST-1 |
| CRAFT-NPC-WIRE-1 | CRAFT1 | impl, protocol review: `NPC_TRADE_SIDE_EXCHANGE`, `NpcTradeWindowV1` field 4, capability `NPC_EXCHANGE_V1` (number on #162), CRAFT1-RL-01..03 (CRAFT1 §4.2) | yes | CRAFT-NPC-1; NPC-TRADE-WIRE-1; NPC-CMD-1b |
| CRAFT-FOOD-1 | CRAFT1 | spell lane, combat review: gourmet dish conditions with source kind `item`, the shared 600 s cooldown (CRAFT1 C7) |  | ITEM-USE-1c; FOOD-REGEN-1; COND-1; COND-DUR-1 |

### HIRE-1

Acceptance gate: no child in this graph is allocated until its owning decision (HIRE1; CANDIDATE, 2026-10-15) is ACCEPTED.

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| HIRE-DATA-1 | HIRE1 | hard, persistence review: hireling rows, lamp mint and retire, house-loss return, session fence |  | HOUSE-1b |
| HIRE-NPC-1 | HIRE1 | impl: hireling NPC entity and Apprentice content |  | HIRE-DATA-1; NPC-TALK-1 |
| HIRE-JOBS-1 | HIRE1 | hard, economy review: Banker, Trader, Steward, Cook | yes | HIRE-NPC-1; NPC-ECON-1; STASH-1 |
| HIRE-WIRE-1 | HIRE1 | hard, protocol review: hireling outfit window, dress, name and sex change | yes | HIRE-DATA-1 |
| HIRE-STORE-1 | HIRE1 | hard, security and economy review: Store delivery consumer; waits for the Platform Store contract (Platform #1625) | yes | HIRE-DATA-1; HIRE-NPC-1; HIRE-JOBS-1; HIRE-WIRE-1; PREMIUM-1; Platform Store contract |

### MONSTER-META-1, EVENTS-RAID-1, EVENTS-WORLD-1, ARENA-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (BOSS_RAID0, MONSTER_D15) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| MONSTER-D15B-1 | MONSTER_D15 | content |  | this decision |
| RAID-CONTENT-1 | BOSS_RAID0 | content |  | this decision; the encounter admission (E4) |
| RAID-1 | BOSS_RAID0 | hard, persistence and concurrency review |  | RAID-CONTENT-1; CHAT-1; CHAT-1b-2b |
| BOSS-1 | BOSS_RAID0 | hard, persistence review |  | this decision |
| BOSS-ROOM-1 | BOSS_RAID0 | hard, security and durability review |  | BOSS-1; SCOPE-HANDOFF-1; the encounter runtime (§6.5) |
| BOSS-REWARD-1 | BOSS_RAID0 | hard, persistence, economy and security review |  | BOSS-1; D3-3; PARTY-XP-1 |
| BOSSTIARY-1 | BOSS_RAID0 | hard, persistence review |  | CHAR-REV-SEQ-1; BOSS-REWARD-1 (the credited set); GOLD-FEE-1b; GOLD-FEE-2 for the fee's bank part |
| BOSS-WIRE-1 | BOSS_RAID0 | impl, protocol review | yes | BOSSTIARY-1; BOSS-REWARD-1; RAID-1 |

### PARTY-1, PARTY-XP-1, PVP-*

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| PARTY-1 | PARTY_PVP0 | hard, persistence, security and privacy review |  | this decision; CHAT-2 |
| PARTY-XP-1 | PARTY_PVP0 | hard (combat), combat review |  | PARTY-1; the D118 XP lane; D3-4 |
| PARTY-CHAT-1 | PARTY_PVP0 | impl, security review |  | PARTY-1; CHAT-2 |
| PVP-1a | PARTY_PVP0 | **MERGED (#2032: migration 0081, the PvP state tables, the deadline write-ahead and the admission read, and a bounded World cleanup function that no node job calls yet).** hard, persistence and security review |  | DEATH-1 (merged) |
| PVP-1b | PARTY_PVP0 | residue of PVP-1 (PARTY_PVP0 line 33): skull evaluation in the death transaction (the points and revenge-mark inserts), the node World cleanup job that calls the 1a cleanup function, and the `rulesets/pvp/` and `rulesets/party/` rows with `pvp_type` `OPTIONAL` for the first World (§6, §9). Wave 2. hard, persistence and security review |  | PVP-1a (merged); DEATH-1 (merged); PARTY-1 |
| PVP-RT-1 | PARTY_PVP0 | hard (combat), combat and security review. #2032 (archive record `PVP-RT-1.md`) merged only PVP-1a, the state tables and the deadline write-ahead; the runtime wiring stays here |  | ATTACK-1b; COND-1; PVP-1a (merged); PVP-1b |
| PVP-DEATH-1 | PARTY_PVP0 | hard (persistence), persistence review |  | DEATH-1; DEATH-3; PVP-1a (merged); PVP-1b |
| PVP-BLOCK-1 | PARTY_PVP0 | impl, movement review |  | PVP-RT-1; SPEED-1 (merged) |
| PVP-WIRE-1 | PARTY_PVP0 | impl, protocol review | yes | PARTY-1; PVP-RT-1; VIS-2; ATTACK-WIRE-1 |

### GUILD-*

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| GUILD-1 | GUILD0 | hard, persistence and security review |  | GUILD-RET-0 (merged, #1748); BANK-1; PREM-WIRE-1 |
| GUILD-RET-0 | GUILD0 | **MERGED (#1748).** control plane routes; privacy review |  | this decision |
| GUILD-BANK-1 | GUILD0 | hard, persistence and economy review |  | GUILD-1; BANK-1 |
| GUILDHALL-1 | GUILD0 | hard, persistence, economy and security review |  | GUILD-1; GUILD-BANK-1; HOUSE-1b; HOUSE-ACL-1 |
| GUILD-WIRE-1 | GUILD0 | impl, protocol review | yes | GUILD-1; GUILD-BANK-1; GUILDHALL-1 for the guildhall variants; HOUSE-WIRE-1 |
| GUILD-CHAT-1 | GUILD0 | impl, security review |  | GUILD-1; CHAT-2 |

### VIP-*

Acceptance gate: no child in this graph whose Brief is CANDIDATE (VIP0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| VIP-1 | VIP0 | impl; persistence, protocol, privacy |  | VIP-0; PARTY-1 |
| VIP-2 | VIP0 | hard; security, privacy |  | VIP-1; CHAT-2 |

### CHAT-1, CHAT-2

Acceptance gate: no child in this graph whose Brief is CANDIDATE (CHAT0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| CHAT-1 | CHAT0 | impl, protocol review; MERGED (CHAT-WIRE-1 on main@ab7d51ae6) | yes | this decision |
| CHAT-1b-2b | CHAT0 | hard, protocol review: resource row `CHAT0-RL-11` (undelivered lines per session) and the connection arm, wire lane (CORE_LOOP_PACKETS §1); no task record is on main, a worker confirms against main whether CHAT-WIRE-1 covered it | yes | CHAT-1 |
| CHAT-2 | CHAT0 | hard, security and privacy review |  | CHAT-1; CHAT-1b-2b (NOT merged: no task record and no resource row yet; Wave 1); CHAT-0 re-review and ACCEPTED (gate) |

### HOUSE-*

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| SCOPE-HANDOFF-1 | HOUSE_RUNTIME0 | **MERGED (#1782).** hard, security and durability review |  | this decision |
| HOUSE-RUNTIME-1a | HOUSE_RUNTIME0 | hard, security and persistence review |  | SCOPE-HANDOFF-1; HOUSE-CUSTODY-1 (merged) |
| HOUSE-RUNTIME-1b | HOUSE_RUNTIME0 | hard |  | HOUSE-RUNTIME-1a; MAP-LOAD-1 |
| HOUSE-RUNTIME-1c | HOUSE_RUNTIME0 | hard |  | HOUSE-RUNTIME-1b; CHAR-POSITION-1 |
| HOUSE-VIEW-1 | HOUSE_RUNTIME0 | impl, protocol review |  | HOUSE-RUNTIME-1a, 1b and 1c; MAP-WIRE-1 children |
| HOUSE-ITEM-WIRE-1 | HOUSE_RUNTIME0 | protocol decision first, then impl with protocol and securit | yes | its own wire decision; HOUSE-RUNTIME-1a, 1b and 1c; HOUSE-VIEW-1; ITEM-MOVE-WIRE-1 |
| HOUSE-1a | HOUSE_OWN0 | hard, persistence, economy and security review |  | BANK-1; HOUSE-RUNTIME-1a, 1b and 1c; PREM-WIRE-1 (SOCIAL_MAP_PACKETS §0.1) |
| HOUSE-1b | HOUSE_OWN0 | hard, persistence, economy and security review |  | HOUSE-1a; INBOX-1a (merged); MAP-OVERLAY-1c |
| HOUSE-ACL-1 | HOUSE_OWN0 | hard, security review |  | HOUSE-1a |
| HOUSE-WIRE-1 | HOUSE_OWN0 | impl, protocol review | yes | HOUSE-1b; HOUSE-ACL-1 |

### HUNT-1, RANK-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (ANALYSERS0, HIGHSCORES0, PREY0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| PREY-CONTENT-1 | PREY0 | content |  | this decision |
| PREY-1 | PREY0 | hard, persistence review |  | CHAR-REV-SEQ-1; PREY-CONTENT-1 |
| PREY-FEE-1 | PREY0 | hard, persistence and economy review |  | PREY-1; GOLD-FEE-1b |
| PREY-EFFECT-1 | PREY0 | combat lane, combat review |  | PREY-1; the XP lane (D118); SPELL-TARGET-1 (ATTACK-0) |
| TASKBOARD-1 | PREY0 | hard, persistence review |  | PREY-1; CHAR-REV-SEQ-1 |
| TASKBOARD-DELIVERY-1 | PREY0 | hard, persistence and economy review |  | TASKBOARD-1; ITEM-USE-1c (item selection) |
| TASKSHOP-1 | PREY0 | hard, persistence and economy review |  | TASKBOARD-1 |
| PREY-WIRE-1 | PREY0 | impl, protocol review | yes | PREY-1; TASKBOARD-1 |
| ANALYSER-WIRE-1 | ANALYSERS0 | **MERGED (archive record OTV2-20261003-analyser-wire1).** impl, protocol review | yes | this decision |
| ANALYSER-EMIT-1 | ANALYSERS0 | combat and item lanes, determinism review |  | ANALYSER-WIRE-1; ATTACK-1b; D3 corpse mint; the consuming item uses |
| ANALYSER-CLIENT-1 | ANALYSERS0 | client lane (client owner) |  | ANALYSER-WIRE-1 |
| HS-1 | HIGHSCORES0 | impl, persistence review |  | CHAR-BUILD-1b (merged) (build writer); CHARM-1 (Charm Points); account achievements (`0021`) |
| HS-WIRE-1 | HIGHSCORES0 | impl, protocol review | yes | HS-1 |

### DAILY-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (DAILY_REWARD0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| DAILY-CONTENT-1 | DAILY_REWARD0 | content |  | WO-1 (merged); EXERCISE-CONTENT-1; RUNE-USE content |
| DAILY-1 | DAILY_REWARD0 | hard (persistence), persistence and value review |  | MARKET-1 (`CharacterInbox`); PREY-1 (wildcard balance); the World reset epoch (ADR-0021) |
| DAILY-BIND-1 | DAILY_REWARD0 | impl |  | DAILY-1 |
| DAILY-WIRE-1 | DAILY_REWARD0 | impl, protocol review | yes | DAILY-1; WORLDINT-USE-1 |

### ACH-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (ACHIEVEMENT0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| ACH-QUEST-1 | ACHIEVEMENT0 | impl, persistence review |  | QUEST-STATE-1 |
| ACH-ENC-1 | ACHIEVEMENT0 | hard (persistence), persistence review |  | ENC-OUTCOME-1 |
| ACH-COUNTER-1 | ACHIEVEMENT0 | impl, persistence review |  | the XP receipt; SKILLS-0's build receipts |
| ACH-NOTIFY-1 | ACHIEVEMENT0 | **MERGED (archive record OTV2-20261003-ach-notify1).** impl, protocol review | yes | the FND-02 state-domain path |
| ACH-COVERAGE-1 | ACHIEVEMENT0 | content |  | none |

### GM-1

Source: ARCH-LIVE-READINESS-0 §1 (OPS-GM-01). Case management, appeals and rollback are not a GM tool. No value-creating staff action exists in either phase (§1 ruling 1).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| OPS-GM-AUDIT-1 | ARCH_LIVE_READINESS | hard (authority, audit), security review. ALPHA, first. `staff_action` event, signed requests, `oteryn-game-ops` | yes | none |
| OPS-GM-SANCTION-2 | ARCH_LIVE_READINESS | hard (authority), migration `character_sanctions`, kick and mute. ALPHA | yes | OPS-GM-AUDIT-1; LOGOUT-WIRE-1 |
| OPS-GM-REPORT-3 | ARCH_LIVE_READINESS | impl. ALPHA: the issue form first, the `reports` commands after WRITE-MOD-1 |  | WRITE-MOD-1 for the commands |
| OPS-GM-ROSTER-4 | ARCH_LIVE_READINESS | hard (authority), migration `game_staff_roles`. EXT |  | OPS-GM-AUDIT-1 |
| OPS-GM-STAFF-5 | ARCH_LIVE_READINESS | impl, protocol review, native client staff panel. EXT | yes | OPS-GM-ROSTER-4 |
| OPS-GM-PREPORT-6 | ARCH_LIVE_READINESS | hard (privacy), migration `game_player_reports`. EXT | yes | OPS-GM-AUDIT-1 |
| OPS-GM-P1 | ARCH_LIVE_READINESS | Platform proposal (account bans and signed kick requests); binds nothing in Game until Platform accepts; routed by the control plane. The owner of bans is unassigned until then |  | Platform's own acceptance |
| ARCH-CONTROL-ANCHOR-0 | ARCH_LIVE_READINESS | architecture decision, EXT gate: no external player before it is delivered |  | ARCH-ALPHA-OPS-0 U7 |

GM-1 is complete only when every Game child above is complete; OPS-GM-P1 and ARCH-CONTROL-ANCHOR-0 are not Game work and gate only the external phase.

### SEC-1

Acceptance gate: no child in this graph whose Brief is CANDIDATE (SEC_CLIENT01) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| SEC-REL-1 | SEC_CLIENT01 | hard (security), security review |  | PROD-COMPAT-01's release train |
| SEC-CHAL-1 | SEC_CLIENT01 | hard (protocol, security), protocol and security review | yes | SEC-REL-1; ADMIT-CAP-1 |
| SEC-TELEM-1 | SEC_CLIENT01 | hard (protocol, privacy), protocol and privacy review | yes | ANL-01; DATA-PRIVACY-01's disclosure (production collection gate, §5.1) |
| SEC-DETECT-1 | SEC_CLIENT01 | impl, security and privacy review |  | SEC-TELEM-1; ANL-03 |
| SEC-CHAL-2 | SEC_CLIENT01 | impl, security review |  | SEC-CHAL-1 |

### ADMIT (World capabilities)

Acceptance gate: no child in this graph whose Brief is CANDIDATE (ADMIT0) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

| Child | Brief | Class | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| ADMIT-CAP-1 | ADMIT0 | hard (authority, protocol), protocol and security review | yes | the numbers of `MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS`, `WORLD_INTERACTION_V1` reserved on #162 by their wire children |
| ADMIT-CAP-2 | ADMIT0 | impl, determinism review |  | ADMIT-CAP-1; FIELD-2 |

### Supporting prerequisite briefs (children that other batches name as prerequisites)

Acceptance gate: no child in this graph whose Brief is CANDIDATE (A13, ACTION_INTEGRATION, B3, CHARACTER_APPEARANCE, D3, DONOR_IDENTITY, FIRST_PLAYER_DEATH, ITEM_IDENTITY, MARKET_COINS0, PREMIUM_ACTIVATION, PREMIUM_DELIVERY0, PROFICIENCY0, STANCE0, VIS, WORLD_OBJECT) is allocated until its owning decision is ACCEPTED (status of main@ab7d51ae6, 2026-10-13).

These briefs define children that rows above depend on (several are CANDIDATE and carry the acceptance gate of the status list above); each child is its own batch with the gate shown. Names with letter suffixes in the packets (for example HOUSE-1a/1b) supersede the unsplit names in the original brief tables above.

| Child | Brief | Scope / class (abridged) | Reg | Depends on (from the brief) |
|---|---|---|---|---|
| CHAR-BUILD-1 | A13 | **MERGED (#1393 (1a, migration 0030) and the 1b writer, archive records OTV2-20260930-char-build-1 and -1b; the residues are W2b, DAWNPORT-1, DEATH-ML-LOSS-1 and LOOP-PROGRESS-1).** Next free migration at allocation, writer and reconcile: the table, the receipt kind, the |  | this decision; `0018`-`0020` |
| DAWNPORT-1 | A13 | Dawnport content and the vocation-choice interaction |  | CHAR-BUILD-1 (merged) |
| DEATH-ML-LOSS-1 | A13 | The flush before a death and the death receipt's build fields (§4.6) |  | CHAR-BUILD-1 (merged), DEATH-1 |
| APP-1 | CHARACTER_APPEARANCE | Durable selected appearance and the AccountUnlock set; validation §4.3; fence |  | Character authority; progression readiness migration numbering |
| APP-2 | CHARACTER_APPEARANCE | Outfit and mount definitions with look types, starter flags, palette |  | content pipeline; #1025 loader follow-up |
| APP-3 | CHARACTER_APPEARANCE | Change command and actor appearance state |  | Combat child E protocol lane; registry lease |
| APP-4 | CHARACTER_APPEARANCE | Client outfit window and rendering of other actors |  | APP-3; client owner |
| GOLD-FEE-1a | GOLD_FEE_BOUNDARY | **MERGED (archive record OTV2-20260930-gold-fee-1a).** Gold coins (`i3031`, worth 1) only: migration 0023 (D173) with the fee record, BURN lines |  | this decision accepted |
| GOLD-FEE-1b | GOLD_FEE_BOUNDARY | **MERGED (archive record OTV2-20260930-gold-fee-1b).** Stack maximum 100 for the three coins (`i3031`, `i3035`, `i3043`; the item-authoring conte |  | 1a; must land before CHARM-6 |
| CHARM-6 | GOLD_FEE_BOUNDARY | Charm unassign (and later reset, by amendment) composing the fee burn. Its migration repla |  | 1a, 1b, D170 |
| D3-1 | D3 corpse | **MERGED (#1213).** Migration: two additive nullable columns on the corpse's own `game_item_mint_receipts` row |  | This decision |
| D3-2 | D3 corpse | **MERGED (archive record OTV2-20260929-d3-2-corpse-loot).** `combat/death_reward.rs`: mint the corpse first (`CORPSE_MATERIALIZATION` cause), then min |  | D3-1 |
| D3-3 | D3 corpse | **MERGED (#1215).** GAME-ABILITY/Combat: runtime per-attacker damage accumulation for a live creature actor (c |  | none (parallel to D3-1/D3-2) |
| D3-4 | D3 corpse | **MERGED (archive record OTV2-20260929-d3-4-corpse-transfer).** Migration + `durability/item_transfer.rs`: admit TRANSFER source `Container(parent=corpse) |  | D3-1 |
| D3-5 | D3 corpse | **MERGED (archive record OTV2-20260929-d3-5-corpse-pickup).** `combat/pickup.rs` (B3-2 extension): a corpse-container pickup request variant wired to D3 |  | D3-4 |
| D3-6 | D3 corpse | **MERGED (archive record OTV2-20260929-d3-6-corpse-decay).** `foundation/owner_timer.rs`: a decay `Family`, scheduled at corpse-MINT commit from `mater |  | D3-1, D3-2 |
| D3-7 | D3 corpse | **MERGED (#1830).** Content: `i00005801` revision (materializable, stack class, container capacity 16, no `con |  | none (parallel); D3-2/D3-6 need it merged before their own tests can use real content |
| DEATH-0 | DEATH-0, FIRST_PLAYER_DEATH | **MERGED (#1264).** (migration 0016, no writer) The migration (§3.1-§3.4: death receipts, blessings, pending respawns), guard changes and |  | this decision; also (FIRST_PLAYER_DEATH): Death receipt and Character migration decision (§4.3 storage prerequisite), then its migra (architecture decision; Character persistence owner) |
| DEATH-1 | DEATH-0, FIRST_PLAYER_DEATH | **MERGED (#1278).** `commit_character_death` and `reconcile_character_death`, the calculator change, pending-r |  | DEATH-0 (merged); progression readiness; also (FIRST_PLAYER_DEATH): Calculator change (§4.3.1) and the durable Character death transaction (DEATH-0 (merged); progression readiness; P03) |
| DEATH-2 | FIRST_PLAYER_DEATH | **MERGED (#1742).** Death trigger, occurrence and respawn in the Channel owner; removal of the D54 floor |  | spell P3b-2 vitals; AI-4 |
| DEATH-3 | FIRST_PLAYER_DEATH | The death item workflow: drops through DUR-03 TRANSFER, the amulet through DUR-03 destroy, |  | DUR-03 TRANSFER (B3) and destroy; container admission for backpacks |
| DEATH-4 | FIRST_PLAYER_DEATH | Blessing state and the NPC blessing service |  | NPC service owner |
| STANCE-0 | STANCE0 | **MERGED (archive record OTV2-20260929-stance0-character-stance-persistence).** Migration (`game_character_stance`, `game_character_stance_receipts`), guard changes, immu |  | this decision |
| STANCE-1 | STANCE0 | **MERGED (`durability/character_stance.rs` `commit_character_stance` on main).** `commit_character_stance`, `reconcile_character_stance`, slot load at admission and respaw |  | STANCE-0; progression readiness |
| WO-1 | WORLD_OBJECT | **MERGED (#1169).** Schema, validator and census for `content/world/terrain/` and `content/world/objects/`, re |  | this decision |
| WO-2 | WORLD_OBJECT, ITEM_IDENTITY, DONOR_IDENTITY | Population: family keys by the D93 rule, facts with provenance, Item `routed_to` pointers; |  | WO-1 (merged); the item re-pin; B1b for donor ids; also: Family keys on the Tibia numbering (ITEM-ID-1); Family keys for the 138 routed donor ids (WO-1 (merged), B1b) |
| WO-3 | WORLD_OBJECT | The optional typed `presentation` reference on LocalObject states (`LocalObjectStateDefini |  | WO-2 |
| VIS-1 | VIS (RL11) | **MERGED (#1153).** (pure interest-set logic; VIS-3 composes on it) Server interest set: area, floors, ceiling and ordering, settings; rows registered |  | this decision |
| VIS-2 | VIS (RL11) | **MERGED (#1392).** The new `WORLD_SPATIAL` schema revision, the `WORLD_SPATIAL_ENTITIES` capability, codec, c | yes | VIS-1 (merged); protocol lane |
| PREM-1 | PREMIUM_ACTIVATION | **MERGED (#1391 and #1678 (1a, 1b)).** Premium entitlement consumer fence and surface policies (§4.1), and the snapshot client of |  | `PROD-ENTITLEMENTS-01` consumer contract; FND-04 admission; PREMIUM-DELIVERY-0 (#1369) |
| PREM-P | PREMIUM_ACTIVATION, PREMIUM_DELIVERY0 (Oteryn-Platform) | Platform producer: Premium entitlement with operator or test grants, no payment, and the s |  | Platform producer contract; PREMIUM-DELIVERY-0; Platform's own acceptance of PREMIUM_DELIVERY0 §3-§5 |
| PREM-2b+PREM-5 | PREMIUM_ACTIVATION (carries mechanics row 81, promotion) | One allocation (PREMIUM_ACTIVATION0 §2, §3 packet them together, released after NPC-TALK-1): (PREM-2b) promotion state, effective promotion at login, soul maxima, death input (§4.2, §4.3), also the Wheel's promotion read (WHEEL0 §6.2); (PREM-5) the promotion and Premium blessing NPC services. PREM-2a's pure rules are merged (`domain/premium.rs`). Hold released (owner/architect ruling 2026-10-15, ACCEPT-BATCH-2026-10-15): the promotion source is the PREM-5 NPC promotion, 20 000 gp, level 20 or higher, Premium (PREMIUM_ACTIVATION §4.2, D70), paid in coins via GOLD-FEE-1b |  | PREM-WIRE-1; NPC-TALK-1; GOLD-FEE-1b; progression readiness; spell P3b-2 vitals |
| PREM-3 | PREMIUM_ACTIVATION | Premium-area flags, entry refusal, login relocation (§4.5) |  | PREM-1; content pipeline |
| PREM-4 | PREMIUM_ACTIVATION | Premium spell cast check (§4.6): superseded, it is PREM-WIRE-1 (PREMIUM_ACTIVATION0 §2.1); no separate batch |  | PREM-WIRE-1 |
| PREM-1b | PREMIUM_DELIVERY0 | **MERGED (#1678).** Oteryn-Game |  | Game-side acceptance of this decision; PREM-1a |
| MKTCOIN-CONTRACT-1 | MARKET_COINS0 | Game, control plane; security review |  | this decision |
| MKTCOIN-P | MARKET_COINS0 | Oteryn-Platform (external) |  | Platform's own acceptance of §5 |
| MKTCOIN-1 | MARKET_COINS0 | Game, hard; persistence, economy and security review |  | MARKET-1; MKTCOIN-CONTRACT-1 |
| MKTCOIN-WIRE-1 | MARKET_COINS0 | Game, impl; protocol review | yes | MARKET-WIRE-1; MKTCOIN-1 |
| MKTCOIN-E2E-1 | MARKET_COINS0 | Game and Platform; security and cross-repository review |  | MKTCOIN-1; MKTCOIN-P |
| PROF-CONTENT-1 | PROFICIENCY0 | **MERGED (#1342).** Definitions, perk map, `profile_binding` only, removal of the inline profile, the crosswal |  | this decision; ITEM-ID-1 keys |
| PROF-1 | PROFICIENCY0 | **MERGED (#1322).** Migration, writer, reconcile, guards, integrity verifier, grants, admission load (§4.2-§4. |  | this decision; A13 (CANDIDATE; its migrations `0018`-`0020` are merged); CHAR-BUILD-1 (merged); H-1 |
| PROF-2 | PROFICIENCY0; PROF_EFFECT0 (amended) | Accrual, level-up, selection and clearing, perk effects, checkpoints, measurement (PROFICIENCY0), merged with the former separate `PROF-2 (amended)` row of PROF_EFFECT0 (hard (combat), combat and determinism review), so there is one PROF-2 row and PROF-PARITY-1 and PROF-WIRE-1 depend on all of it. The ruleset slice (thresholds, Mastery, kill accrual) is MERGED (#1374); the rest is this row |  | PROF-CONTENT-1 (merged), PROF-1 (merged); ATTACK-1b (merged); RANGED-1; SKILLS-0's live skill read; EQUIP-RT-1 |
| PROF-WIRE-1 | PROFICIENCY0 | Capability row shape, domain, command, `.proto`, client view | yes | owner acceptance of §4.4; PROF-2; PROF-ORB-BURN-1 (the wire waits for the orb shape) |
| PROFICIENCY-1 | PROFICIENCY0 | **MERGED (#1475).** Modification, rank, catalysts (decision) |  | Reference evidence for costs |
| B3-1 | B3 inventory | **MERGED (#1152).** Equipment slot and container-entry location tables (migration), TRANSFER to the container |  | DUR-03 C/MINT (done); the Character/item composition decision; a content definition proving a `container`-slot equip pattern |
| B3-2 | B3 inventory | **MERGED (#1192).** Combat pickup through GAME-INTERACTION into B3-1 |  | B3-1; Combat D |
| AI-1 | ACTION_INTEGRATION | **MERGED (#1150).** Channel owner timer lane and injectable clock (§4.2) |  | `foundation/**`; serialize with Combat D's exclusive lease |
| AI-2 | ACTION_INTEGRATION | **MERGED (#1193).** Spawn realization, respawn, the carrier's creature envelope, creature Movement adoption (§ |  | `runtime_actor_carrier.rs`, `movement.rs`, content activation; serialize with Combat D; the room revision goes to the Content/Seam owner |
| AI-3 | ACTION_INTEGRATION | **MERGED (#1196).** Compile `ai/` into the server; perception, chase and wander over owner snapshots (§4.4) |  | `lib.rs`, `ai/**` |
| AI-4 | ACTION_INTEGRATION | **MERGED (#1394).** Typed AI issuer, bite through Ability, player HP floor (§4.6, §4.7) |  | `ability/**`; serialize with P3b-2 |
| HOUSE-CUSTODY-1 | HOUSE_CUSTODY0 | **MERGED (archive record OTV2-20260930-house-custody-1).** One migration: `HouseInterior`, provenance, exclusivity guard, no runtime grants, `_postgr |  | this decision |
| MAP-OVERLAY-1 | HOUSE_CUSTODY0 | Reset preflight, step-4 recheck, "reset touches only Ground" test (§3.6) |  | ADR-0021 accepted |

### Archive follow-ups (residues named by completed task records)

Acceptance gate: none of these has a brief or decision yet; the control plane routes a packet before a worker allocates one (status of main@37294a2, 2026-10-13). Each is scheduled through PREREQ-1.

| Child | Brief | Class | Reg | Depends on (from the archive record) |
|---|---|---|---|---|
| BOOT-RAT-POLICY-1 | OTV2-20261004-spawn-1a, OTV2-20261011-accept-chat-attack-death-0 | hard (combat), combat review: a boot-spawned rat binds an AI policy and qualifies for melee on a no-manifest boot |  | CREATURE-AI-1 (merged); ATTACK-1b (merged); SPAWN-1b (merged) |
| ENTRY-SPELL-TILES-1 | OTV2-20261011-accept-chat-attack-death-0 | **MERGED (#2005).** content: qualified tile facts for every cell of the accepted entry room |  | MAP-BUNDLE-1 (merged) |
| FLOOR-FACTS-BUNDLE-1 | OTV2-20261010-map-floor-1 | hard (map): floor-change facts and their collision fact in a versioned bundle field so the digest authenticates them (ADR-0021 D189) |  | MAP-BUNDLE-1 (merged) |
| LOOT-ITEM-PROFILES-1 | OTV2-20261007-d3-8-cheese | content: item profiles for cave-rat loot `i3492` and `i3598` (control-plane decision) |  | KILL-REWARD-COMP-1 (merged) |
| CONSUMABLE-CONDITIONS-1 | OTV2-20261004-item-sem-use-2 | content lane: the consumables reported not lowered (52 rows: FOOD, potions, conditions) |  | ITEM-SEM-USE; CONDITIONS-0 |
| SCOPE-HANDOFF-2 | OTV2-20261004-scope-handoff-1 | hard (persistence): cross-node house entry, the any-node fallback |  | SCOPE-HANDOFF-1 (merged); HOUSE-RUNTIME-1a |
| SEC-TRUST-1 | OTV2-20261003-p2-followups-batch7 | hard (security): non-restored freshness state and an embedded trust snapshot |  | SEC-REL-1 |

### Children found by the umbrella audit (packets and decisions; the acceptance gate applies to CANDIDATE ones)

| Child | Brief | Class | Reg | Depends |
|---|---|---|---|---|
| SPELL-LOCK-2b | SPELL_LOCK2 | hard, combat review (SPELL_LOCK_2 §2.2): the combat, ordinary-combat, companion and world item cast paths read the spell access facts | | SPELL-LOCK-2a (merged); SPELL-TARGET-1 |
| CHEST-APPEARANCE-ADMIT-1 | CHEST_APPEARANCE_ADMIT | impl, independent identity review: two appearance-only Items, the map-appearance crosswalk rows and two EXACT bindings (ARCH-CHEST-APPEARANCE-ADMIT §2.1) | | #1830 and #1805 (merged); SPAWN-ADMIT-1 (no open head holding `content/world/pins/`) |
| GOLD-FEE-ACT-2 | GOLD_FEE_ACT1 (CANDIDATE decision: not allocated until accepted) | hard, persistence review: the migration that takes the exclusive fence and inserts the one activation row | | GOLD-FEE-ACT-1 (merged); deploy evidence that every node runs it |
| OPS-KEY-SEPARATION-2 | REVOKE_REPORT | hard: the Game part of REVOKE_REPORT §16.2; not allocated until Platform confirms (#1419) | | Platform confirmation (open) |
| PVP-CLIENT-1 | PARTY_PVP0 (PVP gap closure contract is CANDIDATE) | impl, client: party controls for every `PARTY_INTENT` variant, the `PARTY` domain view, expert mode control, skull, frame and shield rendering, PvP refusal text | | PVP-WIRE-1 |

## 8. Names used in Depends that are not batches of this plan

Every name in a `Depends on` column is one of: a batch in section 4, a child in section 7, or a name in this table. Nothing else is accepted, and a row that needs a name not listed here is a defect.

| Name | Kind | Where it lives |
|---|---|---|
| PREM-WIRE-1, PREM-1a, PREM-1c-harden | alias or Premium lane task | PREMIUM-1 is PREM-WIRE-1; PREM-1a and PREM-1c-harden are merged Premium lane tasks (PREMIUM-ACTIVATION-0) |
| BUNDLE-COMBAT-ENABLE-1 (#2025, merged), BUNDLE-TILES-PROJECTILE-1, ENCOUNTER-RT-0 (CANDIDATE decision) | merged or in flight, Wave 0 | section 5, Wave 0 and the gates of their dependents |
| ITEM-USE-1a, ITEM-USE-1b, STEP-PACE-WIRE-1 | merged slices | #2039 (ITEM-USE-1a and 1b) and #2041 (STEP-PACE-WIRE-1) are on main; ITEM-USE-1c and STEP-PACE-ENABLE-1 are the residue batches |
| SESSION-PUSH-1, ENTITY-CLIENT-1, CHAT-CLIENT-1, ITEM-CLIENT-1, CHAT-1b-2a, ACH-NOTIFY-2, CHARM-DESC-A2, VIS-3, ITEM-SEM-2b-3, QUEST-CAT-BOOT-1, CHEST-QUEST-BIND-1, CHEST-PLACE-BIND-1, SPELL-BOOK-ACTIVATE-1, MAP-SPRITE-1, PROGRESSION-CONTENT-1, PROGRESSION-OWNER-1, SPAWN-ADMIT-1, WORLD-BUNDLE-CI-1, WORLD-CONTENT-SERVE-1, N2N3-1, N4-1, N5-1, OPS-ASSIGN-REPORT-1, RUNBOOK-1 | packet tasks with a record in `docs/agents/tasks/archive/` | done; listed so a packet ID is never absent from the plan (CORE_CLIENT, CORE_LOOP, D327, ITEM_EQUIP, QUEST_WIRING, SPELL_ATTACK, WORLD_CONTENT_SERVE, MAP_WIRE, PROGRESSION_SOURCE, LOGIN_FIRST packets) |
| ITEM-SEM-2b-2, N4-P, CLIENT-NEG-1 | packet tasks outside this plan | the ITEM_EQUIP, LOGIN_FIRST and CORE_CLIENT packets; ITEM-SEM-2b-2 is a narrowed #1672 packet |
| ATTACK-1, SPAWN-1, HOUSE-1, HOUSE-RUNTIME-1, MAP-CUTOVER-1, ITEM-VIEW-1, FORGE-1, TIMED-RT-1, CHAT-1b-2 | superseded parent names | split by their packets into the 1a/1b/1c (or 2a/2b) children named in this plan; the parent name is an alias of the whole set |
| SPELL-LOCK-2a, GOLD-FEE-ACT-1, ITEM-SEM-BED-1, ITEM-SEM-USE-1, ITEM-SEM-USE-2, MAP-BUNDLE-2, GAME-LCFA-ENABLE-1, OPS-REVOKE-REPORT-1, CHAT-1a, CHAT-1b-1 | packet or decision tasks with a record in `docs/agents/tasks/archive/` | done |
| GAME-PREPROD-TOPOLOGY-1 | in-flight task, ops | `docs/agents/tasks/OTV2-20261007-game-preprod-topology-1.md`; the topology decision is PROPOSED and outside the mechanics rows |
| PVP-1 | superseded parent name | split into PVP-1a (merged, #2032) and PVP-1b (residue); the parent name is an alias of both |
| LOGOUT-1 | alias | the earlier plan name of LOGOUT-WIRE-1; the batch is LOGOUT-WIRE-1 |
| DEATH-DUR-1 | death module batch B | section 4, "Death module and in-flight work" |
| GAME-CHAR-CMD-0, CHAR-POSITION-1, CHAR-BUILD-1b, CHARM-1, SKILLS-0 | Character-lane contracts and tasks | contracts and tasks outside this plan (status per their own records, not per the section 7 status list) |
| MAP-LOAD-1, MAP-WIRE-1, MAP-BUNDLE-1, MAP-ITEM-REF-1, MAP-VIEWPORT-PERF-1, MAP-VIEWPORT-PERF-2, N8-1 | map and login tracks | MAP_TRACK_PACKETS and MAP_WIRE_PACKETS, owned outside this plan |
| ITEM-USE-0, PARTY-PVP-0, HOUSE-CUSTODY-0, ATTACK-0 | decisions with ACCEPTED briefs (ITEM_USE0, PARTY_PVP0, HOUSE_CUSTODY0, ATTACK0 in the section 7 status list) | docs/architecture/reviews; their children are in section 7 |
| EQUIP-0, EXERCISE-0, OFFLINE-0, CHAT-0, BANK-FEE-0, TIMED-ITEM-0B, VIP-0, PREY-0, CONDITIONS-0 | CANDIDATE decisions (CHAT-0 was reopened by the 2026-10-12 review; EQUIP0, EXERCISE0, OFFLINE0, CHAT0, BANK_FEE0, TIMED_ITEM0B, VIP0, PREY0, CONDITIONS0 in the section 7 status list) | docs/architecture/reviews; their children are in section 7 and carry the acceptance gate |
| ITEM-ID-1, ITEM-MOVE-WIRE-1, WHEEL-GEM-0A | decisions or tasks whose briefs (ITEM_IDENTITY, ITEM_MOVE_WIRE1, WHEEL_GEM0) are CANDIDATE | docs/architecture/reviews; children carry the acceptance gate |
| CAP-NEG-1, CAP-NEG-RESUME-FALLBACK-1, ITEM-SEM-2b, USE-WIRE-V1 | protocol and item tasks or decisions with no brief in the status list | status per their own records |
| CHEST-1, H-1, FND-02, FND-04, GAME-ABILITY-01, GAME-AI-01, VSL-MOVE-01, MOVE-RL-11, PROD-COMPAT-01, PROD-ENTITLEMENTS-01, DATA-PRIVACY-01, ANL-01, ANL-03, PREMIUM-DELIVERY-0, PREMIUM-ACTIVATION-0 | merged foundations, contracts and decisions | outside this plan |
| ADR-0010, ADR-0021 | ADR | docs/adr |
| DUR-02, DUR-03, RUNE-USE, SPELL-D3, SPELL-D4 | merged lanes or content facts | outside this plan |
| WRITE-MOD-1, ARCH-ALPHA-OPS-0, SUMMON-1, SUMMON-WIRE-1, WORLDINT-USE-1, STANCE-0, STANCE-1 | brief children or decisions outside this plan, or listed in App. 7 supporting briefs | the named briefs (their status is in the section 7 status list; STANCE0 is CANDIDATE) |
