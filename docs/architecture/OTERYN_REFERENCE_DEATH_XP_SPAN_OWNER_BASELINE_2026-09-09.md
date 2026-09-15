# Oteryn Reference Death Progression Loss — Owner Baseline

- Status: **OWNER_ACCEPTED REFERENCE DIFFERENCE**
- DecisionStatus: `ACCEPTED`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@e1750ede386c0ee1001894ab9d91129de5d03fce`
- Scope: Oteryn Reference death progression-loss semantics only
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Owner decisions

Oteryn Reference continues to reproduce the accepted Global Tibia Reference death model except for two explicit, narrow Reference differences in Character progression loss.

First, death XP loss must not be calculated from the Character's accumulated/lifetime TotalXP:

> The XP-loss basis is the full amount of experience required to advance from the Character's current level to the next level.

Second, death affects Character experience only:

> Oteryn Reference death must not reduce skills or magic level. Character skill progress and magic-level progress remain unchanged by death.

These are `REFERENCE / DECLARED_DIFFERENCE` rules under the accepted product-profile scope model. They are not Evolved rules and they do not authorize other Reference gameplay divergence.

## 2. Canonical XP basis

For a Character at current level `L`, define:

```text
LevelXPSpan(L) = XPThreshold(L + 1) - XPThreshold(L)
```

The Reference death XP-loss pipeline must use `LevelXPSpan(L)` as its progression magnitude basis.

This is the full current-level span, independent of current progress within that level. For example, if a level-500 Character is already 90% of the way from 500 to 501, the death-loss basis is still the complete XP span from level 500 to level 501, not the remaining 10%.

It must not use:

```text
AccumulatedTotalXP
LifetimeEarnedXP
XPThreshold(L)
XPRemainingToNextLevel
current absolute TotalXP as a percentage basis
```

for the purpose of determining the magnitude of the death XP penalty.

Equivalent invariant:

```text
ReferenceDeathXPBasis = LevelXPSpan(current_level)
ReferenceDeathXPBasis != AccumulatedTotalXP
ReferenceDeathXPBasis != XPRemainingToNextLevel
```

## 3. XP-only death progression loss

Death in Oteryn Reference may reduce Character XP according to the Reference death XP pipeline, but it must not reduce any skill or magic-level progression.

Binding invariants:

```text
DeathSkillLoss = 0
DeathMagicLevelLoss = 0
```

This applies regardless of:

- current skill values;
- current skill progress toward the next skill level;
- current magic level;
- current magic-level progress;
- vocation;
- promotion state;
- regular blessing count;
- PvE/PvP classification;
- Fair Fight / unfair-fight context;
- skull state;
- world type/profile.

The death pipeline must therefore preserve all skill and magic-level authoritative progression state exactly across death, except for unrelated non-death mutations that may occur independently.

Blessing or promotion effects that Global Tibia applies to skill/magic loss have no skill/magic-loss effect in Oteryn Reference because the Reference death skill/magic penalty is zero. Those systems may still affect XP loss, item/equipment protection, blessing consumption or other target-proven death semantics where applicable.

## 4. Relationship to Global Tibia death semantics

Everything else in the Oteryn Reference death model remains governed by the selected Global Tibia Reference target and its evidence/parity classification unless separately declared otherwise.

This includes, where applicable and evidenced for the selected world/profile:

- promotion effects on XP death loss;
- regular blessing effects on XP death loss and other death protections;
- Twist of Fate behavior;
- PvE/PvP death classification;
- Fair Fight / unfair-fight reduction;
- skull-specific behavior;
- world-type/profile-specific death rules;
- item/equipment loss and corpse behavior;
- blessing consumption;
- respawn behavior;
- Death Redemption behavior, except it cannot restore skill/magic progress that Reference death never removed;
- level/delevel semantics;
- ordering and rounding of the target-proven XP death pipeline.

The accepted Reference differences in this baseline are exactly:

1. XP penalty magnitude is based on the full `LevelXPSpan(current_level)`, not accumulated TotalXP and not XP remaining to the next level;
2. death does not reduce skills or magic level.

## 5. Modifier pipeline

Where the Global Tibia death model applies a loss rate, reduction, multiplier or other XP-loss transform, Oteryn Reference applies the target-proven transform to the Reference XP basis defined above rather than to accumulated TotalXP.

Conceptually:

```text
basis = LevelXPSpan(current_level)
loss = GlobalReferenceDeathXPTransform(
    basis,
    promotion,
    blessings,
    pvp_context,
    fair_fight_context,
    world_profile,
    other_target_proven_xp_modifiers
)

skill_loss = 0
magic_level_loss = 0
```

Exact XP ordering, rounding and target-specific edge behavior remain governed by the Reference evidence/parity contract and must not be guessed where evidence is `UNKNOWN` or `CONFLICT`.

## 6. Progression representation versus penalty basis

These decisions constrain both what death may remove and how the XP penalty amount is calculated; they do not require a particular physical persistence representation.

If downstream Character progression stores an authoritative cumulative experience counter, that counter may be adjusted by the already-calculated bounded death XP loss as the representation of losing progression. That does **not** make accumulated TotalXP the death-loss basis.

The forbidden behavior is:

```text
loss = percentage_or_transform(AccumulatedTotalXP)
loss = percentage_or_transform(XPRemainingToNextLevel)
skill_progress -= death_penalty
magic_progress -= death_penalty
```

The required behavior is:

```text
xp_loss = percentage_or_transform(LevelXPSpan(current_level))
skill_loss = 0
magic_level_loss = 0
```

followed by the normal Reference XP progression/delevel application semantics.

## 7. No implicit no-delevel rule

Neither accepted difference imports the Evolved no-delevel / `DeathDebt` design into Reference.

Oteryn Reference keeps the selected Global Tibia level/delevel behavior for XP unless a later explicit Reference difference changes it.

In particular:

- XP death loss may still cross the current level threshold and cause a delevel under the applicable Reference progression rules;
- `DeathDebt` remains Evolved-only;
- `DeathExhaustion` remains Evolved-only;
- corpse-recoverable XP from draft #295 remains Evolved-only;
- the Evolved `15 / 45 / 7.5+7.5 / 30 min` candidate remains unrelated to this Reference decision.

## 8. Examples of the intended distinction

For a Character at level `L`:

```text
TotalXP = very large accumulated value
LevelXPSpan(L) = XP required for the complete L -> L+1 transition
CurrentLevelProgress = any value from 0% to almost 100%
```

Reference death must derive the XP penalty from the complete `LevelXPSpan(L)`.

Two Characters at the same level and equivalent death/blessing/PvP context therefore use the same level-span basis even if one is at 10% current-level progress and the other is at 90%.

After the XP penalty is applied:

```text
skills_before_death == skills_after_death
magic_progress_before_death == magic_progress_after_death
```

for all death-caused skill/magic mutations.

## 9. Test consequences

Future Reference death fixtures must prove at minimum:

1. penalty magnitude is independent of accumulated/lifetime TotalXP when current level and death context are identical;
2. penalty magnitude is independent of the Character's current percentage progress within the current level;
3. penalty magnitude changes when `LevelXPSpan(current_level)` changes;
4. target-proven blessing/promotion/PvP/world-profile XP modifiers are applied to the full level-span basis;
5. death leaves every skill value and skill-progress accumulator unchanged;
6. death leaves magic level and magic-level progress unchanged;
7. blessings cannot cause a hidden skill/magic mutation merely because Global Tibia normally includes skill/magic loss in its death model;
8. Evolved `DeathDebt`/no-delevel/recovery mechanics cannot activate in a Reference profile;
9. Reference level/delevel, corpse, item-loss and blessing behavior otherwise follows the selected Global Tibia Reference target;
10. unknown target edge cases fail closed rather than being filled from Evolved design.

## 10. Precedence

This baseline composes with:

- `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`;
- `OTERYN_PRODUCT_PROFILE_SCOPE_OWNER_BASELINE_2026-09-09.md`;
- `OTERYN_PRODUCT_PROFILE_REFERENCE_TARGET_RECONCILIATION_2026-09-09.md`;
- `GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md`;
- `GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md`.

For Oteryn Reference death progression loss, these later explicit owner decisions are accepted `DECLARED_DIFFERENCE` rules and override Global parity only for:

- the XP penalty basis;
- the absence of death-caused skill loss;
- the absence of death-caused magic-level loss.

All unrelated Reference death semantics remain under the selected Global Tibia target.

## 11. Acceptance boundary

Accepted now:

```text
profile                         = Oteryn Reference
Global death model              = retained by default
XP-loss magnitude basis         = full LevelXPSpan(current_level)
XP remaining to next as base    = forbidden
AccumulatedTotalXP as loss base = forbidden
Death skill loss                = 0
Death magic-level loss          = 0
Evolved death system            = not imported
```

Not decided by this baseline:

- any additional Reference divergence;
- exact unresolved Global PvP/death edge matrices;
- runtime types or implementation modules;
- physical persistence schema;
- exact XP rounding where target evidence is incomplete;
- any Evolved death numeric values.

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
