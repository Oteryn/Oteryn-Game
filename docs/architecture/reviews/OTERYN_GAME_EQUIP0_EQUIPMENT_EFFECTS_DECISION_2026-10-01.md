# EQUIP-0 Equipment effects

- Decision: `EQUIP0-EQUIPMENT-EFFECTS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat and
  determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the base-mechanics close-out plan (#162 5929069698, item 2): no decision makes worn
  equipment give its protections, skill and stat boosts, speed or light. SKILLS-0 left "skill boosts
  from equipment"; CONDITIONS-0 §4.1 names a worn-equipment speed term without an owner.
- Scope cut: items whose effect runs on time or charges (rings, soft boots, charged amulets, lit
  torches), equip and unequip forms, and repairs are **TIMED-ITEM-0**, a later decision: they touch
  item moves, trade, death drops and the wire, and repair is a new fee source that needs an owner
  answer. Until TIMED-ITEM-0, such items grant nothing (§3.2). This decision writes nothing durable.
- Builds on: GAME-ITEM-01 §4.8 and §6.4 (typed modifiers, the deterministic evaluation plan),
  ITEM-MOVE-WIRE-1 §4 (equip, requirements, Premium), IMBUE-FORGE-0 §5 (candidate: protections summed
  at GAME-ABILITY-01 §10), CONDITIONS-0 §3 and §4.1, WORLD-INTERACTION-0 §11.4 (item light),
  SKILLS-0, RANGED-0 §3.3 (the Extra slot, candidate, PR #1439), owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| EQUIP-CONTENT-1 | content lane | typed ability rows on Item definitions (§3.1) from TibiaWiki first, Canary `items.xml` as fallback, with unit conversion (§3.1); items with time or charges flagged `timed` | ITEM-SEM-2b |
| EQUIP-RT-1 | hard (combat), combat and determinism review | the equipment owner's active set and the Reference evaluation plan (§3); effects at their stages (§4); recomputation triggers | ITEM-MOVE-2a; SPEED-1; COND-1 |
| EQUIP-PARITY-1 | impl | fixtures: protection sums and signs, skill, stat and speed values against TibiaWiki | EQUIP-RT-1 |

No new command, capability or durable state. Effects show through existing views: skills and stats
(A13, SKILLS-0), speed (VIS-2), light (VIS-2 `light`).

Later, each with its own decision: TIMED-ITEM-0 (time, charges, equip forms, repair, torches,
equipment regeneration and mana shield), invisibility from equipment (with the invisibility family),
Global special skills from equipment (critical hit, life and mana leech), set bonuses (none in
Tibia).

## 1. Question

What does a worn item without time or charges do for its wearer, and in what order?

## 2. Facts

**PROVEN**

- GAME-ITEM-01 §4.8: modifiers are typed and bounded, "no arbitrary free-form attribute map". §6.4:
  every modifier is a typed definition (target, phase, priority, parameters, key), and every
  consuming ruleset publishes one deterministic evaluation plan.
- ITEM-MOVE-WIRE-1 §4: level and vocation are checked at equip; an item stays equipped when they
  drop; Premium benefits stop when Premium ends.
- IMBUE-FORGE-0 §5 (candidate): protections are "absorb percent, summed with other absorbs" at
  GAME-ABILITY-01 §10; skill boosts are a derived read, not a build-state write.
- CONDITIONS-0 §4.1: effective speed adds "worn-equipment speed (from the item definitions, summed by
  the equipment owner)". §3: conflict keys; "later families (... skill boosts ...)".
- WORLD-INTERACTION-0 §11.4: creature light is the higher of the `LIGHT` condition and the brightest
  equipped item.
- SKILLS-0: "skill boosts from equipment and imbuements" are deferred.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- Abilities apply when an item is equipped in its own slot and its requirements hold
  (`movement.cpp:508-636`): speed, condition suppressions, flat skill and stat modifiers, percent
  stats.
- Protections (`player.cpp:3913-3967`): after armor and defence, per enabled slot in slot order,
  imbuement absorbs then the item's signed `absorbpercent` (plus `fieldabsorbpercent` against
  fields), each applied to the remaining damage.
- `items.xml` speed is in the engine's doubled unit (boots of haste `speed="40"` shows as +20 in the
  client; `PARITY_PENDING`, checked by EQUIP-PARITY-1).

## 3. The equipment owner (EQUIP-RT-1)

### 3.1 Typed abilities

| Ability | Parameters | Consumer |
|---|---|---|
| `SKILL_BOOST` | skill (the seven weapon skills, magic level), signed flat amount | derived skill read (SKILLS-0, A13) |
| `STAT_BOOST` | max health or max mana, signed flat or percent | derived vitals maxima |
| `SPEED` | signed flat amount, in CONDITIONS-0 §4.1's displayed units (content converts Canary's doubled unit) | CONDITIONS-0 §4.1 equipment term |
| `PROTECTION` | element (the GAME-ABILITY-01 combat types), signed percent in [-100, 100], optional `field_only` (damage whose origin is a field) | GAME-ABILITY-01 §10 incoming contribution |
| `SUPPRESS` | a CONDITIONS-0 conflict key | CONDITIONS-0 admission (§4) |
| `LIGHT` | level, colour | WORLD-INTERACTION-0 §11.4 |

### 3.2 When an item is active

- An item is **active** when it sits in the slot its definition names, it met its requirements at
  equip (ITEM-MOVE-WIRE-1 §4), its Premium requirement holds now, and it is not flagged `timed`.
- A `timed` item grants nothing until TIMED-ITEM-0 decides its time or charges (fail closed: no free
  rings or amulets).
- An item in the Extra slot (`ammo`, RANGED-0 §3.3) contributes only `LIGHT` (the manual's torch).
- The channel runtime's equipment owner keeps each actor's active set and recomputes it on equip,
  unequip, death, channel transfer, respawn and Premium changes. It writes nothing durable; after a
  transfer or restart the set is derived again from the equipped items.

### 3.3 The Reference evaluation plan (GAME-ITEM-01 §6.4)

- Contributions are collected in slot order (head, necklace, container, armor, right hand, left
  hand, legs, feet, ring, `ammo`), then definition key, then ability order on the definition.
- Flat skill, stat and speed boosts sum. Percent stat boosts apply to the base value; flats then add.
  Every product is computed in i64 and truncated toward zero once (CONDITIONS-0 §3 arithmetic).
- **Protections sum** per element across items, imbuements (IMBUE-FORGE-0 §5) and other sources at
  the GAME-ABILITY-01 §10 incoming stage, after armor and defence; the sum is clamped to [-100, 100]
  and applied once: `damage - trunc(damage × sum / 100)`. Canary applies them one slot after
  another; the sum is IMBUE-FORGE-0's rule and stays `PARITY_PENDING` until EQUIP-PARITY-1 checks it
  against TibiaWiki.

## 4. Effects (EQUIP-RT-1)

- Skills, stats, speed, protections and light are derived reads; nothing is written to A13 or the
  build state.
- `SUPPRESS` makes CONDITIONS-0 refuse admission of an instance with that conflict key while the item
  is active, and removes an existing one when it becomes active (Canary `conditionSuppressions`).
  Amended: CONDITIONS-0 §3 (this PR).
- A drop of max health or max mana below the current value clamps the current value (Canary).

## 5. Rejected options

- **Abilities as conditions with durations.** A timed condition would outlive an unequip.
- **Writing derived skills to the build state.** SKILLS-0 and IMBUE-FORGE-0 keep boosts as derived
  reads.
- **Multiplicative protections now.** IMBUE-FORGE-0 sums; one rule for every source, checked by parity
  fixtures.
- **Timed items in this decision.** Their time and charges touch moves, trade, death and the wire; a
  half model would lose or create value (the self-review of this decision's first draft).

## 6. Architect rulings (owner rule 5905825574)

- **R1. Protection stacking.** a) Sum, clamp to [-100, 100], apply once, `PARITY_PENDING`
  (recommended: one rule with IMBUE-FORGE-0); b) Canary's per-slot application. **Ruled a).**
- **R2. Timed items.** a) Grant nothing until TIMED-ITEM-0 (recommended: fail closed); b) grant their
  effect without consuming time. **Ruled a).**

## 7. Owner questions

None here. TIMED-ITEM-0 will carry the owner question on soft boots repair (a new fee source).

## 8. Decision test

- **Must decide now:** YES. Without it no armor protects against elements and no skill or speed item
  works.
- **Minimum sufficient:** one closed ability set, one evaluation plan, no durable state.
- **Superseding evidence:** TibiaWiki or an official source showing multiplicative protections.
- **Deliberately not decided:** timed and charged items, equipment regeneration and mana shield,
  invisibility, special skills, imbuement effects (IMBUE-FORGE-0).

## 9. Before-freeze checklist

1. **Contract amendments:** CONDITIONS-0 §3 (`SUPPRESS`); SKILLS-0 (skill boosts point here). Applied
   in this PR.
2. **Serialization:** runtime-only, inside the channel owner's tick.
3. **Restart:** nothing durable; the active set is derived from the equipped items.
4. **Typed references:** definitions are A12 keys; abilities are a closed typed set.
5. **Wire:** none new.
6. **Split work:** none.
