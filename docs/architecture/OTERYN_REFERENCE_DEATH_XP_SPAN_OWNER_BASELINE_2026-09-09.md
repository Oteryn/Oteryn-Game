# Oteryn Reference Death XP Basis — Owner Baseline

- Status: **OWNER_ACCEPTED REFERENCE DIFFERENCE**
- DecisionStatus: `ACCEPTED`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@11fe2548f1d76af49f5149d9deac9a916a5cf8b3`
- Scope: Oteryn Reference death XP-loss basis only
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Owner decision

Oteryn Reference continues to reproduce the accepted Global Tibia Reference death model except for one explicit, narrow Reference difference:

> Death XP loss must not be calculated from the Character's accumulated/lifetime TotalXP. The XP-loss basis is the amount of experience required to advance from the Character's current level to the next level.

This is a `REFERENCE / DECLARED_DIFFERENCE` under the accepted product-profile scope model. It is not an Evolved rule and it does not authorize other Reference gameplay divergence.

## 2. Canonical XP basis

For a Character at current level `L`, define:

```text
LevelXPSpan(L) = XPThreshold(L + 1) - XPThreshold(L)
```

The Reference death XP-loss pipeline must use `LevelXPSpan(L)` as its progression magnitude basis.

It must not use:

```text
AccumulatedTotalXP
LifetimeEarnedXP
XPThreshold(L)
current absolute TotalXP as a percentage basis
```

for the purpose of determining the magnitude of the death XP penalty.

Equivalent invariant:

```text
ReferenceDeathXPBasis = LevelXPSpan(current_level)
ReferenceDeathXPBasis != AccumulatedTotalXP
```

## 3. Relationship to Global Tibia death semantics

Everything else in the Oteryn Reference death model remains governed by the selected Global Tibia Reference target and its evidence/parity classification unless separately declared otherwise.

This includes, where applicable and evidenced for the selected world/profile:

- promotion effects;
- regular blessing effects;
- Twist of Fate behavior;
- PvE/PvP death classification;
- Fair Fight / unfair-fight reduction;
- skull-specific behavior;
- world-type/profile-specific death rules;
- item/equipment loss and corpse behavior;
- blessing consumption;
- respawn behavior;
- Death Redemption behavior;
- skill-loss semantics;
- level/delevel semantics;
- ordering and rounding of the target-proven death pipeline.

The only accepted difference in this baseline is the XP penalty **basis**.

## 4. Modifier pipeline

Where the Global Tibia death model applies a loss rate, reduction, multiplier or other XP-loss transform, Oteryn Reference applies the target-proven transform to the Reference basis defined above rather than to accumulated TotalXP.

Conceptually:

```text
basis = LevelXPSpan(current_level)
loss = GlobalReferenceDeathTransform(
    basis,
    promotion,
    blessings,
    pvp_context,
    fair_fight_context,
    world_profile,
    other_target_proven_modifiers
)
```

Exact ordering, rounding and target-specific edge behavior remain governed by the Reference evidence/parity contract and must not be guessed where evidence is `UNKNOWN` or `CONFLICT`.

## 5. Progression representation versus penalty basis

This decision constrains how the death penalty amount is calculated; it does not require a particular physical persistence representation.

If downstream Character progression stores an authoritative cumulative experience counter, that counter may be adjusted by the already-calculated bounded death loss as the representation of losing progression. That does **not** make accumulated TotalXP the death-loss basis.

The forbidden behavior is:

```text
loss = percentage_or_transform(AccumulatedTotalXP)
```

The required behavior is:

```text
loss = percentage_or_transform(LevelXPSpan(current_level))
```

followed by the normal Reference progression/delevel application semantics.

## 6. No implicit no-delevel rule

The owner described this XP-span basis as the sole Reference death change in this decision.

Therefore this baseline does **not** import the Evolved no-delevel / `DeathDebt` design into Reference.

Oteryn Reference keeps the selected Global Tibia level/delevel behavior unless a later explicit Reference difference changes it.

In particular:

- `DeathDebt` remains Evolved-only;
- `DeathExhaustion` remains Evolved-only;
- corpse-recoverable XP from draft #295 remains Evolved-only;
- the Evolved `15 / 45 / 7.5+7.5 / 30 min` candidate remains unrelated to this Reference decision.

## 7. Examples of the intended distinction

For a Character at level `L`:

```text
TotalXP = very large accumulated value
LevelXPSpan(L) = much smaller XP amount needed from L to L+1
```

Reference death must derive the XP penalty from `LevelXPSpan(L)`.

Two Characters at the same level and equivalent death/blessing/PvP context therefore use the same level-span basis even if an implementation exposes or records different historical/lifetime-earned XP metadata.

## 8. Test consequences

Future Reference death fixtures must prove at minimum:

1. penalty magnitude is independent of accumulated/lifetime TotalXP when current level and death context are identical;
2. penalty magnitude changes when `LevelXPSpan(current_level)` changes;
3. target-proven blessing/promotion/PvP/world-profile modifiers are applied to the level-span basis;
4. Evolved `DeathDebt`/no-delevel/recovery mechanics cannot activate in a Reference profile;
5. Reference level/delevel, corpse, item-loss and blessing behavior otherwise follows the selected Global Tibia Reference target;
6. unknown target edge cases fail closed rather than being filled from Evolved design.

## 9. Precedence

This baseline composes with:

- `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`;
- `OTERYN_PRODUCT_PROFILE_SCOPE_OWNER_BASELINE_2026-09-09.md`;
- `OTERYN_PRODUCT_PROFILE_REFERENCE_TARGET_RECONCILIATION_2026-09-09.md`;
- `GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md`;
- `GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md`.

For Oteryn Reference death XP-loss magnitude, this later explicit owner decision is the accepted `DECLARED_DIFFERENCE` and overrides Global parity only for the penalty basis defined here.

All unrelated Reference death semantics remain under the selected Global Tibia target.

## 10. Acceptance boundary

Accepted now:

```text
profile                         = Oteryn Reference
Global death model              = retained by default
XP-loss magnitude basis         = LevelXPSpan(current_level)
AccumulatedTotalXP as loss base = forbidden
Evolved death system            = not imported
```

Not decided by this baseline:

- any additional Reference divergence;
- exact unresolved Global PvP/death edge matrices;
- runtime types or implementation modules;
- physical persistence schema;
- exact rounding where target evidence is incomplete;
- any Evolved death numeric values.

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
