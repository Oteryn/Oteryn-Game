# Wheel of Destiny state (contract candidate v1)

- Date: 2026-09-29
- Status: **CANDIDATE**. It needs independent review and protected integration. Runtime, migration,
  protocol and production authority: **NONE**. Each child in §6 needs its own #162 allocation.
- Allocation: `OTV2-20260929-spell-part-a-state-contracts` (#162 comment 5884682203); programme story
  KAN-16.
- Parents:
  - ADR-0019 §Wheel of Destiny: allocated points, unlocks and active configuration belong to the
    Character/Progression owner, and the Wheel is a bounded module;
  - `OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` §A.1 step 1 (stages are an input from the Wheel
    owner) and step 10 (augment hooks);
  - S6, S16 and S22 of `OTERYN_SPELL_AUTHORING_SCHEMA_V1.md`, and S27 (#1201);
  - DUR-02 and the character authority (ADR-0012).
- Sources: standing rule 6 (follow Canary/Crystal where they are clear; S21: Canary wins a conflict;
  S24: an official or wiki statement wins where one exists). References below are to Canary `99902524`,
  `src/creatures/players/components/wheel/`, unless stated. Crystal `ff7ede5` has the same table and the
  same 250/500/1000 thresholds (`schema.sql:746-756`, `src/creatures/players/wheel/`).

## 1. Question

The 11 revelation spells, and the augments of level-unlocked spells, read a character's Wheel stages.
No owner and no persisted state exist for the Wheel, so every revelation spell fails closed (S6). What
is stored, who owns it, and how are the stages derived, at the minimum level §A.1 needs?

## 2. Facts (Canary)

- **Storage.** One row per player in `player_wheeldata(player_id, slot blob)`. The blob is a list of
  `(u8 slot, u16 points)` for slots 1..36 (`player_wheel.cpp:1881-1935`). It is loaded at login and
  written at each player save (`src/io/iologindata.cpp:282`; Crystal also writes it when the player
  presses save). Gems, scrolls and mod grades live in separate KV scopes (`:1033-1037`, `:1803-1879`).
- **Slots.** 4 domains (green, red, purple, blue) × 9 slots = 36 (`wheel_definitions.hpp:19-67`). The
  capacities are 50, 75 (×2), 100 (×3), 150 (×2) and 200, so a domain holds at most 1000 points
  (`player_wheel.cpp:2059-2082`).
- **Points available.** `max(0, level - 50)` times the points per level (1 in `player_wheel.hpp:293`,
  overridable by the config `WHEEL_POINTS_PER_LEVEL`, `:370`), plus extra points from scrolls and the
  monk quest (`:1934-1966`). The unused points are the available points minus the sum of the slots (`:894-908`).
- **Eligibility.** A vocation, level above 50, Premium and promotion (`canOpenWheel`, `:1997-2017`).
  Without eligibility no Wheel bonus is loaded (`:2406-2410`).
- **Allocation rule.** A slot accepts points only if the character has at least that slot's minimum
  total points and an adjacent slot nearer the centre is full (`canPlayerSelectPointOnSlot`, `:373-892`).
  A slot may not exceed its capacity or the unused points (`:1588-1603`). Points can be removed only in
  a protection zone within 10 tiles of a temple; elsewhere they can only be added (`getOptions`,
  `:2028-2057`).
- **Revelation stage.** The sum of a domain's slots gives stage 3 at 1000 points, 2 at 500 and 1 at 250,
  otherwise 0 (`getPlayerSliceStage`, `:2781-2868`; `wheel_definitions.hpp:69-80`). This agrees with
  Fandom r1204680. Gem bonuses add to the sum. The perk per domain and vocation (`:2678-2779`):

  | Domain | Knight | Paladin | Sorcerer | Druid | Monk |
  |---|---|---|---|---|---|
  | green | Gift of Life | Gift of Life | Gift of Life | Gift of Life | Gift of Life |
  | red | Executioner's Throw | Divine Grenade | Beam Mastery | Blessing of the Grove | Spiritual Outburst |
  | purple | Avatar of Steel | Avatar of Light | Avatar of Storm | Avatar of Nature | Avatar of Balance |
  | blue | Combat Mastery | Divine Empowerment | Lord of Destruction | Twin Bursts (Ice Burst, Terra Burst) | Ascetic |

- **Augments (Conviction).** A slot's conviction perk applies only when the slot is full
  (`src/io/io_wheel.cpp:364-366`). A spell's augment grade is the number of full slots that grant it
  (`getActiveAugments`, `player_wheel.cpp:2587-2627`). This matches §A.1 step 1: stage 2 needs both
  slices (Fandom r1206174).
- **Level loss.** Canary does not change the allocation when a character loses levels; the unused count
  simply underflows (`:894-908`).

## 3. Decision candidate

### 3.1 Owner

The **Wheel** component of GAME-CHAR Character/Progression owns each character's allocation (ADR-0019).
It is a bounded module. Spells, combat and Character code read derived values through §3.4 and never
read slot points directly.

### 3.2 Durable state (minimum sufficient)

| Field | Meaning | Bound |
|---|---|---|
| `slot_points[1..36]` | points in each slot, as Canary's `player_wheeldata` | each at most its capacity (§2) |
| `ruleset_revision` | the Wheel ruleset revision the allocation was validated under | ≤ 128 B |

- The state is persisted as Character state under DUR-02. It is written only inside a Character event
  fenced by the session generation. The physical schema belongs to the implementing child.
- Storage is a typed Character relation (DUR-02 Character baseline rule 10, which names Wheel points),
  not Canary's opaque blob.
- A character without a row has all slots at 0 under the current ruleset revision.
- A stored value above its slot's capacity under the stored revision is corrupt state and fails the
  load closed.
- Slot identities, capacities, domains, the adjacency and minimum-point rules, and the perk per domain,
  vocation and slot are versioned ruleset data (`rulesets/progression/wheel-of-destiny/`, ADR-0019), not
  engine constants. The ruleset child takes them from Canary `io_wheel.cpp` and `player_wheel.cpp` and
  checks them against the wikis.

### 3.2.1 Ruleset revision compatibility (fix for #1205 finding 4130296771)

DUR-02 Character baseline rules 10 and 19 require explicit revision compatibility, and forbid silently
reinterpreting existing data under another ruleset.

- **Current revision.** The world's active Wheel ruleset revision is current. An allocation whose
  `ruleset_revision` equals it loads and derives normally.
- **Non-current revision: fail closed.**
  - The allocation is kept exactly as stored. It is never reinterpreted against the current slot table,
    capacities or perk tables.
  - Every `revelation_stage` and `augment_stage` of the character derives as 0.
  - Allocation changes (§3.3) are rejected, and write nothing, until a migration has moved the
    allocation to the current revision.
  - The rest of the character loads normally.
- **Migration obligation (the ruleset owner, W-R).** A new Wheel ruleset revision that changes slot
  identities, capacities, domains, adjacency or perk tables must ship exactly one of these:
  1. an explicit source→destination migration of stored allocations, validated against the
     destination capacities and allocation rules and run as a DUR-02 rule 19 staged migration;
  2. a declared-compatible mapping: a validated statement that the source and destination revisions
     read the same allocation identically, so the stored revision can be re-stamped without changing any
     slot;
  3. a reset-with-refund rule: the allocation becomes all zero under the destination revision, and every
     point is available to allocate again.

  The revision's release names which one applies. A revision without one of them cannot become
  current for a world that holds allocations under an older revision.
- V1 has one revision, so no migration exists yet. The first one lands with the first ruleset change.

### 3.3 Change

- One allocation change is one fenced Character event. It commits at once, as Crystal does on save.
- The owner accepts a new allocation only if all of these hold:
  - the character is eligible (§3.5);
  - every slot is within its capacity;
  - the sum is at most the available points;
  - every non-zero slot meets the §2 allocation rule;
  - every decrease happens where Canary allows removal (§2).
- A rejected change writes nothing.
- The runtime actor's derived values (§3.4) change only after the commit.
- Available points are `max(0, level - 50) + extra_points`, following Canary (resolution Q3).
  `extra_points` counts promotion scrolls and the monk quest. Their sources are later items (§5), so
  `extra_points` is 0 until their owners exist.
- **Level loss (resolution Q2, as Canary).** The allocation is kept when the character loses levels, and
  its stages still derive from it.
  - Unused points are computed with saturating arithmetic: `unused = available - allocated` if
    `available >= allocated`, else 0. They never go below 0, unlike Canary's `u16` underflow.
  - While allocated points exceed available points, the owner rejects any increase and accepts
    decreases under the §2 removal rule.

### 3.4 Derived values (the hooks the spells read)

- `revelation_stage(perk)`, 0..3. Sum the slots of the perk's domain for the character's vocation
  (§2 table) and apply the thresholds 1000, 500, 250. Gem bonuses add 0 in V1.
- `augment_stage(spell)`, 0..2. Count the full slots whose ruleset conviction perk names that spell's
  augment, capped at 2. The augment values themselves are in
  `tools/content-schema/spell-authoring/wheel-augments.json`, and the `ProjectV2AugmentBinding` applies
  them (S6, §A.1 step 10).
- Both are computed from the committed allocation. The runtime actor holds a copy from the time it is
  created, refreshed after each committed change. A cast reads the stage once (§A.1 step 2).
- An ineligible character (§3.5) has every stage 0 and every augment stage 0, but the allocation stays
  stored.

### 3.5 Eligibility

Resolution Q1 follows Canary. Eligibility requires all of these:
- a vocation;
- level above 50;
- promotion;
- Premium.

The Premium activation decision (`reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md` §7)
left the Wheel undecided, and this candidate closes that gap.
- Eligibility is checked at each use, like the promotion benefits under D76. When Premium lapses, the
  stages become 0 at once, and the allocation is kept.
- **Explicit dependency.** Eligibility depends on the PREM-1 (Premium entitlement) and PREM-2
  (promotion) runtime owners. Until both exist, no character is eligible and every revelation spell
  stays fail-closed.

**Pointer (pending on acceptance of WHEEL-0 (`reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md` §6).** The "progression readiness" dependency of PREM-2 is #1143, completed.
Under owner answer W1 a (#162 5917665342), the Premium requirement of promotion and of the Wheel is
not applied until PREM-1 delivers `premium_current`.

## 4. Playable-first slice: what the spell gate needs first

1. **W-1: state and derivation.** The durable `slot_points` and its load. `revelation_stage` and
   `augment_stage` as pure functions over the allocation, the vocation, the eligibility and the ruleset.
   The runtime actor copy, passed to the spell core as its Wheel input.
   - This makes the §A.1 gate faithful: every character without an allocation is at stage 0, so the gate
     rejects the cast and charges nothing (§A.1 step 2).
   - The §A.1 stage tables can then be implemented and tested with explicit stages.
2. **W-2: allocation change.** §3.3 as a typed player intent from the protocol lane. It is not
   Canary's Wheel window packets. Until W-2 exists nobody can allocate, and every stage stays 0.
3. **W-R: ruleset data.** The slot table, the domain perks and the conviction perks needed by the spells
   in §A.1. It is a prerequisite of W-1's derivation.

The Part A runtime can start after W-1 and W-R. Players can reach stage 1 or higher only after W-2,
Premium (PREM-1) and promotion (PREM-2).

## 5. Out of scope (explicit later items)

- Gems, the Gem Atelier, fragments, mod grades and gem revelation bonuses.
- Promotion scrolls and the monk quest extra points.
- Dedication perks (health, mana, capacity and resistance per point), and conviction perks that are not
  spell augments.
- Passive revelation effects: Gift of Life, Combat Mastery, Blessing of the Grove, Lord of Destruction,
  and the flat damage and healing per stage (§A.1 step 12).
- The Wheel window protocol, presets and the client UI.
- Vocation change handling beyond using the current vocation's column of the §2 table.
- The Wheel's effect on the Harmony multiplier (Ascetic) is a stage read, which is covered. Its value is
  in §A.2.

**Pointer (pending on acceptance of WHEEL-0 (`reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md` §5).** Dedication perks, non-spell conviction perks and revelation passives are
decided there (W-FX-1); gems, fragments, mod grades and presets go to WHEEL-GEM-0.

## 6. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| W-R | Wheel ruleset data, its validator, and the revision migration obligation (§3.2.1) | content pipeline |
| W-1 | Durable allocation, fence, load, derivation, spell-core input | W-R; Character progression storage and migration numbering; high-risk authority/recovery qualification; PREM-1 and PREM-2 before any stage can be above 0 |
| W-2 | Allocation change intent and validation | W-1; protocol lane (registry lease) |
| W-3 | Client Wheel window | W-2; client owner |

**Pointer (pending on acceptance of WHEEL-0 (`reviews/OTERYN_GAME_WHEEL0_WHEEL_OF_DESTINY_DELIVERY_DECISION_2026-09-30.md` §3, §4, §7).** SPELL-WHEEL-GATE-1 comes first: the reader admits Wheel-gated
spells and a zero `WheelStages` input lets role B and C spells cast with their base behaviour. W-1's
physical shape, its writer on CHAR-REV-SEQ-1, and W-2's full-replacement intent are fixed there.

## 7. Engine tests the children must provide

- A character without a row: every stage is 0, and a revelation spell is rejected with nothing charged.
- Domain points 249, 250, 500, 999 and 1000 give stages 0, 1, 2, 2 and 3.
- The same allocation gives the §2 perk for each vocation.
- An augment with one full slot gives stage 1, and with two full slots stage 2. A partly filled slot
  counts 0.
- After a level loss below the allocated points, the stages are unchanged, unused points are 0, an
  increase is rejected and a decrease near a temple is accepted.
- An allocation stored under a non-current ruleset revision derives every stage as 0. Its stored
  slots are unchanged after the load, and a change is rejected, until a migration, a declared-compatible
  mapping or a reset-with-refund applies. After that, it derives under the current revision.
- A change is rejected, and writes nothing, when:
  - a slot is above its capacity;
  - the sum is above the available points;
  - a slot has no full neighbour;
  - it removes points away from a temple;
  - the fence is stale.
- After a Premium lapse the stages are 0 at the next use, and the allocation survives relog.

## 8. Questions (resolved)

None open. The control plane resolved all three on #1205, following Canary (standing rule 6):
- **Q1, eligibility:** Premium, promotion and level above 50, checked at each use. Explicit dependency
  on PREM-1 and PREM-2 (§3.5).
- **Q2, level loss:** the allocation is kept, and unused points saturate at 0 (§3.3).
- **Q3, extra points:** scroll and monk quest points count. Their sources stay later items, so they
  are 0 until their owners exist (§3.3, §5).

## 9. Handback

```yaml
result: CANDIDATE
owner_decisions: []
durable_decision_ref: docs/architecture/OTERYN_WHEEL_OF_DESTINY_STATE_CONTRACT_CANDIDATE_V1.md
production_authority_changed: false
implementation_may_resume: false   # until this candidate is reviewed and integrated
required_independent_review: "exact-head review (Character state, fence, derivation)"
implementation_lanes: [W-R, W-1, W-2, W-3]
```
