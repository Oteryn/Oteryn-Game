# ADR-0019: Gameplay modularity and anti-monolith boundary

- Status: **Accepted**
- Date: 2026-09-25
- Decision owner: repository owner
- Applies to: gameplay-system implementation inside the Oteryn GameNode, including systems such as Bestiary, Bosstiary, Exaltation Forge, Wheel of Destiny, Prey, Charms, Imbuements, Achievements and Weapon Proficiency
- Refines: ADR-0015 without freezing a crate/module layout
- Preserves: ADR-0009 GameNode process identity, FND-03 runtime authority, DUR-03 item/value transaction ownership, DUR-04 Content/World/scripting boundaries, SIM-DETERMINISM-01, GAME-ITEM-01 and the accepted Character authority boundaries
- Does not authorize: runtime implementation, new services, a generic plugin framework, arbitrary native dynamic libraries, live-scope hot reload, new public protocol semantics or production activation

## Context

Oteryn must avoid the maintenance failure mode seen in legacy game servers where large gameplay systems become cross-cutting code embedded directly throughout one server implementation. In that shape, changing one system such as Forge, Bestiary or Wheel of Destiny can require edits across unrelated combat, item, persistence, protocol and player code, making changes risky and expensive.

Oteryn still benefits from ADR-0009's one-process GameNode for the primary authoritative runtime and from ADR-0015's refusal to freeze a premature physical crate/module topology. The required decision is therefore about **logical coupling and ownership**, not about permanently selecting a directory tree.

The desired result is a server that can remain one efficient process while gameplay systems are independently understandable, testable and replaceable behind typed domain boundaries.

## Decision

Oteryn gameplay uses a **logically modular in-process architecture** by default.

A major gameplay system MUST NOT become an unbounded collection of special cases spread through the GameNode. It MUST have an explicit gameplay boundary and consume canonical domain owners through typed contracts.

This decision does **not** require one Rust crate per gameplay system. A feature may initially be a Rust module, a crate, Content declarations, bounded Component Model/WIT components, or a composition of those mechanisms. Physical placement remains an implementation choice until evidence justifies freezing it.

The following rule is binding:

> Gameplay systems compose canonical Oteryn authorities; they do not create private copies of Character, Item, Creature, Ability, Effect, Economy, Persistence, Content or Simulation truth.

The GameNode remains the composition root. It wires gameplay capabilities to their owning domain contracts while preserving one authoritative runtime process under ADR-0009.

## Required separation

Every material gameplay system MUST identify:

1. **Static authored input** — definitions and tuning that belong in versioned Content/World data.
2. **Runtime logic** — the system-specific rules that interpret commands/events and request typed domain operations.
3. **Mutable authoritative state** — the canonical owner of player/item/world state; gameplay modules do not invent a second owner.
4. **Durability boundary** — which accepted persistence/transaction owner commits state.
5. **Protocol surface** — typed commands/results/state projections when client-visible behavior is required.
6. **Client presentation** — UI is a consumer of authoritative state, not the source of gameplay legality.
7. **Determinism and resource bounds** — use the accepted simulation/runtime contracts rather than private randomness, clocks or unbounded work.

A system that cannot name those boundaries is not ready to be added as another gameplay subsystem.

## Dependency direction

The preferred dependency direction is:

```text
apps/game-server
    |
    +-- gameplay system composition
    |     +-- Bestiary
    |     +-- Forge
    |     +-- Wheel
    |     +-- Prey
    |     +-- ...
    |
    +-- canonical domain contracts
          +-- Character / Progression
          +-- Item / ItemInstance
          +-- Creature
          +-- Ability / Effect
          +-- Economy / DUR-03 transactions
          +-- Persistence / Durability
          +-- Simulation / deterministic RNG
          +-- Content
```

Gameplay systems may depend on accepted domain contracts. They SHOULD NOT depend on unrelated gameplay-system internals.

Cross-system behavior SHOULD be expressed through shared typed domain/effect/event contracts when an accepted owner exists, rather than direct private calls that couple two feature implementations.

Transport, SQL adapters, client rendering and wire encoding remain outside gameplay-rule ownership.

## Code, Content and scripting roles

Oteryn deliberately separates three kinds of change.

### Core Rust/domain code

Rust owns invariants where correctness, authority, conservation, security, determinism or transactionality require trusted engine behavior.

Examples include:

- authoritative item/value transactions and anti-duplication;
- Character and ItemInstance ownership;
- deterministic simulation primitives;
- command legality and commit boundaries;
- persistence fences and recovery semantics.

A gameplay feature MUST use these owners rather than bypass them.

### Gameplay-system logic

A gameplay system owns the feature-specific rules that cannot be represented as plain authored data.

Examples include:

- Bestiary kill-progression decisions;
- Forge eligibility and tier-transition orchestration;
- Wheel allocation/refund rules and derived effect composition.

This layer coordinates domain capabilities but does not acquire arbitrary SQL or global mutation authority.

### Content and bounded scripting

Values, topology, definitions, thresholds and other authored semantics SHOULD live in versioned Content when they do not require a new trusted engine invariant.

Where declarative Content is insufficient, DUR-04 remains the authoritative extension boundary: WebAssembly Component Model components using project-owned, versioned WIT capabilities.

A script/component may propose only the typed actions its capability contract permits. It does not gain direct PostgreSQL access, arbitrary object mutation, unrestricted value creation or ownership of final authoritative state.

Changing Content or script semantics creates a new immutable revision/artifact. This decision does not weaken DUR-04 activation rules and does not redefine hot reload as in-place mutation of a live revision.

## Representative systems

### Bestiary

Bestiary static facts remain associated with canonical Creature definitions.

Player kill counts, unlocked stages and rewards belong to the accepted Character/Progression owner. The Bestiary gameplay logic reacts to authoritative creature-death outcomes and requests progression changes through that owner.

Bestiary MUST NOT become a second Creature database or a second Character persistence model.

### Exaltation Forge

Forge classification and maximum tier may be authored Item-definition metadata.

The current tier and other mutable properties belong to ItemInstance/Character/Progression/Durability according to the accepted owning contracts. Forge operations that consume or transform durable value MUST use the DUR-03 transaction/conservation boundary.

Forge MUST NOT directly subtract currency, rewrite item custody or bypass anti-duplication ownership.

### Wheel of Destiny

Wheel topology, node definitions, costs, prerequisites and effect declarations SHOULD be authored/versioned data where semantics permit.

Allocated points, unlocks and active configuration belong to the accepted Character/Progression owner.

Wheel effects SHOULD compose canonical Ability/Effect/stat-resolution capabilities instead of scattering feature-specific conditionals through combat, spells and Character code.

Exact Wheel runtime semantics remain a later owning gameplay decision; this ADR establishes the modularity boundary only.

## Anti-monolith rules

A new gameplay system MUST NOT:

- place feature-specific mutations directly across unrelated domain modules merely because the types are reachable;
- own a second representation of canonical Character, Item, Creature or economy state;
- issue arbitrary SQL as part of gameplay-rule execution;
- introduce a private RNG/clock when accepted deterministic simulation primitives apply;
- bypass DUR-03 for durable item/currency/value mutation;
- bypass Content revision/provenance rules for authored semantics;
- use an unrestricted global event bus or generic mutation payload as a substitute for typed ownership;
- require unrelated gameplay modules to understand its private state representation;
- add a speculative crate, service or plugin host with no immediate consumer.

When a change would otherwise violate one of these rules, the owning architecture must define the shared contract or explicitly justify and review a boundary change.

## Physical layout remains intentionally open

This decision does not freeze a tree such as:

```text
crates/gameplay/bestiary
crates/gameplay/forge
crates/gameplay/wheel
```

That layout is permitted when implementation evidence shows a crate boundary is useful, but ADR-0015 still governs physical decomposition.

A small feature can remain an internal module. A reusable or independently testable capability can become a crate when it has an immediate consumer and a real dependency boundary. A separately deployed service requires the stronger evidence already demanded by ADR-0015 and does not become part of one GameNode process identity.

The architecture therefore freezes **coupling rules**, not folders.

## Change-impact objective

The implementation SHOULD make the smallest appropriate change surface possible:

- tuning/data-only changes -> new Content revision where semantics allow;
- bounded scripted behavior -> new compatible script/component revision where DUR-04 capabilities allow;
- feature-rule changes -> the owning gameplay system and its focused tests;
- domain-invariant changes -> the canonical owning domain and all affected consumers.

A normal Forge change should not require editing Bestiary or Wheel internals. A Wheel perk value change should not require adding conditionals throughout unrelated combat code when an existing typed Effect/Ability capability can express it.

This is a design objective, not permission to skip cross-domain tests when a real shared contract changes.

## Acceptance expectations for future implementations

Before a material gameplay system is considered implementation-ready, its task/contract should identify at minimum:

- canonical state owner;
- static Content inputs;
- mutable runtime/durable state;
- allowed domain dependencies;
- transaction/conservation boundary where applicable;
- deterministic RNG/time dependency where applicable;
- protocol/client projection if any;
- migration/versioning behavior;
- focused unit/component tests;
- required integration/E2E scenarios.

Architecture checks may later enforce dependency rules once real module/crate boundaries exist. This ADR does not create speculative enforcement machinery before there is code to protect.

## Relationship to ADR-0015

ADR-0015 remains fully in force.

This ADR does **not** freeze a modular-monolith crate graph. It narrows a different question: regardless of whether future implementation uses modules, crates or justified adjacent services, gameplay semantics must preserve explicit domain ownership and avoid uncontrolled cross-feature coupling.

Thus:

- ADR-0015 keeps physical decomposition open;
- ADR-0019 fixes the logical anti-monolith boundary.

## Consequences

Benefits:

- gameplay features can evolve with smaller, reviewable change surfaces;
- canonical ownership remains centralized instead of duplicated by feature systems;
- Content and bounded scripting absorb appropriate data/behavior changes without turning the trusted engine into a collection of hard-coded exceptions;
- one-process GameNode performance is preserved;
- later physical refactoring does not require redefining gameplay truth.

Costs:

- feature authors must work through typed owners instead of using convenient direct mutation;
- shared contracts may need deliberate evolution when genuinely cross-domain mechanics appear;
- not every gameplay change can be reduced to Content or scripting when a trusted invariant really changes.

## Integration status

This document records the owner's 2026-09-25 direction and was protected-integrated through PR #906 at `9728d30669a85579d333f826ebe7f812c76337ad`. Architecture acceptance does not by itself authorize runtime implementation.

`IMPLEMENTATION_AUTHORITY: NONE`

`PRODUCTION_AUTHORITY: NONE`
