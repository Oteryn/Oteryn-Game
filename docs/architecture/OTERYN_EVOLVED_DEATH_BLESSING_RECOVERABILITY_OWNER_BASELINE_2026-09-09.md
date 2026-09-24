# Oteryn Evolved Death Blessing Recoverability — Owner Baseline

- Status: **OWNER_ACCEPTED EVOLVED DIRECTION**
- DecisionStatus: `ACCEPTED`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@0ca0f6d257d6eb982c4ff9d05bc9a84e44ba48da`
- Scope: Oteryn Evolved blessing-dependent permanent/recoverable XP split for player death
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Owner decision

Protected #467 fixes the Evolved core death envelope:

```text
S = LevelXPSpan(current_level)
NominalDeathPenalty = 15% * S
DeathDebtCap        = 45% * S
FullRecoveryWindow  = 30 minutes
```

Protected #463 separately fixes:

```text
EvolvedBlessingNominalDeathXPReduction = 0
```

The owner now accepts the first-generation blessing-dependent XP recoverability endpoints:

```text
NO BLESS:
  10% * S permanent
   5% * S recoverable
  -------------------
  15% * S nominal total

FULL BLESS:
   5% * S permanent
  10% * S recoverable
  -------------------
  15% * S nominal total
```

This decision is `EVOLVED` only. It does not alter Oteryn Reference.

## 2. Meaning of blessings in Evolved death

Blessings do not make the nominal death event smaller.

They change how much of the XP liability can be earned back through the accepted recovery flow.

The player-facing meaning is therefore:

```text
bless != lower the 15% nominal death envelope
bless = make a larger share of the represented loss recoverable
```

For an ordinary death where the full 15% nominal penalty can be represented:

```text
no bless + successful recovery
=> final permanent XP cost = 10% * S

full bless + successful recovery
=> final permanent XP cost = 5% * S

recovery not completed
=> final represented XP cost remains the full charged amount
   (normally 15% * S)
```

Thus blessings are materially valuable preparation without importing the Global/Reference direct XP-loss multiplier model.

## 3. Conservation and cap-edge semantics

Protected #467 requires that a death near the achieved-level floor / DeathDebt cap can never represent more XP liability than current-level progress plus available DeathDebt headroom.

Define:

```text
X = ActuallyRepresentedXPForDeath
```

with:

```text
0 <= X <= 15% * S
```

The blessing split must always partition **X**, never create a claim from an unrepresented part of the nominal 15%.

Therefore:

```text
PermanentXP + RecoverableXP = X
RecoverableXP <= X
```

For a fully representable ordinary death (`X = 15% * S`), the accepted endpoints are exactly the percentages in Section 1.

For a cap-edge death where `X < 15% * S`, preserve the same endpoint proportions over the actually represented amount:

```text
NO BLESS:
  permanent share   = 2/3 of X
  recoverable share = 1/3 of X

FULL BLESS:
  permanent share   = 1/3 of X
  recoverable share = 2/3 of X
```

This proportional cap-edge rule is required so the accepted 10/5 and 5/10 balance remains semantically consistent without allowing a recovery claim to refund XP that was never charged.

Any integer rounding rule used later must be deterministic and must preserve exact conservation:

```text
PermanentXP + RecoverableXP == X
```

The final integer rounding convention may be frozen with the implementation contract; it must never mint or destroy more than the unavoidable sub-XP rounding remainder and that remainder must be assigned deterministically.

## 4. Failed recovery

Blessing state does not reduce the death charge when recovery is not completed.

For a fully representable ordinary death:

```text
no bless, no recovery   = 15% * S final XP cost
full bless, no recovery = 15% * S final XP cost
```

For a cap-edge death:

```text
no recovery final XP cost = X
```

where `X` is the actually represented XP liability permitted by the achieved-level floor and DeathDebt headroom.

The unrepresentable remainder of the nominal envelope does not become hidden XP debt and does not become recoverable XP. Repeated deaths at the cap remain meaningful through the separately bounded DeathExhaustion/recovery-friction path.

## 5. Successful recovery

Recovery operates only on the recoverable component owned by the active death claim.

Protected #467 ordering remains binding:

```text
recoverable XP
-> repay current DeathDebt first
-> only any remainder restores current-level progress
```

The permanent component is not restored by ordinary corpse/recovery completion.

Recovery remains:

- server-authoritative;
- bounded by the owning death occurrence;
- idempotent;
- one-active-XP-claim-per-Character;
- subject to the protected 30-minute full recovery window;
- non-resettable by relog, reconnect, client restart, Channel switch or region failover.

## 6. Partial blessing states

This baseline freezes only the two material endpoints:

```text
zero blessings -> 10% permanent / 5% recoverable
full blessings -> 5% permanent / 10% recoverable
```

Partial blessing states must be monotonic between those endpoints:

- adding valid blessing protection must never reduce recoverability;
- removing blessing protection must never increase recoverability;
- the nominal death envelope remains 15%;
- permanent + recoverable remains the actually represented XP liability.

This baseline deliberately does **not** freeze:

- the final Evolved blessing count;
- blessing names;
- whether every blessing contributes equally;
- the exact discrete partial-blessing ladder/interpolation formula.

Those can be selected later without reopening the accepted zero/full endpoints.

## 7. Illustrative values

For fully representable ordinary deaths:

| Level | LevelXPSpan | Nominal 15% | No bless permanent 10% | No bless recoverable 5% | Full bless permanent 5% | Full bless recoverable 10% |
|---:|---:|---:|---:|---:|---:|---:|
| 200 | 1,970,200 | 295,530 | 197,020 | 98,510 | 98,510 | 197,020 |
| 500 | 12,425,200 | 1,863,780 | 1,242,520 | 621,260 | 621,260 | 1,242,520 |
| 1000 | 49,850,200 | 7,477,530 | 4,985,020 | 2,492,510 | 2,492,510 | 4,985,020 |
| 1500 | 112,275,200 | 16,841,280 | 11,227,520 | 5,613,760 | 5,613,760 | 11,227,520 |

At level 1500 and an illustrative 15,000,000 XP/hour:

```text
no recovery, any blessing endpoint:
  16,841,280 XP ~= 67.4 minutes

successful recovery, no bless:
  11,227,520 XP ~= 44.9 minutes

successful recovery, full bless:
   5,613,760 XP ~= 22.5 minutes
```

The examples assume the full nominal 15% can be represented. Cap-edge deaths use Section 3 proportional partitioning of the smaller actually represented amount.

## 8. Relationship to item protection and DeathExhaustion

This baseline freezes XP recoverability only.

Protected #463 still permits later Evolved blessing rules to affect:

- DeathExhaustion / recovery friction;
- unsecured expedition-loot protection;
- reclaim/salvage conditions;
- other explicitly accepted non-nominal protection effects.

Those effects must not alter the 15% nominal XP envelope and must not duplicate item/value authority.

This baseline does not select any item-loss probability, reclaim fee, salvage capacity, blessing consumption rule or DeathExhaustion tier/duration.

## 9. Reference isolation

Oteryn Reference remains governed by its selected Global Tibia target plus explicit Reference differences.

This Evolved recoverability split must not change Reference:

- blessing XP-loss transforms;
- blessing count/acquisition/consumption;
- item protection;
- PvP/Fair Fight/skull behavior;
- Reference death XP basis;
- Reference skill/magic-level death rules;
- any other target-proven Reference behavior.

Shared implementation may reuse neutral blessing/recovery primitives only if the active product profile selects policy explicitly and the Evolved split cannot activate in Reference.

## 10. Future implementation/test consequences

A later authorized Evolved player-death implementation must prove at minimum:

1. with full representability and zero blessings, a death partitions `15% * S` into `10% * S` permanent + `5% * S` recoverable;
2. with full representability and full blessings, a death partitions `15% * S` into `5% * S` permanent + `10% * S` recoverable;
3. blessing state does not alter `NominalDeathPenalty = 15% * S`;
4. failed recovery leaves the complete actually represented XP charge in effect regardless of blessing endpoint;
5. successful recovery cannot refund the permanent component;
6. cap-edge partitioning operates on actually represented XP, never nominal-but-unrepresented XP;
7. `PermanentXP + RecoverableXP == ActuallyRepresentedXP` after deterministic integer rounding;
8. recovery remains debt-first and idempotent;
9. stale/replayed blessing or recovery state cannot issue a second refund;
10. partial blessing mapping is fail-closed until its discrete ladder is explicitly selected;
11. Reference fixtures prove the Evolved split cannot activate in Reference.

## 11. Deliberately deferred

This baseline does not freeze:

- exact Evolved blessing count/names;
- exact partial-blessing ladder/interpolation;
- blessing acquisition, renewal or consumption semantics;
- blessing prices/currency/economy sinks;
- exact DeathExhaustion mitigation by blessing state;
- unsecured-loot protection/reclaim effects;
- PvP-specific Evolved blessing/death differences;
- persistence schema/protocol/UI representation;
- exact integer rounding convention;
- runtime implementation allocation.

## 12. Acceptance boundary

Accepted now:

```text
profile                                   = Oteryn Evolved
nominal death envelope                    = 15% * LevelXPSpan
zero bless permanent endpoint             = 10% * LevelXPSpan
zero bless recoverable endpoint           = 5% * LevelXPSpan
full bless permanent endpoint             = 5% * LevelXPSpan
full bless recoverable endpoint           = 10% * LevelXPSpan
failed recovery changes charge            = no
cap-edge split basis                       = actually represented XP
partial blessing states                    = monotonic, exact ladder deferred
bless directly reduces nominal death XP   = no
Reference changed                          = no
```

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
