# Mechanics completion plan (server track)

- Plan: `MECHANICS-COMPLETION-PLAN-V1`
- Status: **CANDIDATE** (plan, not a contract). Allocation of each batch needs the control plane; batches that
  change a contract still need their own decision.
- Role: client-track architect, delegated by the control plane (#1622), at the owner's direction of 2026-10-13
  ("all non-DONE mechanics go through the architect")
- Input: control-plane audit of 115 of 139 mechanics rows that are not DONE (row numbers below are the audit's)
- Admission baseline: `main@4ac13390`
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

## 3. Not Game-owned (owner direction 2026-10-13: weather, Store entitlements and server save/restart are IN scope, see batches WEATHER-1, STORE-ENT-1, OPS-SAVE-1)

| Row | Mechanic | Why |
|---|---|---|
| 131 | Create/delete/rename character | Platform owns web identity and the account flow (ADR-0012); Game keeps only the character command contract (GAME-CHAR-CMD-0). Re-evaluate if a Game-side gap appears. |
| 3, 4, 14 | STEP-PACE-WIRE-1 (#2041), auto-walk, minimap | client-owned (#1942). Server dependency: none new for 3 and 4 (the server receives paced steps); 14 needs LOOP-MAP-1 |
| 16 | Light (utevo lux, torches) | client rendering is client-owned; the server light state comes from LOOP-CLOCK-1 |

## 4. Batches

Class: `hard` = persistence, fencing, wire format, authority or durable value (hard worker and independent
review); `impl` = ordinary worker. `Mig` = needs a migration (numbers after 0083 leased by the control plane);
`Reg` = changes the protocol registry (one registry PR at a time).

### Epic 1. Game loop

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| LOOP-WORLD-1 | 6, 8, 9, 10 | confirm teleports, keyed/level/quest doors, levers and tile damage on the bundle node | #2018, ENCOUNTER-RT-0, BUNDLE-COMBAT-ENABLE-1 (the `qualified_room` gate) | impl | no | no |
| LOOP-MAP-1 | 13 | close MAP-VIEWPORT-MEASURE-1 and the cap 18 offer decision | #1942 client proof | impl | no | no |
| LOOP-COMBAT-MODES-1 | 20 | chase mode (CHASE-1), secure mode, fight-mode effect on targeting | none | impl | no | maybe |
| LOOP-DAMAGE-1 | 23, 24, 25, 26 | enable cap 12 (EQUIP-0), resistances, crits/leech, area and chain attacks on the bundle, mana shield | EQUIP-0, BUNDLE-COMBAT-ENABLE-1 | hard (wire) | no | yes |
| LOOP-RANGED-1 | 21, 22, 51 | projectile profiles, distance weapons and ammo (a DUR-03 burn), wands/rods, attack spells | BUNDLE-TILES-PROJECTILE-1, LOOP-DAMAGE-1, ITEM-LOOP-1 | hard (ammo burn) | yes | no |
| LOOP-PROGRESS-1 | 37, 38, 40, 41 | level-up vitals/capacity per vocation, vocations end to end, skill gain on hits, magic level | GAME-CHAR-CMD-0 | hard (Character writes) | yes | no |
| LOOP-VITALS-1 | 30, 32, 33 | durable regeneration accumulator, food-time ingress, soul regen and soul orb, stamina | ITEM-USE-1 (#2039), LOOP-PROGRESS-1 | hard (Character state) | yes | no |
| LOOP-TRAIN-1 | 34, 35 | offline training in beds, exercise weapons | LOOP-VITALS-1, HOUSE-INT-1 (beds), ITEM-TIMED-1 | hard | yes | no |
| LOOP-MONSTER-1 | 94, 96 | world-owned respawn scheduler independent of sessions; monster ranged, spell and summon AI | LOOP-RANGED-1, MAGIC-CAST-1, MAGIC-SUMMON-1 | impl | no | no |
| LOOP-GROUND-1 | 18 | decay of dropped items on Ground (shared with the death drop) | DEATH-DUR-1 | hard (retire) | maybe | no |
| LOOP-CLOCK-1 | 15 | world clock and day phase, server light state | none | impl | no | yes |
| WEATHER-1 | 17 | weather state and change events | LOOP-CLOCK-1 | impl | no | yes |
| LOGOUT-1 | 109 | combat logout and disconnect protection | PVP-RT-1b | hard (fencing) | yes | no |

### Epic 2. Economy and items

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| ITEM-LOOP-1 | 62, 63, 64, 65, 66, 68 | equip/unequip, drop/pickup, nested containers, stacks, cap and weight enforcement, starter pack on the bundle | EQUIP-0, #2039 | hard (durable item) | yes | maybe |
| ITEM-TIMED-1 | 69, 72, 73 | charges and repair, imbuing, forge runtime | ITEM-LOOP-1 | hard | yes | no |
| ITEM-STORAGE-1 | 70, 71, 74 | depot, supply stash, books and papers | ITEM-LOOP-1 | hard | yes | yes |
| ITEM-LOOK-1 | 75 | look/examine message | none | impl | no | yes |
| NPC-ECON-1 | 39, 81 | bank, promotion purchase | #2040, BANK-FEE-0 | hard (value) | yes | no |
| TRADE-PLAYER-1 | 82 | player-to-player trade | ITEM-LOOP-1 | hard | yes | yes |
| MAIL-1 | 88 | mail and parcels | ITEM-STORAGE-1 | hard | yes | yes |
| MARKET-1 | 122 | market | ITEM-STORAGE-1, NPC-ECON-1 | hard | yes | yes |
| CRAFT-1 | 89, 91 | crafting, fishing, digging, cooking | #2039, ITEM-LOOP-1 | hard (item conversions) | yes | no |
| HIRE-1 | 90 | hirelings | HOUSE-OWN-1, NPC-ECON-1 | impl | yes | no |
| QUEST-RT-1 | 86, 87 | quest runtime on the bundle, chest rewards, cap 16 offer | QUEST-LOWER-1, ITEM-LOOP-1 | hard (rewards are mints) | no | yes |

### Epic 3. Magic and conditions

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| MAGIC-CAST-1 | 49, 50, 52, 53, 54 | close SPELL-STARTER-1 RUNTIME_GAP and ADAPTER_MISSING: spellbook, casting, healing, support, parametric spells, cooldown groups | none | impl | no | no |
| MAGIC-COND-1 | 27, 28, 29 | DoT, haste/paralyze/invisibility/outfit, muted/exhaust/infight/pacified/rooted/feared/powerless/agony | BUNDLE-COMBAT-ENABLE-1 | impl | no | no |
| MAGIC-ITEMS-1 | 55, 56 | rune use with target and conjure | #2039 | hard (DUR-03) | maybe | no |
| MAGIC-SUMMON-1 | 57, 58 | summons, familiars, magic fields | LOOP-WORLD-1 (field ingress), MAGIC-CAST-1 | impl | no | no |
| PROG-ADV-1 | 42, 43, 44 | weapon proficiency (PROF-WIRE-0), wheel of destiny, gem atelier | LOOP-PROGRESS-1 | hard | yes | yes |

### Epic 4. World and NPC

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| NPC-WORLD-1 | 83, 84 | NPC travel, walking, voices, focus | #2040, CHAR-POSITION-1 | hard (travel moves a Character) | yes | no |
| MONSTER-META-1 | 98, 99, 100, 101 | bestiary (CHARM-6), charms, bosstiary, bosses | LOOP-MONSTER-1 | impl | maybe | no |
| EVENTS-RAID-1 | 102 | raids | LOOP-MONSTER-1 | impl | no | no |
| EVENTS-WORLD-1 | 103, 104 | world changes, Fury Gates, Nightmare Isles, hazard | EVENTS-RAID-1 | impl | maybe | no |
| ARENA-1 | 106 | Tibiadrome and arenas | LOOP-MONSTER-1 | impl | no | no |
| TUTORIAL-1 | 139 | late test epic, not blocking: Newhaven/Rookgaard content after Thais start is tested | Wave 4 done | impl | no | no |

### Epic 5. Community

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| PARTY-1 | 112, 59 | invite/join/leave, shared XP, party spells | #2032 | hard (shared state, rewards) | yes | yes |
| GUILD-1 | 113 | guilds | PARTY-1 | hard | yes | yes |
| VIP-1 | 114 | VIP list | none | impl | yes | yes |
| CHAT-1 | 116 | chat channels | none | impl | maybe | yes |
| HOUSE-OWN-1 | 119 | house ownership, auction, rent | NPC-ECON-1 | hard (value) | yes | no |
| HOUSE-INT-1 | 120, 121 | interiors, ACL, beds | HOUSE-OWN-1 | impl | maybe | no |
| SOCIAL-LATE-1 | 118 | marriage and PvP arena | GUILD-1, ARENA-1 | impl | maybe | no |

### Epic 6. Meta

| Batch | Rows | Scope | Depends on | Class | Mig | Reg |
|---|---|---|---|---|---|---|
| STORE-ENT-1 | 123 | Store, server side only: entitlement grants and item/outfit/mount delivery; purchase and payment stay with Platform | PREMIUM-1, Platform contract | hard (value, authority) | yes | yes |
| OPS-SAVE-1 | 137 | server save and graceful restart: flush all session-generation fenced writes, drain, resume | none | hard (durability) | maybe | no |
| PREMIUM-1 | 124 | Premium gating across features | NPC-ECON-1 | impl | no | no |
| ACH-1 | 125, 129 | achievements, badges, titles | LOOP-PROGRESS-1 | impl | yes | maybe |
| COSMETIC-1 | 126, 127 | outfit and addon change, mounts | #1942 for the client side | impl | no | yes |
| HUNT-1 | 45, 46 | prey and hunting tasks | LOOP-MONSTER-1 | hard (rewards) | yes | no |
| DAILY-1 | 47 | daily reward | NPC-ECON-1 | hard (rewards) | yes | no |
| RANK-1 | 48, 128 | highscores and analysers as normalized Game exports (Atlas consumes them) | LOOP-PROGRESS-1 | impl | no | no |
| SEC-1 | 135 | server-side integrity checks (bot controls); the 9c/6c3 retirement is separate | none | impl | no | no |
| GM-1 | 136 | GM tools, talkactions, bans | Platform contract | hard (authority) | yes | no |
| SPECTATE-1 | 138 | spectating | LOOP-WORLD-1 | impl | no | yes |

### Death module and in-flight work (not re-planned here)

Rows 107 and 108: DEATH-DUR-1 (B), DEATH-3 (C), PVP-DEATH-1 (D), DEATH-4 (E), optional wire F, per the
DEATH-COMPLETE-0 decision (#2043; fixes in #2044 must land before B, C, E). Row 85 (blessings) is DEATH-4. Rows
110 and 111: PVP-1 (#2032) and PVP-RT-1b. In-flight PRs: 6 (#2018), 31 and 55 and 76 (#2039), 80 (#2040), 105
(#1867), 3 (#2041). They are not duplicated here.

## 5. Waves and parallelism

Throttles: one migration PR at a time and one protocol-registry PR at a time, so wave width is set by `Mig` and
`Reg`, not by the number of workers.

1. **Wave 0 (in flight):** #2018, #2032, #2039, #2040, #2041, #1867, #2044, ENCOUNTER-RT-0, BUNDLE-COMBAT-ENABLE-1.
2. **Wave 1 (no migration, parallel):** OPS-SAVE-1, MAGIC-CAST-1, LOOP-COMBAT-MODES-1, LOOP-WORLD-1, MAGIC-COND-1,
   ITEM-LOOK-1, LOOP-CLOCK-1, SEC-1. In sequence for migrations: ITEM-LOOP-1, then LOOP-PROGRESS-1 (after the
   death module's B and C).
3. **Wave 2:** LOOP-DAMAGE-1, LOOP-RANGED-1, LOOP-VITALS-1, NPC-ECON-1, QUEST-RT-1, MAGIC-ITEMS-1,
   MAGIC-SUMMON-1, LOOP-MONSTER-1, PARTY-1, WEATHER-1.
4. **Wave 3:** ITEM-TIMED-1, ITEM-STORAGE-1, NPC-WORLD-1, LOOP-GROUND-1, LOGOUT-1, CRAFT-1, PROG-ADV-1,
   MONSTER-META-1, HOUSE-OWN-1.
5. **Wave 4:** TRADE-PLAYER-1, MAIL-1, MARKET-1, HOUSE-INT-1, LOOP-TRAIN-1, GUILD-1, VIP-1, CHAT-1, HIRE-1,
   PREMIUM-1, STORE-ENT-1, HUNT-1, DAILY-1, ACH-1, RANK-1, COSMETIC-1.
6. **Wave 5:** EVENTS-*, ARENA-1, SOCIAL-LATE-1, GM-1, SPECTATE-1, TUTORIAL-1.

Waves are an order of dependency, not a schedule; the control plane starts a batch when its dependencies have
merged. The death module runs alongside wave 1 and 2 on its own track.

## 6. Owner answers (2026-10-13, via the control plane)

1. Start stays in Thais; the tutorial island is a late, non-blocking test epic (TUTORIAL-1).
2. Weather, Store (server side only) and server save/restart are in scope (WEATHER-1, STORE-ENT-1, OPS-SAVE-1).
3. Order confirmed.

Original questions, kept for the record:

1. **Tutorial.** Do we need a tutorial island (Rookgaard or the newer Newhaven) before the first public release?
   Recommendation: no; start in Thais as today and decide later.
2. **Out of scope.** Confirm that weather, the Store and server save/restart are not Game mechanics for now.
   Recommendation: confirm.
3. **Order.** The plan finishes the game loop first, then economy, magic and conditions, the world, then
   community and meta. Recommendation: confirm; community features (guilds, market, houses) come last.

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
