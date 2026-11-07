# Oteryn Evolved Death Blessing Role — Owner Baseline

- Status: **OWNER_ACCEPTED EVOLVED DIRECTION**
- DecisionStatus: `ACCEPTED`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@10ce3393a51dac14105b831040e1f4faa3ca565f`
- Scope: Oteryn Evolved blessing role inside player-death/recovery design
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Owner decision

Oteryn Evolved keeps blessings as a recognizable preparation, protection and economy-sink system, but does **not** copy the Global/Reference model where blessings directly multiply down the nominal XP death penalty.

Binding rule:

```text
EvolvedBlessingNominalDeathXPReduction = 0
```

Blessings must not reduce the nominal Evolved death-XP charge selected by the active Evolved death ruleset. In particular, Reference/Global blessing percentages such as per-blessing XP-loss reductions or a large full-blessing aggregate reduction must not be transferred into Evolved by default.

This is an `EVOLVED` product decision under the accepted `SHARED / REFERENCE / EVOLVED` profile split. It does not alter Oteryn Reference blessing/death behavior.

## 2. Why blessings remain in Evolved

Blessings remain useful because they can preserve a recognizable Tibia-derived preparation ritual and gold/value sink without making Evolved death trivial.

The intended Evolved meaning is:

```text
blessing != make death almost free
blessing = improve what can be protected or recovered after death
```

This keeps meaningful pre-hunt preparation while preserving the Evolved death system's own progression-cost envelope.

## 3. Allowed Evolved blessing effect families

Evolved blessings may be used by later accepted rules to improve one or more of the following effect families.

### 3.1 XP recoverability

Blessings may shift a larger portion of the already-charged nominal death penalty from permanent loss into recoverable XP, provided:

- the nominal death penalty itself is unchanged;
- permanent plus recoverable portions never exceed the actually charged penalty;
- recovery cannot mint XP;
- recovery remains idempotent and authoritative;
- any DeathDebt interaction preserves the accepted debt-first settlement order where that Evolved system is active.

Conceptually:

```text
nominal_penalty = EvolvedDeathPenalty(...)
permanent_part + recoverable_part = nominal_penalty

blessings may change the split
blessings must not reduce nominal_penalty
```

Exact split values remain separately owner-controlled balance policy.

### 3.2 DeathExhaustion / recovery friction

Blessings may mitigate a later accepted `DeathExhaustion` or recovery-friction consequence so repeated deaths remain costly without creating a combat-stat death spiral.

Blessings must not turn `DeathExhaustion` into a hidden damage/HP/skill punishment unless a later explicit Evolved decision selects such behavior.

Exact exhaustion semantics and blessing effects remain deferred.

### 3.3 Unsecured expedition loot protection

Blessings may improve the protection, recoverability, reclaim conditions or custody treatment of value explicitly classified by the Evolved ruleset as unsecured expedition loot.

Any such rule remains subordinate to `GAME-ITEM-01` / `DUR-03` one-location, conservation, idempotency and fencing semantics. A blessing may change allowed disposition but cannot duplicate value or create parallel item authority.

Exact item categories, protection percentages, reclaim fees/capacity and PvP exceptions remain deferred.

## 4. Forbidden Evolved blessing behavior

Unless a later explicit owner decision supersedes this baseline, Evolved blessings must not:

1. directly multiply or subtract from the nominal Evolved death-XP penalty;
2. import the Reference/Global per-blessing XP-loss reduction table as Evolved policy;
3. make a fully blessed death effectively negligible solely through XP-loss multiplication;
4. cause hidden skill or magic-level loss merely to preserve an old blessing use case;
5. bypass DeathDebt, recovery-claim, item-custody, idempotency, fencing or authoritative-timer rules;
6. silently change Oteryn Reference blessing/death semantics.

## 5. Relationship to the unresolved Evolved numeric package

The separate candidate package:

```text
15% nominal death penalty
45% DeathDebt cap
7.5% permanent + 7.5% corpse-recoverable
30-minute full recovery window
```

is **not accepted by this blessing decision** merely because it is useful as an example.

This baseline remains valid if those numbers change later. The invariant is:

```text
blessings do not reduce the nominal Evolved death penalty
```

If the owner later accepts a particular nominal penalty and permanent/recoverable split, blessings may alter the recoverability/protection distribution only under a separately accepted quantitative rule.

Illustrative example only, not numeric authority:

```text
without blessings:
  nominal penalty = P
  larger permanent share / smaller recoverable share may be selected later

with full blessings:
  nominal penalty = P
  smaller permanent share / larger recoverable share may be selected later

always:
  permanent + recoverable = actually charged P
```

## 6. Reference isolation

Oteryn Reference remains governed by its selected Global Tibia target plus explicit Reference differences.

This Evolved baseline does not change:

- Reference blessing count or acquisition semantics;
- Reference promotion/blessing XP transforms;
- Reference item protection;
- Reference blessing consumption;
- Reference PvP/Fair-Fight/skull behavior;
- any target-proven Reference death rule.

A shared implementation may reuse neutral blessing/protection infrastructure, but profile policy must remain explicit and cannot allow the Evolved blessing role to activate in Reference.

## 7. Future implementation/test consequences

A later Evolved player-death allocation must prove at minimum:

1. equal death context with/without blessings produces the same nominal XP charge before recovery/protection allocation;
2. blessing logic cannot call or reproduce the Reference/Global XP-loss-reduction multiplier path in Evolved;
3. if blessings change recoverability, permanent + recoverable equals the actually applied penalty and retry/recovery cannot mint XP;
4. blessing state cannot bypass a DeathDebt cap or create negative/duplicated debt settlement;
5. blessing-assisted item protection/reclaim preserves one authoritative item location and value conservation;
6. stale/replayed blessing state cannot authorize a second recovery or item disposition;
7. Reference fixtures prove this Evolved policy cannot activate in the Reference profile;
8. unknown quantitative blessing effects fail closed rather than being invented in runtime code.

## 8. Deliberately deferred

This baseline does not freeze:

- exact Evolved blessing count or names;
- how blessings are acquired, renewed or consumed;
- blessing prices/currency/sinks;
- exact permanent/recoverable XP split at zero/partial/full blessings;
- exact `DeathExhaustion` mitigation;
- exact unsecured-loot protection/reclaim effects;
- PvP-specific Evolved blessing behavior;
- UI presentation;
- persistence schema or protocol fields;
- the separate Evolved death numeric package.

Those require later bounded decisions/evidence and may tune the system without reopening the semantic role frozen here.

## 9. Acceptance boundary

Accepted now:

```text
profile                                = Oteryn Evolved
blessings retained                     = yes
blessings reduce nominal death XP      = no
Reference blessing XP multipliers      = not imported
blessings may improve recoverability   = yes, quantitatively deferred
blessings may mitigate exhaustion      = yes, quantitatively deferred
blessings may protect/recover loot      = yes, quantitatively deferred
Evolved numeric death package          = still separately unresolved
Reference behavior changed             = no
```

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
