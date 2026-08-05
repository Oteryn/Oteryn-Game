# Oteryn v2 Architecture Analysis Gap Register

- Status: Active analysis coverage register
- Date: 2026-08-05
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Applies to: product vision, gameplay, client, content, creator tooling, operations, security, production process, community and long-term sustainability

## Purpose

Preserve the complete set of material areas identified during the 2026-08-05 architecture review that are not yet analysed to contract depth.

This document distinguishes three states:

- **ACCEPTED** — an ADR or accepted contract already freezes the relevant boundary;
- **PARTIALLY_ACCEPTED** — an ADR freezes a high-level direction, while a registered gate still must resolve measurable scope and detailed behavior;
- **REGISTERED_UNRESOLVED** — an existing gate names the area, but its detailed contract is not accepted;
- **NEWLY_IDENTIFIED** — the review found an important area that is not yet represented precisely enough by an existing gate.

This register does not select technologies, formulas, schemas, algorithms, service boundaries, monetization, art direction or final gameplay rules. It does not authorize implementation. Accepted decisions remain in ADRs and dedicated contracts.

The immediate programme action remains `FND-01` — Workspace, Dependency and Existing-Rust Migration Contract. Product and creative analysis may proceed in parallel when it does not redefine accepted foundation boundaries or delay the atomic client migration sequence.

## Already accepted foundation coverage

The following broad foundations are already accepted and therefore are not open for silent redesign:

- native Rust client/server/tooling direction and multichannel-first world model;
- one canonical destination repository/workspace after controlled migration;
- Platform Identity, Game Gateway and authoritative game-server admission boundary;
- PostgreSQL direction and Platform/game data ownership separation;
- project-owned world/content format direction and Oteryn Studio direction;
- Game Intelligence, analytics and durable security/economy audit separation;
- native E2E platform and evidence tiers;
- `protocol-canary` as reference-only migration evidence, excluded from target runtime;
- GameNode process identity, one logical writer per channel, capacity-evidence discipline and recovery baseline.
- Global Tibia parity as the initial product reference plus optional reference/evolved world profiles over one engine, client and `protocol-oteryn`, with default world-scoped gameplay-value isolation (ADR-0010).

The exact implementation contracts beneath these foundations remain gated where listed below.

# A. Product identity and creative direction

## 1. Product vision and core gameplay loop

- Coverage status: **PARTIALLY_ACCEPTED** by ADR-0010 and `PRODUCT_DIRECTION_BASELINE.md`
- Registered gate: `GAME-VISION-01` — Product Vision, Parity Scope and World Profile Contract
- Priority: parallel analysis with `FND-01`; must be accepted before broad gameplay and content production

Accepted direction: begin from Global Tibia behavioral parity; permit separate reference and evolved Oteryn worlds over one shared engine/client/protocol; keep gameplay value world-scoped by default.

Still unresolved:

- the primary player promise and the reasons to choose Oteryn over Tibia, classic OTS projects and modern MMORPGs;
- target audiences and the intended accessibility/complexity balance;
- core activity loop over one minute, one session, one week and long-term play;
- relative importance of exploration, combat, progression, economy, quests, PvP, social play and world events;
- sandbox, theme-park, living-world and persistent-world proportions;
- intended risk, loss, death, recovery and reward philosophy;
- expected progression duration, endgame structure and replayability;
- solo, party, guild and large-group expectations;
- exact relationship among current-Global, historical-classic and evolved profiles, including which profiles are actual launch products;
- explicit design pillars and anti-pillars used to reject technically attractive but product-incoherent features;
- observable product-success criteria beyond technical correctness.

Risk if omitted: the project can produce a strong generic engine without a coherent game identity, causing incompatible mechanics, content workflows and client UX to be designed independently.

## 2. Creative, visual, audio and readability direction

- Coverage status: **NEWLY_IDENTIFIED**
- Recommended candidate gate: `CREATIVE-DIRECTION-01`
- Priority: before renderer, asset pipeline and large-scale content production are frozen

Still unresolved:

- visual style, camera/perspective and world scale;
- sprite, model, animation, lighting and effect direction;
- combat readability and information hierarchy;
- environmental storytelling and biome identity;
- audio, music, ambience and feedback language;
- UI visual language and consistency with world presentation;
- minimum asset quality and performance budgets;
- accessibility-safe use of motion, colour, sound and effects;
- asset provenance, licensing and replacement strategy for all reference content;
- which presentation choices are shared by the game client and Oteryn Studio.

Risk if omitted: renderer, asset format, authoring tools and content budgets may be frozen before the intended presentation is known.

# B. Foundation contracts not yet accepted

## 3. Workspace, migration and dependency structure

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gates: `FND-01`, then `VSL-02`
- Priority: immediate next programme action

Still unresolved:

- exact inventory and classification of every existing Rust client subsystem;
- minimal initial workspace members with immediate consumers;
- legal dependency directions and forbidden edges;
- crate ownership of domain, identifiers, protocol, fixtures and tooling;
- toolchain, edition, resolver, lockfile, lint, feature and target policies;
- machine-readable workspace boundary enforcement;
- exact cutover SHA, provenance, path mapping, source freeze and rollback;
- atomic destination migration and later source-only closeout;
- proof that Canary code remains excluded from the production dependency graph.

## 4. Identifier, protocol, runtime and admission contracts

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gates: `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`

Still unresolved:

- complete identifier vocabulary, storage/wire forms, generation and validation;
- full `protocol-oteryn` v1 framing, messages, capabilities, errors and limits;
- command ordering, ticks, clocks, scheduling, bounded queues and overload behavior;
- snapshot, delta, reconciliation and reconnect semantics;
- session issuance, admission, character lease, generation fencing and duplicate-login behavior;
- deterministic execution and failure scenarios at transport/runtime boundaries.

## 5. Durable gameplay and content foundations

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gates: `DUR-01` through `DUR-04`

Still unresolved:

- durable identifier representation and migration;
- character persistence, transactions, revisioning, backup and restore;
- item/currency conservation and single-location invariants;
- native World Project, compiler, World Bundle and runtime loader details;
- scripting/capability boundary, deterministic execution and resource limits;
- content revision compatibility and safe migration of live durable state.

## 6. Analytics event contracts

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gates: `ANL-01` through `ANL-04`

Still unresolved:

- canonical event envelope and versioning;
- transactional outbox, ordering, idempotency and replay;
- operational versus best-effort versus durable audit pipelines;
- anomaly/economy/integrity projections and evidence quality;
- investigation access, retention, pseudonymization and human-review workflows.

# C. Core gameplay domain

## 7. Character lifecycle and progression

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `GAME-CHAR-01`
- Must precede: final durable character schema in `DUR-02`

Still unresolved:

- creation, naming, slots, world assignment, deletion, restore and retention;
- account-wide, world-wide and character-local state ownership;
- levels, experience, skills, attributes, capacity and derived statistics;
- vocation/class, promotion, mastery, specialization and respec boundaries;
- death, respawn, penalties, protection and recovery;
- offline progression/training, if any;
- rename, transfer and ruleset-version migrations;
- deterministic formulas and progression fixtures.

## 8. Item model, equipment and transformations

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `GAME-ITEM-01`
- Must precede: final item transaction model in `DUR-03`

Still unresolved:

- `ItemType` versus `ItemInstance` identity and lifecycle;
- stack, quantity, charges, durability, decay and expiration;
- equipment slots, requirements and exclusive combinations;
- modifiers, resistances, tiers, enchantments and upgrades;
- binding, uniqueness and ownership scope;
- containers, nesting, cycle prevention, weight and capacity;
- split, merge, transform, crafting and provenance continuity;
- content-revision migration and deterministic derived-stat ordering;
- integration with loot, trade, market, bank, depot, mail, rewards and houses.

## 9. Movement, collision and visibility

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `VSL-MOVE-01`; later refined by runtime and client contracts

Still unresolved:

- position and direction representation;
- orthogonal/diagonal movement ordering and timing;
- collision, pushing, floor transitions, stairs, ramps and teleports;
- simultaneous movement conflicts;
- view range and interest management;
- snapshot/delta visibility behavior;
- lag, reconciliation and bounded prediction;
- chunk/region boundary behavior during movement;
- deterministic legality and acceptance fixtures.

## 10. Combat, abilities, spells and conditions

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gates: `VSL-COMBAT-01`, `GAME-ABILITY-01`, `ALPHA-RULESET-01`

Still unresolved:

- common action model for weapons, spells, runes, active skills, passives and item-triggered abilities;
- resource costs, cast times, cooldowns and cooldown groups;
- targeting, line of sight, ranges, shapes and area effects;
- validation, interruption, cancellation and retry;
- damage/healing pipeline, ordering and formula versioning;
- conditions, buffs, debuffs, stacking, refresh, replacement, immunity and dispel;
- periodic effects, logout persistence and channel-transfer behavior;
- proc/trigger ordering, recursion limits and loop prevention;
- PvP, friendly fire, protection zones, skull/frag and ruleset boundaries;
- server authority versus client prediction/presentation.

## 11. Creature AI, spawn, NPC and pathfinding

- Coverage status: **REGISTERED_UNRESOLVED** with an NPC-specific precision gap
- Existing gate: `GAME-AI-01`; NPC content also intersects `ALPHA-CONTENT-01`

Still unresolved:

- state-machine, behavior-tree, utility or bounded-script representation;
- perception, aggro, threat, targeting, memory and leash;
- pathfinding ownership, budgets, cancellation and stale-result rejection;
- spawn definitions, population control, respawn and occupancy;
- channel-local versus world-shared encounter scope;
- boss phases and crash recovery;
- summon/pet ownership, commands, attribution and despawn;
- NPC movement, schedules, service behavior and conversation ownership;
- overload degradation that cannot block the authoritative channel writer.

## 12. World interaction and environmental mechanics

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `GAME-INTERACTION-01`

Still unresolved:

- doors, switches, levers, teleports, fields, traps and hazards;
- permissions for readable/writable objects;
- item use on self, target, tile, creature and item;
- movement-triggered and timer-triggered actions;
- local/channel/instance/world-shared state ownership;
- reset, persistence, recovery and content-revision migration;
- script capabilities, typed actions and abuse limits;
- deterministic authoring and tests.

## 13. Quests, narrative and dialogue runtime

- Coverage status: **NEWLY_IDENTIFIED precision gap**
- Existing broad gate: `ALPHA-CONTENT-01`
- Recommended candidate refinement: `CONTENT-QUEST-01`

Still unresolved:

- quest graph/state representation;
- personal, party, guild, world, channel and instance quest scope;
- branching, consequences, prerequisites and mutually exclusive paths;
- NPC dialogue, journal and player-visible progress;
- repeatability, schedules, resets and time windows;
- idempotent rewards and anti-duplication behavior;
- migration of active quests when content revisions change;
- dependency cycles and compatibility validation;
- deterministic headless tests for complex quest mechanics;
- graphical authoring, debugging and simulation support in Oteryn Studio.

Risk if left only inside a broad content gate: quests may become an unbounded scripting subsystem with weak state ownership and poor migration/testability.

## 14. Dynamic events, raids, bosses and living-world state

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `EXP-EVENTS-01`

Still unresolved:

- scheduling and activation ownership;
- channel-local versus world-shared uniqueness;
- cooldowns and anti-hopping;
- participation, contribution and reward eligibility;
- scaling with population and ruleset;
- conflict between overlapping events;
- persistent world changes and reset policy;
- crash recovery and reward reconciliation;
- use of `EncounterZone`, `RaidCell` and `RaidAnchor` geometry.

# D. Client and player experience

## 15. Native client architecture

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `ALPHA-CLIENT-01`

Still unresolved:

- renderer and scene boundaries;
- camera, animation, lighting, particles and effects;
- UI composition and state management;
- input, remapping and controller support;
- networking, prediction/reconciliation and loading states;
- asset packaging/streaming and revision compatibility;
- audio architecture;
- settings, persistence and account/device scope;
- crash reporting and diagnostics;
- supported operating systems, hardware and graphics backends;
- reusable low-level components shared with Oteryn Studio.

## 16. Localization, onboarding and accessibility

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `UX-I18N-A11Y-01`

Still unresolved:

- localization keys, authored-content translation and fallback locales;
- pluralization, formatting, font coverage and text expansion;
- keyboard, mouse, controller and remapping conflicts;
- UI scaling, contrast, colour-vision support and reduced motion;
- semantic/screen-reader feasibility and supported scope;
- tutorial, onboarding and contextual-help ownership;
- synchronization and persistence of accessibility settings;
- automated and manual test strategy.

# E. Economy, social systems and world topology

## 17. Economy, trade and value stability

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gates: `EXP-ECONOMY-01`, `GAME-ITEM-01`, `DUR-03`

Still unresolved:

- direct trade, offers, escrow, cancellation and partial completion;
- market, bank, depot, mail and offline delivery;
- currency/item sources and sinks;
- fees, taxes, inflation controls and economy-health targets;
- crafting and upgrade economy;
- concurrency, reconciliation, fraud and duplication resistance;
- rollback and support correction without violating conservation.

## 18. Party, guild, chat, friends and presence

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `EXP-SOCIAL-01`

Still unresolved:

- world/channel/cross-world scope;
- party membership, leadership and shared experience;
- guild identity, roles, membership and durable history;
- chat channels, moderation and offline messages;
- friends, blocks and presence privacy;
- consistency during reconnect, channel switch and world transfer;
- social abuse limits and audit needs.

## 19. Houses

- Coverage status: **REGISTERED_UNRESOLVED / DEFERRED**
- Existing gate: `EXP-HOUSES-01`

Still unresolved:

- final channel topology and access model;
- one-state-per-world simulation owner;
- rent, auctions, ownership and transfer;
- doors, guest lists, beds and persistence;
- crash recovery and anti-duplication;
- relationship to instances, channels and world lifecycle.

## 20. Instances, matchmaking, arenas and spectating

- Coverage status: **REGISTERED_UNRESOLVED / EXPANSION**
- Existing gate: `GAME-INSTANCES-01`

Still unresolved:

- instance creation, admission, lifecycle and recovery;
- matchmaking, queues, ratings and cancellation;
- checkpoints, lockouts and reward eligibility;
- arenas, tournaments and fair-start snapshots;
- spectators, replay/event streams and privacy;
- origin-channel return and duplicate-session prevention.

## 21. World creation, transfer, merge and archival

- Coverage status: **REGISTERED_UNRESOLVED / EXPANSION**
- Existing gate: `GAME-WORLD-LIFECYCLE-01`

Still unresolved:

- world creation, cloning, maintenance, closure and archive;
- ruleset/content upgrades;
- character transfer and collision handling;
- world merge reconciliation for economy, guilds, houses, market and rankings;
- staged migration, rollback and verification;
- backup/restore and channel-topology changes without changing logical world identity.

# F. Live operation, release and infrastructure

## 22. LiveOps and runtime configuration

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `PROD-LIVEOPS-01`

Still unresolved:

- static versus runtime configuration;
- feature flags, staged rollout, maintenance mode and kill switches;
- event/rate schedules and bounded overrides;
- ownership, validation, signatures and audit;
- propagation ordering, stale-state behavior and rollback;
- safe defaults and fail-closed behavior;
- separation from content bundles and executable deployment.

## 23. GM, support and moderation operations

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `OPS-GM-01`

Still unresolved:

- role and capability boundaries;
- mute, ban, kick, teleport, inspect and recovery actions;
- audited domain transactions for corrections;
- impersonation/account-access policy;
- case management, evidence, appeals and review;
- dual control for high-risk mutations;
- immutable administrative audit and emergency rollback.

## 24. Compatibility, release train, launcher and updater

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gates: `PROD-COMPAT-01`, `EXP-UPDATE-01`

Still unresolved:

- compatibility matrix among client, protocol, server, Gateway, content, assets, ruleset and database revisions;
- minimum/maximum clients and forced-update policy;
- mixed-version deployment and capability negotiation;
- release channels, staged rollout and rollback order;
- signed manifests, delta updates, CDN/mirror behavior and archive safety;
- immutable release evidence and cross-repository contract locks.

## 25. Performance, capacity and overload behavior

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `PERF-01`

Still unresolved:

- named reference hardware and deployment cells;
- players/channel, channels/GameNode and logical-world capacity;
- tick/scheduling, latency and queue-age objectives;
- CPU, memory, network and database budgets;
- representative hunting, crowd, raid, reconnect-storm, recovery and soak workloads;
- overload shedding/degradation policy;
- safety headroom and first-violated-objective reporting;
- infrastructure cost per player/channel/world.

The cost dimension must be included in `PERF-01` evidence or later refined by a dedicated FinOps decision; no separate implementation gate is accepted here.

## 26. Deployment, orchestration, recovery and observability

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gates: `OPS-CHANNEL-01`, `EXP-OPS-01`, `EXP-OBS-01`

Still unresolved:

- process/container packaging and external orchestrator authority;
- registration, health, readiness and capacity reporting;
- placement, hysteresis, draining and closure;
- checkpoint, replay, replacement and fresh-session recovery;
- RPO, RTO, reconnect grace and blast radius;
- environments, secrets, migrations, backup and disaster recovery;
- structured logs, metrics, traces, correlation and alerts;
- privacy and cardinality limits;
- operational cost and ownership.

# G. Security, privacy and trust

## 27. Client integrity and anti-cheat

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `SEC-CLIENT-01`

Still unresolved:

- threat model for official and modified clients;
- server-authoritative prevention versus client integrity signals;
- executable, library, asset and manifest verification;
- tamper, debugger and injection signals with platform limitations;
- botting, automation and impossible-behavior detection;
- command validation, rate limits and evidence correlation;
- privacy, false positives, sanctions and human review;
- update/rollback behavior for compromised clients.

## 28. Product privacy and data lifecycle

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `DATA-PRIVACY-01`

Still unresolved:

- data classification across Platform, gameplay, analytics, support and security;
- retention, deletion, anonymization and legal hold;
- account deletion consequences for characters/world records;
- export/access requests and provenance;
- consent and optional telemetry;
- pseudonymous analytics and re-identification controls;
- backup/audit exceptions and privacy-safe diagnostics.

## 29. Broader security and supply chain

- Coverage status: **REGISTERED_UNRESOLVED**
- Existing gate: `EXP-SECURITY-01`

Still unresolved:

- dependency and build-chain security;
- signing key and trust-anchor lifecycle;
- secret management and least privilege;
- replay and abuse controls across services;
- administrative and incident-response controls;
- vulnerability response, revocation and emergency release process.

# H. Creator tooling and content production

## 30. Oteryn Studio detailed architecture

- Coverage status: **NEWLY_IDENTIFIED precision gap under ADR-0005**
- Recommended candidate gate: `TOOL-CONTENT-01` — Creator Tooling and Content Production Pipeline

Still unresolved:

- editor UX and extensibility boundaries;
- map, area, subarea, encounter, quest, dialogue, AI and ability authoring;
- validation, diagnostics and live preview;
- deterministic simulation and headless test generation;
- content diff/review and merge-conflict handling;
- multi-author collaboration and ownership;
- hot reload safety and environment limits;
- asset/content revisioning, packaging, signing, publishing and rollback;
- performance/resource-cost estimation for scripts, encounters and assets;
- legacy import review and provenance reporting;
- separation between source authoring formats and canonical runtime bundles.

Risk if omitted: content throughput and maintainability become the project bottleneck even if the runtime is fast.

## 31. Game-production operating model

- Coverage status: **NEWLY_IDENTIFIED**
- Recommended candidate gate: `PROD-DEV-01`

Still unresolved:

- workflow among programmer, designer, writer, artist, audio creator and tester;
- definition of done for mechanics, quests, maps and events;
- review/approval roles and ownership of canonical content;
- content budgets, automated validators and quality bars;
- versioning of design documents and traceability to runtime content;
- seasonal/update planning and compatibility review;
- test fixture ownership and regression policy;
- deprecation and long-term maintenance of mechanics/content;
- release evidence for content-only changes;
- process for experimental content without contaminating production contracts.

Risk if omitted: technical CI can be mature while actual game production remains manual, inconsistent and difficult to scale.

# I. Business, community and ecosystem

## 32. Entitlements and commerce mechanics

- Coverage status: **REGISTERED_UNRESOLVED / DEFERRED**
- Existing gate: `PROD-ENTITLEMENTS-01`

Still unresolved:

- Platform/payment versus game-delivery ownership;
- entitlement identity, scope, expiry and revocation;
- idempotent purchase delivery, refunds and chargebacks;
- premium/account benefits versus character/world grants;
- real-money versus in-game economy separation;
- fraud, audit, support correction and failure behavior.

No monetization choice is implied.

## 33. Business and sustainability model

- Coverage status: **NEWLY_IDENTIFIED**
- Recommended candidate gate: `BUSINESS-MODEL-01`
- Status recommendation: `DEFERRED` until the owner wants to decide product funding and operation

Still unresolved:

- whether the project is a private game, official service, commercial product, open platform or mixed model;
- expected hosting/support/content-production budget;
- funding model and constraints on game design;
- official server policy and service commitments;
- relationship between monetization, fairness and progression;
- ownership/licensing of original and community-created content;
- long-term maintenance and shutdown/archival obligations.

## 34. Community governance and server ecosystem

- Coverage status: **NEWLY_IDENTIFIED**
- Recommended candidate gate: `COMMUNITY-GOV-01`

Still unresolved:

- official versus community-hosted server policy;
- code/content distribution and supported customization boundaries;
- rules, moderation, appeals and acceptable automation policy;
- community asset/plugin/content licensing and provenance;
- compatibility expectations for community deployments;
- telemetry/privacy boundaries for non-official servers;
- security and trust presentation to players;
- migration or federation expectations, if any.

## 35. External APIs, notifications and integrations

- Coverage status: **REGISTERED_UNRESOLVED / EXPANSION**
- Existing gate: `INTEGRATION-API-01`

Still unresolved:

- public/read-only and authenticated partner APIs;
- webhooks, retries, signatures and replay protection;
- rankings, status, notifications and offline delivery;
- rate limits, privacy, data minimization and deprecation;
- Discord/community integration and separation from authoritative mutation paths.

## 36. Modding and plugin ecosystem

- Coverage status: **REGISTERED_UNRESOLVED / DEFERRED**
- Existing gate: `MOD-ECOSYSTEM-01`

Still unresolved:

- trusted first-party versus untrusted community extensions;
- plugin API/ABI stability and capabilities;
- sandboxing, resource limits and deterministic execution;
- signing, provenance, distribution and revocation;
- client/server/content extension boundaries;
- multiplayer compatibility, anti-cheat and support scope;
- prevention of hidden protocol forks.

# Recommended analysis order

## Track 0 — immediate

1. `FND-01` — workspace, dependency and existing-client migration contract.
2. `GAME-VISION-01` — refine ADR-0010 into measurable parity scope, product pillars and launch strategy, in parallel without blocking workspace analysis.

## Track 1 — before durable gameplay schemas freeze

1. `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`.
2. `GAME-CHAR-01` before final `DUR-02`.
3. `GAME-ITEM-01` before final `DUR-03`.
4. `DUR-01` through `DUR-04` and `ANL-01` foundations.

## Track 2 — before the foundation vertical slice is fully specified

1. `VSL-MOVE-01`.
2. `VSL-COMBAT-01`.
3. `VSL-CONTENT-01`.
4. bounded minimal AI, interaction and client-visible acceptance.

## Track 3 — before Playable Alpha is declared complete

1. `GAME-ABILITY-01`, `GAME-AI-01`, `GAME-INTERACTION-01`.
2. `ALPHA-CLIENT-01`, `ALPHA-RULESET-01`, `ALPHA-CONTENT-01`.
3. quest/narrative precision contract and creator-tooling contract.
4. creative direction and production operating model.
5. compatibility, LiveOps, security, privacy, GM, accessibility, quality and performance gates.
6. `OPS-CHANNEL-01` where automatic production scaling/recovery is claimed.

## Track 4 — expansion or deferred product systems

- economy, social, houses, events, instances and world lifecycle;
- updater/distribution, broader operations and observability;
- entitlements, business model, community governance, external APIs and modding.

# Cross-cutting contract checklist

Every future dedicated contract derived from this register must define, where applicable:

- authoritative owner and exact state scope;
- durable versus runtime state;
- identity and revision/generation fencing;
- consistency, ordering, idempotency and concurrency behavior;
- failure, retry, recovery and rollback behavior;
- protocol and client-visible consequences;
- content/ruleset extension points;
- security, privacy, abuse and resource limits;
- deterministic test fixtures and observable E2E acceptance;
- migration and compatibility with existing content/data;
- operational diagnostics and support correction path;
- production/creator workflow and ownership;
- performance and infrastructure-cost implications.

# Non-decisions

This register does not decide:

- exact classes, skills, formulas, items, quests or balance values;
- scripting engine, AI framework or pathfinding algorithm;
- renderer, UI framework, asset format or supported platform list;
- anti-cheat vendor or invasive client technology;
- deployment topology, message broker or cloud provider;
- payment provider or monetization model;
- public modding or community-server support;
- final art style, audio style or narrative setting.

These require dedicated evidence and owner acceptance.

# Maintenance rule

When a dedicated gate is accepted:

1. link its ADR/contract from this register;
2. change the corresponding coverage status to **ACCEPTED** or remove duplicated question detail in favour of the canonical contract;
3. reconcile the global decision register and gameplay/product horizon;
4. preserve unresolved adjacent questions rather than silently absorbing them;
5. keep `FND-01` as the immediate next action until its own accepted contract explicitly advances the programme.
