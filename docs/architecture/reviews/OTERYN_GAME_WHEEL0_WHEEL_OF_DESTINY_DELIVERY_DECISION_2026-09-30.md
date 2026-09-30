# WHEEL-0 Wheel of Destiny delivery

- Decision: `WHEEL0-WHEEL-OF-DESTINY-DELIVERY-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (Character
  state, persistence, protocol) and protected integration. Owner question W1 (§12) is answered a
  (verbatim record on #162 5917665342); §6 applies it. Owner decision W2 (#162, 2026-09-30,
  Wheel reset) is recorded in §12; §4 and §5.1 apply it (WHEEL0-RST-1).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction (2026-09-30, verbatim: "tak samo whel of destiny blokuje mi reszte
  spelli bo go nie ma"); the spell lane's audit (#162 5916978013: "4 ready reader refusals
  (position/Wheel gates)"); the control plane's open ruling on "progression readiness" for
  STANCE-1 (#162 5916023254), which PREM-2 also names
- Builds on: the Wheel of Destiny state contract candidate
  (`OTERYN_WHEEL_OF_DESTINY_STATE_CONTRACT_CANDIDATE_V1.md`, on `main`: owner, `slot_points[1..36]`,
  ruleset revision compatibility, stage derivation, eligibility, W-R/W-1/W-2/W-3); the spell native
  behaviours candidate §A.1 (roles A-D, gate, augments, passives step 12); the spell authoring
  schema S6 and S22; ADR-0019 (Wheel as a bounded module); DUR-02 Character baseline rules 2, 10
  and 19; QUEST-STATE-0 §5.2 (CHAR-REV-SEQ-1); the composition decision rules 1-2; PREMIUM-ACTIVATION
  §4.2 and D70-D76; PREMIUM-DELIVERY-0 (`premium_current`); the Tibia manual `characters.md` §5.1.7;
  owner rule 5905825574 (Global parity)
- Amends, pending on acceptance of WHEEL-0, in this PR: the Wheel state contract candidate §3.2.1,
  §3.5, §5 and §6 (pointers); PREMIUM-ACTIVATION §4.2, §4.6 (the revelation spells, WHEEL0-PS-1) and §5
  (the PREM-2, PREM-4 and PREM-5 rows); PREMIUM-DELIVERY-0 implementation brief (the PREM-2..5 row)
  (§6.2).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| SPELL-WHEEL-GATE-1 | impl, spell review | the ready reader admits `wheel_unlock` and augment-bound spells; `CasterState` carries a `WheelStages` input, all 0 until W-1; roles B and C cast with their base behaviour, role A fails closed at cast (§3); no cast Premium check for `wheel_unlock` spells until the Premium switch-over (WHEEL0-PS-1, §6.2) | this decision |
| W-R | impl, content review | the Wheel ruleset: slots, capacities, adjacency, minimum points, perks per domain, vocation and slot, the dedication values, the conviction perks (spell and non-spell), revelation values; validator; each later revision declared value-only or Wheel reset (WHEEL0-RST-1, §5.1) | this decision |
| W-1 | hard, persistence review | allocation tables, a new receipt kind, `commit_character_wheel` on CHAR-REV-SEQ-1, the admission Wheel reset (WHEEL0-RST-1), admission load, derivation into `WheelStages` (§4) | W-R; CHAR-REV-SEQ-1 |
| W-2 | impl, protocol review | capability `WHEEL_V1`, `WHEEL_QUERY` and `WHEEL_INTENT` (§7) | W-1 |
| W-FX-1 | hard, combat review | dedication perks (max health, max mana, capacity), non-spell conviction perks, passive revelation effects (§5.2) as stat and effect contributions | W-1; the vitals owner; CONDITIONS-0 for timed effects |
| PREM-2 | as allocated (PREMIUM-ACTIVATION) | promotion state; its "progression readiness" dependency is satisfied (§6.2) | #1143 (completed); spell P3b-2 vitals (merged); not PREM-1 (W1 a, §6.2) |
| W-3 | client owner | the Wheel window | W-2 |

Later, each with its own decision: gems, the Gem Atelier, vessels, fragments and mod grades
(WHEEL-GEM-0: new item families, drops, gold and fragment sinks under D178); presets; promotion
scrolls and the monk quest points (their item and quest owners).

## 1. Question

What must exist so that Wheel-gated spells can be cast, what does the full Wheel contain, and in
what order is it built?

## 2. Facts

**PROVEN**

- The Wheel state candidate (on `main`) owns each character's `slot_points[1..36]` and ruleset
  revision under Character/Progression; derives `revelation_stage(perk)` 0..3 at 250/500/1000
  domain points and `augment_stage(spell)` 0..2 from full slots; keeps the allocation on level loss
  with unused points saturating at 0; fails closed on a non-current ruleset revision; requires a
  vocation, level above 50, promotion and Premium, checked at each use; and depends on PREM-1 and
  PREM-2 "before any stage can be above 0". None of W-R, W-1, W-2 is allocated.
- Spell native behaviours §A.1: role A (11 revelation spells) is castable only at stage 1 or more;
  role B (5 level spells: Energy Wave, Strong Ice Wave, Mass Healing, Front Sweep, Flurry of Blows)
  and role C (Energy Beam, Great Energy Beam under Beam Mastery) are authored with their base
  behaviour and take the Wheel as an augment input; role D has no Wheel branch. Step 12 lists the
  passives that are the Wheel owner's, not spell data.
- The spell lane audit (#162 5916978013): 172 ready spells, 4 ready spells refused by the reader
  on position or Wheel gates; `CharacterCastFacts` is `None` in the live path.
- QUEST-STATE-0 (on `main`) §5.2: CHAR-REV-SEQ-1 moves every `CharacterRevision` writer onto one
  per-Character sequencer; new writers are built on it.
- PREMIUM-ACTIVATION §4.2: promotion is durable Character state bought at an NPC (level 20,
  20,000 gold, Premium current); PREM-2 builds it after "PREM-1; progression readiness".
  "Progression readiness" is the D88 initializer task `OTV2-20260928-char-progression-init`
  (#1143), completed: the XP writer and its root fields are on `main`.
- HOUSE-OWN-0 owner answer H2a: a Premium rule waits until Premium is delivered.

**CIPSOFT_OFFICIAL** (the Tibia manual `characters.md` §5.1.7)

- From level 51, promoted characters on Premium accounts earn 1 promotion point per level. Points
  are spent anytime, anywhere online; removing points or resetting needs a temple.
- 4 domains of 9 slices; Dedication (small, per point, capped per slice), Conviction (a full slice;
  stacks across copies), Revelation (one per domain, by points spent in the domain, three stages).
- Each vocation has its own layout; a slice unlocks when an adjacent slice nearer the centre is full.
- All Wheel benefits are suspended when Premium lapses and restored on renewal.
- Gems, vessels, fragments and presets exist (the Gem Atelier and Fragment Workshop).

## 3. The spell gate now (SPELL-WHEEL-GATE-1)

- **Admission, not refusal.** The ready reader admits spells with `requirements.wheel_unlock` and
  spells with augment bindings. The Wheel is a cast-time input (S6, §A.1 step 1), never a reason to
  refuse a bundle.
- **Input.** `CasterState` carries `WheelStages` (per perk 0..3, per augment 0..2). Until W-1 it is
  all 0 for every character.
- **Effect at stage 0:**
  - role A: the cast fails with nothing charged (§A.1 step 2);
  - role B and C: the spell casts with its base behaviour and no augment;
  - role D: unchanged.
- So the Wheel no longer blocks the level spells; only the 11 revelation spells wait for points.
- **Premium check of the revelation spells (WHEEL0-PS-1, §6.2).** The 11 revelation spells are
  authored `premium` (§A.1 parameters), and `spell/cast.rs` builds `CasterState` with
  `premium: false` (SPELL-D5), so `spell/mod.rs` refuses them as `PremiumRequired`. Until the
  switch-over of §6.2, SPELL-WHEEL-GATE-1 does not apply the cast Premium check to a spell with
  `requirements.wheel_unlock`; every other cast check, and the Premium check of every other spell,
  is unchanged.
- The position gate named in the audit is not a Wheel matter; the spell lane routes it separately.

## 4. Allocation storage and writer (W-1)

The state candidate §3.2-§3.4 is binding; this section fixes its physical shape.

- **Tables** (the next free migration at allocation):
  - `game_character_wheel_state`: `character_id` (primary key), `wheel_ruleset_revision`,
    `wheel_revision` (starts at 0, +1 per change), `allocated_total`, `committed_character_revision`,
    `last_wheel_occurrence_id`;
  - `game_character_wheel_slots`: (`character_id`, slot 1..36, points > 0); a missing slot is 0;
  - `game_character_wheel_receipts`: one immutable row per change, unique on (`character_id`,
    occurrence), on (`character_id`, `original_character_revision`), on (`character_id`,
    `committed_character_revision`) and on (`character_id`, `after_wheel_revision`). It carries:
    - **the common CharacterRevision receipt envelope**, exactly the chain columns of the `0020`
      receipts that QUEST-STATE-0 §5.1 also adopts: `original_character_revision`,
      `committed_character_revision` (= original + 1), `level_before`, `level_after` (= before),
      `experience_before`, `experience_after` (= before), the eight interpretation revisions
      (`profile_revision`, `ruleset_revision`, `content_revision`, `simulation_revision`,
      `evidence_revision`, `declaration_revision`, `policy_revision`, `reward_revision`) and
      `committed_at`;
    - **the Wheel columns:** occurrence, `kind` (`ALLOCATION` or `RULESET_RESET`), SHA-256 binding
      of the request only, `before_wheel_revision`, `after_wheel_revision` (= before + 1), the before
      and after slot vectors (36 small integers each), `before_wheel_ruleset_revision` and
      `wheel_ruleset_revision` (the after revision). The Wheel ruleset revisions are their own
      columns and are never written into the envelope's `ruleset_revision`, which stays the
      progression interpretation revision.
  - `wheel_ruleset_revision` is the physical column of the state candidate's `ruleset_revision`.
    The only write that moves it is the Wheel reset (WHEEL0-RST-1), itself a change on the
    CHAR-REV-SEQ-1 sequencer with one `RULESET_RESET` receipt, so the row tip below always holds.
    There is no ruleset migration writer.
- **Guards** (deferred), on the model of the stance and Bestiary arms of `0020`:
  - `allocated_total` equals the sum of the slots; each slot is within its capacity under
    `wheel_ruleset_revision` (a ruleset table the W-R child loads);
  - **full chain:** ordered by `committed_character_revision`, the first receipt has
    `before_wheel_revision` 0 and an all-zero before vector, and every later receipt's
    `before_wheel_revision` and before vector equal its predecessor's `after_wheel_revision` and after
    vector; every receipt has `after_wheel_revision = before_wheel_revision + 1`;
  - **ruleset chain:** the first receipt has `before_wheel_ruleset_revision` equal to its
    `wheel_ruleset_revision`, and every later receipt's `before_wheel_ruleset_revision` equals its
    predecessor's `wheel_ruleset_revision`. An `ALLOCATION` receipt has equal before and after
    ruleset revisions. A `RULESET_RESET` receipt has an all-zero after vector and an after ruleset
    revision newer than its before revision, with at least one reset revision (§5.1) after the
    before revision up to and including the after revision, per the ruleset revision table the W-R
    child loads. A skipped, reversed or undeclared transition therefore fails the guard;
  - **row tip:** the state row's `wheel_revision`, `wheel_ruleset_revision`,
    `committed_character_revision` and `last_wheel_occurrence_id` equal the latest receipt's
    `after_wheel_revision`, `wheel_ruleset_revision`, `committed_character_revision` and occurrence,
    and the stored slots equal its after vector;
  - a state or slot row with no receipt, or a receipt with no state row, is inconsistent;
  - a violation raises `23514`, as the other arms.
- **Receipt kind.** A new CharacterRevision receipt kind next to those CHAR-REV-SEQ-1 lists; the
  consistency guard gains its arm in the cross-kind chain with the envelope columns above (exactly
  one receipt per revision; each receipt's level and experience before equal its predecessor's
  after, of whatever kind), plus the Wheel full-chain and row-tip arm above. Whichever of W-1 and the other guard rewriters lands later carries every arm (the
  #162 guard serialization rule).
- **Writer.** `commit_character_wheel` runs on the CHAR-REV-SEQ-1 sequencer: session fence and
  `character_root` FOR UPDATE (composition rule 2), then the wheel rows. The request carries the
  expected `wheel_revision`, not the CharacterRevision: a stale `wheel_revision` is
  `STALE_REVISION`; a CharacterRevision move by another writer retries once (QUEST-STATE-0's rule
  for writers whose binding excludes the revision).
- **Validation** at commit, all against the stored ruleset: eligibility (§6), capacities, the
  points rule (WHEEL0-PT-1), the adjacency and minimum-point rules for every non-zero slot, and
  every decrease only at a temple (§7.3). A rejected change writes nothing.
- **No-op rejection (WHEEL0-NOOP-1).** A replacement whose slot vector equals the stored vector is
  rejected as `NO_CHANGE` before any write: no receipt, no `wheel_revision` or CharacterRevision
  advance, and the occurrence is not consumed. Every receipt therefore records a real change, which
  bounds receipt growth to actual allocation changes.
- **Wheel reset (WHEEL0-RST-1; owner decision W2, #162 2026-09-30).** There is no ruleset
  migration writer and no offline or administrative Wheel writer. A stored allocation is current
  when its `wheel_ruleset_revision` is the active revision or every later revision up to the active
  one is value-only (§5.1); it is then kept as stored, pinned to its revision, and read and validated
  with the active revision's tables. When a reset revision lies after the stored revision, the
  allocation is cleared at the character's next admitted session: after the admission's own session
  fence and `character_root` FOR UPDATE (composition rule 2), before the allocation is loaded into
  the actor, one commit on the CHAR-REV-SEQ-1 sequencer writes one `RULESET_RESET` receipt (the same
  envelope, chain, ruleset-chain and row-tip guards) with the stored vector as before vector, an
  all-zero after vector, the stored revision as `before_wheel_ruleset_revision` and the active
  revision as `wheel_ruleset_revision`, deletes the slot rows and sets `allocated_total` to 0 and
  `wheel_revision` to before + 1. Every promotion point returns unallocated for the player to
  redistribute; the reset is free and applies no eligibility, points or temple rule. Its occurrence
  is derived from (`character_id`, source revision, destination revision), so a replay finds it
  already applied (the stored revision is the active one) and writes nothing; `NO_CHANGE` does not
  apply, because a reset always advances `wheel_ruleset_revision`. Until the reset commits (for
  example when a reset revision activates during a live session), the old allocation grants
  nothing under the new revision: every stage and contribution derives as 0 and
  `commit_character_wheel` rejects changes as `RULESET_NOT_CURRENT` (fail closed, state candidate
  §3.2.1). A character with no row has nothing to reset.
- **Points rule (WHEEL0-PT-1; state candidate §3.3, level loss).** A change that raises any slot
  requires the new sum at most the available points. A strict decrease (no slot raised, at least one
  slot lowered) is admitted whatever the available points, so a character over-allocated after a
  level loss can reduce step by step (100 allocated, 90 available: 95 is admitted); its other rules
  (eligibility, adjacency, minimum points, temple) still apply. A change that raises one slot and
  lowers another is a raise. A rejection is `OVER_POINTS`.
- **Load.** Admission loads the allocation into the runtime actor; a committed change refreshes the
  allocation after commit. No row means every slot 0. The actor caches the allocation only, never
  an eligibility result.
- **Cast snapshot (WHEEL0-EL-1).** `WheelStages` is produced when the cast snapshot is built, from
  the cached allocation and the eligibility read at that moment (§6.1): the character's current
  vocation, current level, current `promoted` state and, once PREM-1 delivers it,
  `premium_current(account)`. Any failing condition yields all-zero `WheelStages` for that cast; a
  cached stage from admission or from the last allocation commit is never reused. A level loss, a
  vocation change or a mid-session Premium lapse therefore takes effect at the next cast without a
  relog or an allocation change, and a restoration does likewise. The W-FX-1 contributions (§5.2)
  follow the same rule at their own authoritative points (each vitals, capacity, regeneration or
  damage step), as the promotion benefits do under D76.
- **Level loss** keeps the allocation (state candidate §3.3).

## 5. Ruleset and effects

### 5.1 Ruleset (W-R)

`rulesets/progression/wheel-of-destiny/`, versioned (ADR-0019): per vocation the 36 slots with
domain, capacity (50, 75, 100, 150, 200), adjacency and minimum points; per slot its dedication
effect and value per point and its conviction perk; per domain and vocation the revelation perk and
its stage values; the augment table already in `wheel-augments.json`. Source order: official
statements win (S24), then Canary `io_wheel.cpp` and `player_wheel.cpp` (S21). The first revision
has no predecessor. Each later revision is declared in its release as one of two kinds
(WHEEL0-RST-1, owner decision W2, #162 2026-09-30):

- **value-only:** it changes only numeric values (dedication values per point, conviction and
  revelation values) and leaves slot identities, domains, capacities, adjacency, minimum points and
  the perk placed in each slot and domain unchanged; stored allocations are kept as they are, pinned
  to their revision (§4);
- **Wheel reset:** any other change, which could invalidate a stored allocation; each allocation
  under an earlier revision is cleared at its character's next admitted session (§4).

Of the state candidate §3.2.1 options, WHEEL-0 uses the declared-compatible mapping (value-only,
without a re-stamp write) and reset-with-refund (at the next admitted session); the explicit
source-to-destination migration is not used.

### 5.2 Effects (W-FX-1)

- **Dedication:** max health, max mana and capacity per point in a slice, per vocation; they add to
  the vitals and capacity owners' maxima through one Wheel contribution, never through scattered
  conditionals (ADR-0019).
- **Conviction, non-spell:** skill, resistance, leech and other perks of full slices, stacking
  across copies, as stat contributions.
- **Revelation passives:** §A.1 step 12 (Gift of Life, Combat Mastery, Blessing of the Grove, Lord
  of Destruction, Divine Empowerment's field is spell data) and the flat damage and healing per
  stage.
- All contributions are 0 while the character is not eligible (§6); the allocation stays stored.

## 6. Eligibility and Premium

### 6.1 Rule

A vocation, level above 50, promotion, and Premium (the state candidate §3.5; the manual). Checked
at each use, never cached (WHEEL0-EL-1, §4): a lapse makes every stage and contribution 0 at once and
keeps the allocation.

### 6.2 Dependencies

- **Promotion** is PREM-2's durable state. PREM-2's "progression readiness" dependency is #1143,
  completed; the same ruling answers STANCE-1's dependency (#162 5916023254).
- **PREM-2 does not depend on PREM-1 (architect ruling applying W1 a).** PREM-2 depends on #1143
  (completed) and the vitals owner (P3b-2, merged) for soul; the promotion purchase of PREM-5
  depends on PREM-2 and the NPC service owner. Neither waits for PREM-1: before PREM-1, the Premium
  condition is not applied (next rule). Whichever of PREM-1 and PREM-2 (or PREM-5 for the purchase)
  lands later wires `premium_current` into the promotion purchase, the promotion benefits and the
  Wheel eligibility (WHEEL0-EL-1), and that change is merged before PREM-1's activation record names
  Premium as delivered. PREM-3, PREM-4 and the Premium blessing service of PREM-5 keep their PREM-1
  dependency. PREMIUM-ACTIVATION §5 and the PREMIUM-DELIVERY-0 implementation brief carry this as a
  pending amendment (§13).
- **Revelation spells (WHEEL0-PS-1, architect ruling applying W1 a).** W1 a waives the Wheel's
  Premium condition until delivery; a revelation spell is a Wheel benefit, so its cast Premium check
  (PREMIUM-ACTIVATION §4.6, PREM-4) is waived for the same period and no longer: until PREM-1's
  activation record names Premium as delivered, a spell with `requirements.wheel_unlock` casts
  without the Premium check (§3). The switch-over is the same one as for promotion and the Wheel:
  whichever of PREM-1 and PREM-4 lands later wires `premium_current` into the cast Premium check of
  these spells, and that change is merged before the activation record names Premium as delivered;
  from then on they cast only while Premium is current. PREM-4 keeps its PREM-1 dependency for
  every other Premium spell, which stays refused until then. PREMIUM-ACTIVATION §4.6 carries this as
  a pending amendment (§13).
- **Premium** (owner answer W1 a, a supersession of D70's Premium condition until delivery): until PREM-1 delivers `premium_current`, the
  Premium requirement of promotion (D70) and of the Wheel is not applied; from then on both apply,
  and a promotion bought before then is kept under the lapse rules of D73 and D76. The same applies
  to the cast Premium check of the revelation spells (WHEEL0-PS-1). This mirrors H2a for houses and
  G1 for guilds.
- Until PREM-2 and the promotion NPC (PREM-5) exist, no character is promoted, so no character is
  eligible: W-1 and W-2 can land first and store allocations that count from then on.

### 6.3 Points

`max(0, level - 50)` plus extra points (0 until scrolls, the monk quest and Grade IV mods exist).

## 7. Wire (W-2)

### 7.1 Capability and commands

Capability `WHEEL_V1`; its number, two command types and one domain are reserved on #162 at
allocation.

- **`WHEEL_QUERY`:** the acting character's slots, `wheel_revision`, available and unused points,
  derived stages, and its eligibility with the failing reason (`NO_VOCATION`, `LEVEL`,
  `NOT_PROMOTED`, `NOT_PREMIUM`).
- **`WHEEL_INTENT set_allocation {slots[36], expected_wheel_revision}`:** a full replacement,
  validated as §4. Results: `OK`, `NOT_ELIGIBLE`, `OVER_CAPACITY`, `OVER_POINTS`, `NOT_ADJACENT`,
  `REMOVAL_NOT_AT_TEMPLE`, `RULESET_NOT_CURRENT`, `STALE_REVISION`, `NO_CHANGE`, plus the common results.
- **Domain:** the acting character's derived stages, so the client shows spell availability.

### 7.2 Why a full replacement

The client plans a whole configuration (the Tibia window applies a planned set), and one validated
replacement is one receipt; no per-point command stream.

### 7.3 Temple

A decrease is admitted only while the character stands in a protection zone within 10 tiles of a
town temple position (Canary `getOptions`; the manual says "at a temple"). Temple positions are
the town records' temple positions in the active bundle.

## 8. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `WHEEL0-RL-01` slots | 36 (4 domains × 9) |
| `WHEEL0-RL-02` revelation thresholds | 250, 500, 1000 domain points |
| `WHEEL0-RL-03` augment stages | 0 to 2 |
| `WHEEL0-RL-04` unlock level | above 50; 1 point per level |
| `WHEEL0-RL-05` removal radius | protection zone within 10 tiles of a temple |
| `WHEEL0-RL-06` receipt size | 2 × 36 small integers plus fixed fields |
| Allocation change | 0 items, 0 value lines, 1 CharacterRevision, 1 receipt |
| Wheel reset (WHEEL0-RST-1) | 0 items, 0 value lines, 1 CharacterRevision, 1 receipt, once per character per reset revision |

## 9. Rejected options

- **Refusing Wheel spells at the reader.** It blocks the level spells whose augment is optional.
- **Canary's opaque slot blob.** DUR-02 rule 10 names Wheel points as a typed Character relation.
- **Per-point commands.** Many receipts for one plan; the full replacement is one.
- **Wheel state outside CharacterRevision.** It is Character build state (DUR-02 rule 10).
- **Waiting for gems.** The state candidate put gems out of scope; revelation stages need no gems.

## 10. Owner-rule applications (Global parity, 5905825574)

Kept as in Tibia: unlock at 51 for promoted Premium characters, 1 point per level, 36 slices,
adjacency, the three perk tiers, removal at a temple, suspension on lapse. Declared differences:
unused points saturate at 0 on level loss (Canary underflows); the Premium rule of promotion, the
Wheel and the revelation spells waits until Premium is delivered (W1 a, WHEEL0-PS-1); a Wheel
ruleset change that could invalidate an allocation resets it at the next login (W2, WHEEL0-RST-1).

## 11. Decision test

- **Must decide now:** YES. The owner reports the Wheel blocks the remaining spells.
- **Minimum sufficient:** one cast-time input now; one allocation table pair and receipt; one
  replacement command; effects as contributions.
- **Superseding evidence:** WHEEL-GEM-0 adds gem bonuses to the domain sums.
- **Deliberately not decided:** gems, vessels, fragments, mod grades, presets, scrolls, the client
  window.
- **Presets (owner note with W2: players keep saved Wheel profiles):** when a Wheel reset happens,
  the later presets child must drop or mark invalid every preset that is invalid under the active
  ruleset.

## 12. Owner questions (answered)

Owner answer, verbatim record on #162 5917665342: "3a" (W1 a).

**W2. Wheel ruleset changes (owner decision, #162, 2026-09-30).** The ruleset migration writer
(former WHEEL0-MIG-1, `migrate_character_wheel`) is dropped. A value-only ruleset change keeps
allocations, pinned to their revision; a change that could invalidate an allocation is activated as
a Wheel reset, applied at the character's next admitted session under its own session fence, free,
idempotent, fail-closed until it commits, and recorded by one receipt naming the old and new
ruleset revisions (WHEEL0-RST-1, §4, §5.1).

**W1. Promotion and the Wheel before Premium exists?** Tibia requires Premium for promotion and
for the Wheel, and the Game has no Premium yet (PREM-1 is in review). a) Not required until Premium
is delivered, then the Tibia rules apply, and a promotion bought before is kept (recommended: the
same as your answers for houses and guilds; revelation spells become reachable before Premium);
b) keep the requirement: revelation spells wait for Premium.

## 13. Before-freeze checklist

1. **Contract amendments:** the Wheel state candidate §3.2.1, §3.5, §5 and §6 pointers; PREMIUM-ACTIVATION
   §4.2, §4.6 and §5 (PREM-2, PREM-4 and PREM-5 rows); the PREMIUM-DELIVERY-0 implementation brief
   (PREM-2..5 row); each written "pending on acceptance of WHEEL-0".
2. **Serialization:** W-1 on the CHAR-REV-SEQ-1 sequencer; composition rule 2 fence first; the
   Wheel receipt carries the common envelope and the full-chain and row-tip guards (§4).
3. **Restart:** receipts keyed by occurrence; replays return the first outcome; a Wheel reset's
   occurrence is derived from the character and the source and destination revisions.
4. **Typed references:** CharacterId, slot index, ruleset revision, occurrence.
5. **Wire:** §7, capability `WHEEL_V1`.
