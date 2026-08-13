# GAME-ABILITY-01 — Effect Families and Reference Mechanic Catalogue Owner Baseline

- Status: **OWNER-ACCEPTED PARTIAL BASELINE / GAME-ABILITY-01 REMAINS OPEN**
- Date: 2026-08-13
- Gate: `GAME-ABILITY-01`
- Scope: effect-family vocabulary boundary, cross-domain transition ownership and Reference ability/combat mechanic catalogue semantics
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**

## Decision

Oteryn separates three semantic levels that must not collapse into one another:

```text
concrete Ability / Mechanic Definition
-> composition of small typed Effect Families
-> typed domain-owned transition proposals where applicable
-> validated bounded Effect Plan
-> authoritative domain owner(s) / PRIMARY COMMIT boundaries

Reference Mechanic Catalogue
-> identifies one concrete Reference mechanic
-> links evidence + exact behavior-affecting revision bindings
-> records evidence / implementation / parity state
-> does not execute gameplay
```

An **Effect Family** describes a reusable semantic kind of consequence and its validation/composition contract. It is not a spell name, presentation opcode, unrestricted state patch, domain ownership transfer or requirement for one Rust enum variant per mechanic.

A **Reference Mechanic Catalogue entry** describes how one observed Reference ability/combat mechanic is represented and evidenced. Catalogue membership does not make the entry executable, authoritative or parity-confirmed.

## Effect-family boundary

The core vocabulary should remain small and semantically stable. Illustrative reusable families include damage, healing, condition transitions, cooldown/ability-charge transitions and explicitly typed character-resource transitions. Dispel/cleanse may remain typed condition-transition semantics rather than forcing a new top-level family. These examples do **not** freeze an exhaustive enum.

A generic resource transition, if retained later, must name its resource semantic class and authoritative owner. Currency, item quantity, loot, market value or another conserved durable value may never be smuggled through a generic `ResourceDelta`-like effect to bypass `GAME-ITEM` / `DUR-03`.

Effect-family identity is semantic and versioned where behavior changes. Exact Rust names, numeric discriminants, ID widths, wire representation and serializer are deliberately not frozen.

Adding, removing or materially changing a **core** Effect Family is a public architecture-contract change and requires explicit architecture review/owner acceptance. By contrast, adding a new concrete Reference catalogue entry that composes already accepted families/domain integrations is content/evidence work and must not require a new core architecture primitive merely because another mechanic is catalogued.

## Criteria for adding a core family

A new core effect family is justified only when the mechanic represents a recurring semantic consequence or invariant boundary, has a clear validation/ownership model, can be bounded and replayed deterministically, improves static validation/testing/Studio/Game Intelligence value, and cannot be represented safely and legibly by composition of existing families or a bounded extension.

A new family is **not** justified merely because one named spell, monster, item or quest mechanic is unusual. One-off behavior should prefer composition, an owning-domain primitive or the existing capability-bounded Wasm/WIT escape hatch before expanding core vocabulary.

No `GenericPatch`, arbitrary component mutation, unrestricted callback/event-bus payload or script-owned final-state delta is accepted as an effect family.

## Domain ownership is not effect vocabulary

An ability may initiate a typed consequence without taking mutation ownership from the domain that owns the affected invariant.

The boundary is:

```text
Ability / mechanic occurrence
-> typed effect or domain-transition proposal
-> owning-domain validation
-> Effect Plan / ordered domain action as applicable
-> owning-domain authoritative commit
```

A typed domain-transition proposal is **not automatically a new core Effect Family**. Domain-specific operations may remain typed integration surfaces owned by that domain when promoting them into the core vocabulary would blur invariants or create primitive proliferation.

Examples of domain-routed consequences include:

- item, stack, currency, loot or other conserved value -> `GAME-ITEM` / `DUR-03`;
- teleport, movement, push, pull or occupancy change -> world/movement ownership and later interaction/spatial contracts;
- summon, spawn, despawn or entity-lifecycle change -> authoritative entity/world lifecycle owner;
- aggro, threat or AI-control state -> `GAME-AI-01` owner;
- tile/world-object activation or interaction -> world/content and `GAME-INTERACTION-01` boundaries;
- character resource/state outside the explicitly GAME-ABILITY-owned availability/condition surfaces -> the owning character/simulation domain.

The final exhaustive domain map is deferred. These examples freeze the **ownership rule**, not new domain APIs.

Wasm/WIT, content, AI and ability definitions may propose only approved bounded typed operations. They cannot directly mutate another domain, create private ownership, bypass its fencing/conservation/legality rules or return an arbitrary authoritative state patch.

## Cross-domain plans

One mechanic may require several typed consequences, for example damage plus movement, damage plus item consumption or a condition plus an AI reaction. The shared Effect Plan may correlate and order such proposals, but this baseline does **not** invent one global database transaction or make GAME-ABILITY the owner of every participating domain.

Existing domain ownership, fencing, idempotency and conservation contracts remain authoritative. Any mechanic that requires stronger atomic cross-domain semantics than currently accepted remains fail-closed until the affected owner contracts define that behavior explicitly. Hidden partial mutation is not an acceptable fallback.

## Reference Mechanic Catalogue

The Reference Mechanic Catalogue is an evidence-linked semantic registry/index for concrete Reference abilities and combat mechanics. It is separate from both core effect vocabulary and executable content definitions.

A catalogue entry must be capable of binding, as applicable:

```text
stable mechanic identity
+ accepted Reference target/revision context
+ Reference evidence-manifest case references
+ Ability/Mechanic Definition revision reference
+ target-policy revision
+ cast/cost/commit/cooldown/charge policy revisions
+ effect-family composition
+ formula / damage-heal composition revision
+ condition-policy references
+ deterministic RNG semantic revision/purpose references
+ owning-domain integration references
+ expected observable/parity fixture references
+ evidence, implementation and parity classification
+ declared Reference/Evolved differences where relevant
```

The catalogue's stable mechanic identity is a catalogue-local semantic/provenance key. This baseline does **not** create a new global foundation identity, protocol discriminator, runtime entity ID, `WorldId`/`ChannelId`-like identity or cross-domain ownership token. Exact physical type/width is deferred.

The exact physical catalogue schema and exact field names are not frozen. The binding set above is semantic: an entry must preserve enough exact behavior-affecting provenance to explain what was evidenced, what Oteryn implements and what parity claim—if any—is justified.

## Evidence and parity fail-closed rule

The existing Reference evidence/parity manifest remains the authority for Reference evidence cases and its evidence/implementation/parity axes. The catalogue may reference those cases; it may not upgrade, duplicate or override their classification.

Therefore:

- an empty manifest case set remains an evidence gap;
- `UNKNOWN`, `CONFLICT` or otherwise insufficient evidence remains fail-closed;
- catalogue presence alone never means `PARITY_CONFIRMED`;
- a concrete Oteryn mechanic existing in content never proves Reference behavior;
- a strong Reference evidence case without an accepted Oteryn implementation/parity fixture never proves implementation parity.

Exact catalogue status-enum names are deferred; they must compose with, not fork, the existing manifest axes.

## Reference versus Evolved

The Reference catalogue describes the accepted immutable Reference target context. Evolved rulesets may reuse a Reference mechanic identity as provenance and explicitly declare a versioned difference, but an Evolved change must not rewrite Reference evidence or silently turn the Reference catalogue into an Evolved content source.

Reference and Evolved may share effect families and execution infrastructure without sharing exact formula, timing, target or content revisions.

## Catalogue versus executable content

A catalogue record is not runtime code and is not the sole authoring source of an ability. It may point to the exact Oteryn Ability/Mechanic Definition and behavior revisions used for parity, but loading a catalogue record must not itself activate a spell, mutate world state or expose a new protocol opcode.

Physical authoring, build-time compilation, generated indexes and whether the catalogue is hand-authored or derived are later tooling decisions.

## Wasm/WIT escape hatch

Exceptional mechanics may use the existing bounded DUR-04 Wasm/WIT extension only to consume approved snapshot/capability inputs and return bounded typed proposals that re-enter these same effect-family/domain-owner boundaries.

A script cannot define a private authoritative effect family, bypass the Reference evidence/parity contract, directly mutate another domain or turn catalogue metadata into executable authority.

Repeated evidence that many representative mechanics require the same awkward extension is a signal to reconsider the core-family vocabulary through an explicit architecture decision, not permission for silent primitive proliferation.

## Problem and constraints

**Problem:** broad Reference/Evolved combat content needs a reusable consequence vocabulary and a concrete parity catalogue without hardcoding every spell/mechanic into core or letting a generic scripting/event layer bypass domain invariants.

**Constraints:** one typed Effect Plan/commit model; targeting/legality and timing/commit boundaries already accepted; condition/cooldown and damage/heal composition already accepted; domain ownership/conservation remain authoritative; Reference evidence is fail-closed; DUR-04 scripting remains proposal-only; no runtime implementation authority exists here.

## Realistic options

### A — small typed families + domain-owned transitions + evidence-linked catalogue — **SELECTED**

Keeps the engine vocabulary reusable while concrete Reference mechanics remain data/evidence compositions. It maximizes static validation, parity traceability, Studio support and domain safety, at the cost of disciplined taxonomy and integration contracts.

### B — one core enum/handler per concrete mechanic — REJECTED

Initially simple to dispatch, but every content addition churns core code, ties Reference names to engine architecture, encourages duplicated special cases and makes Evolved variants expensive.

### C — generic patch/event graph — REJECTED

Highly flexible but weakens ownership, deterministic explanation, static validation and anti-abuse auditing; safe constraints would eventually recreate typed families less explicitly.

### D — script-first executable Reference catalogue — REJECTED

Fast for importing one-off mechanics but creates a second gameplay engine, weakens parity provenance and lets script/runtime concerns invade evidence metadata.

## Trade-offs, risks and impact

The selected model costs more up-front taxonomy/evidence work. Main risks are family proliferation, over-generic families, duplicated catalogue/content truth, domain-routing ambiguity, cross-domain partial-failure ambiguity and false parity by catalogue presence.

Mitigations are explicit family-admission criteria, no generic state patch, owner-domain routing, no new cross-domain atomicity claim, catalogue-to-content references rather than duplicate executable definitions, and mandatory fail-closed evidence/parity composition.

**Player impact:** fewer mechanic-specific inconsistencies, safer fixes across related abilities and clearer parity claims. **Producer impact:** new content normally composes existing primitives instead of requiring engine changes, while catalogue coverage exposes evidence/parity gaps. **Operational impact:** typed family/domain lineage improves telemetry, exploit analysis and migration audits without making analytics authoritative.

## Decision timing

**Must decide now: YES.** The next Reference ability/combat catalogue, representative parity fixtures, Studio authoring model and later AI/interaction integrations need to know whether concrete mechanics are core primitives, data compositions or domain-routed transitions.

**Downstream blocked:** Reference ability/combat catalogue population, parity-fixture shape, stable Studio mechanic composition, `GAME-AI-01` ability-use integration and `GAME-INTERACTION-01` effect initiation boundaries.

**Harder later:** once many mechanics become bespoke enum variants, generic state patches or script-owned domain mutations, normalizing them requires content migration, replay/provenance reconstruction and domain-invariant audits.

**Supersession evidence:** representative Reference/Evolved mechanics that cannot be expressed safely without excessive indirection; measured performance after semantic-preserving optimization; Studio authoring evidence showing unacceptable complexity; repeated bounded-extension patterns proving a missing family; security/replay/domain-ownership evidence favoring another boundary.

**Deliberately not decided:** exhaustive effect-family list; exact family IDs/Rust enum/type graph; physical catalogue schema/serializer; exact Reference catalogue entries; exact formulas/timing/RNG values; movement/entity/AI/world/item domain APIs; cross-domain transaction protocol; protocol/client UI; persistence/DDL; runtime implementation.

Unresolved Reference-sensitive behavior remains fail-closed.

## Current status

```text
GAME-ABILITY-01 -> REQUIRED_FOR_ALPHA / OPEN
accepted -> small typed reusable effect-family boundary
accepted -> cross-domain consequences stay under owning domains
accepted -> Reference Mechanic Catalogue != effect vocabulary != executable content
accepted -> catalogue composes existing evidence/implementation/parity axes
accepted -> no catalogue-presence parity claim
next -> populate representative Reference mechanic cases / parity fixtures without runtime implementation
```
