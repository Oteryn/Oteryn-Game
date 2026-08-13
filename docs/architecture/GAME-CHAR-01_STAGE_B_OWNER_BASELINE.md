# GAME-CHAR-01 — Stage B Owner Baseline

- Status: **OWNER_ACCEPTED**
- Owner decision date: 2026-08-12
- Owner decision time: 00:17 +02:00
- Repository recording date: 2026-08-12
- Gate: `GAME-CHAR-01` Stage B
- Overall `GAME-CHAR-01` DecisionStatus: **ACCEPTED**
- DeliveryStatus: **OPEN** during this owner-baseline delivery; **LIFECYCLE_CLOSED** after terminal closeout
- ImplementationStatus: **NOT_STARTED**
- Runtime authority: **NONE**
- Reference target: **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**
- Source type: `USER_SOURCE`
- Decision source: `GAME-CHAR-01_STAGE_B_MINIMUM_CLOSURE_DECISION_PACKET.md`
- Does not authorize: runtime/client/protocol/physical persistence/content/Platform implementation, PostgreSQL DDL/migrations, production rollout, unproven Reference formulas/values, first Reference PvP/world-profile selection or any intentional Reference difference not separately accepted

## 1. Owner source and acceptance

### USER_SOURCE — accepted 2026-08-12 00:17 +02:00

After the Stage-B evidence programme and minimum-closure synthesis had been delivered and lifecycle-closed, the owner was asked whether to accept the complete recommended **minimum `GAME-CHAR-01` Stage-B closure package**.

The owner explicitly answered:

> tak

This accepts as one coherent architecture package:

- decisions 1-11 in sections 4-14 of `GAME-CHAR-01_STAGE_B_MINIMUM_CLOSURE_DECISION_PACKET.md`;
- the post-acceptance effect in section 16 of that packet;
- the packet's supersession/reopening discipline and explicit non-meanings.

Stage A in `GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` remains binding. Stage B layers the Reference-sensitive semantic/durability closure on top of Stage A; it does not rewrite the historical Stage-A record.

## 2. What `GAME-CHAR-01 = ACCEPTED` means

Acceptance closes the **Character architecture gate** at the semantic ownership/versioning/migration level needed for safe durability design.

It establishes:

```text
accepted Character architecture
!= complete July-28 behavior knowledge
!= PARITY_CONFIRMED for every character feature
!= physical PostgreSQL schema
!= runtime implementation
!= production enablement
```

After this owner-baseline lifecycle closes, current status is:

```text
GAME-CHAR-01
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
```

During this delivery only, `DeliveryStatus = OPEN` because a concrete acceptance-recording task/PR owns the gate bookkeeping.

## 3. Accepted closure boundary

The owner accepts this responsibility split:

```text
GAME-CHAR-01
owns / freezes
-> Character semantic catalogue
-> Character-specific ownership scope
-> lifecycle/build/progression semantics needed by durability
-> version/revision/fencing/migration envelopes
-> target-policy boundaries
-> explicit UNKNOWN / CONFLICT fail-closed handling
-> what the profile-neutral DUR-02 core must be able to represent

Reference ruleset / SIM / content / world-profile gates
own / freeze later where applicable
-> exact arithmetic
-> unresolved numeric target values
-> exact starter content
-> exact PvP/world-type edge matrices
-> profile-specific durable facts not required by the neutral core
-> deterministic parity fixtures

DUR-02
owns later
-> profile-neutral physical schema/constraints/indexes/transactions/migrations
-> typed/versioned extension and migration boundaries
-> physical decomposition of character-owned child state
```

Exact behavior is not weakened or waived. The owning downstream gate must still prove each exercised Reference rule before a parity claim.

## 4. Accepted decision 1 — naming semantic envelope

The owner accepts:

1. Reference character names occupy one **logical global Character Authority namespace**, not independent per-world namespaces.
2. A Character has a player-visible display name plus an authoritative **canonical comparison key** produced by a versioned naming policy.
3. The canonical comparison key is the semantic uniqueness/conflict value; the exact normalization algorithm is not guessed while target evidence remains incomplete.
4. Current name, searchable former-name alias/history and deleted-name reservation are distinct semantics.
5. Naming-policy revision changes require explicit validation, migration and conflict handling; existing names may not be silently reinterpreted.
6. `CharacterId` remains identity through rename; a name is never the Character identity.

### Still parity-pending

This acceptance does **not** invent or freeze:

- exact Unicode/case/space normalization;
- exact permitted repertoire or 29-letter target continuity;
- exact restricted/reserved-pattern revision;
- deleted-name release algorithm or timing.

`DUR-02` may represent an authoritative comparison key/registry but may not select canonicalization semantics from PostgreSQL collation/database convenience.

## 5. Accepted decision 2 — lifecycle and quota policy boundary

Stage A's semantic lifecycle remains binding:

```text
ACTIVE -> DELETION_SCHEDULED -> RETIRED
```

The owner additionally accepts:

1. account/world eligibility and quota checks are Character Authority policy evaluated atomically with lifecycle operations;
2. the first Reference active-character quota is **25 active characters**, based on the accepted evidence classification;
3. deletion grace, scheduled-deletion total quota, undelete conditions and deleted-name hold/recycling timings are versioned Reference policy values;
4. unresolved target values remain `PARITY_PENDING_EVIDENCE` and may not be inferred from current Global, OTS behavior or implementation convenience;
5. changing those policy values does not require inventing new Character lifecycle states merely to encode a number.

### Still parity-pending

- exact July-28 deletion grace (`60 days` is not accepted as target truth here);
- exact total active + deletion-scheduled quota (`30` is not accepted as target truth here);
- exact undelete interaction rules;
- exact deleted-name hold/release timing.

## 6. Accepted decision 3 — creation uses versioned ruleset/content/starter context

The owner accepts:

1. Character creation remains the atomic/idempotent Character Authority operation from Stage A.
2. Creation binds explicit profile/ruleset/content/starter-template revision context sufficient for deterministic interpretation and migration.
3. Exact starter inventory/equipment, route/quest data and tutorial content are content/ruleset template semantics, not hard-coded Character-constructor defaults.
4. Character build state must represent the target's pre-vocation/unselected period where the active Reference flow requires it; a universal invariant that every newly created character already has a final vocation is forbidden.
5. World/route/quest ownership remains content/world authority even when the resulting state is projected on a character-facing surface.
6. Exact starter state remains a Reference content/parity-fixture gate before that flow can be claimed complete.

## 7. Accepted decision 4 — first Reference vocation and skill semantic catalogue

The first Reference ruleset semantic vocabulary includes:

### Vocation families and promoted forms

- Druid -> Elder Druid;
- Knight -> Elite Knight;
- Monk -> Exalted Monk;
- Paladin -> Royal Paladin;
- Sorcerer -> Master Sorcerer;
- an explicit pre-vocation/unselected build state where the target flow requires it.

### Skill categories

- Fist Fighting;
- Club Fighting;
- Sword Fighting;
- Axe Fighting;
- Distance Fighting;
- Shielding;
- Fishing;
- Magic Level.

Architecture rules:

1. this catalogue belongs to the **Reference ruleset**, not a universal engine enum or protocol fork;
2. durable Character state refers to stable versioned vocation/build/skill definitions so future profiles are not forced to share this exact catalogue;
3. promotion is a versioned transition within the Character build model;
4. a later accepted Reference revision or Evolved profile may alter the catalogue only through explicit ruleset/version/migration policy.

## 8. Accepted decision 5 — progression catalogue versus exact arithmetic

The owner accepts:

1. Character domain owns authoritative persistent progression facts required by the active ruleset.
2. Reference durability must support the evidenced level/experience/skill vocabulary and migration-sensitive attributes such as capacity without assuming every value is a pure current function.
3. Persisted authoritative facts and deterministic derived projections remain explicitly distinguished.
4. Interpretation of progression facts/projections is bound to explicit ruleset/profile revision context.
5. Exact XP->level, skill advancement, HP/mana/capacity/speed and rounding arithmetic belong to ruleset/SIM parity gates **unless evidence proves a formula changes durable identity, ownership, atomicity, irreversible representation or migration semantics**.
6. `DUR-02` may use formula-neutral durable representations and may not encode an unproven formula as a schema invariant.

### Hard implementation/parity gate

No unresolved arithmetic may:

- be implemented as Reference truth;
- be enabled in an exercised Reference milestone as if known;
- be marked `PARITY_CONFIRMED`;

without target evidence and deterministic fixtures from its owning gate.

## 9. Accepted decision 6 — promotion achievement versus entitlement-derived activation

The owner accepts:

1. promotion/profession progression is Character-owned versioned build state;
2. achieved promotion state/history is not erased merely because account entitlement changes;
3. whether promotion benefits are currently active may be derived from Character promotion state + active ruleset + Platform-owned entitlement input;
4. exact promotion fee and exact entitlement-lapse/reactivation target behavior remain versioned parity-policy gates where evidence is incomplete;
5. Platform remains entitlement authority and never becomes steady-state Character mutation authority.

This accepts no monetization model and authorizes no Premium/VIP implementation.

## 10. Accepted decision 7 — death/protection is profile-scoped

The owner accepts:

1. Character owns durable character-specific death/protection consequences and protection/progression facts required by an active profile.
2. Item loss, corpse/loot ownership and value conservation remain combat + `GAME-ITEM-01` / `DUR-03` authority.
3. Death/protection evaluation is bound to explicit Reference ruleset/world-profile policy revision.
4. Regular blessing/protection state must be representable without encoding one universal world-independent death formula.
5. PvP-only persistent Character facts are introduced only when their owning profile requires them, with explicit semantic ownership and fencing.
6. A complete Twist of Fate/fair-fight/skull/Death Redemption edge matrix does **not** block the profile-neutral Character durability core before the first PvP/world profile is selected.
7. Exact PvP/death behavior remains a hard implementation/parity gate for every profile that exercises it.

This baseline does **not** select the first Reference PvP/world type. `GAME-CHANNEL-01` and later world/PvP policy gates remain responsible for those decisions.

## 11. Accepted decision 8 — offline-training semantic counter versus effectiveness

The owner accepts offline training as an explicit ruleset capability rather than universal engine behavior.

For the first Reference target, the accepted semantic counter model is:

```text
activation selected
-> at least 10 minutes offline before training gain
-> maximum 12 hours effective continuous training
-> consume 1 pool second per training second
-> restore 1 pool second per second online
-> restore 1 pool second per second offline without training
-> depletion may refill while offline
-> consuming refilled pool requires a new activation
```

The owner accepts:

1. activation/counter/pool state is character-specific durable progression state where the ruleset enables offline training;
2. the timer/pool semantics above are part of the Reference semantic contract;
3. exact effectiveness coefficients, advancement arithmetic, rounding, temporary/loyalty modifier interactions and version-sensitive selectable-skill details remain ruleset/SIM parity gates where evidence remains incomplete;
4. `DUR-02` may represent the counter/capability without embedding advancement formulas.

## 12. Accepted decision 9 — modern character-specific progression scope

The Character **domain ownership scope** includes target-required character-specific progression/capability facts evidenced by the Stage-B programme, including:

- per-weapon Weapon Proficiency Progress and player-owned proficiency state where character-specific;
- charms, charm points and charm expansion state;
- Hunting Task Points;
- permanent Hunting Task slots;
- permanent Prey slots;
- Wheel/Promotion Point character-build state;
- Animus Mastery as character-specific progression within the accepted evidence boundary.

The owner accepts these architecture rules:

1. `character-specific` means game-domain ownership scope, **not** one giant aggregate/table/transaction.
2. `DUR-02` may use explicit character-owned child aggregates when size, lifecycle, contention, transaction or migration evidence justifies them.
3. Ruleset/content definitions own trees, thresholds, perk/mastery definitions, formulas and compatibility rules.
4. `GAME-ITEM-01` / `DUR-03` owns item/resource conservation and value transfer semantics.
5. Platform owns commercial entitlement source.
6. Character-specific progress references stable versioned definition identities and requires explicit migration when definitions become incompatible.
7. A resource being character-bound does not automatically transfer its economy/conservation authority into the core Character aggregate.

Physical child-aggregate/table decomposition remains `DUR-02` work.

## 13. Accepted decision 10 — unresolved Reference rules are explicit hard parity gates

The following rule is binding:

```text
UNKNOWN / CONFLICT target rule
-> may have a safe versioned semantic/policy envelope
-> may NOT be filled by current Global, Canary, crystalserver, another OTS or implementation convenience
-> may NOT be enabled as claimed Reference behavior
-> may NOT be PARITY_CONFIRMED
-> must be evidenced, explicitly owner-accepted as DECLARED_DIFFERENCE, or excluded from the exercised release scope
```

For an external Reference milestone, every **exercised** Character behavior must be either:

- `PARITY_CONFIRMED`; or
- an explicit owner-accepted `DECLARED_DIFFERENCE`.

Known `UNKNOWN`/`CONFLICT` cannot silently pass as Reference fidelity.

## 14. Accepted decision 11 — GAME-CHAR scope refinement does not relax parity

The owner accepts this gate refinement:

`GAME-CHAR-01` architecture acceptance requires:

- complete semantic ownership boundaries for its declared scope;
- lifecycle/build/progression catalogue sufficient for safe durability;
- version/revision/migration/fencing rules;
- safe policy envelopes for target-specific values;
- explicit hard gates for unresolved Reference values/formulas/profile-specific facts.

It does **not** require exact numeric arithmetic or content owned by later ruleset/SIM/content/world-profile gates **unless that detail constrains durable identity, ownership, transaction atomicity, irreversible representation or migration**.

This changes the placement of proof obligations, not the strictness of Reference parity.

## 15. Profile-neutral `DUR-02` consequence

After this owner-baseline lifecycle is closed, `DUR-02` may proceed to final **paper-only profile-neutral core Character schema architecture** using accepted GAME-CHAR semantics.

It must define explicit typed/versioned extension and migration boundaries for later accepted profile-specific Character state.

This does **not** mean one Character schema package is complete for every future world/PvP profile. If a selected profile requires additional durable Character facts, schema completeness for that profile remains blocked until those facts, ownership and policy are accepted.

The profile-neutral core may not:

- implement PostgreSQL DDL/migrations without separate implementation authority;
- hard-code unresolved Reference formulas/values;
- choose name normalization through database convenience;
- collapse independent progression systems into one aggregate merely for convenience;
- encode one universal PvP/death profile;
- use an untyped/generic key-value or JSON "misc state" bag to avoid semantic ownership decisions;
- treat current Global or OTS behavior as July-28 target truth.

Later profile-specific schema extensions/migrations must preserve Character revision/fencing, explicit semantic ownership, compatible migration/rollback policy and the accepted evidence boundary.

## 16. Still unresolved and deliberately downstream

Acceptance does not resolve the following target details.

### Naming/lifecycle

- exact canonicalization/repertoire/29-letter target continuity;
- deleted-name release algorithm;
- exact 60-day deletion-grace continuity;
- exact total-30 quota continuity;
- related undelete/name-hold details.

### Creation/content

- exact starter inventory/equipment/stats/home/route content;
- remaining target-specific creation validation details.

### Arithmetic

- exact XP/level and skill curves;
- exact derived-stat formulas/rounding where unproven.

### Promotion/entitlement

- exact 20,000 gp target continuity;
- exact Premium-lapse benefit semantics.

### Death/PvP

- first Reference PvP/world type;
- complete Twist/fair-fight/skull edge matrix;
- exact target Death Redemption history/window semantics;
- remaining exact death arithmetic.

### Offline training

- effectiveness coefficients and detailed advancement arithmetic.

### Modern progression

- exact formulas/content definitions;
- physical child-aggregate/table placement;
- incompatible-definition migration details not yet established by their owning gates.

These remain hard-gated where exercised; acceptance does not create defaults for them.

## 17. Supersession and reopening

A later decision may supersede a Stage-B rule only with named evidence such as:

- stronger July-28 target-period primary evidence contradicting an accepted `DERIVED` rule;
- an explicit later accepted Reference revision;
- evidence that a deferred formula/value actually changes durable identity, ownership, atomicity, irreversible representation or migration semantics;
- legal, privacy, security or integrity constraints;
- a separately accepted owner product/ruleset strategy change.

Implementation convenience, current Global behavior or an OTS implementation is never sufficient supersession evidence.

## 18. Relationship to open factual erratum PR #191

At the trusted-base start of this delivery, PR #191 remained open and disjoint. It corrects one provenance year in `GAME-CHAR-01_STAGE_B_REFERENCE_EVIDENCE_DELTA_02.md` from `2001` to `2002` while explicitly preserving the substantive classification: promotion level >=20 + Premium remains `DERIVED / very strong primary continuity` and the unresolved promotion fee/lapse edges remain `UNKNOWN`.

This owner acceptance does not depend on the incorrect year and does not take ownership of #191. If #191 merges, it is a historical evidence-provenance correction and does not supersede this baseline.

## 19. No runtime or production authority

This architecture acceptance does **not** authorize:

- Character runtime implementation;
- Reference progression/death/offline-training implementation;
- PostgreSQL physical schema, migration or data mutation;
- gameplay protocol changes;
- client implementation;
- content/ruleset implementation;
- Platform writes or entitlement activation;
- production deployment, traffic or live-data changes;
- autonomous parity assumptions for unresolved mechanics.

A later implementation package must consume this baseline plus its owning ruleset/SIM/content/profile/persistence contracts and provide named tests/E2E/evidence appropriate to the behavior it claims.

## 20. Canonical status after lifecycle closeout

After the task delivering this baseline is merged, archived and ownership is released:

```text
GAME-CHAR-01
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
Reference parity     = per-behavior evidence-gated; no aggregate parity status is implied by architecture acceptance
```

The next allowed Character-durability step is **paper-only profile-neutral `DUR-02` schema architecture**, while `GAME-CHANNEL-01`, `GAME-ITEM-01`, ruleset/SIM/content evidence and profile-specific decisions retain their separate ownership gates.
