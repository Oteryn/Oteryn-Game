# Oteryn Game — Rested Progression and Death-Recovery Owner Decision Checkpoint

- Date: 2026-09-04
- Issue: #220
- Admission protected main: `68ecbad7f6a0dbe7d6214654f8a57c75a3d7c705`
- Previous owner checkpoint: PR #221 / merge `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis/documentation only
- Runtime implementation authority: `NONE`

## Purpose

Persist the owner-selected Rested progression direction and the current death/recovery design direction reached during the continuation of `Oteryn: architektura`, without silently rewriting accepted Reference/profile, Character, item, durability or multichannel contracts.

This document is a continuation checkpoint, not a final superseding gameplay contract. Exact numeric balance, final death economics, final corpse-expiry/salvage policy and deferred housing details remain open unless explicitly marked owner-selected below.

## Continuation and terminology

For player-facing discussion the owner prefers the plain terminology:

- **Tryb zgodności z Tibią** — the profile/ruleset whose exercised behavior follows the accepted Reference evidence/parity discipline;
- **Oteryn** — the project-owned ruleset/product direction where intentional improvements may differ from Reference through explicit owner decisions.

This is discussion terminology only and does not by itself rename canonical identifiers or rewrite existing profile-family contracts.

Housing remains intentionally deferred for later continuation. No final house topology, ownership model, auction model or cross-Channel house-presence behavior is frozen by this checkpoint.

## Rested progression — owner-selected direction

### R1 — Reference stamina versus Oteryn Rested

`OWNER_SELECTED`.

- Tryb zgodności z Tibią preserves stamina according to proven Reference behavior where exercised.
- Oteryn does not use classic stamina as a punishment budget.
- Oteryn uses a positive Rested XP mechanic.
- Rested exhaustion returns the Character to ordinary baseline XP; it does not apply a low-stamina XP punishment tier.

### R2 — Rested is a bonus-XP-denominated pool

`OWNER_SELECTED`.

Rested is stored and consumed as bonus-XP credit, not as minutes of active hunting.

```text
RestedBonus = min(CurrentRestedPool, EligibleRawXP * RestedRate)
RestedPoolAfter = CurrentRestedPool - RestedBonus
```

An incidental low-value kill therefore consumes only the proportional bonus XP actually awarded.

### R3 / R9 / R17 — recovery model

`OWNER_SELECTED`.

- every correctly offline Character receives baseline Rested recovery after the minimum continuous-offline threshold;
- public inns may recover faster than ordinary offline;
- house beds and guildhall beds may recover faster than public inns;
- house and guildhall beds use the same moderate recovery class unless later balance evidence justifies a distinction;
- recharge is expressed as a percentage of `MaxRestedPool` per unit of recovery time, not a fixed XP/hour amount and not historical player XP/hour;
- exact multipliers remain `BALANCE_DEFERRED`.

Disconnect/reconnect grace does not count as rest. Recovery begins only after the authoritative playable presence/lease has ended and the Character is actually absent.

### R4 — Premium status

`OWNER_SELECTED`.

Premium status by itself does not increase `RestedRate`, Rested recovery speed or `MaxRestedPool`. Premium may grant access/convenience through separately accepted product features, but there is no direct hidden Rested progression multiplier merely for entitlement state.

### R5 / R15 / R18 — eligible XP sources

`OWNER_SELECTED_CURRENT_POLICY`.

- ordinary repeatable monster-hunt XP is Rested-eligible;
- quest, exploration, achievement and scripted one-time progression rewards are Rested-ineligible by default;
- standard bosses and raid bosses are currently Rested-ineligible and do not consume the pool;
- a future explicitly classified farmable encounter may opt into ordinary-hunt treatment through a separate content/ruleset decision;
- Rested currently affects XP only, not loot, rarity, drop chance, gold, boss rewards, skill gain or magic training.

The words `currently` / `current policy` preserve an explicit future extension point rather than silently making these categories universal forever.

### R6 / R7 — account and multi-character behavior

`OWNER_SELECTED`.

- Rested state is per Character;
- no transfer, sale, pooling or merging of Rested between Characters or accounts;
- multiaccount use is not punished merely for being multiaccount;
- every Character independently satisfies recovery requirements;
- multiple offline Characters on one account may recover their own Rested simultaneously even if another Character on the same account is currently online.

### R8 / R12 / R19 — scaling and level changes

`OWNER_SELECTED`.

- `MaxRestedPool` scales through a versioned level/progression-tier curve rather than one absolute XP amount for all Characters;
- `RestedRate` is one constant percentage across levels for the active Oteryn ruleset unless later balance evidence explicitly changes that policy;
- level-up recalculates the cap but does not reset, zero or percentage-rescale `CurrentRestedPool`;
- the absolute Rested amount already earned remains unchanged through ordinary level-up.

Exact curve and percentage remain `BALANCE_DEFERRED`.

### R10 — bed/location convenience, not cash-store power

`OWNER_SELECTED_DIRECTION`.

- public inn: moderate recovery convenience above baseline offline;
- house bed and guildhall bed: equal stronger recovery convenience above the inn class;
- exact recovery multipliers remain deferred;
- cash-store furniture does not add additional Rested power merely because it is purchased for real money.

### R11 / R14 — raw-XP accounting and party attribution

`OWNER_SELECTED`.

Rested is calculated only from server-authoritative `EligibleRawXP`:

```text
Monster/source raw XP
-> authoritative attribution / party-share split
-> Character EligibleRawXP
-> RestedBonus
-> independent downstream XP modifiers
-> FinalXP
```

Consequences:

- Double XP, prey, Store boosts, event bonuses, Premium/VIP modifiers and other downstream multipliers do not increase RestedBonus or RestedPool consumption;
- party/shared-XP bonuses applied after the Character's authoritative raw share do not enlarge the Rested base;
- summons/pets use the same authoritative attribution path and cannot create a second Rested award path;
- a Character never calculates Rested from another party member's share or the monster's full XP when that XP was not attributed to that Character.

### R13 — no manual Rested pause

`OWNER_SELECTED`.

There is no player-facing Rested on/off or pause switch in the selected design. If an XP source is Rested-eligible and `RestedPool > 0`, the bonus applies automatically.

### R16 — minimum continuous-offline threshold

`OWNER_SELECTED_DIRECTION`.

Rested recovery begins only after a minimum uninterrupted period of authoritative Character absence. Exact duration remains `BALANCE_DEFERRED`; short relog/disconnect cycles must not create rounding or recharge abuse.

## Food / cooking / fishing interaction

`DEFERRED_CANDIDATE / PRESERVE_FOR_LATER_ANALYSIS`.

The owner wants the later gameplay design to consider food, special meals, cooking and fishing as meaningful inputs to recovery/rest quality. The current preferred concept is that food may improve Rested recovery quality/rate rather than mint Rested XP instantly.

No duration, recipe, stacking rule, rarity curve, fishing progression, numeric multiplier or final food-to-Rested contract is frozen now. This topic must be revisited under its owning gameplay/economy/content gate rather than expanded inside the current Rested checkpoint.

## Death progression — owner-selected direction

### D1 — achieved level is a durable milestone in Oteryn

`OWNER_SELECTED`.

For Oteryn ordinary death does not delevel a Character below an already achieved level milestone.

Death first removes XP progress earned toward the next level. If the calculated death penalty exceeds the available current-level progress, the excess becomes `DeathDebt` rather than reducing the achieved level.

Conceptually:

```text
CalculatedDeathPenalty
-> consume current-level progress down to zero
-> overflow becomes DeathDebt
-> achieved Level does not decrease
```

Tryb zgodności z Tibią remains governed separately by proven Reference death behavior.

### D2 — DeathDebt

`OWNER_SELECTED_DIRECTION`.

- `DeathDebt` is recorded as an absolute XP amount, not as an implicit negative level;
- future earned XP services the debt before it advances the next-level progress;
- DeathDebt and Rested are separate systems;
- Rested is not consumed as death insurance and death does not erase the RestedPool merely to pay the death penalty.

The exact debt cap and exact interaction with independent XP bonuses remain to be finalized. The current architectural preference is that debt service uses the same raw-XP discipline rather than allowing downstream event/store multipliers to inflate debt repayment.

### D3 — repeated-death exhaustion

`OWNER_SELECTED_DIRECTION / NUMERICS_DEFERRED`.

Repeated deaths should remain meaningful even when an XP/debt cap is eventually reached. The design therefore keeps a separate bounded `DeathExhaustion` / recovery consequence.

Guardrails:

- do not reduce combat damage, max HP, core movement speed or similar combat stats merely because the player died; avoid a self-reinforcing death spiral;
- repeated deaths may increase recovery friction, recovery time and/or economic recovery cost;
- relog, Channel switch or ordinary reconnect must not clear the exhaustion state;
- death must not become a beneficial reset for boss lockouts, cooldowns, Rested, anti-abuse telemetry or other protected state.

Exact exhaustion levels, timers and sinks remain open.

## Corpse / death-recovery direction

### D4 — protect long-term equipment, risk recent expedition value

`OWNER_SELECTED_DIRECTION`.

The preferred player-facing model is to make corpse recovery meaningful without randomly destroying long-term equipment progression.

- equipped/secured long-term value is not randomly dropped merely because an ordinary Oteryn death occurred;
- loot/value explicitly classified as `UNSECURED_EXPEDITION_LOOT` may transfer into a typed death-recovery/corpse custody on death;
- returning to the corpse/death-recovery location is intended to restore that recoverable expedition value;
- the system must preserve one-authoritative-location / anti-duplication semantics from GAME-ITEM-01 and DUR-03; an item cannot simultaneously remain in Character custody and corpse custody.

The exact definition of `UNSECURED_EXPEDITION_LOOT`, when loot becomes secured, PvP differences, theft/access rules and expiration outcome remain open.

### D5 — recoverable XP at corpse

`PREFERRED_CANDIDATE / NOT_YET_NUMERICALLY_FROZEN`.

A portion of the nominal death XP penalty may be recoverable by returning to the corpse/death-recovery point. Recovery should first reduce `DeathDebt`; only any remainder may restore current-level progress.

The current illustrative split is:

```text
nominal death penalty: 15% of current LevelXPSpan
permanent component:    7.5%
recoverable component:  7.5%
```

These exact percentages are **not owner-frozen by this checkpoint**. They remain a balance candidate requiring later explicit disposition.

A second death before recovering the prior recoverable-XP portion may make the older recoverable XP permanently lost; this remains a preferred tension mechanic, not a final frozen rule.

### D6 — corpse recovery window

`PREFERRED_CURRENT_CANDIDATE / NOT_YET_FINAL_BALANCE`.

Persist the current preferred full corpse-recovery window as:

```text
30 minutes
```

During the full window the intended model permits recovery of the eligible corpse/death-cache value and the recoverable XP portion. The timer is server-authoritative and must not reset through relog, Channel switch or client restart.

A real server/Channel outage must not unfairly consume the player's recovery opportunity; exact pause/extension semantics require later operations/recovery design.

A possible later salvage/reclaim phase after the full window was discussed but is **not accepted** by this checkpoint.

## Numeric death candidate still requiring explicit owner disposition

The following values were proposed during analysis but are deliberately not misrepresented as already frozen owner decisions:

```text
NominalDeathPenalty = 15% of current LevelXPSpan
possible DeathDebt cap = 45% of one LevelXPSpan
possible recoverable split = 7.5% permanent + 7.5% corpse-recoverable
```

These numbers are the current balance candidate only. The invariant that achieved level does not decrease and overflow becomes DeathDebt is selected; the exact numeric curve/caps require later explicit acceptance.

## Abuse and safety invariants to preserve

The later contract must ensure at minimum:

- death cannot reset protected cooldowns, boss lockouts, reward eligibility, Rested state, anti-abuse telemetry or authority/fencing state in a beneficial way;
- Channel switch/reconnect/region failover cannot duplicate Rested credit, corpse value, recoverable XP or DeathDebt settlement;
- client time is never authoritative for recovery, debt, Rested or corpse windows;
- every durable item moved to corpse/death custody retains exactly one authoritative semantic location;
- retries and ambiguous commits cannot duplicate corpse recovery or recoverable-XP settlement;
- repeated intentional deaths remain costly enough that reaching a Debt cap does not make further deaths free, while recovery mechanics must not create a combat-stat death spiral.

## Decision timing

### Must decide now?

`PARTIAL YES`.

The semantic shape of Rested, raw-XP accounting, per-Character ownership and non-delevel DeathDebt direction should be preserved now because future Character/progression/ruleset/persistence implementation must not accidentally hard-code classic stamina minutes or universal delevel semantics.

Exact percentages, pool sizes, recharge multipliers, debt caps, exhaustion timings, corpse expiry outcomes and deferred food/housing details do **not** need to be frozen now.

### Concrete downstream work blocked by the selected semantic direction

Future Oteryn Character/progression, ruleset, persistence, XP attribution and death/corpse transaction contracts need to represent:

- bonus-XP-denominated RestedPool;
- `EligibleRawXP` as a stable semantic accounting boundary;
- per-Character recovery state;
- achieved-level milestone semantics distinct from current-level progress;
- bounded `DeathDebt` representation;
- typed corpse/death-recovery custody compatible with DUR-03 conservation.

### What evidence may justify supersession later?

- player testing showing the model makes death trivial or excessively punitive;
- economy/progression telemetry demonstrating unhealthy XP or loot inflation/deflation;
- abuse findings around party attribution, repeated deaths, corpse recovery, multiaccounting or recovery windows;
- Reference-profile evidence affecting only the compatibility profile;
- deterministic simulation/persistence evidence showing the proposed semantic split creates unsafe or disproportionate implementation complexity;
- explicit later owner product strategy change.

## Deliberately unresolved

- exact `RestedRate`;
- exact `MaxRestedPool` curve;
- exact ordinary/inn/house/guildhall recovery multipliers;
- exact continuous-offline threshold;
- final food/cooking/fishing interaction;
- exact death-penalty percentage/curve;
- exact DeathDebt cap and repayment interaction with non-Rested XP modifiers;
- exact DeathExhaustion states/timers/economic sinks;
- exact `UNSECURED_EXPEDITION_LOOT` classification and securing boundary;
- exact corpse access/theft/PvP behavior;
- exact permanent/recoverable XP split;
- exact behavior after the full corpse-recovery window;
- final housing topology and housing ownership/economy details.

No runtime/client/server/protocol/DDL/migration/deployment/production/Platform/Atlas implementation is authorized by this checkpoint.
