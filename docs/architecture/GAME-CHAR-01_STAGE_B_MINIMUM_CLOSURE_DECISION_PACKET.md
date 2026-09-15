# GAME-CHAR-01 — Stage B Minimum Closure Decision Packet

- Status: **PRE-DECISION SYNTHESIS / NOT ACCEPTED**
- Date: 2026-08-11
- Gate: `GAME-CHAR-01` Stage B
- Reference target: **Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary**
- Trusted repository base: `blakinio/Oteryn-v2@4dce1e4da5c7c9e442abe99975aac3e7913b46b4`
- Evidence inputs: Stage A owner baseline + #183 general Stage-B reconciliation + #185 B1-B3 acquisition + #187 B4-B8 delta
- Decision owner: product owner
- Runtime authority: **NONE**
- Does not authorize: runtime/client/protocol/physical persistence/content/Platform implementation, production rollout, unproven Reference formulas/values, first Reference PvP world-type selection or intentional Reference differences not separately accepted

## 1. Executive result

### Complete Reference character parity proven?

**NO.** Material target-visible gaps remain, including exact naming normalization/recycling, deletion-policy numbers, exact starter content, XP/skill arithmetic, promotion fee/Premium-lapse edges, full PvP/death edges, offline-training effectiveness and some definition-migration details.

None may be guessed or marked `PARITY_CONFIRMED`.

### Durable semantic Character architecture closable?

**RECOMMENDATION: YES — if the owner accepts this packet.**

The accumulated evidence is sufficient to freeze:

- durable Character ownership scope;
- semantic lifecycle/build/progression catalogue;
- target vocation/skill vocabulary needed by durability;
- version/revision/migration envelopes;
- policy boundaries for unresolved target values;
- which unknowns belong to later ruleset/SIM/content/world-profile parity gates instead of database topology.

This refines **where** exact parity is proven. It does not weaken **whether** exact parity must be proven.

## 2. Decision-discipline test

### Must semantic ownership be decided now?

**YES.** Final character-bearing `DUR-02` architecture needs to know what Character durability must represent and which authorities remain outside Character.

### Must every exact value/formula be decided now?

**RECOMMENDATION: NO**, unless the value/formula constrains durable identity, authority, transaction atomicity, irreversible representation or migration semantics.

Unresolved values remain mandatory gates before the corresponding Reference behavior is implemented, enabled, externally exercised or claimed as parity.

### Cost of keeping all numbers inside the schema gate

- persistence is pressured to encode gameplay formulas;
- current Global/OTS values can leak into the July-28 target by convenience;
- SIM/content/PvP research blocks unrelated ownership design;
- future formula corrections create schema churn;
- Stage B remains monolithic despite enough evidence for safe semantic closure.

### Risk of deferring too much

- missing durable state becomes expensive to add;
- name uniqueness can be frozen under the wrong comparison model;
- character-specific progression can leak to account/item/content authority;
- profile-specific death state can be hard-coded universally;
- UNKNOWN can become an accidental default.

The recommended split freezes semantic envelopes and keeps unresolved parity fail-closed.

## 3. Recommended closure boundary

```text
GAME-CHAR Stage B
freezes
-> Character semantic catalogue
-> Character-specific ownership scope
-> version/revision/migration envelopes
-> policy boundaries
-> explicit UNKNOWN/CONFLICT handling
-> what the profile-neutral DUR-02 core must be able to represent

Reference ruleset / SIM / content / world-profile gates
freeze later where applicable
-> exact arithmetic
-> unresolved numeric target values
-> exact starter content
-> exact PvP/world-type edge matrix
-> profile-specific durable facts not required by the neutral core
-> deterministic parity fixtures

DUR-02 core
owns later
-> profile-neutral physical schema/constraints/indexes/transactions/migrations
-> typed, versioned extension/migration boundaries for later accepted profile-specific durable facts
-> only after accepted Character semantics
```

Architecture acceptance remains distinct from `PARITY_CONFIRMED` implementation evidence.

The extension boundary is not permission for an untyped key/value or JSON "misc state" bag. Any later profile-specific durable state still requires an explicit semantic owner, stable typed/versioned representation, migration rules and the evidence/acceptance required by its owning gate.

## 4. Decision 1 — naming semantic envelope

### Evidence

#185 provides strong continuity evidence for a Tibia-wide/global name namespace and separate rename/history semantics. Exact July-28 canonicalization/recycling remains incomplete.

### RECOMMENDATION

Freeze:

1. Reference names occupy one **logical global Character Authority namespace**, not independent per-world namespaces.
2. Character has a player-visible display name plus an authoritative **canonical comparison key** produced by a versioned naming policy.
3. The comparison key is a semantic uniqueness value; exact normalization is not invented while target evidence is incomplete.
4. Current name, former-name alias/history and deleted-name reservation are distinct semantics.
5. Naming-policy revision changes require explicit validation/migration/conflict handling; no silent reinterpretation.
6. CharacterId remains identity through rename; a name is not identity.

### Parity-pending

- exact Unicode/case/space normalization;
- exact allowed repertoire / 29-letter target continuity;
- exact restricted-pattern revision;
- deleted-name release algorithm/timing.

### DUR-02 boundary

DUR-02 may represent an authoritative comparison key/registry but must not choose normalization via database collation convenience.

## 5. Decision 2 — lifecycle/quota values are versioned policy

Stage A already accepts:

```text
ACTIVE -> DELETION_SCHEDULED -> RETIRED
```

#185 strongly supports an active-character quota of `25`; exact July-28 continuity for grace `60 days`, total `30`, undelete interactions and name hold remains incomplete.

### RECOMMENDATION

Freeze:

1. Stage-A lifecycle unchanged.
2. Account/world eligibility and quota checks are Character Authority policy evaluated atomically with lifecycle operations.
3. **25 active characters** as the Reference active-character quota based on strong primary continuity evidence.
4. Deletion grace, scheduled-deletion total quota, undelete rules and name-hold/recycling timing are versioned Reference policy values.
5. Unknown target values remain `PARITY_PENDING_EVIDENCE`, never inferred from current Global.

Unknown policy values do not require extra lifecycle states or database topology.

## 6. Decision 3 — creation uses versioned ruleset/content/starter context

### Evidence

#185 establishes Newhaven before target, strong tutorial/Newhaven/vocation-selection continuity and Targuna as the post-Newhaven path before July 28. Exact starter stats/items/home state remain unresolved.

### RECOMMENDATION

Freeze:

1. Creation remains the atomic/idempotent Character Authority operation from Stage A.
2. Creation records explicit profile/ruleset/content/starter-template revision context sufficient for deterministic interpretation/migration.
3. Exact starter inventory/equipment, route/quest data and tutorial content are content/ruleset template semantics, not Character-constructor defaults.
4. Character build state must represent the target's pre-vocation selection period; a universal invariant that every newly created character already has a final vocation is forbidden.
5. World/route/quest ownership remains content/world authority even when projected on a character surface.
6. Exact starter package remains a Reference content/parity-fixture gate.

## 7. Decision 4 — freeze Reference vocation and skill vocabulary, not engine enums

### Evidence

#183 target-day Character Bazaar evidence strongly aligns with five vocation families and promoted forms around the selected cut and exposes eight skill categories.

### RECOMMENDATION

Freeze the first Reference semantic vocabulary:

#### Vocation families / promoted forms

- Druid -> Elder Druid;
- Knight -> Elite Knight;
- Monk -> Exalted Monk;
- Paladin -> Royal Paladin;
- Sorcerer -> Master Sorcerer;
- plus an explicit pre-vocation/unselected build state where target flow requires it.

#### Skill categories

- Fist Fighting;
- Club Fighting;
- Sword Fighting;
- Axe Fighting;
- Distance Fighting;
- Shielding;
- Fishing;
- Magic Level.

Architecture rules:

1. These are Reference ruleset definitions/state, not universal engine/protocol forks.
2. Durable Character state refers to versioned vocation/build/skill definitions rather than assuming all future profiles share this exact catalogue.
3. Promotion is a versioned transition within the character build model.
4. A later Reference revision or Evolved profile may change the catalogue only through explicit versioned ruleset/migration policy.

## 8. Decision 5 — progression catalogue versus arithmetic

### Evidence

Target-era evidence supports level, experience, HP, mana, capacity, speed and the skill vocabulary. Historical capacity migration proves not every visible stat is safely assumed to be a pure current-formula projection.

### RECOMMENDATION

Freeze:

1. Character domain owns authoritative persistent progression facts required by the active ruleset.
2. Reference durability supports evidenced level/experience/skill vocabulary and migration-sensitive attributes such as capacity without assuming universal pure functions.
3. Persisted facts and deterministic derived projections remain explicitly distinguished under Stage A.
4. Interpretation always carries explicit ruleset/profile revision context.
5. Exact XP->level, skill advancement, HP/mana/capacity/speed and rounding arithmetic are **ruleset/SIM parity gates** unless a formula is proven to constrain identity/ownership/atomicity/irreversible representation.
6. DUR-02 may use formula-neutral representations and must not encode unproven arithmetic as schema invariants.

### Hard implementation gate

No unresolved arithmetic may be implemented as Reference truth or marked `PARITY_CONFIRMED` without target evidence and deterministic fixtures.

## 9. Decision 6 — promotion achievement versus entitlement-derived activation

### Evidence

#187 provides very strong continuity evidence for promotion at level >=20 with Premium eligibility. Exact July-28 fee/Premium-lapse edges remain incomplete.

### RECOMMENDATION

Freeze:

1. Promotion/profession progression is Character-owned versioned build state.
2. Achieved promotion state/history is not erased merely because account entitlement changes.
3. Active benefits/eligibility may derive from Character promotion state + active ruleset + Platform-owned entitlement input.
4. Exact fee and lapse/reactivation semantics remain versioned parity-policy gates.
5. Platform stays entitlement authority and does not become steady-state Character mutation authority.

No Premium/commerce implementation is authorized.

## 10. Decision 7 — death/protection is profile-scoped

### Evidence

#187 establishes strong base death/blessing continuity and proves material PvP/world-profile differences. The accepted PvP product baseline explicitly leaves exact world types and whether the first Reference proof is PvP-enabled unresolved.

### RECOMMENDATION

Freeze:

1. Character owns durable character-specific death/protection consequences and protection/progression facts required by an active profile.
2. Item loss, corpse/loot and value conservation remain combat/GAME-ITEM/DUR-03 owned.
3. Death/protection evaluation is under explicit Reference ruleset/world-profile policy revision.
4. Regular blessing/protection state remains representable without one universal formula.
5. PvP-only persistent facts are introduced only when their owning profile requires them and with explicit ownership/fencing.
6. Complete Twist/fair-fight/skull/Death Redemption edge semantics do **not** block generic Character durability before PvP world/profile policy is selected.
7. Exact PvP/death behavior remains a hard parity/implementation gate for every profile that exercises it.

This packet does not choose the first Reference PvP world type; that remains separate product/world-policy work coordinated with `GAME-CHANNEL-01` and related PvP/recovery gates.

## 11. Decision 8 — offline-training counter versus effectiveness

### Evidence

#187 provides strong continuity for:

- >=10 minutes offline before training gain;
- maximum 12 hours effective continuous training;
- 1 pool second consumed per training second;
- 1 pool second restored per second online or offline without training;
- reactivation needed to consume refilled pool after depletion.

### RECOMMENDATION

Freeze:

1. Offline training is an explicit ruleset capability, not universal engine behavior.
2. Where the target enables it, activation/counter/pool state is character-specific durable progression state.
3. The evidenced 10-min / 12-h / 1:1 timer-pool semantics form the Reference semantic counter model.
4. Exact effectiveness coefficients, advancement arithmetic, rounding, modifiers and version-sensitive selectable skills remain ruleset/SIM parity gates.

DUR-02 may represent the counter/capability without embedding effectiveness formulas.

## 12. Decision 9 — modern character-specific progression scope

### Evidence

#187 establishes or strongly supports:

- Weapon Proficiency Progress explicitly character-bound/non-transferable;
- charms/charm points/charm expansion character-specific under the Bazaar transfer contract;
- Hunting Task Points character-specific;
- permanent Hunting Task and Prey slots character-specific;
- Wheel/Promotion Points as strong character-build state;
- Animus Mastery as strong character-specific progression.

### RECOMMENDATION

Freeze:

1. Character **domain ownership scope** includes target-required character-specific progression/capability facts evidenced above.
2. Character-specific does not mean one giant aggregate/table/transaction.
3. DUR-02 may use character-owned child aggregates when size, contention, lifecycle or migration evidence justifies them.
4. Ruleset/content definitions own trees, thresholds, perks/mastery definitions, formulas and compatibility.
5. GAME-ITEM/DUR-03 owns item/resource conservation and value transfer.
6. Platform owns commercial entitlement source.
7. Character-specific progress refers to stable versioned definition identities and requires explicit migration for incompatible revisions.

This freezes ownership scope, not physical decomposition.

## 13. Decision 10 — every unresolved target rule is an explicit parity gate

### RECOMMENDATION

```text
UNKNOWN / CONFLICT target rule
-> may have a safe versioned semantic/policy envelope
-> may NOT be filled by current Global, OTS code or implementation convenience
-> may NOT be enabled as claimed Reference behavior
-> may NOT be PARITY_CONFIRMED
-> must be evidenced, explicitly DECLARED_DIFFERENCE, or excluded from the exercised release scope
```

Applies to remaining naming, lifecycle values, starter content, arithmetic, promotion edges, PvP edges, offline effectiveness and definition migrations.

For an external Reference milestone, every exercised character behavior must be either:

- `PARITY_CONFIRMED`; or
- an explicit owner-accepted `DECLARED_DIFFERENCE`.

Known `UNKNOWN/CONFLICT` cannot silently pass as fidelity.

## 14. Decision 11 — refine GAME-CHAR gate scope, not parity strictness

Current horizon wording can be read as requiring every exact formula before durable Character architecture closes. Evidence now shows that this conflates architecture and behavioral proof.

### RECOMMENDATION

GAME-CHAR architecture acceptance requires:

- complete semantic ownership boundaries;
- lifecycle/build/progression catalogue sufficient for durability;
- version/revision/migration/fencing rules;
- safe policy envelopes for target-specific values;
- explicit hard gates for all unresolved Reference values/formulas.

It does **not** require exact numeric arithmetic owned by later ruleset/SIM/content/world-profile contracts unless that arithmetic constrains durable identity, ownership, atomicity, irreversible representation or migration.

Reference parity itself remains unchanged and fail-closed.

## 15. Deliberately unresolved after recommended closure

### Naming/lifecycle

- exact normalization/repertoire/29-letter target continuity;
- deleted-name release algorithm;
- deletion grace `60 days`, total `30` and related July-28 continuity.

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

- first Reference PvP world type;
- complete Twist/fair-fight/skull edge matrix;
- exact target Death Redemption history/window semantics;
- remaining exact death arithmetic.

### Offline training

- effectiveness coefficients and advancement arithmetic.

### Modern progression

- exact formulas/definitions;
- physical child-aggregate/table placement;
- incompatible-definition migration details.

All remain hard-gated at their owning implementation/parity boundary.

## 16. Effect if owner accepts

After the owner-baseline delivery lifecycle closes:

```text
GAME-CHAR-01
DecisionStatus       = ACCEPTED
DeliveryStatus       = LIFECYCLE_CLOSED
ImplementationStatus = NOT_STARTED
Runtime authority    = NONE
```

Stage A remains binding; accepted Stage B becomes the semantic Reference/durability layer on top of it.

### DUR-02 effect

`DUR-02` may proceed to final **paper-only profile-neutral core Character schema architecture** using accepted semantics and must define explicit typed/versioned migration/extension boundaries for later accepted profile-specific character state.

This is not a claim that one schema package is complete for every future world/PvP profile. In particular, if a selected PvP-enabled or other world profile requires additional durable Character facts, final schema completeness for that profile remains blocked until those facts and their ownership/policy are accepted.

The profile-neutral core still may not:

- implement PostgreSQL DDL/migrations without separate authority;
- hard-code unproven values/formulas;
- select name normalization via DB convenience;
- collapse child progressions without evidence;
- encode one universal PvP/death profile;
- use an untyped/generic miscellaneous-state bag to defer semantic ownership;
- treat current Global as July-28 truth.

Later profile-specific schema extensions/migrations must preserve Stage-A revision/fencing, explicit semantic ownership, compatible migration/rollback policy and the accepted Reference evidence boundary.

### Runtime/content effect

None. GAME-CHAR acceptance alone authorizes no runtime, content, client, protocol, Platform or production implementation.

## 17. Alternative: wait for every exact value

Possible, but **NOT RECOMMENDED**. It would block durability on historical values owned by other evidence/gate domains, pressure persistence to infer policy, delay later architecture and encourage current-Global/OTS substitution.

The recommended split still has a strict parity gate — at the feature/ruleset/fixture boundary where exact behavior matters.

## 18. Owner decision package

### RECOMMENDATION — NOT OWNER-ACCEPTED

Accept decisions 1–11 (sections 4–14) and the effect in section 16.

Compact form:

1. global logical name namespace + versioned canonical comparison policy; exact normalization/recycling parity-pending;
2. Stage-A lifecycle retained; Reference active quota `25`; other unresolved lifecycle numbers are versioned parity-pending policy;
3. creation uses versioned ruleset/content/starter context; exact starter content stays content/parity-pending; pre-vocation state must be representable;
4. first Reference vocabulary explicitly includes five vocation families/promoted forms plus pre-vocation state and eight evidenced skill categories;
5. progression ownership/catalogue closes independently from exact formulas when durability stays formula-neutral/versioned;
6. promotion achievement is Character-owned; benefits may depend on Platform entitlement/ruleset; fee/lapse edges stay parity-pending;
7. death/protection is profile-scoped; generic Character durability does not require exhaustive PvP edges before world-profile selection;
8. offline-training counter/capability is Character-owned with evidenced 10-min/12-h/1:1 semantics; effectiveness stays ruleset/SIM-pending;
9. modern character-specific progression belongs to Character domain scope but may use child aggregates; definition/item/economy/Platform authorities remain separate;
10. every remaining `UNKNOWN/CONFLICT` is an explicit hard parity gate, never an implementation default;
11. refine GAME-CHAR architecture scope to semantic ownership/versioning/migration; exact arithmetic remains mandatory before Reference implementation/parity claim when it does not constrain durable topology.

If accepted, overall `GAME-CHAR-01` may become `ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED` after the acceptance-delivery lifecycle, and `DUR-02` may proceed to final paper-only **profile-neutral core** Character schema architecture with explicit typed/versioned profile-extension and migration boundaries. Any profile-specific durable Character facts remain separately gated before that profile's schema may be called complete.

## 19. Supersession / reopening rule

A later proposal may supersede a decision from this package only with named evidence such as:

- stronger July-28 target-period primary evidence contradicting a `DERIVED` rule;
- an explicit later accepted Reference revision;
- evidence that a deferred formula/value actually changes durable identity, ownership, atomicity, irreversible representation or migration;
- legal/privacy/security/integrity constraints;
- a separately accepted owner product/ruleset strategy change.

Implementation convenience, current Global behavior or an OTS implementation is never sufficient supersession evidence.

## 20. Acceptance does not mean

- all July-28 character behavior is known;
- current values are automatically July-28 values;
- unknown formulas may be guessed;
- Reference character runtime may bypass later gates;
- PostgreSQL implementation is authorized;
- first Reference PvP world type is selected;
- one Character schema is complete for all future world/PvP profiles;
- GAME-CHANNEL, GAME-ITEM, SIM or content gates are accepted;
- Premium/VIP implementation is authorized;
- external repository writes are authorized.

Until the owner explicitly accepts or modifies section 18, this document remains **PRE-DECISION SYNTHESIS / NOT ACCEPTED** and `GAME-CHAR-01` remains `PROPOSED / PLANNED / NOT_STARTED`.
