# Oteryn Evolved Seven-Blessing Recoverability Ladder — Owner Baseline

- Status: **OWNER_ACCEPTED EVOLVED DIRECTION**
- DecisionStatus: `ACCEPTED`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@6b0d6d48637a7f25d6060c5974df825f9e8b6c4e`
- Scope: exact first-generation Evolved blessing count and equal XP-recoverability contribution ladder
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Binding owner decision

Protected #463 keeps blessings in Oteryn Evolved while forbidding direct blessing reduction of the nominal death-XP charge.

Protected #467 fixes the Evolved death core:

```text
NominalDeathPenalty = 15% * LevelXPSpan(current_level)
DeathDebtCap        = 45% * LevelXPSpan(current_level)
FullRecoveryWindow  = 30 minutes
```

Protected #469 fixes the blessing recoverability endpoints:

```text
0 blessings:
  permanent   = 10% * LevelXPSpan
  recoverable =  5% * LevelXPSpan

full blessings:
  permanent   =  5% * LevelXPSpan
  recoverable = 10% * LevelXPSpan
```

The owner now accepts:

```text
EvolvedBlessingCount = 7
FullBlessingState     = 7 valid blessings
Each valid blessing contributes equally to the XP recoverability shift.
```

This decision is `EVOLVED` only. It does not alter Oteryn Reference.

## 2. Exact equal-contribution ladder

Let:

```text
b = number of valid active Evolved blessings
b ∈ {0,1,2,3,4,5,6,7}
S = LevelXPSpan(current_level)
```

For a fully representable ordinary death, the accepted exact rational ladder is:

```text
RecoverableShareOfSpan(b) = 5% + (5% * b / 7)
PermanentShareOfSpan(b)   = 10% - (5% * b / 7)

RecoverableShareOfSpan(b) + PermanentShareOfSpan(b) = 15%
```

Equivalent values:

| Valid blessings | Permanent share of LevelXPSpan | Recoverable share of LevelXPSpan | Nominal total |
|---:|---:|---:|---:|
| 0 | 10.000000% | 5.000000% | 15% |
| 1 | 9.285714% | 5.714286% | 15% |
| 2 | 8.571429% | 6.428571% | 15% |
| 3 | 7.857143% | 7.142857% | 15% |
| 4 | 7.142857% | 7.857143% | 15% |
| 5 | 6.428571% | 8.571429% | 15% |
| 6 | 5.714286% | 9.285714% | 15% |
| 7 | 5.000000% | 10.000000% | 15% |

The decimal table is explanatory only. The implementation contract must use the exact rational relationship, not rounded decimal policy constants.

## 3. Exact represented-XP form

Protected #467/#469 require cap-edge safety: blessings split only XP liability that was actually represented for the death.

Let:

```text
X = ActuallyRepresentedXPForDeath
0 <= X <= NominalDeathPenalty
```

For `b` valid blessings, the exact rational shares are:

```text
RecoverableWeight(b) = 7 + b
PermanentWeight(b)   = 14 - b
TotalWeight           = 21

RecoverableXP ∝ (7 + b) / 21 of X
PermanentXP   ∝ (14 - b) / 21 of X
```

At the endpoints:

```text
b = 0:
  recoverable = 7/21 = 1/3 of X
  permanent   = 14/21 = 2/3 of X

b = 7:
  recoverable = 14/21 = 2/3 of X
  permanent   = 7/21 = 1/3 of X
```

For every valid blessing state:

```text
PermanentXP + RecoverableXP = X
```

No blessing may create a refund claim from nominal-but-unrepresented XP.

## 4. Equal blessing value

For XP recoverability, every one of the seven valid Evolved blessings has equal marginal value.

Moving from `b` to `b + 1` changes the represented-XP split by exactly:

```text
+ 1/21 of X recoverable
- 1/21 of X permanent
```

For a fully representable ordinary death, this is equivalent to:

```text
+ 5/7 percentage point of LevelXPSpan recoverable
- 5/7 percentage point of LevelXPSpan permanent
```

No blessing is a privileged or amplified XP-recovery blessing in this first-generation ladder.

This does not prevent later explicit non-XP differences between blessing identities, provided those differences do not alter this accepted equal XP-recoverability contribution unless the owner explicitly supersedes this baseline.

## 5. What full blessings means

Within this Evolved XP-recoverability policy:

```text
full blessings = all 7 valid Evolved blessings active for the death event
```

A state with fewer than seven valid blessings is a partial-blessing state and uses its exact `b` row.

Stale, expired, already-consumed, invalid or otherwise non-authoritative blessing state must not be counted in `b`.

The server-authoritative death transaction must bind the blessing state used for the split so retry/reconnect cannot change the owning death occurrence after the fact.

## 6. Nominal penalty remains unchanged

Protected #463/#467 remain binding:

```text
EvolvedBlessingNominalDeathXPReduction = 0
NominalDeathPenalty = 15% * S
```

Therefore:

- 0 blessings does not increase the nominal penalty above 15%;
- 7 blessings does not reduce the nominal penalty below 15%;
- intermediate blessing states do not change the nominal penalty;
- blessings change only the permanent/recoverable distribution of the actually represented XP liability.

If the player does not complete the accepted recovery flow, the complete actually represented XP charge remains in effect regardless of blessing count.

## 7. Recovery and DeathDebt interaction

Protected recovery ordering remains unchanged:

```text
successful recoverable-XP settlement
-> reduce current DeathDebt first
-> only remainder restores current-level progress
```

The permanent component is never refunded by ordinary recovery.

The ladder must not:

- bypass the 45% DeathDebt cap;
- create hidden debt beyond the cap;
- delevel an achieved Evolved level;
- refund the same death twice;
- reset the protected 30-minute recovery window;
- activate Evolved death semantics in Reference.

## 8. Integer conservation

The accepted product policy is rational and exact. Runtime XP amounts are integer values, so the later implementation contract must choose one deterministic integer allocation rule satisfying:

```text
PermanentXP + RecoverableXP == X
```

for every `X` and every `b ∈ 0..7`.

The rounding convention is not selected by this baseline. It must not create or destroy XP and must produce the same result under retry/replay/failover.

## 9. Examples at level 1500

Using the protected example:

```text
LevelXPSpan(1500) = 112,275,200 XP
Nominal 15%       = 16,841,280 XP
```

For a fully representable death, approximate player-facing values are:

| Blessings | Permanent XP | Recoverable XP |
|---:|---:|---:|
| 0 | 11,227,520 | 5,613,760 |
| 1 | ~10,425,554 | ~6,415,726 |
| 2 | ~9,623,589 | ~7,217,691 |
| 3 | ~8,821,623 | ~8,019,657 |
| 4 | ~8,019,657 | ~8,821,623 |
| 5 | ~7,217,691 | ~9,623,589 |
| 6 | ~6,415,726 | ~10,425,554 |
| 7 | 5,613,760 | 11,227,520 |

These intermediate integer examples are illustrative approximations only; the later deterministic integer allocation rule remains the authority for exact whole-XP values.

## 10. Deliberately deferred

This baseline does not freeze:

- blessing names or Tibia-derived identity mapping;
- blessing acquisition locations/NPCs/quests;
- prices, currencies, scaling or gold-sink tuning;
- renewal and consumption semantics;
- whether PvP uses the same seven-blessing disposition;
- Twist-of-Fate-like Evolved behavior;
- DeathExhaustion mitigation by blessings;
- item/expedition-loot protection and reclaim rules;
- exact integer rounding convention;
- persistence/protocol/UI representation;
- runtime implementation allocation.

Those may be decided independently without reopening the accepted seven-count and equal XP-recoverability ladder.

## 11. Future implementation/test consequences

A later authorized Evolved player-death implementation must prove at minimum:

1. accepted blessing count is exactly seven;
2. `b` is server-authoritative and bounded to `0..7`;
3. nominal death XP remains 15% for every `b`;
4. recoverability increases monotonically and equally by one `1/21` represented-XP step per blessing;
5. permanent liability decreases by the same amount;
6. `b=0` reproduces protected #469 `10% permanent / 5% recoverable` for a fully representable death;
7. `b=7` reproduces protected #469 `5% permanent / 10% recoverable`;
8. all intermediate states preserve `PermanentXP + RecoverableXP == ActuallyRepresentedXP`;
9. cap-edge deaths split only actually represented XP;
10. stale/replayed blessing state cannot alter or duplicate an already-bound death claim;
11. recovery remains debt-first and idempotent;
12. Reference fixtures prove this Evolved ladder cannot activate in Reference.

## 12. Acceptance boundary

Accepted now:

```text
profile                                  = Oteryn Evolved
Evolved blessing count                   = 7
full blessing state                      = 7 valid blessings
equal XP-recoverability contribution     = yes
recoverable represented-XP weight        = (7 + b) / 21
permanent represented-XP weight          = (14 - b) / 21
nominal death envelope                   = unchanged at 15% * LevelXPSpan
0-bless endpoint                         = 10% permanent / 5% recoverable
7-bless endpoint                         = 5% permanent / 10% recoverable
Reference changed                        = no
blessing names/prices/acquisition        = deferred
integer rounding                         = deferred
```

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
