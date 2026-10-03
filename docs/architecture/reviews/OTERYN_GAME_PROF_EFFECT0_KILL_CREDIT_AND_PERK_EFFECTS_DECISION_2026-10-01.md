# PROF-EFFECT-0 Weapon Proficiency kill credit, points and perk effects

- Decision: `PROF-EFFECT0-KILL-CREDIT-AND-PERK-EFFECTS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat and
  determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane's request for the Weapon Proficiency multiplayer kill credit and the
  phase, stacking and rounding of the 33 perk kinds, as a separate decision. It also answers
  PROFICIENCY-0 §4.5's parity gates for multi-player weighting and the point table.
- Builds on: PROFICIENCY-0 §4.3 (accrual in the live session, checkpoints), CHARM-0 owner answers 6a
  (5-minute damage window), 12b (proc commits) and 14a (caller-supplied damage time), D186 (leech
  order), D199 (perk mapping and units), EQUIP-0 §3.3 (evaluation plan), IMBUE-FORGE-0 §5 and §12,
  RANGED-0 §5, GAME-ABILITY-01 whole gate §10 and §11, the FORMULA source order (owner rule
  5905825574).
- Runtime, migration, content and production authority: **NONE**. Each child needs its own #162
  allocation. Nothing durable is added: accrual and checkpoints stay as PROFICIENCY-0 decided.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| PROF-CONTENT-2 | content lane | the point table (§3.2) as content rows; the `bestiary_class_id` and `spell_client_id` crosswalks (§4.1) with coverage reported | PROF-CONTENT-1 |
| PROF-2 (amended) | hard (combat), combat and determinism review | kill credit (§3.1) and points (§3.2) in the accrual PROFICIENCY-0 §4.3 already assigns to PROF-2; the active perk set (§4.1) and every effect at its stage (§4.3) | ATTACK-1; RANGED-1; SKILLS-0's live skill read; EQUIP-RT-1 |
| PROF-PARITY-1 | impl | fixtures for §3 and §4.3 against TibiaWiki, TibiaPal and Canary | PROF-2 |

No new command, capability, state domain or durable table.

## 1. Question

Who earns Weapon Proficiency progress when several characters damage one creature, how much, and
where and how does each of the 33 perk kinds act on combat?

## 2. Facts

**PROVEN (official)**

- Tibia manual §5.3.4 (`docs/reference/tibia-manual/combat.md`): progress comes from monsters
  defeated while the weapon is equipped, by Bestiary-style rules: "all damage-contributing players
  get credit".

**PROVEN (English TibiaWiki, `Weapon_Proficiency`, read 2026-10-01)**

- "Anyone who contributed damage gains progress", as for Bestiary and Bosstiary.
- Only the weapon equipped when the creature dies is rewarded, even if other weapons did the damage
  or it was not equipped then. Runes and fields count. A weapon that breaks on the killing blow still
  earns.
- Creatures without a Bestiary or Bosstiary entry give nothing. A summoned creature's kill gives
  nothing.
- Points per kill by Bestiary difficulty and influence stacks, and by Bosstiary category: the table in
  §3.2.
- Perks of a weapon whose class requirements the character does not meet stay inactive; progress
  still accrues.
- D199's perk table (TibiaWiki `Weapon_Proficiency_Tables`, revid 1206177): Alpha Strike acts on
  targets above 95% health, Omega Strike below 30%.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- Points go only to the last hitter (`monster.cpp:3232-3301`), the base by Bestiary stars through a
  polynomial that gives 1, 30, 70, 100, 165, 240 for 0-5 stars (`weapon_proficiency.cpp:802-808`),
  bosses 500, 5,000 and 15,000 (`:788-800`). The official rule overrides the last-hitter choice; the
  base column and boss values agree with TibiaWiki.
- Life and mana on hit run after the block check, on every damaging hit by the player
  (`combat.cpp:883-884`); on kill, for the last hitter (`monster.cpp:3280-3281`).
- Attack range adds to the weapon's shoot range (`weapons.cpp:136`); ranged hit chance adds to the
  bow's hit chance (`weapons.cpp:831`); skill-percentage perks add to the damage before the block
  check (`combat.cpp:870-872`).
- Canary implements neither Alpha nor Omega Strike, armor penetration, elemental pierce nor the
  homing missile.

**Existing rows.** `COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE` 16 distinct characters per creature
generation; `COMBAT01-REWARD-PRINCIPALS` 1 (XP only); `ABILITY01-CALC-STAGES` 8.

## 3. Kill credit and points (PROF-2)

### 3.1 Who earns (architect ruling R1)

- At a creature's death, every character in the creature's damage-contributor record
  (`COMBAT01-DAMAGE-CONTRIBUTORS-PER-CREATURE`) whose last damage to it is within 5 minutes before
  the death (CHARM-0 6a, the time supplied by the caller as 14a) earns the **full** points for the
  weapon equipped at the death, in its own live session. Points are not divided, as Bestiary kills
  are not.
- Damage by a character's summon counts as the character's. Damage by runes and fields counts.
- The weapon is the item key in the character's weapon slot at the death; for the character whose
  own blow killed, it is the key as that blow started, so a weapon that broke or was used up by the
  killing blow still earns (TibiaWiki). Progress is per proficiency track, not per instance.
- No points when the creature has no Bestiary or Bosstiary entry, is a summon, or the character has
  no equipped weapon with a promoted proficiency (PROFICIENCY-0 §4.1). A character whose actor is not
  in the creature's channel at the death earns nothing.
- This is independent of `COMBAT01-REWARD-PRINCIPALS`, which governs XP. Accrual is already
  session-local and durable only at PROFICIENCY-0's checkpoints, so crediting several characters adds
  no durable write per kill. The death settlement hands each credited session one bounded accrual
  input `{creature definition, influence stacks, death occurrence, weapon item key}`. The settlement
  freezes the weapon item key per credited character inside the death commit, from the actor state
  it already reads (for the killer, the key captured when the killing blow started), so a weapon the
  blow broke or used up is still credited; the session never re-reads its slot after the commit. The
  session adds the points to that key's track, or nothing when the key has no promoted proficiency. The settlement emits these inputs once, after its commit; a
  settlement resumed after a channel restart emits none, since those sessions' pending progress was
  lost with the restart anyway (PROFICIENCY-0 §4.3). As a second guard a session ignores a death
  occurrence it already accrued (it keeps the last `PROFEFF0-RL-01`).
- More than 16 contributors is bounded by the existing row; a contributor the record dropped earns
  nothing. Tibia has no such bound: a declared difference owned by that row, not by this decision.

### 3.2 Points (content, PROF-CONTENT-2)

TibiaWiki's table enters as content rows, literally (no formula), keyed by the creature's Bestiary
difficulty and influence stacks, or its Bosstiary category:

| Difficulty | 0 | 1 | 2 | 3 | 4 | 5 | Fiendish |
|---|---|---|---|---|---|---|---|
| Harmless | 1 | 1 | 1 | 1 | 1 | 1 | 2 |
| Trivial | 30 | 33 | 36 | 39 | 42 | 45 | 75 |
| Easy | 70 | 77 | 84 | 91 | 98 | 105 | 175 |
| Medium | 100 | 110 | 120 | 130 | 140 | 150 | 250 |
| Hard | 165 | 181 | 198 | 214 | 230 | 247 | 412 |
| Challenging | 240 | 264 | 288 | 312 | 336 | 360 | 600 |

Bosstiary: Bane 500, Archfoe 5,000, Nemesis 15,000. A creature with both entries earns its
Bosstiary value (Canary adds both; TibiaWiki lists one value per kill: `PARITY_PENDING`).

- Until influenced and fiendish creatures have an owning decision, every creature uses column 0.
- A creature definition without a difficulty or category that maps to this table earns nothing (fail
  closed); PROF-CONTENT-2 reports coverage against the 686 Bestiary definitions (CHARM-0).
- The level thresholds already come from TibiaWiki through the proficiency catalogue's
  `threshold_tables`; nothing changes there.

## 4. Perk effects (PROF-2)

### 4.1 The active set

- A perk is active when its slot's selection is committed (PROFICIENCY-0 §4.3), its weapon is the
  character's equipped weapon and active under EQUIP-0 §3.2, the character meets the weapon's class
  requirements (TibiaWiki), and its track is not waiting for a migration receipt (PROFICIENCY-0
  §4.1). Only the equipped weapon's track contributes. A modified perk (PROFICIENCY-1) is not active
  until PROFICIENCY-1's value shapes are admitted.
- A perk that names another family (`spell_client_id`, `bestiary_class_id`) is active only when
  PROF-CONTENT-2's crosswalk maps that id; an unmapped one is inactive and reported.
- The runtime keeps the active set as a derived read, recomputed with EQUIP-0's triggers and at each
  perk selection commit. It writes nothing.

### 4.2 Units, stacking and rounding (architect ruling R2)

- Units are D199's: flat for Types 0-4, 18-22 and 24; an exact fraction otherwise (`0.05` = 5%); a
  spell `Cooldown` augment is negative seconds.
- **One rule with EQUIP-0 and IMBUE-FORGE-0.** Contributions to one quantity (a skill, a crit chance,
  a leech share, a damage percent) are summed in exact rationals across every source that the
  quantity's stage reads: proficiency, equipment, imbuements, tier, Wheel and charms. The sum is
  clamped where the quantity has a range (a chance to [0, 1]; a cooldown to at least 0) and applied
  once; every product is computed in i64 and truncated toward zero once, as EQUIP-0 §3.3.
- **Percent damage terms** of proficiency (§4.3 rows marked "dmg %") sum into one proficiency term
  `P` per hit, applied as `damage + trunc(damage × P)`. Its place among other sources' multiplicative
  terms is GAME-ABILITY-01 §10's contribution order for the hit, as each source's own decision places
  it; this decision adds no stage, so `ABILITY01-CALC-STAGES` stays 8. The order between the
  proficiency term and other terms is `PARITY_PENDING` until PROF-PARITY-1 checks it against
  TibiaPal.
- Evaluation order inside the term follows EQUIP-0 §3.3: level order, then the perk's source index.

### 4.3 Each kind

"Hit" means a damaging hit by the character that passed the block check. "Auto-attack" means an
ATTACK-0 or RANGED-0 swing. RNG purposes are new names under the existing SIM-DETERMINISM streams.

| Kind (D199) | Unit | Stage | Rule |
|---|---|---|---|
| `AttackDamage` | flat | §10 outgoing, auto-attack | adds to the weapon's attack value in the ATTACK-0/RANGED-0 formula (`PARITY_PENDING`: Canary adds it to base damage) |
| `Defence` | flat | §10 incoming, defence | adds to the weapon's defence |
| `WeaponShieldDefence` | flat | §10 incoming, defence | adds to the defence the weapon gives in place of a shield |
| `SkillBonus` | flat | derived skill read | adds like EQUIP-0 `SKILL_BOOST`; it never writes A13 |
| `SpecializedMagicLevel` | flat | derived read | adds to magic level for spells and runes of its element only (Healing: healing spells) |
| `SpellAugment` `BaseDamage`, `Healing` | dmg % | §10 of that spell | joins `P` for that spell's hits or heals |
| `SpellAugment` `Cooldown` | seconds | cooldown owner | reduces that spell's cooldown, clamped at 0 |
| `SpellAugment` `LifeLeech`, `ManaLeech` | fraction | §11 descendant | adds to the leech share for that spell's hits |
| `SpellAugment` crit chance and extra | fraction | §10 critical | adds to the crit quantities for that spell's hits |
| `BestiaryClassDamage` | dmg % | §10 outgoing | creatures of that Bestiary class |
| `BossDamage` | dmg % | §10 outgoing | bosses, and influenced and fiendish creatures once they exist (Canary "powerful foe") |
| `CriticalHitChance`, `CriticalExtraDamage` | fraction | §10 critical | all the character's damaging hits |
| `ElementalCriticalHitChance`, `ElementalCriticalExtraDamage` | fraction | §10 critical | hits of that element |
| `RuneCriticalHitChance`, `RuneCriticalExtraDamage` | fraction | §10 critical | rune hits |
| `AutoAttackCriticalHitChance`, `AutoAttackCriticalExtraDamage` | fraction | §10 critical | auto-attack hits |
| `LifeLeech`, `ManaLeech` | fraction | §11 descendant | summed with imbuement leech in D186's order |
| `LifeOnHit`, `ManaOnHit` | flat | §11 descendant | per hit, a heal or mana gain of the value (Canary) |
| `LifeOnKill`, `ManaOnKill` | flat | death settlement | for the last hitter only (Canary; `PARITY_PENDING`) |
| `DamageAtRange` | dmg % | §10 outgoing, auto-attack | the Perfect Shot: a distance swing at exactly `range` tiles (Chebyshev) |
| `RangedHitChance` | flat % | RANGED-0 §5 hit chance | adds to the bow's hit chance before the draw, capped as RANGED-0 caps it |
| `AttackRange` | flat tiles | RANGED-0 §4 range | adds to the shoot range; the sum is capped at RANGED-0's 7 (`PARITY_PENDING`) |
| `SkillScaledAutoAttackDamage` | fraction | §10 outgoing, auto-attack | adds `trunc(skill × value)` damage, skill from the derived read |
| `SkillScaledSpellDamage`, `SkillScaledHealing` | fraction | §10 of spells | the same for spell damage or spell healing |
| `AlphaStrikeDamage` | dmg % | §10 outgoing | target health above 95% before the hit (TibiaWiki) |
| `OmegaStrikeDamage` | dmg % | §10 outgoing | target health below 30% before the hit (TibiaWiki) |
| `ArmorPenetration` | fraction | §10 target armor | the target's armor reduction is computed with `armor × (1 − value)`; physical hits only |
| `ElementalPierce` | fraction | §10 target resistance | the target's resistance to that element is reduced by `value` of itself before it applies |
| `HomingMissile` | chance, multiplier | §11 descendant | see below |

- **Critical.** One roll per hit (purpose `crit`), with the summed chance; on success the summed
  extra damage applies once. Tibia shows one Critical Hit Chance and one Critical Extra Damage for
  the character, so imbuement Strike (IMBUE-FORGE-0 §5) joins the same roll instead of rolling
  under `imbue_crit` (amended there; `PARITY_PENDING` until PROF-PARITY-1).
- **Reaction perks are inactive (R4).** `LifeLeech`, `ManaLeech`, the `SpellAugment` leech
  augments, `LifeOnHit`, `ManaOnHit`, `LifeOnKill`, `ManaOnKill` and `HomingMissile` act through
  post-commit descendant occurrences (GAME-ABILITY-01 §11). Their ceilings `AB-RL-14` to `AB-RL-16`
  (depth, fan-out, root work) are still `OWNER_DECISION_REQUIRED` and unregistered, and the
  first-slice limits disable post-commit reactions. These perks therefore stay **inert** (selectable,
  shown, no effect) until those rows are registered; that is the blocking dependency of this part
  of PROF-2, and this decision does not set the limits. The rules in the table above are what PROF-2
  builds once they are.
- **On kill and on hit** gains, once active, are healing and mana gain occurrences of origin
  `weapon_proficiency`; like CHARM-0 12b procs, they trigger no charm and no leech.
- **Homing missile.** After a hit with the weapon, with `probability` (purpose `prof_homing`), one
  descendant damage occurrence of the perk's element hits the same target for
  `trunc(parent damage × multiplier)`, shown with `missile_client_id`. It is its own commit after the
  parent, triggers no charm, leech, on-hit gain or homing, and counts for credit (CHARM-0 12b).
  Which hits can trigger it and what "damage" means (before or after mitigation) are not evidenced:
  the perk stays **inactive** until PROF-PARITY-1 evidences both (fail closed, R3) and R4's rows
  exist.
- **ArmorPenetration and ElementalPierce** follow D199's units; the formulas above are the reading
  of "armor penetration (1.0 = +100%)" and "elemental pierce". Both are `PARITY_PENDING`, and
  PROF-PARITY-1 checks them against TibiaPal before release.

### 4.4 Parity and release

Every `PARITY_PENDING` above is a release gate of PROF-2, not a build gate: PROF-2 builds the rule as
written, and proficiency is not enabled in production (PROFICIENCY-0 §4.5) until PROF-PARITY-1
confirms each or the owner accepts a `DECLARED_DIFFERENCE`. On a conflict between sources the
FORMULA order decides, and an unresolved conflict keeps the perk inactive.

## 5. Rows

| Row | Value |
|---|---|
| `PROFEFF0-RL-01` death occurrences remembered per session for credit replay | 64 |

Tested with max and max+1. Also tested: two characters damaging one creature both earn its full
points; a character whose last damage is 5 min + 1 ms old earns nothing; a summon's kill earns
nothing; a weapon switched in before the death earns, the one used earlier does not; a replayed death
adds nothing; an inactive perk of a weapon the character's class cannot use; crit chance from
imbuement and proficiency in one roll; a weapon broken by the killing blow is credited by its frozen
key; a leech or on-hit perk does nothing while `AB-RL-14`..`16` are unregistered; Alpha Strike at exactly 95% does not apply; a homing perk
does nothing.

## 6. Rejected options

- **Last hitter only (Canary).** The manual and TibiaWiki both credit every damaging player.
- **Dividing the points.** Bestiary credit is not divided; TibiaWiki gives the table per kill.
- **Crediting through XP reward principals.** That row is for XP and is 1 today; proficiency accrual
  is session-local and needs no shared settlement.
- **A new calculation stage for proficiency.** Every kind fits an existing stage; a new stage would
  raise `ABILITY01-CALC-STAGES`.
- **Guessing the homing missile.** Fail closed until evidenced.

## 7. Architect rulings (owner rule 5905825574)

- **R1. Credit.** a) Every damaging character within the 5-minute window earns full points for its
  equipped weapon (recommended: manual and TibiaWiki); b) last hitter only (Canary). **Ruled a).**
- **R2. Stacking.** a) Sum per quantity across sources, clamp, apply once, truncate once (recommended:
  one rule with EQUIP-0 and IMBUE-FORGE-0); b) apply each source in turn. **Ruled a)**,
  `PARITY_PENDING`.
- **R3. Unevidenced kinds.** a) Inactive until evidenced (recommended: fail closed); b) a guessed
  formula. **Ruled a)** for the homing missile only; the others have a source reading.
- **R4. Reaction perks.** a) Inert until `AB-RL-14`..`16` are registered (recommended: no limit is
  decided here; the owner question goes through the control plane when it is needed); b) set the
  limits here. **Ruled a).**

## 8. Owner questions

None. Every choice follows the official manual, TibiaWiki or an existing owner answer.

## 9. Decision test

- **Must decide now:** YES. PROF-2 cannot accrue for groups or apply any perk without it.
- **Minimum sufficient:** no durable state, no new stage, one row; content rows for the point table.
- **Blocked:** PROF-2's accrual for groups and every perk effect; PROF-CONTENT-2's point rows. The
  reaction perks additionally wait for `AB-RL-14`..`16` (R4).
- **Harder later:** crediting every contributor fixes the death settlement's fan-out to the damage
  record, so narrowing it later changes player-visible progress rates; one critical roll shared with
  imbuements (amended IMBUE-FORGE-0 §5) binds every later crit source to that roll; the point table as
  content rows ties PROF-2 to Bestiary difficulty and Bosstiary category keys.
- **Superseding evidence:** an official source on perk formulas or credit; TibiaPal fixtures.
- **Deliberately not decided:** influenced and fiendish creatures, proficiency catalysts, modified
  perks (PROFICIENCY-1), Mastery titles and achievements.

## 10. Before-freeze checklist

1. **Contract amendments:** PROFICIENCY-0 §4.3 (credit) and §4.5 (gates answered); IMBUE-FORGE-0 §5
   (one critical roll). Applied in this PR.
2. **Serialization:** no durable write per kill; accrual stays PROFICIENCY-0's checkpoint writer.
3. **Restart:** pending progress follows PROFICIENCY-0 (at most one checkpoint lost).
4. **Typed references:** creature definition keys, ItemInstanceId of the equipped weapon,
   death occurrence, CharacterId.
5. **Wire:** none.
6. **Split work:** content (PROF-CONTENT-2), runtime (PROF-2), parity (PROF-PARITY-1).
