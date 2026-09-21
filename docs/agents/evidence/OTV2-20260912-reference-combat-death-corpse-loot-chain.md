# Oteryn Reference Combat — ordinary death/corpse/loot chain evidence

Status: **READ-ONLY EVIDENCE / NON-AUTHORITY**
Date: 2026-09-12; reconciled 2026-09-16
Refs: #483, #506, #513
Downstream gate: #514 (no manifest mutation authority from this document)
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
- PvP/player-death/skulls/fair-fight/item-loss breadth;
- bosses/raids/events/multi-principal rewards;
- broad Quick Loot Nearby Corpses behavior;
- full corpse decay/despawn timing;
- exact natural loot probabilities unless separately target-proven;
- proprietary/internal CipSoft persistence, database, item identity or transaction implementation;
- Evolved product mechanics;
- parity-manifest/schema/registry mutation.

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

- `PROVEN` — directly established by qualifying primary evidence for the stated field, or an accepted Oteryn-native owner decision when explicitly labelled as such;
- `DERIVED` — conservative inference from proven/official facts;
- `UNKNOWN` — not sufficiently proven and must remain open;
- `CONFLICT` — a candidate interpretation contradicted by stronger evidence.

Oteryn-native accepted differences are never rewritten as Global truth.

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

### 3.1 Accepted Oteryn Reference player-death declared difference

Protected `docs/architecture/OTERYN_REFERENCE_DEATH_XP_SPAN_OWNER_BASELINE_2026-09-09.md` is `OWNER_ACCEPTED REFERENCE DIFFERENCE` / `ACCEPTED`.

It governs **Character/player-death progression loss**, not creature-kill XP. The accepted Oteryn Reference differences are exactly:

```text
ReferenceDeathXPBasis = LevelXPSpan(current_level)
ReferenceDeathXPBasis != AccumulatedTotalXP
ReferenceDeathXPBasis != XPRemainingToNextLevel
DeathSkillLoss = 0
DeathMagicLevelLoss = 0
```

Everything else in the player-death model remains governed by the selected Global target unless separately declared otherwise. Exact unresolved Global ordering/rounding remains `UNKNOWN` and must not be guessed.

This document therefore keeps these facts separate:

```text
creature death -> creature XP reward consequence
!=
Character death -> Reference death-progression penalty
```

It is a `CONFLICT` to treat Oteryn's zero skill/magic death loss as Global Tibia truth, and it is a `CONFLICT` to apply the player-death `LevelXPSpan` penalty basis to creature-kill reward XP.

## 4. Protected first-creature fixture and official source index

Protected `docs/architecture/reviews/OTERYN_REFERENCE_FIRST_CREATURE_FIXTURE_RAT_2026-09-12.md` selects `Rat` as the recommended first creature-content fixture for the minimal Reference `kill -> XP -> corpse -> loot` path.

That fixture supersedes Skeleton only as the recommended first research fixture. It does not mutate the parity manifest and does not claim `PARITY_CONFIRMED`.

Protected Rat fields used by this evidence pack remain:

```yaml
fixture_id: reference.first_creature.rat
creature: Rat
hp: 20                    # DERIVED HIGH
base_xp: 5                # DERIVED HIGH
corpse_family: Dead Rat   # DERIVED HIGH
ordinary_loot_candidate_types:
  - Gold Coin             # DERIVED HIGH
  - Cheese                # DERIVED HIGH
loot_probabilities: UNKNOWN
exact_flee_threshold: UNKNOWN
numeric_corpse_item_id: UNKNOWN / OUT_OF_SCOPE
```

The Rat tuple is `DERIVED HIGH`, not `PROVEN`, because no directly retrievable target-era CipSoft Rat record or controlled exact-target observation proves the full tuple.

Primary source anchors used by the bounded findings:

- CipSoft 2006 gameplay update — first-10-second highest-damage loot protection and corpse immovability;
- CipSoft 2010 Spring Patch — stamina reward behavior and no monster loot for a highest-damage character at/below the final 14 stamina hours;
- current official Tibia stamina support/manual — 42h total stamina, Premium +50% XP in the first three hours down to hour 39, and half XP in the final 14 hours with no loot at/below 14h for the highest-damage character;
- CipSoft 2023 Loot Highlight — first-10-second authorized-looter continuity;
- CipSoft 2023 programmer explanation — simple/static loot may exist before death while dynamic loot may be resolved at death;
- CipSoft 2024 Quick Loot Nearby Corpses — breadth extension of existing quick-loot behavior;
- CipSoft 2026-07-28 Balancing, Fixes and Changes — direct target-boundary XP rows, including Iceplume Strider `7,500 -> 8,150`;
- current official Controls/Quickstart — ordinary corpse opening/looting, first-10-second highest-damage rule, corpse immovability and inventory/capacity behavior;
- official shared-experience/combat support — damage-based distribution when shared XP is not active;
- target-near structured Rat evidence retained by the protected Rat fixture, with `DERIVED HIGH` classification only.

The repository #483 evidence continuation additionally records target-era corpse/lootability evidence shortly before the target boundary. That supports continuity of the `death -> corpse -> lootable object` model but does not prove every corpse rule or exact timer quantisation.

## 5. Atomic evidence matrix

| ID | Classification | Atomic assertion | Continuity to target | Owner |
|---|---|---|---|---|
| `DEATH-01` | `DERIVED` strong | ordinary creature death produces a corpse/body/container state that can participate in loot interaction | longstanding official corpse/container semantics plus target-era support; special-creature exceptions are not generalized | #506 |
| `ATTR-01` | `DERIVED` scoped | first fixture may explicitly construct one player as sole damage contributor and source of the committed lethal occurrence | fixture precondition, not a universal Global attribution algorithm | #506 |
| `ATTR-02` | `DERIVED` strong | when shared XP is not active, XP distribution follows damage contribution | long official continuity | #506 |
| `ATTR-03` | `DERIVED` | one sole contributor with 100% contribution has a 100% damage-derived share before separately modelled modifiers | arithmetic consequence of `ATTR-02` | #506 |
| `ATTR-04` | `DERIVED` strong | protected corpse/loot principal is highest-total-damage character, not merely last hitter | official historical + current continuity | #506 |
| `ATTR-05` | `CONFLICT` | `last_hit == loot_owner == xp_owner` as a universal model | contradicted: XP contribution and protected loot authority use different predicates | #506 |
| `RAT-01` | `DERIVED` HIGH | Rat is the selected first creature fixture; base HP `20`, base XP `5` | protected target-near Rat fixture; not exact-target primary proof | #483 input -> #506 |
| `RAT-02` | `DERIVED` HIGH | Rat corpse family is `Dead Rat`; ordinary loot candidate types are `Gold Coin` and `Cheese` | protected Rat fixture | #483 input -> #506 |
| `RAT-03` | `UNKNOWN` | exact natural Rat loot probabilities/RNG distribution | intentionally unresolved | #483/#504 input |
| `XP-01` | `PROVEN` target-boundary | Iceplume Strider base creature XP is `8,150` after the 2026-07-28 server-save change (`7,500 -> 8,150`) | direct official target-boundary source; retained as a control anchor, not the selected first creature | #483 input -> #506 |
| `XP-02` | `DERIVED` | base creature XP is not automatically final Character XP delta in every context | stamina/party/events/boosts remain separate layers | #506 / GAME-CHAR |
| `XP-03` | `DERIVED` strong | final 14 stamina hours reduce XP and highest-damage character at/below that threshold can cause monster loot to be absent/destroyed | official historical + current continuity | #506 |
| `XP-04` | `DERIVED` strong fixture precondition | `20h` remaining stamina is an interior neutral stamina point: above final-14h penalty/no-loot zone and below the Premium first-three-hours/green-stamina bonus zone | selected to avoid both known modifier bands without asserting boundary quantisation | #506 fixture only |
| `CORPSE-01` | `DERIVED` strong | first 10 seconds after ordinary creature kill use highest-damage loot authorization | official 2006 -> 2023 -> current continuity | #506 |
| `CORPSE-02` | `DERIVED` strong | corpse movement is blocked during same protected first-10-second interval | official continuity | #506 |
| `CORPSE-03` | `UNKNOWN` | exact `t = 10.000s` server-tick/rounding/clock quantisation | no qualifying source found; do not invent | #506 |
| `LOOT-01` | `DERIVED` strong | `who may loot`, `whether corpse may move`, and `whether loot exists` are distinct facts | highest-damage protection and stamina-driven no-loot are separate rules | #506 |
| `LOOT-02` | `DERIVED` strong | ordinary interaction supports opening corpse/container, inspecting possessions and choosing an item to loot | official Controls + longstanding model | #506 -> #513 |
| `LOOT-03` | `DERIVED` strong | successful ordinary pickup observably moves chosen item from corpse/container to player inventory/container, subject to legality/space/capacity | official continuity | GAME-ITEM -> #513 |
| `LOOT-04` | `PROVEN` pre-target internal explanation | not all Global loot is necessarily selected at death; simple/static loot may be generated earlier while dynamic loot may be resolved at death | official CipSoft programmer explanation; exact target-day internals are not promoted | #506 caution |
| `LOOT-05` | `CONFLICT` | `all Global loot RNG occurs in the death handler` | contradicted by `LOOT-04` | #506 |
| `DUR-01` | `UNKNOWN` from Global | Global public behavior does not prove `ItemInstanceId`, DB commit point, `TransactionId`, durable custody storage or retry policy | intentionally outside observable parity | #513 |
| `DUR-02` | `PROVEN` Oteryn-native | first VSL materialises acknowledged loot before interactable pickup and transfers same durable ItemInstance on pickup | binding #513/VSL-COMBAT decision | #513 |
| `DUR-03` | `PROVEN` Oteryn-native | pickup cannot create second mint or leave simultaneous source+destination truth | binding DUR-03 anti-duplication boundary | #513 |
| `RDEATH-01` | `PROVEN` Oteryn-native `DECLARED_DIFFERENCE` | Character-death XP-loss basis is full `LevelXPSpan(current_level)` | protected accepted owner baseline | GAME-CHAR boundary |
| `RDEATH-02` | `PROVEN` Oteryn-native `DECLARED_DIFFERENCE` | Character death causes zero skill loss and zero magic-level loss | protected accepted owner baseline | GAME-CHAR boundary |
| `RDEATH-03` | `CONFLICT` | zero skill/magic death loss is Global truth or creature-kill reward semantics | explicitly outside accepted difference scope | GAME-CHAR boundary |

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

## 7. XP consequence and neutral fixture preconditions

### 7.1 Selected first creature fixture — Rat

The protected first-creature fixture is Rat:

```text
Given:
  target creature identity = Rat
  creature fixture HP candidate = 20               [DERIVED HIGH]
  creature base XP candidate = 5                   [DERIVED HIGH]
  one solo player
  player contribution = 100%
  shared XP not exercised
  stamina remaining = 20h                          [neutral interior fixture point]
  Premium green/happy-hour stamina bonus inactive
  final-14h stamina penalty inactive
  XP boost inactive
  XP event/double-XP modifier inactive
  prey/other explicit XP modifier inactive
  boosted-creature state = false
  ordinary spawn = true
  raid/event context = false
  weapon-proficiency effects affecting case = none
  charms affecting case = none
When:
  one legal final damage occurrence commits
Then:
  alive -> dead occurs once
  base creature-XP candidate = 5                   [DERIVED HIGH, not PARITY_CONFIRMED]
  corpse/loot-authority transition begins
```

`20h` is deliberately used instead of merely `>14h`: it sits safely above the final-14h half-XP/no-loot zone and below the first-three-hours Premium +50% XP band. This avoids relying on exact `14h` or `39h` boundary quantisation.

The no-boost/no-event/no-prey/no-party settings are **fixture preconditions**, not claims that those systems do not exist.

The first-fixture assertion is therefore not:

```text
Character.experience += 5 unconditionally
```

It is:

```text
Rat base creature XP candidate = 5 [DERIVED HIGH]
with separately modelled reward modifiers neutralised by fixture preconditions
```

### 7.2 Retained exact target-boundary XP control anchor

Iceplume Strider remains useful as a separate exact-boundary control:

```text
base_creature_xp_consequence = 8150
classification = PROVEN target-boundary
source = direct 2026-07-28 official change 7500 -> 8150
```

It is **not** the selected first creature fixture after protected Rat evidence. Retaining it does not upgrade Rat's `5 XP` from `DERIVED HIGH` to `PROVEN`.

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

### `CONFLICT-08` — Iceplume remains the selected first creature fixture

Reject after protected Rat fixture integration. Iceplume `8150` remains a `PROVEN target-boundary` XP anchor only; the selected first creature fixture is Rat with `DERIVED HIGH` static fields.

### `CONFLICT-09` — `stamina > 14h` alone proves neutral XP

Reject. Premium +50% stamina XP applies in the first three hours down to hour 39. The fixture uses `20h` to sit inside the normal interior band instead of merely saying `>14h`.

### `CONFLICT-10` — Oteryn zero skill/magic death loss is Global truth

Reject. It is an accepted Oteryn Reference `DECLARED_DIFFERENCE` for Character/player death only.

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

### `UNKNOWN-06` — exact natural Rat loot probabilities/RNG distribution

Protected Rat evidence leaves this `UNKNOWN BY REQUIREMENT`. This can block a fully natural target-loot probability fixture but does not block death/corpse/authorization semantics or synthetic resource-limit fixtures.

### `UNKNOWN-07` — proprietary Global durable transaction implementation

Intentionally unnecessary for observable parity; #513 provides Oteryn-native correctness.

### `UNKNOWN-08` — exact player-death XP transform ordering/rounding where target evidence is incomplete

The accepted Oteryn declared difference selects the XP **basis** and zero skill/magic loss. It does not manufacture unresolved Global modifier ordering or rounding.

### `UNKNOWN-09` — Rat exact flee threshold and numeric corpse item ID

Preserved from the protected first-creature fixture: flee threshold is `UNKNOWN`; numeric `Dead Rat` item ID is `UNKNOWN / OUT_OF_SCOPE` until the correct target mapping is evidenced.

## 14. Questions owned by #506

#506 must own or consume evidence for:

- exactly-one death occurrence per creature generation/lethal occurrence;
- death identity/replay relationship;
- runtime corpse projection;
- highest-damage authorization fact;
- semantic protected deadline;
- corpse immovability during protected interval;
- stamina-driven zero-loot case;
- bounded loot candidates, outputs and RNG work;
- exact loot output intent;
- separate XP consequence;
- only any Combat-retained opaque descendant references actually required by implementation.

#506 must not:

- implement a second item transaction engine;
- become durable ItemInstance/value authority;
- write Character XP directly;
- invent DUR-03 retry/audit maxima;
- reinterpret accepted Character player-death declared differences as creature-kill reward rules.

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
- 10-second Global access rule;
- stamina loot suppression;
- creature loot probability/RNG;
- Character player-death progression semantics.

## 16. Minimal positive Reference fixture

```text
Given:
  selected first creature = Rat
  Rat HP candidate = 20 [DERIVED HIGH]
  Rat base XP candidate = 5 [DERIVED HIGH]
  Rat corpse family = Dead Rat [DERIVED HIGH]
  Rat ordinary loot candidate types subset of {Gold Coin, Cheese} [DERIVED HIGH]
  one admitted player
  player is sole damage contributor
  player contribution = 100%
  shared XP is not exercised
  stamina remaining = 20h
  Premium happy-hour XP bonus inactive
  final-14h XP penalty/no-loot state inactive
  XP boost/event/prey/other explicit XP modifiers inactive
  boosted creature = false
  raid/event context = false
  weapon-proficiency effects affecting case = none
  charms affecting case = none
  creature already has a legal lethal remaining-HP state

When:
  one legal committed player damage occurrence reaches zero HP

Then:
  exactly one CreatureDeathOccurrence exists
  exactly one runtime corpse projection exists

XP descendant:
  one base creature-XP consequence exists
  Rat value = 5 remains DERIVED HIGH, not PARITY_CONFIRMED

Loot descendant:
  bounded loot selection produces zero-or-more exact output intents
  ordinary output item types, if any, remain within {Gold Coin, Cheese}
  exact natural probabilities remain UNKNOWN

Corpse authorization:
  player is highest-damage principal
  during protected 10-second interval:
    player is authorised to loot
    corpse movement is denied

Ordinary interaction:
  player opens corpse/container
  player inspects contents
  player chooses one item when an eligible output exists

Durability boundary:
  exact selected output reaches acknowledged durable corpse/container custody
  loot state becomes LOOT_READY

Pickup:
  GAME-ITEM destination legality passes
  DUR-03 transfers same durable ItemInstance
  corpse custody -> Character inventory custody

Invariants:
  no second death
  no duplicate XP
  no duplicate mint
  no source+destination double truth
  authorization expiry cannot fabricate durable value
```

A separate exact-target control may use Iceplume Strider `8150` only for the directly proven XP boundary field; it is not the first-creature fixture.

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

N4 Rat natural loot probability assertion without target proof
   -> reject / remain UNKNOWN
```

### Oteryn correctness cases

```text
N5 authorised principal + LOOT_SETTLEMENT_PENDING
   -> no durable movement

N6 destination capacity/legality failure
   -> no successful pickup / no partial durable mutation

N7 duplicate or concurrent pickup
   -> at most one authoritative destination custody

N8 committed pickup + lost response/retry
   -> reconcile same logical committed result
   -> do not restore source custody

N9 committed materialisation + stale/recreated corpse projection
   -> no second mint

N10 duplicate lethal replay
    -> no second death occurrence
    -> no duplicate XP
    -> no duplicate loot workflow

N11 Character/player-death fixture uses accumulated TotalXP as penalty basis
    -> reject under accepted Reference DECLARED_DIFFERENCE

N12 Character/player death mutates skill or magic-level progress
    -> reject under accepted Reference DECLARED_DIFFERENCE
```

## 18. Target-continuity disposition

### `PROVEN` exact target-boundary field

- Iceplume Strider base XP `8,150` after the 2026-07-28 server save — retained as an exact-boundary control, not the first-creature fixture.

### `PROVEN` Oteryn-native declared difference

- Character/player-death XP-loss basis is `LevelXPSpan(current_level)`;
- Character/player death causes `DeathSkillLoss = 0` and `DeathMagicLevelLoss = 0`.

These are Oteryn Reference declared differences, not Global facts.

### `DERIVED HIGH` protected first-creature fixture

- Rat HP `20`;
- Rat base XP `5`;
- Rat relevant damage profile/basic melee family;
- `Dead Rat` corpse family;
- ordinary candidate item types `Gold Coin`, `Cheese`;
- continuity to the exact 2026-07-28 target remains `DERIVED HIGH`, not `PROVEN`.

### `DERIVED` strong continuity

- ordinary corpse/container loot interaction;
- highest-damage first-10-second authorization;
- corpse immovability during that interval;
- non-shared damage-based XP distribution;
- stamina reward/no-loot bands used to choose an interior neutral fixture point;
- selected-item corpse-to-inventory observable pickup.

These have strong official historical/current continuity but are not promoted to exact-target `PROVEN` merely because no intervening change was found.

### `UNKNOWN`

- exact 10-second tick/rounding;
- ties/summons/multi-attacker breadth;
- Rat exact natural loot probabilities/RNG;
- Rat exact flee threshold;
- numeric `Dead Rat` item ID;
- proprietary Global durability implementation;
- unresolved Global Character-death transform ordering/rounding outside the accepted Oteryn difference.

## 19. Manifest-v4 boundary

#514 remains a preparation gate only. Its live contract says:

```text
manifest_mutation_authority = NONE_UNTIL_EXACT_ALLOCATION
allocation_authority = 162_ONLY
```

This document does not mutate or authorize:

- `REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`;
- its schema;
- manifest revision 4;
- runtime/content/parity fixtures.

Rat and the accepted Reference death difference are evidence inputs that a later exactly allocated #514 worker may classify/register. They are not promoted by #576.

## 20. Completion result

For the requested ordinary single-player scope, the observable Global semantics needed to shape the first #506/#513 Reference fixture are evidence-closed without inventing durability ownership.

The protected first-creature fixture is now reconciled as Rat; the accepted Oteryn Reference player-death declared difference is explicitly fenced from creature-kill reward semantics; and the neutral XP fixture uses an interior `20h` stamina point rather than the insufficient `>14h` condition.

Remaining open items are deliberately separate:

1. controlled target observation if exact 10-second boundary quantisation is ever required;
2. #483/#504 target evidence for exact Rat natural loot probabilities and remaining static unknowns;
3. #506 Combat resource-limit acceptance and physical runtime carrier work;
4. #513 DUR-03 resource-limit acceptance and implementation allocation;
5. Character/DUR-02 durable XP apply readiness;
6. #514 manifest-v4 allocation only if #162 grants exact mutation authority.

Those are not silently converted to PASS by this evidence pack.

Final evidence status:

`COMPLETE / ORDINARY_SINGLE_PLAYER_DEATH_CORPSE_LOOT_SEMANTICS_EVIDENCE_CLOSED / RAT_FIRST_FIXTURE_DERIVED_HIGH / REFERENCE_DEATH_DIFFERENCE_FENCED / NO_DURABILITY_OWNERSHIP_INVENTED / NO_MANIFEST_AUTHORITY`
