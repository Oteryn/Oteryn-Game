# Oteryn v2 Gameplay and Product Architecture Horizon

- Status: Active open-decision horizon
- Date: 2026-08-11
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Applies to: gameplay domain, client/product experience, security, operations and release architecture

## Purpose

Ensure that core gameplay and product domains are not omitted while Oteryn v2 resolves foundation contracts in dependency order.

This document registers required future decisions. It does **not** accept implementation technologies, schemas, algorithms, service boundaries or final gameplay rules. Accepted decisions remain in ADRs and dedicated contracts.

The current product-sensitive progression starts from the accepted minimum `GAME-VISION-01` baseline. `GAME-CHAR-01` is the next product-sensitive architecture gate; `GAME-CHANNEL-01` and bounded `DUR-02` discovery may proceed in parallel within their existing boundaries. Runtime implementation remains separately unauthorized.

## Relationship to existing architecture

This horizon complements, and does not replace:

- accepted `GAME-VISION-01` minimum product direction plus its explicitly deferred/hard-gated downstream subjects;
- `FND-01` through `FND-04` for workspace, protocol, runtime and admission;
- `DUR-01` through `DUR-04` for durable identity, persistence, item transactions and content/scripting;
- `ANL-01` through `ANL-04` for events, analytics, integrity and investigation;
- `VSL-*` for the bounded foundation vertical slice;
- `ALPHA-CLIENT-01`, `ALPHA-RULESET-01`, `ALPHA-CONTENT-01`, `ALPHA-QUALITY-01` and `ALPHA-MILESTONE-01`;
- existing expansion gates for events, houses, social systems, economy, security, updater, operations, observability and advanced scaling.

A dedicated gate may refine an existing broader gate, but it may not silently redefine an accepted ADR or owner baseline boundary.

## Gameplay idea inventory and provenance

The following documents in `blakinio/canary` are registered as a read-only discovery inventory for future Oteryn gameplay and product analysis:

- `docs/ai-agent/OTS_FUTURE_GAMEPLAY_SYSTEMS.md` — the primary catalogue of future gameplay-system proposals;
- `docs/ai-agent/OTS_FUTURE_GAMEPLAY_SYSTEMS_CLASSIFICATION.md` — the classification index for the catalogue;
- `docs/agents/tasks/archive/CAN-20260721-ots-future-gameplay-roadmap.md` — the archived source task and provenance checkpoint;
- pull request `blakinio/canary#664` — the historical delivery record for the inventory.

These sources are not canonical Oteryn v2 architecture, an accepted product roadmap, an implementation backlog or authorization to copy Canary runtime design. Every proposal remains `PROPOSED / NOT ACCEPTED` until it is:

1. mapped to an existing gate in this horizon or registered through a new non-overlapping gate;
2. checked against accepted Oteryn v2 ADRs, product direction, legal provenance and shared-engine/profile boundaries;
3. refined into explicit ownership, state, protocol, persistence, security, migration and acceptance decisions where applicable;
4. accepted by the owner through the corresponding ADR or dedicated contract.

When an inventory proposal conflicts with accepted Oteryn v2 architecture, the accepted Oteryn v2 decision remains authoritative. Future reviews should reference the inventory rather than duplicate its complete contents in this repository.

## Decision discipline

For every gate below, a future contract must define:

- authoritative owner and scope;
- durable versus runtime state;
- consistency, ordering, idempotency and failure behavior;
- protocol and client-visible consequences;
- content/ruleset extension points;
- security, abuse, privacy and resource limits;
- deterministic acceptance scenarios;
- migration, rollout and rollback where applicable.

No gate authorizes speculative empty crates or services. Every implementation member requires an immediate consumer and observable acceptance.

# Product direction gate

## `GAME-VISION-01` — Product Vision, Parity Scope and World Profile Contract

- Status: **`ACCEPTED` for the minimum product-vision gate scope; implementation `NOT_STARTED`.**
- Canonical minimum source: `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` plus the seven earlier dedicated owner baselines.
- This minimum gate no longer blocks baseline-neutral product-sensitive architecture merely because the vision itself is open.
- Acceptance does not authorize gameplay/content/runtime implementation and does not close explicitly deferred/downstream subjects.

Accepted minimum product direction:

- first external evaluation is Reference-first;
- released Reference revisions are immutable while newer upstream evidence may be observed continuously and explicitly promoted into later revisions;
- preserve recognizable Tibia depth/readability/persistent-world identity, modern reliable native quality and explicit/versioned/measurable intentional Oteryn differences;
- first Evolved differentiation is reliability/UX-first rather than broad systemic gameplay redesign;
- PvP is a supported secondary pillar;
- ordinary progression is solo viable while coordinated party play is materially rewarded;
- Reference parity takes precedence over future-facing GAME-VISION gameplay preferences for actual Reference mechanics;
- accepted session loop is player goal -> preparation -> travel/access -> risk/activity -> secure committed progress/value -> recovery/restock/trade/reorganize -> next goal;
- accepted long-term horizons include character capability, equipment/wealth/resources, exploration/access/quest/encounter mastery, social/world relationships and increasingly difficult/rare/prestigious objectives;
- Reference economy uses mechanical source/sink parity rather than historical market-price/supply parity, with conservation before tuning, measurable provenance, semantic scarcity and no hidden macro tuning;
- accepted success categories cover Reference correctness, player interaction quality, progress/value trust, core-loop health, economy health and product/operational health; numeric thresholds remain milestone-owned.

Deliberately deferred or downstream-owned after minimum acceptance:

- exact Global Tibia patch/date/behavior baseline for the first Reference revision — **`DEFERRED WITH HARD GATE`** before broad Reference mechanics/content or final parity fixtures that require concrete target semantics and before the first external Reference evaluation/release contract;
- exact Reference revision naming scheme;
- exhaustive numbered pillars/anti-pillars formalization while the current accepted player-promise/product filters remain sufficient;
- exact death/progression/PvP/party formulas;
- exact economy rates, prices, drops, fees, sink values and scarcity thresholds;
- numeric product/KPI thresholds until their named milestone;
- exact first Evolved feature inventory beyond the accepted reliability/UX-first strategy;
- branding/public marketing wording;
- monetization/Premium/VIP economics;
- LiveOps cadence or automatic economy control.

Must preserve:

- one canonical engine, client and `protocol-oteryn`;
- explicit versioned product/ruleset/content profiles rather than forks;
- distinct `WorldId` values across profile families, with one inherited profile family for every channel of a logical world;
- default world-scoped character and economy isolation;
- no proprietary code, protocol or asset copying under a parity claim;
- fail-closed behavior when a downstream Reference decision cannot remain baseline-neutral: select the exact named Reference baseline instead of guessing.

# Core gameplay domain gates

## `GAME-CHAR-01` — Character Lifecycle and Progression

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- **Current role: next product-sensitive architecture gate.**
- Must be accepted before `DUR-02` freezes the durable character schema and before broad progression implementation.
- Consume `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md`, the Reference parity-precedence owner baseline, character-authority/session/lease contracts and current persistence invariants.
- May remain baseline-neutral where possible; any concrete Reference semantic that cannot be decided neutrally is blocked on the exact first Reference baseline rather than guessed.

Must decide:

- character creation, naming, world assignment, slots, deletion, restore and retention;
- account-wide, world-wide and character-local ownership of state;
- level, experience, skills, attributes, capacities and derived statistics;
- vocation/class, promotion, mastery, specialization and future respec boundaries;
- death, respawn, penalties, blessings/protection and recovery behavior;
- offline progression or training, if supported;
- world transfer, rename and migration semantics without duplicating state;
- revision/fencing interaction with Game Session and character lease;
- ruleset version changes and safe progression migrations;
- deterministic formulas and test fixtures where their owning Reference/Evolved rules are actually known.

Must preserve:

- one authoritative active character session;
- no client-authoritative progression;
- no schema finalization before progression ownership is explicit;
- Reference parity precedence over future-facing Evolved product preferences.

## `GAME-ITEM-01` — Item Model and Equipment Rules

- Status: `BLOCKS_DURABLE_GAMEPLAY`
- Must be accepted before `DUR-03` finalizes transfer invariants and before broad item/content import.

Must decide:

- `ItemType` versus `ItemInstance` identity and lifecycle;
- stackability, quantities, charges, durability, decay and expiration;
- equipment slots, requirements, two-handed and mutually exclusive rules;
- modifiers, resistances, bonuses, tiers, enchantments and upgrade extension points;
- binding, ownership restrictions, uniqueness and account/character/world scope;
- containers, nesting limits, weight/capacity and cycle prevention;
- transformations, split/merge, crafting inputs/outputs and provenance continuity;
- serialization, content revision compatibility and migration of changed definitions;
- deterministic derived-stat calculation and ordering;
- interaction with loot, trade, market, bank, depot, mail, rewards and houses.

Must preserve:

- `DUR-03` as the authority for conservation and single-location invariants;
- no item behavior encoded only in the client or wire format;
- explicit limits for nesting, stack size, modifiers and transformation chains;
- accepted GAME-VISION economy/scarcity principles, including no hidden Reference macro tuning.

## `GAME-ABILITY-01` — Ability, Spell and Condition Architecture

- Status: `REQUIRED_FOR_ALPHA`
- The foundation vertical slice may implement a bounded minimal combat contract first, but playable-alpha breadth requires this gate.

Must decide:

- common model for spells, weapon actions, runes, active skills, passive effects and item-triggered abilities;
- resource costs, cast times, cooldowns and cooldown groups;
- target selection, ranges, line of sight, shapes and area effects;
- validation, interruption, cancellation and retry semantics;
- damage/healing pipeline extension points and formula versioning;
- conditions, buffs, debuffs, stacking, refresh, replacement, immunity and dispel;
- periodic effects, timers, persistence across logout and channel transfer rules;
- proc/trigger ordering, recursion limits and loop prevention;
- client prediction/presentation without client authority;
- data/policy versus typed Rust domain logic boundaries;
- deterministic fixtures for formulas, ordering and edge cases.

Must preserve:

- one server-authoritative action result;
- no protocol fork per ruleset;
- bounded execution and explicit recursion/event limits.

## `GAME-AI-01` — Creature AI, Spawn and Pathfinding Architecture

- Status: `REQUIRED_FOR_ALPHA`
- Minimal deterministic creature behavior may support the vertical slice; production AI breadth requires this gate.

Must decide:

- creature, monster, summon, pet and NPC runtime ownership;
- perception, aggro, threat, targeting, memory and leash behavior;
- behavior representation: typed state machines, behavior trees, scripts or bounded composition;
- pathfinding ownership, budgets, cancellation, stale-result rejection and deterministic test mode;
- spawn definitions, population controllers, respawn timers and occupancy rules;
- channel-local versus world-shared spawn/event scope;
- boss phases, encounter state and crash recovery;
- summon ownership, command rights, experience/loot attribution and despawn;
- dynamic populations/ecology only through explicit bounded policies;
- overload degradation that cannot block the authoritative channel loop.

Must preserve:

- channel-local simulation unless a named world/instance owner exists;
- no unbounded pathfinding or script work on the channel writer;
- deterministic acceptance scenarios for aggro, pathing, spawn and recovery.

## `GAME-INTERACTION-01` — World Interaction and Environmental Mechanics

- Status: `REQUIRED_FOR_ALPHA`

Must decide:

- doors, switches, levers, teleports, fields, traps and environmental hazards;
- readable/writable objects and interaction permissions;
- item use on self, target, tile, creature and other items;
- movement-triggered and timer-triggered world actions;
- local, channel, instance and world-shared state ownership;
- reset, persistence, crash recovery and content-revision migration;
- script capability boundaries and typed domain actions;
- anti-abuse limits for interaction frequency and chained effects;
- deterministic authoring and test fixtures.

Must preserve:

- no global mutable world object available to scripts;
- no interaction may bypass item, lease, transaction or channel ownership invariants.

# Product, safety and operational gates

## `PROD-LIVEOPS-01` — Live Operations and Runtime Configuration

- Status: `REQUIRED_FOR_ALPHA`

Must decide:

- static configuration versus runtime-controlled configuration;
- feature flags, staged rollout, maintenance mode and emergency kill switches;
- event activation, rates, schedules and bounded overrides;
- configuration ownership, versioning, validation and signatures;
- audit trail, approval policy and least-privilege mutation;
- propagation ordering, stale configuration behavior and rollback;
- safe defaults and fail-closed behavior for security/economy-sensitive controls;
- separation from content bundles and executable code deployment.

Must preserve:

- no unaudited live mutation of authoritative rules;
- runtime flags cannot silently change protocol or durable schema compatibility;
- GAME-VISION acceptance does not authorize automatic economy/gameplay tuning.

## `PROD-COMPAT-01` — Release Compatibility and Version Train

- Status: `REQUIRED_FOR_ALPHA`

Must decide:

- compatibility matrix for client, `protocol-oteryn`, server, Platform/Gateway, content and asset revisions;
- minimum/maximum supported client versions and forced-update policy;
- capability negotiation versus hard incompatibility;
- release channels, staged rollout, canary populations and rollback order;
- database/content migration compatibility windows;
- maintenance windows and mixed-version behavior;
- signed release manifests and relationship to `EXP-UPDATE-01`;
- release train ownership and immutable cross-repository contract locks;
- exact evidence required before compatibility claims.

Must preserve:

- protocol, content and ruleset revisions remain distinct;
- no mutable PR head is a canonical compatibility dependency.

## `SEC-CLIENT-01` — Client Integrity and Anti-Cheat Boundary

- Status: `REQUIRED_FOR_ALPHA`

Must decide:

- server-authoritative prevention versus client-side integrity signals;
- executable, library, asset and manifest signature verification;
- tamper detection, debugger/injection signals and platform limitations;
- botting, automation and impossible-behavior detection boundaries;
- rate limits, command validation and evidence correlation;
- telemetry privacy, false positives and evidence quality;
- sanctions, escalation and human review integration with `OPS-GM-01`;
- update/rollback interaction and compromised-client assumptions;
- threat model for official and potentially modified clients.

Must preserve:

- client integrity signals are not authoritative gameplay truth;
- no autonomous punishment solely from an opaque anomaly score;
- secrets and trust anchors are not embedded as reusable server credentials.

## `DATA-PRIVACY-01` — Product Privacy and Data Lifecycle

- Status: `REQUIRED_FOR_ALPHA`

Must decide:

- data classification across Platform, gameplay, analytics, support and security evidence;
- controller/owner boundaries and cross-database references;
- retention, deletion, anonymization and legal-hold behavior;
- account deletion and character/world-data consequences;
- export/access requests and provenance of exported data;
- consent and optional telemetry controls where applicable;
- pseudonymous analytics identity and re-identification controls;
- minors/age-policy extension points if the product requires them;
- backup and audit-log deletion exceptions;
- privacy-safe test fixtures and production diagnostics.

Must preserve:

- ADR-0006 analytics privacy rules;
- authoritative security/audit evidence may have different retention from best-effort telemetry;
- no direct shared-table coupling between Platform and game ownership.

## `UX-I18N-A11Y-01` — Localization, Input, Onboarding and Accessibility

- Status: `REQUIRED_FOR_ALPHA`

Must decide:

- localization keys for UI and authored content;
- locale-independent domain logic and stable content identifiers;
- pluralization, formatting, fallback locales and font coverage;
- asset/text packaging and content revision compatibility;
- keyboard, mouse, controller and remapping architecture;
- UI scale, color-vision support, reduced motion and readable contrast;
- screen-reader/semantic UI feasibility and supported scope;
- tutorial, onboarding and contextual help ownership;
- accessibility settings synchronization and persistence;
- test strategy for layout expansion, missing translations and input conflicts.

Must preserve:

- translated strings never become protocol or domain identifiers;
- accessibility cannot depend on server-side trust or alter authoritative rules.

## `OPS-GM-01` — Support, Moderation and GM Operations

- Status: `REQUIRED_FOR_ALPHA`

Must decide:

- support, moderator, GM, security and administrator role boundaries;
- least-privilege command capabilities and environment restrictions;
- mute, ban, kick, teleport, inspect and recovery actions;
- item/character correction through audited domain transactions, never raw unsafe edits;
- impersonation and account-access prohibition/approval rules;
- case management, evidence attachment, appeals and review;
- dual control for high-risk economy or identity mutations;
- complete immutable audit of administrative actions;
- emergency controls and rollback;
- integration with Platform roles and `ANL-03` evidence without autonomous enforcement.

Must preserve:

- no hidden unaudited GM mutation path;
- stale administrators cannot bypass session, item or revision fences.

## `PROD-ENTITLEMENTS-01` — Entitlements, Premium and Commerce Boundary

- Status: `DEFERRED`
- Required before any paid entitlement, premium time, cosmetic purchase or store delivery is enabled.

Must decide:

- Platform/payment ownership versus authoritative game-delivery ownership;
- entitlement identity, scope, expiration and revocation;
- idempotent purchase delivery and retry behavior;
- refunds, chargebacks, fraud holds and reconciliation;
- premium/account benefits versus world/character grants;
- cosmetic ownership and content-version compatibility;
- separation of real-money records from in-game currency/economy;
- audit, privacy and support correction workflows;
- failure behavior when payment or Platform services are unavailable.

Must preserve:

- the client never grants entitlements;
- payment success does not bypass an idempotent authoritative delivery transaction;
- no monetization decision is implied by registering this gate or accepting the minimum GAME-VISION baseline.

# Broader gameplay and product expansion gates

## `GAME-META-01` — Collections, Achievements and Recurring Progression

- Status: `EXPANSION`

Covers future architecture for:

- achievements, bestiary, collections and account/character milestones;
- outfits, mounts, titles and cosmetic unlocks;
- daily/weekly tasks, streaks and recurring rewards;
- crafting, upgrades and enhancement systems not already covered by item fundamentals;
- offline training or account-wide progression extensions;
- rankings, seasons and reset/migration policies;
- reward idempotency and anti-hopping across channels;
- versioned criteria, retroactive evaluation and projection rebuilds.

## `GAME-INSTANCES-01` — Dungeons, Arenas, Matchmaking and Spectating

- Status: `EXPANSION`

Covers future architecture for:

- instance creation, admission, ownership, lifecycle and recovery;
- party/solo matchmaking, queues, rating and cancellation;
- dungeon checkpoints, lockouts and reward eligibility;
- arenas, tournaments, seasons and fair-start snapshots;
- spectators, replay/event streams and privacy;
- instance migration or failover only when explicitly supported;
- origin-channel return and duplicate-session prevention.

## `GAME-WORLD-LIFECYCLE-01` — World Lifecycle, Transfer and Merge

- Status: `EXPANSION`

Covers future architecture for:

- world creation, cloning, maintenance, closure and archival;
- ruleset/content revision upgrades;
- character transfer between worlds;
- world merge and identity collision handling;
- economy, guild, house, market and ranking reconciliation;
- staged migration, rollback, verification and player communication;
- backup/restore and disaster-recovery interaction;
- channel topology changes without changing logical world identity.

## `INTEGRATION-API-01` — External APIs, Notifications and Integrations

- Status: `EXPANSION`

Covers future architecture for:

- public/read-only APIs and authenticated partner APIs;
- webhooks, delivery retries, signatures and replay protection;
- mail, notification and offline-delivery presentation;
- ranking, status and world-information projections;
- rate limits, privacy, data minimization and abuse prevention;
- stable versioning and deprecation;
- separation from authoritative mutation paths;
- optional community/Discord or external-service integrations.

## `MOD-ECOSYSTEM-01` — Modding and Plugin Ecosystem

- Status: `DEFERRED`

Covers future architecture for:

- trusted first-party extensions versus untrusted community packages;
- plugin ABI/API stability and capability permissions;
- sandboxing, resource limits and deterministic execution;
- signing, provenance, distribution and revocation;
- client-only versus server/content extensions;
- multiplayer compatibility and anti-cheat implications;
- support boundaries and crash isolation;
- prevention of plugins becoming hidden protocol forks.

No public mod ecosystem is implied until this gate is explicitly accepted.

# Dependency and ordering rules

1. `GAME-CHAR-01` is the next product-sensitive architecture gate and must precede the final durable character schema in `DUR-02`.
2. `GAME-ITEM-01` must precede the final item transaction model in `DUR-03`; `DUR-03` remains the conservation authority.
3. The foundation vertical slice may use bounded minimal movement/combat/creature/interaction contracts, but Playable Alpha requires `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01`.
4. `PROD-COMPAT-01` must precede production release-train and updater compatibility claims.
5. `SEC-CLIENT-01`, `DATA-PRIVACY-01`, `UX-I18N-A11Y-01`, `OPS-GM-01` and `PROD-LIVEOPS-01` are required before Playable Alpha is declared operationally complete.
6. `PROD-ENTITLEMENTS-01` remains deferred unless the owner explicitly chooses monetization or paid entitlements.
7. Expansion gates do not block foundation work when their extension points and invariants remain safe.
8. Existing gates for social, economy, houses, events, updater, operations, observability and scaling remain authoritative for their named scopes.
9. Any future implementation package must update this horizon, the global register and the corresponding dedicated contract status.
10. Accepted `GAME-VISION-01` must preserve ADR-0010 and may not turn Reference/Evolved profiles into protocol, engine, client or repository forks; downstream Reference semantics that cannot remain baseline-neutral are hard-blocked on the exact named first Reference baseline rather than guessed.
11. `GAME-CHANNEL-01` and bounded `DUR-02` discovery may proceed in parallel with `GAME-CHAR-01` only with explicit path/contract ownership and without pre-accepting their unresolved semantics.

# Explicitly not decided here

This horizon does not select:

- the exact first Global Tibia Reference patch/date/behavior baseline;
- gameplay formulas or balance values;
- exact character classes, skills or item systems;
- exact economy rates/prices/drop tables/scarcity thresholds;
- numeric alpha/release KPI thresholds;
- exact first Evolved feature inventory beyond the accepted reliability/UX-first strategy;
- scripting engine, AI framework or pathfinding algorithm;
- anti-cheat vendor or invasive client technology;
- payment provider or monetization model;
- final supported platforms/locales/accessibility scope;
- deployment topology, messaging technology or service decomposition;
- LiveOps cadence or automatic economy-control policy;
- modding support.

Those choices require dedicated evidence, contracts and owner acceptance at the appropriate gate.
