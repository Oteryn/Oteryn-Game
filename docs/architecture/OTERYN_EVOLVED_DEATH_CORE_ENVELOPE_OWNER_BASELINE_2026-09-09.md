# Oteryn Evolved Death Core Envelope — Owner Baseline

- Status: **OWNER_ACCEPTED EVOLVED DIRECTION**
- DecisionStatus: `ACCEPTED`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@d56bfd26a14c0ac89d5b8f0d804c50c463eff970`
- Scope: Oteryn Evolved player-death XP envelope and recovery-window core
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Owner decision

The first Oteryn Evolved player-death core uses the following numeric envelope:

```text
S = LevelXPSpan(current_level)

NominalDeathPenalty = 15% * S
DeathDebtCap        = 45% * S
FullRecoveryWindow  = 30 minutes
```

This is an `EVOLVED` product decision. It does not alter Oteryn Reference.

The previously discussed universal `7.5% permanent + 7.5% recoverable` split is **not** accepted by this baseline. Protected #463 allows Evolved blessings to change recoverability while forbidding blessings from reducing the nominal death-XP charge, so permanent/recoverable distribution is a separate later balance decision.

## 2. Level-span basis

The death envelope is based on the full XP span from the current achieved level to the next level:

```text
LevelXPSpan(L) = XPThreshold(L + 1) - XPThreshold(L)
```

The basis does not depend on:

- accumulated/lifetime TotalXP;
- remaining XP to next level;
- current percentage progress within the level;
- blessing state.

Two Characters at the same level therefore have the same nominal 15% Evolved death envelope even if one has very little current-level progress and the other is nearly at the next level.

## 3. No-delevel achieved-level invariant

An achieved Evolved level is not removed by ordinary death.

Death applies progression cost in this order:

```text
1. consume available current-level XP progress;
2. place eligible overflow into DeathDebt;
3. never cross below the achieved-level floor.
```

A Character therefore remains at the achieved level even when current-level progress is insufficient to absorb the nominal penalty.

This no-delevel rule is Evolved-only and must not activate in Reference.

## 4. DeathDebt

`DeathDebt` is an absolute-XP liability serviced before new current-level progress.

Future eligible earned XP applies conceptually as:

```text
EarnedProgressXP
-> repay DeathDebt first
-> only remainder increases current-level progress
```

The first-generation admission ceiling is:

```text
DeathDebtCap = 45% * LevelXPSpan(current_level)
```

The cap is evaluated against the current achieved-level span for admission of new debt from the death occurrence.

Existing DeathDebt remains an absolute XP amount. A later level transition does not retroactively percentage-rescale already recorded debt.

## 5. Nominal penalty versus actually represented XP liability

`NominalDeathPenalty = 15% * S` defines the intended XP cost envelope of one death occurrence before progression-floor/cap constraints.

The system must not create hidden negative XP below the achieved-level floor and must not create XP debt above the accepted DeathDebt cap.

Therefore the actually represented XP liability for one death is bounded by the Character's available current-level progress plus remaining DeathDebt headroom.

Conceptually:

```text
NominalPenalty = 15% * S
DebtHeadroom   = max(0, DeathDebtCap - CurrentDeathDebt)

RepresentableXPLiability =
    min(NominalPenalty,
        CurrentLevelProgress + DebtHeadroom)
```

The death occurrence itself remains authoritative even when the full nominal 15% cannot be represented as XP liability because the achieved-level floor and Debt cap are already saturated.

The unrepresentable remainder must **not** become:

- hidden uncapped XP debt;
- negative lifetime XP;
- a delevel;
- duplicated deferred XP liability.

Repeated deaths at or near the cap remain meaningful through the separately bounded Evolved `DeathExhaustion` / recovery-friction path; reaching the cap does not create a free-death state.

## 6. Blessing relationship

Protected #463 remains binding:

```text
EvolvedBlessingNominalDeathXPReduction = 0
```

Blessings do not reduce the 15% nominal death envelope and do not import Reference/Global direct XP-loss multipliers.

Blessings may later change how much of the **actually represented** death XP liability is recoverable versus permanent, subject to a separately owner-accepted quantitative rule.

Any later blessing split must satisfy:

```text
PermanentXP + RecoverableXP
= actually represented XP liability for that death
```

and recovery may never refund more XP than was actually charged.

The exact zero/partial/full-blessing permanent/recoverable percentages remain deliberately unresolved by this baseline.

## 7. Recovery window

The full recovery opportunity for the active recoverable-XP claim is:

```text
30 minutes
```

The timer is server-authoritative.

Relog, reconnect, client restart, Channel switch or region failover must not reset or duplicate the window.

Proven server/Channel/service unavailability must not unfairly consume the player's recovery opportunity; exact pause/extension accounting remains an operations/recovery implementation decision, but client time is never authoritative.

This baseline fixes the **window duration**, not the exact recoverable percentage.

## 8. One active recoverable-XP claim

The accepted first-generation shape keeps at most one active recoverable-XP claim per Character.

A later qualifying death before recovery of the previous XP claim replaces/expires the older XP recovery claim rather than stacking an arbitrary chain of recoverable XP claims.

This rule applies to XP claims only. Item/corpse custody is separate and must not be silently destroyed merely because an XP claim expires or is replaced.

## 9. Recovery ordering and idempotency

Successful XP recovery settles in this order:

```text
1. reduce current DeathDebt;
2. only any remainder may restore current-level progress.
```

Recovery is bounded by the actually charged recoverable amount for the owning death occurrence.

Retry, reconnect, failover or duplicated delivery must not apply the same refund twice.

Ambiguous settlement must reconcile the same logical recovery operation rather than issue a second independent refund.

## 10. Cap-edge safety

A death near the DeathDebt cap must never mint XP.

If only part of the nominal 15% can be represented because progress is low and DebtHeadroom is small, any future recoverable claim must be bounded by that actually represented amount.

Example invariant:

```text
ActuallyRepresentedXP = X
RecoverableXP <= X
PermanentXP + RecoverableXP = X
```

Never:

```text
refund based on nominal 15%
when less than 15% was actually represented
```

This is required to prevent a cap-edge death/recovery cycle from increasing Character XP.

## 11. DeathExhaustion boundary

`DeathExhaustion` remains a separate bounded Evolved recovery-friction mechanism whose concrete tiers/durations/economy effects are not selected here.

It must not become:

- lower HP;
- lower damage/healing;
- lower skills or magic level;
- lower speed/resistance;
- reduced ordinary raw-XP earning;
- hidden XP debt beyond the 45% cap;
- a shorter 30-minute base recovery window.

Its purpose is only to ensure repeated deaths still matter when additional XP debt cannot be admitted.

Blessings may later mitigate approved DeathExhaustion/recovery friction without changing the nominal 15% death envelope.

## 12. Illustrative values

Using the current Tibia-compatible level-span formula already used by architecture analysis:

```text
LevelXPSpan(1500) = 112,275,200 XP
```

The accepted Evolved envelope at level 1500 is therefore:

```text
NominalDeathPenalty = 16,841,280 XP
DeathDebtCap        = 50,523,840 XP
FullRecoveryWindow  = 30 minutes
```

At an illustrative 15,000,000 XP/hour hunt, the full nominal penalty corresponds to roughly 67 minutes of hunting if the full 15% is actually represented and none of its later-designated recoverable portion is recovered.

This example does not freeze blessing-dependent permanent/recoverable percentages.

## 13. Reference isolation

Oteryn Reference remains governed by its selected Global Tibia target plus explicit Reference differences.

This baseline does not change Reference:

- death XP arithmetic;
- delevel behavior;
- blessing transforms;
- corpse/item behavior;
- PvP/Fair Fight/skull behavior;
- Death Redemption;
- any other target-proven death semantic.

Shared implementation may reuse neutral progression/death infrastructure only when profile policy remains explicit and Evolved-only state cannot activate in Reference.

## 14. Future implementation/test consequences

A later authorized Evolved player-death implementation must prove at minimum:

1. `NominalDeathPenalty = 15% * LevelXPSpan(current_level)` independent of TotalXP and remaining-to-next-level progress;
2. achieved level never decreases from ordinary Evolved death;
3. current-level progress is consumed before DeathDebt;
4. new earned progression XP repays DeathDebt before increasing level progress;
5. DeathDebt admission never exceeds `45% * LevelXPSpan(current_level)`;
6. cap-edge death cannot mint XP and cannot create hidden debt beyond the cap;
7. blessing state cannot reduce the nominal 15% charge;
8. one active recoverable-XP claim exists per Character;
9. recovery is debt-first, bounded by actually charged XP and idempotent;
10. the 30-minute window is server-authoritative and cannot reset through reconnect/Channel change;
11. DeathExhaustion remains non-combat and does not become hidden XP liability;
12. Reference fixtures prove Evolved no-delevel/DeathDebt/recovery semantics cannot activate in Reference.

## 15. Deliberately deferred

This baseline does not freeze:

- zero/partial/full-blessing permanent/recoverable XP percentages;
- exact Evolved blessing count/names/prices/consumption;
- exact DeathExhaustion tiers, duration or economy effects;
- corpse item expiry/reclaim/salvage details;
- PvP-specific Evolved death/corpse differences;
- exact outage-extension implementation for the recovery timer;
- persistence schema/protocol/UI representation;
- runtime implementation allocation.

Those are later bounded decisions and may tune the system without reopening the accepted `15% / 45% / 30 minutes` core envelope.

## 16. Acceptance boundary

Accepted now:

```text
profile                         = Oteryn Evolved
nominal death penalty           = 15% * LevelXPSpan(current_level)
DeathDebt cap                   = 45% * LevelXPSpan(current_level)
full recovery window            = 30 minutes
achieved-level delevel          = no
progression settlement          = progress first, then DeathDebt
future earned progression XP    = DeathDebt first
one active recoverable XP claim = yes
recovery settlement             = debt first, idempotent
bless reduces nominal penalty   = no
universal 7.5% + 7.5% split     = no / deferred
Reference changed               = no
```

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
