# Oteryn v2 Product Direction Baseline

- Status: Accepted owner direction baseline
- Canonical decision: ADR-0010
- Date: 2026-08-06
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Applies to: product identity, world profiles, rulesets, content planning, parity analysis and future differentiation

## Owner direction

Oteryn v2 will initially reproduce the important observable gameplay and product behavior of the official live Tibia experience, referred to in this programme as **Global Tibia**. Over time, Oteryn will deliberately improve and expand areas that the official product has neglected, constrained through legacy architecture or left underdeveloped.

The platform may expose both directions to players at the same time:

1. a **reference world profile** focused on faithful Global Tibia behavioral parity;
2. an **evolved Oteryn world profile** based on the same foundation but containing explicitly approved Oteryn changes and improvements.

Players may choose the world profile they prefer.

This direction is accepted at product level. The detailed `GAME-VISION-01` contract, exact parity target, user-facing names, formulas, content scope and release sequence remain unresolved.

## Architectural interpretation

The two product experiences must not become two engines, two clients, two protocols or two uncontrolled code forks.

Both world-profile families must use:

- the same canonical Oteryn v2 repository and Rust workspace;
- the same authoritative domain engine;
- the same native Rust client;
- the same `protocol-oteryn` family;
- the same identifier, session, persistence, transaction, analytics and security foundations;
- the same World Project, compiler, bundle and Oteryn Studio architecture;
- versioned ruleset, content, asset and presentation profiles selected explicitly for each logical world.

Differences belong in data, policy and capability-bounded domain modules where the relevant accepted contract permits them. They must not be encoded through protocol forks, duplicated server implementations, hidden conditionals tied to world names or independent repositories.

## World-profile model

A logical world must declare an immutable, versioned product-profile assignment composed conceptually of:

- `WorldProductProfileId` and revision;
- `RulesetId` and revision;
- `ContentRevision`;
- `AssetRevision` where client-visible presentation differs;
- compatibility requirements for client, server and protocol capabilities.

These names describe required concepts, not accepted schema or wire identifiers. Exact identifiers belong to future foundation and ruleset contracts.

A product-profile revision may be upgraded only through an explicit compatibility, migration, validation and rollback process. A world must never silently switch from reference parity to evolved behavior or vice versa.

Reference and evolved profiles are separate logical worlds with distinct `WorldId` values. Every channel of one logical world inherits the same product-profile, ruleset and compatible content revision. Channel switching cannot change profile family, and an instance cannot be used as a hidden bridge between differently balanced worlds or economies.

## Reference world profile

The reference profile is intended to reproduce Global Tibia closely enough that a knowledgeable player can identify the same fundamental rules and system behavior, and every material deviation can be documented.

Parity analysis must eventually cover, as applicable:

- character creation, vocations, progression, skills, attributes and death;
- movement, collision, floors, visibility and interactions;
- combat, spells, runes, weapons, conditions, cooldowns and PvP rules;
- creatures, NPCs, spawning, pathfinding and encounter behavior;
- items, equipment, containers, loot, depots, banks and transformations;
- quests, dialogue, shops, achievements, bestiary and recurring progression;
- parties, guilds, chat, friends, houses, market, trade and economy;
- regions, raids, bosses, events and world lifecycle;
- client interaction patterns, controls, feedback and player-facing semantics;
- account, login, session, updater, support and moderation behavior relevant to gameplay.

The reference world must maintain a parity matrix that classifies each evaluated behavior as:

- `PARITY_CONFIRMED`;
- `PARITY_PENDING_EVIDENCE`;
- `INTENTIONAL_DIFFERENCE`;
- `TECHNICAL_OR_LEGAL_SUBSTITUTION`;
- `OUT_OF_SCOPE_FOR_CURRENT_MILESTONE`.

A reference world is not allowed to accumulate undocumented gameplay differences under the label of bug fixes or implementation convenience.

## Evolved Oteryn world profile

The evolved profile begins from the shared parity foundation but may contain owner-approved improvements and original expansion.

Candidate areas include, without accepting final designs:

- stronger performance, latency handling, recovery and scalability;
- multichannel-first world operation without character, item, economy or house duplication;
- richer creature AI, encounter behavior and dynamic populations;
- more capable quests, branching narrative and persistent world consequences;
- better raids, bosses, dynamic events and living-world systems;
- safer and more transparent item/economy transactions;
- improved economy health, fraud detection and audited correction;
- modern client UX, controls, accessibility, localization and onboarding;
- clearer visual, audio and combat feedback;
- stronger anti-cheat evidence and fair human-reviewed enforcement;
- first-class Game Intelligence for balance, economy, integrity and world analysis;
- Oteryn Studio and substantially better content-production workflows;
- deterministic simulation, testing, replay and recovery;
- new progression, social, world and endgame systems accepted through later contracts.

Every difference must have an explicit owner, rationale, scope, compatibility impact and acceptance evidence. “Improvement” is not self-proving; changes must be assessed against player experience, balance, economy, security, operational cost and maintainability.

## Shared account and isolated world state

The Platform account and reusable identity may be shared across both world-profile families, consistent with accepted Identity boundaries.

Unless a later dedicated contract explicitly permits otherwise:

- characters are world-scoped;
- inventories, currencies, depots, banks, houses, market state and progression are world-scoped;
- guild and ranking state are world-scoped;
- no item, currency, character or economy transfer occurs between reference and evolved profile families;
- no reward may be claimed in one profile and redeemed in the other;
- account-wide entitlements, cosmetics or achievements are not assumed to be portable;
- profile-family boundaries must be enforced by durable transactions, session admission and analytics.

This default prevents balance differences from becoming an arbitrage or duplication path.

## Player choice and product clarity

The client and world-selection flow must make the profile family visible before character creation and login.

Players must be able to understand:

- whether a world follows the reference or evolved profile;
- the profile revision and major intentional differences;
- transfer restrictions;
- update cadence and compatibility expectations;
- whether the world tracks a moving Global Tibia baseline or a pinned reference release.

The final user-facing names must avoid misleading affiliation claims and require a separate branding/legal review. Internal architecture terminology must not be treated as public branding.

## One protocol, one engine, multiple rulesets

The dual-profile strategy reinforces existing non-negotiable architecture:

- `protocol-oteryn` remains the single gameplay protocol family;
- classic, current-Global and evolved behavior are ruleset/content profiles, not separate wire protocols;
- the client negotiates capabilities and receives profile/content revisions without becoming authoritative;
- the server owns legality, ordering and results;
- shared domain code must not contain uncontrolled `if reference_world` / `if evolved_world` branching;
- differences must be expressed through explicit policy interfaces, typed capabilities and versioned content;
- parity fixtures and evolved fixtures must execute against the same domain boundaries.

## Testing and evidence

The product profiles require separate but related evidence suites:

### Reference parity evidence

- behavior matrices derived from lawful observable evidence and maintained sources;
- deterministic fixtures for formulas, ordering and edge cases;
- golden scenario comparisons where legally and technically possible;
- explicit deviations and unresolved evidence gaps;
- regression protection against accidental divergence.

### Evolved-profile evidence

- tests proving the intended difference rather than accidental divergence;
- migration and rollback scenarios from prior evolved revisions;
- balance, economy, performance and abuse analysis;
- client-visible E2E scenarios;
- compatibility evidence across client, protocol, server and content revisions.

### Cross-profile isolation evidence

- no cross-profile character or session admission error;
- no inventory, currency, reward, market, house or progression transfer;
- no analytics or administrative command confusion between profile families;
- no client cache/content confusion when switching worlds;
- no profile-specific behavior leaks through process-global mutable state.

## Game Intelligence comparison

Game Intelligence may compare aggregated health and player-experience signals between reference and evolved profiles to help evaluate changes.

This must preserve accepted ADR-0006 boundaries:

- analytics does not mutate authoritative gameplay;
- anomaly or balance scores do not autonomously punish players or deploy changes;
- profile comparison uses privacy-safe identifiers and explicit retention;
- causal conclusions require human analysis and controlled experiments;
- reference-world telemetry must not be treated as automatic permission to copy every behavior into the evolved profile.

## Risks and mitigations

### Player-base fragmentation

Operating two profile families may divide population and social activity.

Mitigation requirements:

- capacity and population thresholds before opening additional worlds;
- clear world positioning;
- no assumption that both profiles launch simultaneously;
- ability to delay, merge or retire a profile only through `GAME-WORLD-LIFECYCLE-01` rules;
- no silent cross-profile merge.

### Double content-maintenance burden

Maintaining parity and evolved content can double authoring, testing and support work.

Mitigation requirements:

- shared canonical content sources where behavior is equal;
- explicit versioned deltas/overlays where profiles differ, subject to `DUR-04` and creator-tooling contracts;
- automated compatibility and divergence reports;
- no copy-pasted content trees without provenance and ownership.

### Ambiguous meaning of “1:1”

Global Tibia is a moving product and not every internal rule is publicly observable.

Mitigation requirements:

- `GAME-VISION-01` or a dedicated parity contract must choose whether each reference world tracks a pinned release, a dated behavior baseline or a continuously updated live target;
- unsupported internal assumptions remain `UNKNOWN`;
- every approximation or substitution is explicit;
- parity claims are scoped to named systems, versions and evidence.

### Inherited defects and legacy limitations

Blind parity could reproduce bugs, poor UX or architectural constraints.

Mitigation requirements:

- reference parity preserves player-observable behavior only when intentionally accepted;
- security, data integrity, privacy and legal safety always override defect compatibility;
- a defect may be reproduced only through an explicit product decision and never by weakening authoritative invariants;
- the evolved profile provides a controlled path for improvements.

### Balance and economy arbitrage

Different formulas or rewards can make transfers exploitable.

Mitigation requirements:

- default hard isolation of characters, items, currencies and progression;
- no transfer until a dedicated migration contract proves conservation and balance;
- profile-scoped analytics, administration and support tools;
- explicit account-wide entitlement rules before any shared benefit is enabled.

## Legal and provenance boundary

Behavioral parity does not authorize copying proprietary source code, server binaries, packet implementations, databases, maps, quests, text, graphics, audio, trademarks or other protected assets.

Oteryn v2 must use:

- independently implemented code;
- project-owned `protocol-oteryn`;
- content and assets with verified rights and provenance;
- lawful observation, documentation and reference sources;
- explicit substitutions where parity cannot be achieved safely or legally.

Legacy repositories and data remain bounded migration/reference evidence under accepted ADRs; they are not implicit runtime dependencies.

## Required future decisions

This baseline leaves the following decisions open:

1. exact definition and measurement of Global Tibia parity;
2. pinned versus continuously tracking reference-world policy;
3. first launch profile and whether both profiles launch together;
4. exact product-profile/ruleset/content revision contract and its World Registry/Game Gateway admission binding;
5. initial list of intentional Oteryn improvements;
6. world naming, branding and player-facing disclosure;
7. transfer, shared-entitlement and shared-cosmetic policy;
8. content inheritance/delta strategy;
9. profile-specific LiveOps and release cadence;
10. population thresholds and lifecycle policy for each profile family.

These belong to `GAME-VISION-01`, `ALPHA-RULESET-01`, `PROD-COMPAT-01`, `GAME-WORLD-LIFECYCLE-01`, `DUR-04` and related dedicated contracts.

## Dependency and programme effect

- `FND-01`, `VSL-02` and the atomic destination cutover are complete.
- The remaining cutover action is the source-only `blakinio/otclient` historical marker; `FND-ID-01` follows after that closeout is merged and verified.
- Both profile families continue to share the same canonical workspace, client and foundation.
- `FND-02` must preserve one `protocol-oteryn` family with profile/ruleset capability references rather than protocol forks.
- `FND-04` and the Platform World Registry/Game Gateway contract must prevent admission to a world/profile revision different from the one selected and authorized.
- `DUR-02`, `DUR-03` and `DUR-04` must preserve world/profile isolation and versioned migration.
- `ALPHA-RULESET-01` must define how reference and evolved behavior are selected without engine forks.
- `GAME-VISION-01` must refine this baseline into measurable product pillars and parity scope.
- `GAME-WORLD-LIFECYCLE-01` must govern creation, transfer, merge, retirement and archival across profile families.
- No implementation begins solely because this direction is recorded.
