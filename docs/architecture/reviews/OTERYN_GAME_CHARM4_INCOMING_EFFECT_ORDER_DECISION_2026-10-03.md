# CHARM-4 Incoming Charm Effect Order

- Decision: `CHARM4-INCOMING-EFFECT-ORDER-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat and
  determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  - control-plane allocation D300 (#1622), under owner extension D296;
  - the CONFLICT "incoming effect order" and "Parry armor callsites" of
    `docs/agents/tasks/archive/OTV2-20261001-charm-mechanics-preparation.md`;
  - the open acceptance item "Dodge, Parry and Void Inversion have accepted incoming HP/mana
    application and ordering" of `OTV2-20261001-charm-runtime-completion`.
- Builds on:
  - CHARM-0 (the 25 charms; owner answer 5a: follow TibiaWiki on source conflicts; Parry reflects
    as neutral, displayed as physical);
  - `apps/game-server/src/combat/charm_effects.rs` (CHARM-4: hooks, categories, the trigger draw
    `hook × 2 + category`, outcomes sorted by hook then category, no chaining);
  - ATTACK-0 §4 (creatures hit back through the same pipeline with the incoming hooks);
  - CONDITIONS-0 (conditions attached to attacks; §3.2: each tick is an occurrence and defensive
    charms apply to it; §3.3: the mana shield after block, mitigation and defensive charms);
  - `tools/content-schema/charm-authoring/INTEGRATION.md` (source discrepancies and Global
    evidence);
  - owner rule 5905825574.
- Amends: none. The draws and their order are what `evaluate_charm_hook` already returns; this
  decision fixes where the incoming consumer applies each outcome relative to the damage commit.
  CONDITIONS-0 §3.2 (defensive charms on ticks) and §3.3 (mana shield after the defensive charms)
  are applied as written (§3, §4).
- Runtime, migration and production authority: NONE. Each child needs its own #1622 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| CHARM-DEF-1 | combat lane, combat and determinism review | the incoming pipeline of §3 on ATTACK-1's creature attack: Dodge, the hit gate, Parry, the commit after the charms, the suppression after a dodge, the liveness check of §4 | ATTACK-1 (creature attacks on characters) |

The minor effects themselves (Numb, Adrenaline Burst, Cleanse) and Void Inversion keep their
consumers in the runtime-completion task; they plug into steps 5 and 7 of §3.

## 1. Question

When a creature attacks a character with charms on its race, in which order do Dodge, Parry, the
defensive minor charm and Void Inversion run, on which damage value, and what does a dodge cancel?

## 2. Facts

**PROVEN** (repository)

- Dodge runs on hook 3 `IncomingCreatureAttack`, before damage. Parry, Numb, Adrenaline Burst and
  Cleanse run on hook 4 `IncomingCreatureHit`. Void Inversion runs on hook 5
  `IncomingManaDrain`.
- Dodge and Parry are major; Numb, Adrenaline Burst and Cleanse are minor; a race holds at most
  one major and one minor charm, so hook 4 runs at most Parry and one minor.
- Each roll is its own draw, `hook × 2 + category`, keyed by the occurrence. The order of
  evaluation never changes a draw.
- `evaluate_charm_hook` returns outcomes sorted by (hook, category): major before minor.
- `IncomingCreatureHit` carries `base_damage`, the creature's damage before the character's own
  resistances and armor, and Parry's outcome reflects that amount.
- All four defensive families fail closed today (`IncomingCreatureDamage`, `ParalysisCondition`,
  `HasteCondition`, `ConditionCleanse`).

**TIBIAWIKI_STRUCTURED** (CHARM-0 answer 5a makes the wiki govern source conflicts)

- Parry revision 1084043: the reflected base is the damage before the player's resistances.
- Parry: the creature's armor reduces the reflected damage.
- The defensive minors act "after" the creature's attack or hit.

**CIPSOFT_OFFICIAL**

- Archive 4386: reflected Parry damage ignores the monster's resistances, and the monster's armor
  applies.

**OTS_HYPOTHESIS_ONLY**

- Canary runs the defensive minor before Dodge; Crystal runs it after Dodge.
- Both OTS Parry handlers support armor, but their call sites leave it off.

**UNKNOWN**

- Whether a dodge also cancels conditions attached to the attack. A player report says it does;
  no current test exists.

## 3. The incoming pipeline (CHARM-DEF-1)

One creature attack on a character is one occurrence. Its components (health damage of one or more
elements, an explicit mana drain, attached conditions) are handled together, inside the combat
owner's commit of that occurrence. All charm draws of the occurrence happen before anything of it
is committed.

1. **Dodge (hook 3).** Evaluated once per occurrence, before block, armor, resistances and mana
   shield. On success, the whole occurrence is cancelled: no damage, no mana drain, no attached
   condition (R3), and steps 2 to 7 do not run. Nothing is rolled for them.
2. **Mitigation.** The combat owner computes the health damage after block, armor and resistances.
   Nothing is committed yet, and the mana shield is not applied yet (CONDITIONS-0 §3.3).
3. **Hit gate.** Hook 4 runs only when the occurrence has a health-damage component and its health
   damage after step 2 is greater than 0. A fully blocked attack is not a hit. Damage the mana
   shield will take in step 6 still counts: the shield splits a hit, it does not undo it. An explicit
   mana drain is not a hit: an occurrence that is only a mana drain skips hook 4 (its
   `base_damage` would be 0, which `evaluate_charm_hook` refuses).
4. **Hook 4 draws, then Parry.** Hook 4 is evaluated once; its outcomes come back as Parry, then
   the minor. **Parry** reflects `base_damage`: the sum of the occurrence's health-damage
   components as the creature rolled them, before the character's block, armor, resistances and
   mana shield (R2). The reflected damage is `CharmDamage` (no chaining), neutral and shown as
   physical (CHARM-0 5a), reduced by the creature's armor and not by its resistances (archive
   4386).
5. **Void Inversion (hook 5), before the drain is committed (R5).** Hook 5 runs only when the
   occurrence has an explicit mana drain greater than 0. On success the drain is not applied:
   the character gains `mana_gained`, the amount the drain would have taken, capped at maximum
   mana, as `InvertManaDrain` says ("gained instead of lost"). There is no loss to compensate.
6. **Commit.** The combat owner commits the occurrence in one step, after the defensive charms as
   CONDITIONS-0 §3.3 orders: the mana shield takes its share of the step-2 health damage, health
   takes the rest, the explicit drain is applied unless step 5 inverted it, and the attached
   conditions are applied.
7. **The minor** (Numb, Adrenaline Burst or Cleanse), drawn in step 4, is applied after the commit:
   the minors act after the hit. Cleanse chooses among the conditions active after step 6, so a
   condition the attack just applied can be removed.

**Mixed occurrences (R6).** An occurrence with both health damage and an explicit mana drain runs
both hooks on their own components: hook 4 on the health damage (steps 3, 4, 7), hook 5 on the
drain only (step 5). Each hook is its own draw, so one never changes the other. Mana the character
loses through the mana shield is health damage redirected (CONDITIONS-0 §3.3), not a drain: Void
Inversion never applies to it.

## 4. Applying outcomes in order

- The consumer draws the outcomes of one occurrence in the order returned (hook, then category)
  and applies them at the steps of §3: Parry (step 4) and Void Inversion (step 5) before the
  commit, the minor (step 7) after it. Within each step the returned order holds, so Parry is
  always applied before the minor.
- An effect that acts on the creature (Numb) and finds it dead after an earlier effect of the same
  occurrence (Parry killed it) applies nothing and is recorded as `NoLivingTarget`. Effects on the
  character (Adrenaline Burst, Cleanse) still apply.
- A creature killed by Parry gives the character the kill as a `CharmDamage` kill: no charm
  evaluates for that kill (no Carnage).
- **Condition ticks (CONDITIONS-0 §3.2, unchanged).** Each damage tick is its own occurrence and
  runs §3 like any creature attack: Dodge on the tick, then the hit gate, Parry on the tick's
  damage before the character's mitigation, the commit, and the minor. A tick runs charms only when its frozen
  source is a creature that is still present (§3.2's source lookup), since the race comes from it;
  a tick with no attacker evaluates no charm. Parry's reflection goes to that source creature.

## 5. Rejected options

- **Minor before Dodge (Canary).** A dodged attack is not a hit, and the minors act after a hit.
- **Parry on the mitigated damage.** TibiaWiki (CHARM-0 5a) says before the player's resistances;
  the earlier preparation recommendation was withdrawn on the same evidence.
- **Parry without the creature's armor (OTS call sites).** TibiaWiki and archive 4386 apply armor.
- **Void Inversion after the drain is committed.** The outcome says the mana is gained instead of
  lost; applying it after the loss would need a compensation and briefly expose the lowered mana.
- **The mana shield before the defensive charms.** CONDITIONS-0 §3.3 puts the shield after them.
- **Rolling hook 4 charms on a dodged attack and discarding them.** Same draws, but it records
  outcomes for an occurrence that did not happen.

## 6. Architect rulings (owner rule 5905825574)

- **R1, minor timing: a) after Dodge, only on a committed hit** (Crystal, the "after" wording);
  b) before Dodge (Canary). Recommendation and ruling: a).
- **R2, Parry base: a) the creature's damage before the character's mitigation** (TibiaWiki);
  b) the mitigated damage. Recommendation and ruling: a), `PARITY_PENDING` until the owner's
  live check (CHARM-0 5a) confirms it.
- **R3, a dodge and attached conditions: a) cancelled with the damage**; b) delivered anyway.
  Recommendation and ruling: a), `PARITY_PENDING` (player report only).
- **R4, order inside hook 4: a) Parry, then the minor**, as the code returns; b) the minor first.
  Recommendation and ruling: a); with separate draws only a Parry kill before Numb differs.
- **R5, Void Inversion: a) before the drain is committed, replacing the loss** (`InvertManaDrain`,
  "instead of lost"); b) after the commit, with a compensation. Recommendation and ruling: a).
- **R6, mixed occurrences: a) both hooks, each on its own component; mana-shield loss is never a
  drain**; b) exclude mixed occurrences from hook 5. Recommendation and ruling: a), the draws are
  independent and the shield redirects damage.

## 7. Owner questions

None. Every choice above is a reversible architect ruling under owner rule 5905825574. R2 is
already covered by the owner's live check of Parry under CHARM-0 answer 5a.

## 8. Decision test

- **Must decide now:** YES. Control-plane allocation D300 (owner D296); the runtime-completion task
  cannot close its incoming item without it.
- **Blocked without it:** CHARM-DEF-1 and the incoming consumers of the minors and Void Inversion.
- **Harder later:** the order shapes replay fixtures of incoming combat once ATTACK-1 ships.
- **Supersede if:** the owner's live Parry check disagrees with R2; a current test of Dodge with
  attached conditions; official evidence on the minor timing.
- **Deliberately not decided:** Cleanse eligibility and immunity; the speed formulas of Numb and
  Adrenaline Burst; PvP (charms apply to creatures only).

## 9. Before-freeze checklist

1. **Contracts:** none amended.
2. **Serialization:** one occurrence, steps in §3 order, inside the owner's commit of that
   occurrence: every draw before the commit; Parry and Void Inversion before it, the minor after.
3. **Restart:** nothing durable; replay of a committed occurrence gives the same draws and order.
4. **Typed references:** CharacterId, race key, occurrence id, charm key.
5. **Wire:** none.
6. **Split work:** at most one major and one minor per race; one roll each per occurrence.
