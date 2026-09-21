# Oteryn Evolved PvE Blessing Item Protection Supersession — Owner Baseline

- Status: **OWNER_ACCEPTED FIRST-GENERATION DEFAULT**
- DecisionStatus: `ACCEPTED_WITH_PLAYABLE_EVIDENCE_REVISIT`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@75dc6dee53df9945a4a89b2acdb91823919c392f`
- Scope: ordinary Oteryn Evolved PvE blessing-based backpack/equipment loss and corpse/reclaim composition
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Owner decision

For first-generation ordinary Oteryn Evolved PvE, item-loss risk is blessing-based and intentionally simple.

The previously protected expedition-risk model from PR #478 is superseded for ordinary PvE item-loss selection.

The active first-generation model is:

```text
valid active Evolved blessings at death
  -> determine backpack/container drop chance
  -> determine per-equipped-item drop chance
  -> dropped value enters DEATH_CORPSE_CUSTODY
  -> direct corpse recovery during the protected 30-minute recovery window
  -> if unrecovered when the window expires:
       RECLAIM_SALVAGE_CUSTODY under protected #474
```

This is an `EVOLVED` ordinary-PvE decision only.

It does not alter Oteryn Reference and does not decide PvP, theft, public-loot, skull or Fair Fight behavior.

## 2. Explicit supersession of #478

Protected PR #478 remains historical provenance, but its first-generation requirement for:

```text
SECURED_LONG_TERM_VALUE
UNSECURED_EXPEDITION_LOOT
SECURE_EXTRACTION
```

is no longer active ordinary-Evolved-PvE product authority where it conflicts with this baseline.

First-generation ordinary Evolved PvE therefore does **not** require:

- persistent secured/unsecured risk state on ordinary items solely for death protection;
- an expedition extraction lifecycle;
- a secure-extraction button/service;
- anti-laundering lineage solely to preserve that superseded expedition-risk model.

GAME-ITEM-01 and DUR-03 identity, location, conservation, idempotency and fencing requirements remain fully binding.

Any unrelated reusable safety insight from #478 remains historical evidence, but it must not reintroduce the superseded product mechanic without a later explicit owner decision.

## 3. First-generation item-loss ladder

Let:

```text
b = number of valid active Evolved regular blessings bound to the death occurrence
b in {0,1,2,3,4,5,6,7}
```

The ordinary-PvE item-loss ladder is:

| Valid blessings | Backpack/container drop chance | Drop chance per eligible equipped item |
|---:|---:|---:|
| 0 | 100% | 10% |
| 1 | 70% | 7% |
| 2 | 45% | 4.5% |
| 3 | 25% | 2.5% |
| 4 | 10% | 1% |
| 5 | 0% | 0% |
| 6 | 0% | 0% |
| 7 | 0% | 0% |

Binding first-generation consequence:

```text
b >= 5
=> full ordinary-PvE protection for the backpack/container and eligible equipped items
```

Blessings six and seven do not increase ordinary-PvE item protection beyond full protection.

They remain meaningful through the separately protected seven-blessing XP-recoverability ladder and any later explicitly accepted non-item blessing effects.

## 4. Current Global Tibia evidence anchor

The owner selected the ladder above after checking the current Global Tibia item-loss model.

As of 2026-09-09, the official Tibia game guide states that a vocation character has a base 10% chance per equipped item to drop it, while containers such as backpacks are otherwise dropped with their contents; blessings reduce equipment/container loss by 30%, 55%, 75%, 90% and 100% for one through five-or-more blessings.

Evidence anchor:

`https://www.tibia.com/gameguides/?section=characters&subtopic=manual`

The numerical ladder in this Oteryn document is owner-selected Oteryn Evolved policy. A later change by CipSoft does not silently mutate Oteryn behavior.

## 5. Backpack/container semantics

For the ordinary carried backpack/container family selected by the later item/equipment implementation contract:

- one blessing-derived drop decision applies to that carried container;
- if it is retained, its contained value remains in Character-owned custody;
- if it drops, its contained item tree follows the same committed custody disposition with the parent, subject to GAME-ITEM container legality and explicit protected-item overrides;
- the system must not duplicate descendants between Character and corpse custody;
- a container drop is a custody move, not destruction.

Exact first-generation equipment-slot naming and UI representation remain implementation-owned under GAME-ITEM/protocol boundaries.

## 6. Equipped-item semantics

Each ordinary eligible equipped item uses the per-item drop chance from the accepted ladder.

This is not a default all-equipment-at-once bundle loss.

Conceptually:

```text
for each eligible equipped ItemInstance at the death occurrence:
    evaluate ordinary-PvE blessing item-protection policy
    outcome = RETAIN or CORPSE_CUSTODY
```

The authoritative server decides the outcome.

The exact deterministic RNG primitive, seed derivation and simulation implementation are deferred to the owning SIM/gameplay implementation contract, but the result must be replay-safe/idempotent for one logical death occurrence: retry, reconnect or failover must never reroll the same death into a different item disposition.

## 7. Corpse and reclaim composition

When an item is selected to drop under this ladder:

```text
Character custody
  -> DEATH_CORPSE_CUSTODY
  -> owner recovery during protected recovery window
  -> if unrecovered at full-window expiry:
       RECLAIM_SALVAGE_CUSTODY
```

Protected PR #474 remains active authority for the post-window reclaim transition.

Therefore ordinary Evolved PvE item loss means meaningful recovery friction/risk, not silent permanent destruction at the death deadline.

Reclaim fees, capacity, retention and terminal salvage remain deferred exactly as protected #474 requires.

## 8. Why blessings remain meaningful

This model intentionally gives blessings two distinct first-generation roles:

```text
blessings 1..5
  -> progressively improve item protection
  -> also improve XP recoverability through the protected seven-blessing XP ladder

blessings 6..7
  -> item protection already full
  -> continue improving XP recoverability
```

The protected invariant from #463 remains unchanged:

```text
EvolvedBlessingNominalDeathXPReduction = 0
```

Blessings do not reduce the nominal 15% Evolved death-XP charge. They change XP recoverability and, under this baseline, item-loss protection.

## 9. Relationship to the seven-blessing XP ladder

Protected #471 remains unchanged.

All seven valid blessings contribute equally to the XP permanent/recoverable transition:

```text
RecoverableWeight(b) = 7 + b
PermanentWeight(b)   = 14 - b
TotalWeight           = 21
```

The fact that ordinary-PvE item protection reaches 100% at five blessings does not alter equal XP-recoverability contribution for blessings six and seven.

The item and XP ladders are separate policy surfaces evaluated from the same authoritative valid blessing count at death.

## 10. No expedition-risk state in first generation

For ordinary Evolved PvE, the first generation does not distinguish death protection by whether an ordinary item was looted recently, returned to town, equipped during a hunt or previously secured.

Ordinary item protection is determined by:

```text
item eligibility policy
+ valid blessing count at the owning death occurrence
```

not by a mandatory expedition extraction state machine.

This intentionally prioritizes player clarity and Tibia-recognizable preparation semantics over a more complex extraction-style loop.

## 11. Explicit item-policy overrides

This baseline is the ordinary default, not authority to make every possible item family droppable.

A separately accepted item policy may protect or specially dispose of item families such as:

- quest-critical items;
- unique/nonreplaceable progression items;
- system keys/tokens;
- explicitly bound/nontransferable narrative items;
- non-item ledger/account value;
- other typed protected-value families.

If a critical/unknown item family lacks accepted death semantics, implementation must fail closed or preserve the item rather than invent destructive behavior.

## 12. Blessing validity and authority

The server-authoritative valid blessing set is bound to the death occurrence.

Stale, expired, invalid or already-consumed blessing state cannot be used to obtain protection after the fact.

This baseline does not yet freeze:

- acquisition locations/NPCs/quests;
- prices/currency;
- renewal/consumption timing;
- whether ordinary PvE death consumes any or all blessings;
- persistence schema or protocol representation.

Those are separate decisions.

## 13. PvP and Reference isolation

This baseline does not authorize:

- Oteryn Reference item-loss changes;
- red/black-skull behavior;
- PvP lootability/theft/public corpse rules;
- Twist-of-Fate-like Evolved behavior;
- PvP blessing consumption/protection;
- Channel-PvP-specific item-risk modifiers.

Those remain separate profile/world-mode decisions.

## 14. Failure, retry and anti-duplication

Death item disposition is one authoritative outcome per logical death occurrence.

The implementation must preserve:

- one authoritative item location;
- one authoritative death occurrence identity;
- one committed retain/drop outcome per item/container decision;
- idempotency under retry;
- revision/generation fencing;
- deterministic reconciliation after ambiguous outcomes.

Never:

```text
same item retained by Character
AND
same item present in corpse/reclaim
```

A missing required authority fails closed.

## 15. Evidence-revisitable status

This is intentionally a first-generation playable default, not a claim that final balance is known before Oteryn exists as a representative physical game.

```text
REVISIT_AFTER_PLAYABLE_GAME_EVIDENCE = true
```

After real gameplay exists, the owner explicitly expects review of at least:

- whether 0-bless backpack loss is too punitive or too weak;
- whether the per-equipped-item probabilities create meaningful tension;
- whether full item protection at five blessings is too easy or appropriately recognizable;
- whether blessings six and seven need an additional non-XP purpose;
- whether corpse -> reclaim makes item loss too forgiving;
- real replacement cost of BIS and ordinary gear;
- actual blessing cost versus player income;
- player understanding and frustration/support signals;
- death frequency and reclaim usage.

The future review may retain, tune or supersede the table through explicit owner-reviewed architecture authority.

A future evidence-driven change is expected and is not considered an architecture failure.

Until such supersession is protected, runtime must not silently drift from this baseline.

## 16. Future implementation/test consequences

A later authorized implementation must prove at minimum:

1. `b=0` gives 100% ordinary backpack/container drop chance and 10% per eligible equipped item;
2. `b=1` gives 70% / 7%;
3. `b=2` gives 45% / 4.5%;
4. `b=3` gives 25% / 2.5%;
5. `b=4` gives 10% / 1%;
6. `b>=5` gives 0% / 0% ordinary-PvE item loss;
7. blessings six and seven still affect XP recoverability under protected #471;
8. retry/reconnect/failover cannot reroll a committed logical death disposition;
9. a dropped container cannot duplicate or orphan its descendants;
10. dropped value enters the protected corpse -> reclaim custody path;
11. no mandatory `SECURE_EXTRACTION` or `UNSECURED_EXPEDITION_LOOT` state is required by ordinary first-generation PvE;
12. explicit protected-item overrides remain possible and unknown critical families fail closed;
13. Reference and PvP fixtures prove this Evolved ordinary-PvE policy cannot activate outside its scope;
14. telemetry exists to support the planned playable-evidence review.

## 17. Deliberately deferred

This baseline does not freeze:

- exact blessing prices;
- blessing NPCs/quests/acquisition;
- blessing renewal/consumption semantics;
- Amulet-of-Loss-like Evolved behavior;
- exact protected quest/critical item taxonomy;
- PvP item-loss rules;
- red/black-skull behavior;
- reclaim fee/capacity/retention;
- exact deterministic RNG implementation;
- persistence schema;
- protocol/UI representation;
- runtime implementation allocation;
- live deployment authority.

## 18. Acceptance boundary

Accepted now:

```text
profile                               = Oteryn Evolved
scope                                 = ordinary PvE
item-risk model                       = blessing-based
0 bless backpack/container drop       = 100%
1 bless backpack/container drop       = 70%
2 bless backpack/container drop       = 45%
3 bless backpack/container drop       = 25%
4 bless backpack/container drop       = 10%
5+ bless backpack/container drop      = 0%
0 bless per-equipped-item drop        = 10%
1 bless per-equipped-item drop        = 7%
2 bless per-equipped-item drop        = 4.5%
3 bless per-equipped-item drop        = 2.5%
4 bless per-equipped-item drop        = 1%
5+ bless per-equipped-item drop       = 0%
bless 6..7 item protection            = no additional effect beyond full protection
bless 6..7 XP recoverability          = yes, protected #471 unchanged
#478 expedition-risk product model    = superseded for ordinary PvE item-loss selection
mandatory secure extraction           = no
post-window dropped-item outcome       = protected #474 reclaim
Reference                             = unchanged
PvP                                   = deferred
playable evidence revisit             = explicitly required/allowed
silent runtime drift before supersession = forbidden
```

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
