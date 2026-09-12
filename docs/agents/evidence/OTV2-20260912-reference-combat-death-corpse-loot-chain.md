# Oteryn Reference Combat — ordinary death/corpse/loot chain evidence

Status: **READ-ONLY EVIDENCE / NON-AUTHORITY**  
Date: 2026-09-12  
Refs: #483, #506, #513  
Target: `global-tibia-observable-2026-07-28-post-server-save`  
Scope: ordinary single-player creature path only  
Durability rule: **NO_DURABILITY_OWNERSHIP_INVENTED**

## 1. Purpose and exclusions

This document closes the narrow Global-observable semantics needed by #506/#513 for:

```text
committed lethal result
-> ordinary creature death
-> XP consequence
-> corpse creation/access
-> initial loot authorization + immovability
-> ordinary corpse interaction
-> boundary between Combat loot selection and durable pickup authority
```

Explicitly excluded:

- party/shared XP and party eligibility breadth;
- PvP/player death/skulls/fair-fight/item loss;
- bosses/raids/events/multi-principal rewards;
- broad Quick Loot Nearby Corpses behavior;
- full corpse decay/despawn timing;
- exact natural loot probabilities unless separately target-proven;
- proprietary/internal CipSoft persistence, database, item identity or transaction implementation;
- Evolved product mechanics.

No OTS implementation is used as Global Reference authority. Canary/Crystal/other OTS code may only be hypothesis/test-discovery input under #483.

## 2. Evidence discipline

The repository evidence policy is fail-closed:

```text
current official page after 2026-07-28
!= automatic proof of exact 2026-07-28 behavior

no discovered patch note
!= proof that behavior did not change

OTS implementation
!= proof of Global behavior
```

Source priority:

1. official CipSoft/Tibia public sources and provenance-cleared primary captures;
2. lawful controlled black-box observation;
3. reputable community corroboration;
4. OTS as `OTS_HYPOTHESIS_ONLY`.

Classifications used here:

- `PROVEN` — directly established by qualifying primary evidence for the stated field;
- `DERIVED` — conservative inference from proven/official facts;
- `UNKNOWN` — not sufficiently proven and must remain open;
- `CONFLICT` — a candidate interpretation contradicted by stronger evidence.

## 3. Binding Oteryn owner separation

Live #506 binds the first Combat death workflow:

```text
one admitted player / current runtime owner
-> GAME-ABILITY committed result
-> at most one creature death occurrence per creature generation
-> runtime corpse projection
-> two separately reconciled descendants:
     loot materialization
     single-principal XP reward
-> acknowledged durable loot
-> retry-safe pickup through GAME-INTERACTION + GAME-ITEM + DUR-03
```

Loot and XP are not one distributed transaction. Creature death/runtime corpse stays runtime-owned; durable item/value truth remains DUR-03; persistent XP remains GAME-CHAR/DUR-02.

Live #513 binds the durable side:

```text
Combat owns death occurrence + deterministic loot-selection intent/plan
GAME-ITEM owns item/location/container legality
DUR-03 owns durable ItemInstance/value transaction and conservation
```

First VSL ordering is explicitly:

```text
loot output intent
-> DUR-03 MINT
-> acknowledged durable ItemInstance / corpse custody
-> LOOT_READY
-> player pickup
-> DUR-03 TRANSFER same ItemInstance
-> Character destination
```

This is an Oteryn correctness/authority decision. It is **not** claimed to reproduce proprietary CipSoft internals.

## 4. Official source index

Primary source anchors used by the bounded findings:

- CipSoft 2006-08-01 Major Game Update 7.8 — first-10-second highest-damage loot protection:
  - https://www.tibia.com/news/?id=399&subtopic=newsarchive
- CipSoft 2010 Spring Patch — stamina reward behavior, including no monster loot for highest-damage character at/below the final 14 stamina hours:
  - https://www.tibia.com/news/?id=1263&subtopic=newsarchive
- CipSoft 2023-06-09 More Convenience Features — Loot Highlight tied to the first 10 seconds and corpse/container lifecycle:
  - https://www.tibia.com/news/?id=7337&subtopic=newsarchive
- CipSoft 2023 programmer explanation of loot generation timing — simple loot may be generated with the monster while dynamic loot may be resolved at death:
  - https://www.tibia.com/news/?id=7409&subtopic=latestnews
- CipSoft 2024 Quick Loot Nearby Corpses — breadth extension of existing quick-loot behavior:
  - https://www.tibia.com/news/?id=7907&subtopic=newsarchive
- CipSoft 2026-07-28 Balancing, Fixes and Changes — direct target-boundary XP rows, including Iceplume Strider `7,500 -> 8,150`:
  - https://www.tibia.com/news/?id=8905&subtopic=newsarchive
- current official Controls manual — ordinary corpse opening/looting, first-10-second highest-damage rule, corpse immovability, inventory/capacity behavior:
  - https://www.tibia.com/gameguides/?section=controls&subtopic=manual
- current official Quickstart:
  - https://www.tibia.com/gameguides/?subtopic=quickstart
- official shared-experience / combat support and manual surfaces for damage-based distribution when shared XP is not active:
  - https://www.tibia.com/support/?entryid=92&subtopic=gethelp

The repository #483 evidence continuation additionally records a target-era 2026 corpse/lootability bugfix shortly before the target boundary. That is useful support for continuity of the `death -> corpse -> lootable object` model, but does not by itself prove every corpse rule or exact timer quantisation.

## 5. Atomic evidence matrix

| ID | Classification | Atomic assertion | Continuity to target | Owner |
|---|---|---|---|---|
| `DEATH-01` | `DERIVED` strong | ordinary creature death produces a corpse/body/container state that can participate in loot interaction | supported by longstanding official corpse/container semantics plus 2026 target-era corpse/lootability evidence; exact special-creature exceptions are not generalized | #506 |
| `ATTR-01` | `DERIVED` scoped | the first fixture may explicitly construct one player as the sole damage contributor and source of the committed lethal damage occurrence | fixture precondition, not a universal Global attribution algorithm | #506 |
| `ATTR-02` | `DERIVED` strong | when shared XP is not active, XP distribution follows damage contribution | long official continuity from shared-XP introduction through current official support | #506 |
| `ATTR-03` | `DERIVED` | one sole contributor with 100% contribution has a 100% damage-derived share before separately modelled modifiers | arithmetic consequence of `ATTR-02` | #506 |
| `ATTR-04` | `DERIVED` strong | protected corpse/loot principal is the highest-total-damage character, not merely the last hitter | official 2006 rule + 2023/current continuity | #506 |
| `ATTR-05` | `CONFLICT` | `last_hit == loot_owner == xp_owner` as a universal model | contradicted: XP contribution and protected loot authority use different semantic predicates | #506 |
| `XP-01` | `PROVEN` target-boundary | Iceplume Strider base creature XP is `8,150` after the 2026-07-28 server-save change (`7,500 -> 8,150`) | direct official target-boundary source | #483 input -> #506 |
| `XP-02` | `DERIVED` | `8,150` is not automatically the final Character XP delta in every context | stamina/party/events/boosts are separate layers; first fixture must neutralise or stop at the base reward boundary | #506 / GAME-CHAR |
| `XP-03` | `DERIVED` strong | final 14 stamina hours reduce XP and a highest-damage character at/below that threshold can cause monster loot to be absent/destroyed | official 2010 rule with continuing official support | #506 |
| `CORPSE-01` | `DERIVED` strong | first 10 seconds after the ordinary creature kill use highest-damage loot authorization | official 2006 -> 2023 -> current continuity chain | #506 |
| `CORPSE-02` | `DERIVED` strong | corpse movement is blocked during the same protected first-10-second interval | current official Controls/Quickstart with continuity to the established protection model | #506 |
| `CORPSE-03` | `UNKNOWN` | exact `t = 10.000s` server-tick/rounding/clock quantisation | no qualifying source found; do not invent | #506 |
| `LOOT-01` | `DERIVED` strong | `who may loot`, `whether the corpse may move`, and `whether loot exists` are distinct facts | highest-damage protection and stamina-driven no-loot are separate official rules | #506 |
| `LOOT-02` | `DERIVED` strong | ordinary interaction supports opening the corpse/container, inspecting possessions and choosing a concrete item to loot | current official Controls plus longstanding pre-target looting model | #506 -> #513 boundary |
| `LOOT-03` | `DERIVED` strong | successful ordinary pickup observably moves the chosen item from corpse/container state into player inventory/container state, subject to legality/space/capacity | official Controls and earlier quick-loot semantics | GAME-ITEM -> #513 |
| `LOOT-04` | `PROVEN` pre-target internal explanation | not all Global loot is necessarily selected at death: some simple/static loot may be generated earlier while dynamic loot can be resolved on death | official CipSoft programmer explanation; exact target-day internals are not promoted | #506 caution |
| `LOOT-05` | `CONFLICT` | `all Global loot RNG occurs in the death handler` | contradicted by `LOOT-04` | #506 |
| `DUR-01` | `UNKNOWN` from Global | Global public behavior does not prove `ItemInstanceId`, DB commit point, `TransactionId`, durable custody storage or retry policy | intentionally outside observable parity | #513 |
| `DUR-02` | `PROVEN` Oteryn-native | first VSL materialises acknowledged loot before interactable pickup and transfers the same durable ItemInstance on pickup | binding #513/VSL-COMBAT decision | #513 |
| `DUR-03` | `PROVEN` Oteryn-native | pickup cannot create a second mint or leave simultaneous source+destination truth | binding DUR-03 anti-duplication boundary | #513 |

## 6. Kill attribution: do not invent one generic owner

The first single-player fixture does not require a general multi-attacker `killer_owner` algorithm.

Keep these facts logically distinct:

```text
lethal_source
    = source of the committed lethal damage occurrence

xp_contribution
    = damage contribution used by the non-shared XP consequence

protected_loot_principal
    = character with highest total damage for the protected corpse interval
```

For the first bounded fixture:

```text
player_count = 1
damage_contribution[player] = 100%
shared_xp = NOT_EXERCISED
player supplies committed lethal damage occurrence
```

All three facts point to the same character only because the fixture contains one contributing player.

Do **not** generalise to:

```text
killer_owner = last_hit_character
killer_owner = xp_owner = corpse_owner
```

Tie handling, summons and multi-attacker ownership remain outside this slice.

## 7. XP consequence

The evidence-clean target-boundary case is Iceplume Strider:

```text
Given:
  target creature identity = Iceplume Strider
  creature already has a legal lethal remaining-HP state
  one solo player
  player contribution = 100%
  stamina > 14h
  shared XP not exercised
  no separately modelled event/boost modifier
When:
  one legal final damage occurrence commits
Then:
  alive -> dead occurs once
  base target-cut XP consequence = 8,150
  corpse/loot-authority transition begins
```

The strong assertion is:

```text
base_creature_xp_consequence = 8150
```

not an unconditional:

```text
Character.experience += 8150
```

because final Character reward can include separately modelled reward modifiers.

This fixture intentionally does **not** assert creature max HP, full AI, complete attack journey or complete loot table.

## 8. Corpse creation and protected access

For the ordinary creature path the evidence supports:

```text
committed creature death
-> corpse/body/container becomes observable and loot-interactable

protected interval:
  semantic duration = 10 seconds
  authorised principal = highest-damage character
  corpse movement = denied
```

The continuity classification remains `DERIVED` strong rather than `PROVEN` exact-target because no exact 2026-07-28 archived manual snapshot was found for the timer rule and repository evidence discipline forbids treating absence of a discovered change as proof that no change occurred.

### Exact timer boundary

Do not claim:

```text
at death_time + exactly 10_000 ms:
  permission flips in the same exact server tick
```

without controlled evidence.

The first parity fixture can instead distinguish safely:

```text
inside protected interval -> protection applies
clearly after interval     -> special highest-damage exclusivity no longer applies
```

A future target-controlled observation may close exact boundary quantisation if such precision is required.

### Protection is not a durable owner lease

The 10-second rule represents loot authorization + corpse immovability. It is not evidence of a durable `corpse_owner` lease.

In particular:

```text
highest-damage principal known
AND
stamina <= 14h
```

can result in no loot value at all.

Therefore preserve separate state for:

```text
loot_access_principal
corpse_movement_protection
loot_selection/output
```

## 9. Ordinary corpse interaction

The first non-quick-loot breadth fixture needs only:

```text
player opens corpse/body container
-> contents become inspectable
-> player selects one concrete item
-> player requests Loot
-> GAME-ITEM destination legality/space/capacity is checked
-> successful movement becomes observable
```

Do not broaden this child into:

- Quick Loot Nearby Corpses aggregation;
- nine surrounding fields;
- per-field multi-corpse breadth;
- accepted/skipped loot lists;
- managed loot containers;
- recursive container-routing policy.

Those are later quick-loot/container breadth concerns.

## 10. Loot selection is not durable pickup authority

### #506 / Combat side

Combat owns the death occurrence and bounded loot-selection intent/plan:

```text
committed lethal result
-> exactly one death occurrence
-> runtime corpse projection

loot descendant:
  evaluate bounded candidate set
  apply target content/rules
  consume bounded RNG/work
  produce zero-or-more exact loot output intents
```

#506 resource rows for loot candidates, planned outputs, RNG work and retained plan state therefore measure Oteryn Combat implementation resources. They are not claims that CipSoft uses the same internal algorithm or timing.

### #513 / Durability side

The boundary starts when one exact acknowledged loot output must become durable value:

```text
one committed Combat death occurrence
+ one exact acknowledged loot-output intent
+ exact item definition/revisions/provenance
-> DUR-03 MINT
-> exactly one durable ItemInstance/custody result
```

Pickup is a separate custody transfer:

```text
one already-durable eligible ItemInstance
+ source corpse/container custody
+ Character inventory destination
+ GAME-ITEM legality
-> DUR-03 TRANSFER
-> exactly one authoritative destination custody
```

No second mint is permitted at pickup.

## 11. Why Global durability ownership remains UNKNOWN

Public Global evidence can support only the observable transition:

```text
item is present in corpse/container interaction state
-> player chooses/picks it up
-> item is observably present in inventory/container state
```

It does not prove:

- durable item identifier type;
- database row/storage shape;
- transaction identifier;
- exact commit instant;
- audit/outbox model;
- retry count/backoff;
- source/destination persistence representation;
- restart reconciliation internals.

Those properties are Oteryn-native correctness contracts owned by DUR-03. They must not be reverse-inferred from Global observable behavior or copied from OTS code.

## 12. Conflict ledger

### `CONFLICT-01` — last hit as protected loot owner

Reject. Official loot protection names the highest-total-damage character, not merely the character delivering the fatal hit.

### `CONFLICT-02` — one universal killer owner

Reject. XP contribution and corpse protection are separate rules. They coincide in the one-player fixture but are not one semantic field.

### `CONFLICT-03` — authorization implies loot exists

Reject. Stamina can remove/destroy monster loot independently of which player is authorised.

### `CONFLICT-04` — all loot selected at death

Reject. Official CipSoft programmer evidence documents mixed generation timing.

### `CONFLICT-05` — 10-second expiry creates durable readiness

Reject. Global access timing and Oteryn durable settlement readiness are independent.

### `CONFLICT-06` — runtime corpse owns durable value

Reject. Runtime corpse projection is not a second durable item/value authority.

### `CONFLICT-07` — pickup mints the first VSL item

Reject for the accepted first VSL. Materialisation precedes interactable pickup.

## 13. Unknown ledger

### `UNKNOWN-01` — exact 10-second tick/rounding

No qualifying official evidence establishes precise boundary quantisation.

Impact: do not write a parity-confirmed `9999/10000/10001 ms` fixture yet. This does not block an inside-window versus clearly-after-window fixture.

### `UNKNOWN-02` — equal highest-damage tie handling

Out of scope for the sole-player fixture.

### `UNKNOWN-03` — summon attribution

Out of scope for the direct sole-player fixture.

### `UNKNOWN-04` — general multi-attacker kill-credit algorithm

Not needed for the selected first child.

### `UNKNOWN-05` — full corpse decay/despawn timing

Explicitly deferred by #506 beyond the exact first-child need.

### `UNKNOWN-06` — exact natural loot table/probabilities for the selected target creature

This is a #483/#504 Content evidence problem. It can block a fully natural target-loot fixture, but does not block death/corpse/authorization semantics or synthetic resource-limit fixtures.

### `UNKNOWN-07` — proprietary Global durable transaction implementation

Intentionally unnecessary for observable parity; #513 provides Oteryn-native correctness.

## 14. Questions owned by #506

#506 must own or consume evidence for:

- exactly-one death occurrence per creature generation/lethal occurrence;
- death identity/replay relationship;
- runtime corpse projection;
- highest-damage authorization fact;
- semantic protected deadline;
- corpse immovability during the protected interval;
- stamina-driven zero-loot case;
- bounded loot candidates, outputs and RNG work;
- exact loot output intent;
- separate XP consequence;
- only any Combat-retained opaque descendant references actually required by the implementation.

#506 must not:

- implement a second item transaction engine;
- become durable ItemInstance/value authority;
- write Character XP directly;
- invent DUR-03 retry/audit maxima.

## 15. Questions owned by #513

#513 owns:

- durable MINT of one exact acknowledged output;
- stable durable item identity;
- source corpse/container custody;
- atomic pickup TRANSFER to Character inventory;
- destination legality handoff;
- duplicate/concurrent pickup reconciliation;
- lost-response and ambiguous-commit recovery;
- stale runtime/session/lease fence rejection;
- transaction audit/outbox and retry/reconciliation bounds;
- no duplicate mint and no split source+destination durable truth.

#513 does not own:

- kill attribution;
- XP distribution;
- highest-damage corpse authorization;
- the 10-second Global access rule;
- stamina loot suppression;
- creature loot probability/RNG.

## 16. Minimal positive Reference fixture

```text
Given:
  ordinary creature
  one admitted player
  player is sole damage contributor
  player contribution = 100%
  shared XP is not exercised
  stamina > 14h
  no separately modelled XP/loot modifier
  creature already has a legal lethal remaining-HP state

When:
  one legal committed player damage occurrence reaches zero HP

Then:
  exactly one CreatureDeathOccurrence exists
  exactly one runtime corpse projection exists

XP descendant:
  one base creature-XP consequence exists
  direct target case may assert Iceplume Strider = 8150

Loot descendant:
  bounded loot selection produces zero-or-more exact output intents

Corpse authorization:
  player is highest-damage principal
  during protected 10-second interval:
    player is authorised to loot
    corpse movement is denied

Ordinary interaction:
  player opens corpse/container
  player inspects contents
  player chooses one item

Durability boundary:
  exact selected output reaches acknowledged durable corpse/container custody
  loot state becomes LOOT_READY

Pickup:
  GAME-ITEM destination legality passes
  DUR-03 transfers the same durable ItemInstance
  corpse custody -> Character inventory custody

Invariants:
  no second death
  no duplicate XP
  no duplicate mint
  no source+destination double truth
  authorization expiry cannot fabricate durable value
```

## 17. Minimal negative evidence suite

### Global-observable cases

```text
N1 unauthorized other player inside protected interval
   -> loot denied
   -> corpse move denied

N2 highest-damage principal with stamina <= 14h
   -> no loot output
   -> authorization does not fabricate loot

N3 clearly after protected interval
   -> special highest-damage exclusivity no longer applies
   -> no assertion about exact 10.000-second tick
```

### Oteryn correctness cases

```text
N4 authorised principal + LOOT_SETTLEMENT_PENDING
   -> no durable movement

N5 destination capacity/legality failure
   -> no successful pickup / no partial durable mutation

N6 duplicate or concurrent pickup
   -> at most one authoritative destination custody

N7 committed pickup + lost response/retry
   -> reconcile same logical committed result
   -> do not restore source custody

N8 committed materialisation + stale/recreated corpse projection
   -> no second mint

N9 duplicate lethal replay
   -> no second death occurrence
   -> no duplicate XP
   -> no duplicate loot workflow
```

## 18. Target-continuity disposition

### `PROVEN` exact target-boundary field

- Iceplume Strider base XP `8,150` after the 2026-07-28 server save.

### `DERIVED` strong continuity

- ordinary corpse/container loot interaction;
- highest-damage first-10-second authorization;
- corpse immovability during that interval;
- non-shared damage-based XP distribution;
- stamina loot suppression;
- selected-item corpse-to-inventory observable pickup.

These have strong official historical/pre-target/current continuity but are not promoted to exact-target `PROVEN` merely because no intervening change was found.

### `UNKNOWN`

- exact 10-second tick/rounding;
- ties/summons/multi-attacker breadth;
- exact natural target creature loot table unless separately proven;
- proprietary Global durability implementation.

## 19. Completion result

For the requested ordinary single-player scope, the observable Global semantics needed to shape the first #506/#513 Reference fixture are evidence-closed without inventing durability ownership.

Remaining open items are deliberately separate:

1. controlled target observation if exact 10-second boundary quantisation is ever required;
2. #483/#504 target evidence for exact natural creature loot table/probabilities;
3. #506 Combat resource-limit acceptance and physical runtime carrier work;
4. #513 DUR-03 resource-limit acceptance and implementation allocation;
5. Character/DUR-02 durable XP apply readiness.

Those are not missing semantic claims in this evidence pack.

Final evidence status:

`COMPLETE / ORDINARY_SINGLE_PLAYER_DEATH_CORPSE_LOOT_SEMANTICS_EVIDENCE_CLOSED / NO_DURABILITY_OWNERSHIP_INVENTED`
