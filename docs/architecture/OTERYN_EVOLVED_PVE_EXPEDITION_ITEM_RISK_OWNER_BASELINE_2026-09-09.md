# Oteryn Evolved PvE Expedition Item-Risk — Owner Baseline

- Status: **OWNER_ACCEPTED FIRST-GENERATION DEFAULT**
- DecisionStatus: `ACCEPTED_WITH_PLAYABLE_EVIDENCE_REVISIT`
- Date: 2026-09-09
- Coordination issue: `#220`
- Source type: `USER_SOURCE`
- Protected source base: `main@edc1c39adbfbcf0e58dbeb268adab175e00003bd`
- Scope: ordinary Oteryn Evolved PvE item-risk classification and secure-extraction boundary
- Runtime/client/server/protocol/DDL/migration/Platform/Atlas/production authority: **NONE**

## 1. Owner decision

For first-generation ordinary Oteryn Evolved PvE, death must create meaningful item-risk without randomly destroying established long-term equipment progression.

The accepted product split is:

```text
SECURED_LONG_TERM_VALUE
  = previously secured long-term equipment/value
  -> protected from ordinary Evolved PvE corpse transfer

UNSECURED_EXPEDITION_LOOT
  = value obtained during the current expedition after the last accepted secure extraction
  -> remains death-risk eligible until an explicit server-authoritative secure extraction succeeds
```

This is an `EVOLVED` ordinary-PvE decision only. It does not alter Oteryn Reference and does not select PvP/theft/public-loot rules.

## 2. Evidence-revisitable status

This is intentionally a first-generation default for building and testing a physically playable Oteryn Evolved loop.

```text
REVISIT_AFTER_PLAYABLE_GAME_EVIDENCE = true
```

Once the game has representative hunting, loot, item values, death/recovery, reclaim and economy behavior, the owner explicitly permits this model to be retained, tuned or superseded through normal reviewed architecture authority.

Relevant evidence includes:

- whether expedition risk creates meaningful tension;
- whether secured gear protection feels too safe or correctly protects long-term progression;
- whether players understand secured versus unsecured value;
- how often normal hunts end in death versus successful extraction;
- real loot value/hour and equipment replacement cost;
- abuse through trading, alts, containers, crafting, currency conversion or stack operations;
- reclaim frequency and congestion;
- blessing interaction;
- support/UX burden from risk-state confusion.

Until a superseding owner-reviewed decision is protected, runtime must not silently drift from this baseline.

## 3. Core semantic principle

Risk state belongs to authoritative value provenance, not to presentation slot or current holder alone.

Therefore:

```text
unsecured item equipped != secured item
unsecured item moved into an old backpack != secured item
unsecured item transferred to another Character != secured item
```

A change in inventory position, equipment slot, container parent, Character custody, Channel or session does not itself grant secure status.

This follows GAME-ITEM-01 separation of ItemInstance identity from current location and DUR-03 one-location/conservation rules.

## 4. Secured long-term value

`SECURED_LONG_TERM_VALUE` is ordinary-PvE protected value that has already crossed an accepted secure-extraction boundary or is otherwise explicitly protected by a separately accepted item policy.

First-generation ordinary PvE intent:

- established equipped gear is not randomly dropped merely because the Character dies;
- previously secured inventory/value remains outside ordinary expedition-corpse transfer unless another explicit rule says otherwise;
- valuable long-term progression should not be repeatedly destroyed/reacquired as the primary ordinary-death loop.

This does not make secured value universally immune to every game system. PvP, explicit destruction/consumption, crafting, trade, binding, decay or other authorized systems remain under their owning contracts.

## 5. Unsecured expedition loot

`UNSECURED_EXPEDITION_LOOT` is value whose current expedition provenance has not yet crossed an accepted secure-extraction boundary.

Typical candidate sources include later-authorized:

- creature loot materialized during the expedition;
- physical currency/valuable items obtained during the expedition;
- expedition reward objects obtained outside a secure settlement boundary;
- derived value produced from unsecured inputs before secure extraction.

The exact source taxonomy is not frozen here. The invariant is that value explicitly admitted as unsecured remains unsecured until an authoritative secure operation says otherwise.

## 6. Ordinary PvE death disposition

For a qualifying ordinary Evolved PvE death:

```text
SECURED_LONG_TERM_VALUE
  -> remains in protected Character/owning durable custody

UNSECURED_EXPEDITION_LOOT selected as corpse-bound
  -> DEATH_CORPSE_CUSTODY
  -> direct corpse recovery during the protected recovery window
  -> if unrecovered at window expiry:
       RECLAIM_SALVAGE_CUSTODY
```

Protected #474 remains the authority for the post-window reclaim transition.

This baseline does not change the protected XP death envelope, DeathDebt, blessing XP ladder or 30-minute recovery window.

## 7. Secure extraction is explicit

Only an explicit server-authoritative secure-extraction operation may promote eligible unsecured expedition value into secured value.

Conceptually:

```text
UNSECURED_EXPEDITION_LOOT
  -- accepted SECURE_EXTRACTION -->
SECURED_LONG_TERM_VALUE
```

A secure extraction must be an authoritative game event/transaction, not a client interpretation such as "I entered a safe-looking tile".

The exact first-generation qualifying services/locations are deliberately deferred. Candidate forms may later include an accepted safe hub, depot/settlement service, temple/town extraction point or another explicitly versioned secure boundary.

## 8. Anti-laundering invariants

The following operations must not independently convert unsecured value into secured value:

- equipping the item;
- unequipping it;
- moving it between inventory slots;
- placing it inside a previously secured container;
- removing it from a container;
- relog/reconnect/client restart;
- Channel switch or region failover;
- ordinary same-World Character-to-Character transfer;
- transfer to an alt Character;
- stack split;
- stack merge;
- identity-preserving transform;
- crafting/transform/conversion whose inputs are unsecured, unless the operation itself is explicitly also an accepted secure-extraction boundary;
- server restart or persistence rehydration.

No subsystem may treat a change of owner/location/representation as implicit laundering of expedition risk.

## 9. Transfer semantics

If unsecured value is legally transferred before secure extraction, the receiving value remains unsecured unless the owning operation is explicitly defined as a secure-extraction boundary.

Conceptually:

```text
Character A / UNSECURED
  -- ordinary transfer -->
Character B / UNSECURED
```

This prevents use of alts, party members or ordinary trade as a free extraction channel.

Exact trade/market/mail/depot admissibility for unsecured value is deferred to the owning domain contracts. If a surface cannot preserve the risk classification safely, it must fail closed rather than silently secure the value.

## 10. Split, merge and fungible-value conservation

Risk provenance must survive quantity operations.

For stackable/fungible value:

- splitting unsecured quantity cannot create secured units;
- merging unsecured and secured value cannot erase the unsecured amount;
- later implementation must preserve enough typed lineage/accounting to prove how much resulting value remains unsecured;
- the exact physical representation of mixed secured/unsecured fungible quantities is deferred.

Safety invariant:

```text
UnsecuredValueAfterOperation
>= conserved unsecured input value
  - explicitly authorized consumption/burn
  - explicitly successful secure-extraction amount
```

A merge/split implementation that cannot prove this must fail closed.

## 11. Transform, crafting and conversion

An ordinary value transform must not be a laundering path.

If unsecured inputs produce a new value-bearing output before secure extraction, the output must retain equivalent unsecured exposure according to the owning transform/conservation rule unless that operation is explicitly accepted as a secure-extraction boundary.

DUR-03 mint/burn/transform/conversion lineage remains binding. No conversion may silently transform expedition-risk value into protected long-term value merely because ItemInstance identity changes.

Exact inheritance for multi-input recipes, partial consumption and mixed provenance remains a later implementation-contract problem, but conservation of unsecured exposure is mandatory.

## 12. Containers

Container position does not define risk.

Examples:

```text
secured backpack + unsecured loot inside
!= all contents secured

unsecured container + previously secured item inside
!= secured item becomes unsecured merely from parent change
```

Each live value-bearing item/quantity follows its accepted risk/provenance policy. Container moves must preserve descendant legality and DUR-03 one-location semantics without flattening risk state into the container itself unless a later typed rule explicitly does so.

## 13. Equipment

Equipping freshly looted gear during the expedition is permitted only if GAME-ITEM/ruleset legality allows it, but equipment occupancy does not secure it.

Therefore an item can be:

```text
equipped + UNSECURED_EXPEDITION_LOOT
```

and remain eligible for the ordinary-PvE death disposition selected for unsecured value.

This prevents a one-click equip action from bypassing the expedition loop.

## 14. Currency and ledger boundary

Physical item-currency/value obtained during an expedition may be classified as unsecured under this model.

Non-item bank/account ledger state is not automatically a corpse item and remains under its owning economy authority.

However, depositing or converting unsecured physical value into ledger value must not become an implicit laundering path. Such an operation may secure the value only if the owning service is explicitly accepted as a secure-extraction boundary.

Exact bank/depot/economy integration remains deferred.

## 15. Quest, unique, bound and critical-value overrides

Some item families may require explicit policy overrides for safety/game design, including later-defined:

- quest-critical items;
- unique/nonreplaceable progression items;
- account/Character-bound items;
- system tokens/keys;
- nontransferable narrative objects.

This baseline does not invent their final classification.

If no accepted rule exists for a critical item family, runtime must fail closed or keep it protected rather than infer destructive/corpse behavior from generic loot classification.

## 16. Blessing relationship

Protected Evolved blessing architecture permits blessings to improve unsecured expedition-loot protection/reclaim conditions.

This baseline does not choose:

- a per-blessing item-protection percentage;
- guaranteed protected slots;
- reclaim-fee reduction;
- whether all seven blessings are required for a particular item effect.

Any later blessing item effect must preserve the nominal XP rule, one authoritative item location, risk provenance and DUR-03 conservation.

## 17. PvP and Reference isolation

This baseline does not authorize:

- Reference item-loss behavior changes;
- PvP lootability/theft/public-corpse rules;
- skull/Fair Fight disposition;
- Twist-of-Fate-like behavior;
- Channel-PvP-specific risk multipliers.

Those remain separate product decisions.

## 18. Failure and reconciliation

Secure extraction, death disposition and reclaim transitions are authoritative value mutations and must be retry-safe.

If outcome is ambiguous:

- do not mark value secured merely because the client saw success;
- do not duplicate value into both Character and corpse/reclaim custody;
- reconcile the same operation identity and durable receipt;
- stale runtime state cannot overwrite a newer risk/custody result.

A missing required authority fails closed.

## 19. Future implementation/test consequences

A later authorized implementation must prove at minimum:

1. secured long-term value is not moved to ordinary Evolved PvE corpse custody by default;
2. newly admitted unsecured expedition loot remains unsecured until explicit secure extraction;
3. equip/unequip cannot secure unsecured value;
4. container moves cannot secure unsecured value;
5. relog/reconnect/Channel switch/failover cannot secure unsecured value;
6. ordinary Character-to-Character transfer cannot secure unsecured value;
7. stack split/merge cannot erase unsecured quantity exposure;
8. transform/craft/conversion cannot launder unsecured value;
9. successful accepted secure extraction changes risk state exactly once and idempotently;
10. ordinary PvE death moves qualifying unsecured value into the protected corpse/reclaim custody flow without duplication;
11. critical/unknown item families fail closed rather than receive invented risk semantics;
12. Reference and PvP fixtures prove this Evolved ordinary-PvE policy cannot activate outside its scope;
13. telemetry exists for the planned post-playable review.

## 20. Planned playable-evidence review

Once representative Oteryn Evolved gameplay exists, explicitly review:

- whether all expedition loot should remain at risk until extraction;
- whether selected categories should secure earlier/later;
- whether specific safe boundaries are too convenient or too punitive;
- whether ordinary trade/party logistics need exceptions;
- whether blessing item protection is needed;
- whether reclaim makes unsecured risk too forgiving;
- whether long-term gear protection removes too much death tension.

A future adjustment is expected to be evidence-driven and may supersede this baseline with explicit migration/disposition rules.

## 21. Deliberately deferred

This baseline does not freeze:

- exact item-source taxonomy classified as unsecured;
- exact safe hubs/NPCs/services that perform secure extraction;
- exact UI terminology/iconography;
- reclaim fees/capacity/retention;
- blessing item-protection numbers;
- PvP item-loss rules;
- market/mail/trade/depot detailed treatment;
- mixed-provenance stack storage representation;
- persistence schema/protocol fields;
- runtime implementation allocation;
- live deployment authority.

## 22. Acceptance boundary

Accepted now:

```text
profile                                  = Oteryn Evolved
scope                                    = ordinary PvE
secured long-term gear/value             = protected from ordinary corpse transfer by default
current-expedition unsecured loot        = death-risk eligible
secure promotion                         = explicit server-authoritative SECURE_EXTRACTION only
equip/container/relog/channel switch     = never implicit securing
ordinary transfer to another Character  = preserves unsecured state
split/merge/transform                    = cannot launder unsecured value
ordinary death disposition               = corpse -> protected #474 reclaim flow when applicable
quest/critical overrides                 = deferred / fail closed
blessing item effects                    = deferred
PvP                                      = deferred
Reference                                = unchanged
playable evidence revisit                = explicitly required/allowed
silent runtime drift before supersession = forbidden
```

`IMPLEMENTATION_AUTHORITY: NONE`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
