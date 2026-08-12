# Oteryn v2 Gameplay and Product Architecture Horizon

- Status: Active open-decision horizon
- Date: 2026-08-13
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Applies to: gameplay domain, client/product experience, security, operations and release architecture

## Purpose

Ensure that core gameplay and product domains are not omitted while Oteryn v2 resolves foundation contracts in dependency order.

This document registers required future decisions. It does **not** accept implementation technologies, schemas, algorithms, service boundaries or final gameplay rules. Accepted decisions remain in ADRs and dedicated contracts.

The current product-sensitive progression starts from the accepted minimum `GAME-VISION-01` baseline, the accepted immutable first Reference target, the owner-accepted complete semantic `GAME-CHAR-01` architecture, the owner-accepted whole `DUR-02 — Persistence v1` architecture, the accepted/lifecycle-closed `GAME-ITEM-01` item semantic architecture, the accepted/lifecycle-closed `DUR-03` item/currency/value transaction and anti-duplication architecture, the accepted/lifecycle-closed `GAME-CHANNEL-01` multichannel product/lifecycle policy and the accepted/lifecycle-closed `DUR-04` content/world/scripting architecture. The accepted Character persistence partial baseline remains a binding sub-baseline. Common server/persistence/item/content implementation may proceed only under separate explicit implementation authority. Under the owner-accepted programme ordering, `SIM-DETERMINISM-01` is now the selected next bounded paper-only architecture gate; Reference evidence/parity tooling remains separately ownable parallel paper-only work. Runtime implementation remains separately unauthorized until the owner explicitly grants implementation authority.

## Relationship to existing architecture

This horizon complements, and does not replace:

- accepted `GAME-VISION-01` minimum product direction plus its explicitly deferred/downstream subjects;
- `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md` for the accepted immutable first Reference behavior cut and evidence/provenance model;
- `GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` for binding baseline-neutral Character ownership/lifecycle/revision/migration safety;
- `GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md` for the owner-accepted Reference-sensitive semantic closure and hard parity-gate discipline;
- `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md` for accepted item definition/instance semantics, equipment/container legality, definition compatibility/migration and item-domain boundaries;
- `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` for accepted durable item/currency/value location, identity-transition, conservation, idempotency, runtime↔durable handoff, custody, audit and restore-integrity semantics;
- `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` for accepted channel identity/selection/queue/co-location, durable anti-hopping, value-source multiplicity, qualitative lifecycle, PvP/community and same-Channel recovery product semantics;
- `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` for accepted semantic package/content identity, deterministic locked compilation, immutable bundle staging/activation/migration, bounded loading/provenance and authoritative Component Model/WIT scripting capability/determinism boundaries;
- `DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` for the binding profile-neutral Character persistence sub-scope;
- `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` for the owner-accepted whole Persistence-v1 architecture and exhaustive historical-scope reconciliation;
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

- Status: **`ACCEPTED` for the minimum product-vision gate scope and the first Reference target cut; implementation `NOT_STARTED`.**
- Canonical minimum source: `GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` plus the seven earlier dedicated owner baselines.
- Canonical first Reference target source: `GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md`.
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
- accepted success categories cover Reference correctness, player interaction quality, progress/value trust, core-loop health, economy health and product/operational health; numeric thresholds remain milestone-owned;
- the first Reference target is Global Tibia production-observable behavior after the **2026-07-28 server-save/maintenance change boundary**;
- that target is immutable; later Global changes require explicit later Reference revision promotion;
- target selection is separate from evidence completeness: material behaviors remain `PROVEN`, `OBSERVED`, `DERIVED`, `UNKNOWN`, `CONFLICT` or `DECLARED_DIFFERENCE` until evidence resolves them;
- official public evidence is primary but not assumed exhaustive, controlled black-box observation is admissible with provenance, community sources are corroborative/discovery inputs, and OTS implementations are hypothesis/inventory inputs only.

Deliberately deferred or downstream-owned after these acceptances:

- exact Reference revision naming scheme;
- exhaustive numbered pillars/anti-pillars formalization while the current accepted player-promise/product filters remain sufficient;
- exact death/progression/PvP/party formulas until their owning Reference/Evolved gates establish them;
- exact economy rates, prices, drops, fees, sink values and scarcity thresholds until evidenced/accepted under their owning gates;
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
- the accepted 2026-07-28 target as the default first Reference cut across character/item/combat/content/economy parity work unless an explicit scoped owner decision or later Reference revision supersedes it;
- no assumption that patch-note/search absence proves no upstream change;
- fail-closed behavior for `UNKNOWN` or conflicting Reference semantics rather than guessing from current Global, OTS code or implementation convenience.

# Core gameplay domain gates

## `GAME-CHAR-01` — Character Lifecycle and Progression

- Overall `DecisionStatus`: **`ACCEPTED`**; owner-accepted Stage A + Stage B semantic architecture is binding.
- Delivery status: `LIFECYCLE_CLOSED`.
- Implementation: **`NOT_STARTED`**; runtime authority **`NONE`**.
- Canonical sources: `GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md` and `GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md`.
- Architecture acceptance is not complete Reference parity and does not imply all-profile persistence completeness.

Accepted Stage A baseline-neutral semantics include:

- a bounded Character aggregate/domain that owns character lifecycle/current owner/current world/name/progression/build/revision semantics while item/economy/social/house/market/session authorities remain separate;
- semantic lifecycle `ACTIVE -> DELETION_SCHEDULED -> RETIRED`, with restore/cancel only before terminal retirement and CharacterId never reused;
- atomic idempotent creation with authoritative name reservation and versioned starter/ruleset context;
- a Character-state revision/fence distinct from GameSession/CharacterLease/connection/runtime ownership generations;
- conservative first-generation quiescence for terminal retirement, world transfer and account transfer: actor `ABSENT` and no current playable CharacterLease before commit;
- separation of authoritative progression facts from derived values, without assuming all progression is monotonic;
- an idempotent/versioned Character-owned death-consequence boundary while exact death mechanics and item effects remain with their owning later contracts;
- explicit ruleset/profile migration with no silent reinterpretation of incompatible persisted state;
- vocation/build state as versioned ruleset-owned character state rather than an engine/protocol fork;
- offline progression only as an explicit ruleset capability;
- Character Authority as final naming/quota arbiter;
- world/account transfer as an architecture capability rather than a first-launch promise.

Accepted Stage B Reference-sensitive semantic closure adds:

- one logical global Character Authority name namespace with a versioned canonical-comparison policy; exact normalization/repertoire/recycling remain per-behavior parity gates;
- Character Authority lifecycle/quota policy with **25 active characters** accepted for the first Reference target, while unresolved deletion grace/total quota/name-hold values remain versioned `PARITY_PENDING_EVIDENCE` policy rather than defaults;
- versioned profile/ruleset/content/starter-template creation context and explicit support for the Reference pre-vocation/unselected period; exact starter content remains content/parity-owned;
- first Reference vocation families/promoted forms: Druid/Elder Druid, Knight/Elite Knight, Monk/Exalted Monk, Paladin/Royal Paladin and Sorcerer/Master Sorcerer, plus pre-vocation state;
- first Reference skill vocabulary: Fist Fighting, Club Fighting, Sword Fighting, Axe Fighting, Distance Fighting, Shielding, Fishing and Magic Level;
- Character-owned authoritative progression facts with formula-neutral durable representation; exact XP/skill/derived-stat arithmetic belongs to ruleset/SIM parity gates unless it constrains identity/ownership/atomicity/irreversible representation/migration;
- Character-owned promotion achievement/build state while current benefit activation may consume Platform-owned entitlement input; unresolved fee/lapse behavior remains parity-gated;
- profile-scoped death/protection semantics; item/corpse/value conservation remains combat + GAME-ITEM/DUR-03 authority and PvP-specific persistent state remains gated by the owning profile/world policy;
- Character-owned offline-training activation/counter/pool semantics for the first Reference target: >=10 minutes offline before gain, maximum 12 hours effective continuous training, 1:1 pool drain while training, 1:1 refill online or offline without training, and reactivation after depletion/refill; exact effectiveness remains ruleset/SIM-gated;
- target-required character-specific progression scope including Weapon Proficiency Progress, charms/charm points/charm expansion, Hunting Task Points, permanent Hunting Task slots, permanent Prey slots, Wheel/Promotion Point state and Animus Mastery within the accepted evidence boundary; character-specific does not mean one giant aggregate/table;
- stable versioned definition identities and explicit migration for incompatible progression definitions;
- a binding fail-closed rule for every unresolved target mechanic.

Binding parity rule:

```text
UNKNOWN / CONFLICT target rule
-> may have a safe versioned semantic/policy envelope
-> may NOT be filled by current Global, Canary, crystalserver, another OTS or implementation convenience
-> may NOT be enabled as claimed Reference behavior
-> may NOT be PARITY_CONFIRMED
-> must be evidenced, explicitly owner-accepted as DECLARED_DIFFERENCE, or excluded from the exercised release scope
```

For every external Reference milestone, each exercised Character behavior must therefore be `PARITY_CONFIRMED` or an explicit owner-accepted `DECLARED_DIFFERENCE`.

### DUR-02 persistence consequence

`DUR-02_PROFILE_NEUTRAL_CHARACTER_PERSISTENCE_OWNER_BASELINE.md` binds the profile-neutral persistence envelope for accepted Character semantics: normalized current state with one CharacterRevision; account portfolio guards; domain-canonical global name registry; typed Character/profile child relations; separate AccountPresenceClaim/CharacterLease/GameSession/ControlLoss authorities; atomic fresh admission and reconnect/recovery; durable receipts; explicit locking/isolation proof; audit/publication atomicity; staged migration; and fail-closed no-authority-resurrection after restore.

`DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` now closes the whole common Persistence-v1 architecture around that Character sub-baseline. It accepts migration authority/artifacts, common transaction correctness rules, crash-safe ANL-compatible outbox/journal substrate, durable-ack versus runtime checkpoint versus disaster-RPO separation, PITR/restore safety and common schema evolution.

Whole DUR-02 acceptance does **not** make one Character persistence package complete for all future PvP/world profiles and does not resolve any remaining Reference arithmetic/content/profile evidence gap. It also does not authorize SQL DDL, migrations or runtime implementation.

### Deliberately downstream / parity-pending

Still unresolved under their owning gates:

- exact naming canonicalization/repertoire/29-letter continuity and deleted-name release algorithm;
- exact 60-day deletion-grace, total-30 quota and related undelete/name-hold target continuity;
- exact starter inventory/equipment/stats/home/route content and remaining creation-validation details;
- exact XP/level/skill/derived-stat arithmetic and rounding;
- exact promotion fee and entitlement-lapse target behavior;
- first Reference PvP/world type and complete Twist/fair-fight/skull/Death Redemption edge matrix;
- offline-training effectiveness coefficients/advancement arithmetic;
- exact formulas/content definitions and physical decomposition/migration for modern character-specific progression;
- profile-specific persistence extensions.

These are hard parity/implementation or architecture gates where exercised; persistence may not invent their gameplay semantics.

## `GAME-ITEM-01` — Item Model and Equipment Rules

- DecisionStatus: **`ACCEPTED`**.
- DeliveryStatus: **`LIFECYCLE_CLOSED`**.
- ImplementationStatus: **`NOT_STARTED`**; runtime/DDL/production authority **`NONE`**.
- Canonical sources: `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_ANALYSIS.md` and `GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md`; delivery PR #205 final head `53d0189a114c99b4e7d44ca8d0db7a6bf5f3ea1a`, squash merge `5c502d24557621efc798def87b68f137ba23fad8`.
- Reference-sensitive item semantics use the accepted 2026-07-28 target unless explicitly superseded/scoped; exact unevidenced values/formulas/edge cases remain `PARITY_PENDING_EVIDENCE` and fail closed.

Accepted semantic closure includes:

- `ItemType` as a stable namespaced immutable/versioned authored semantic definition;
- `ItemInstance` as one concrete mutable gameplay lifecycle using DUR-01 `ItemInstanceId`;
- `StaticItemPlacement` as authored world/content placement that is not automatically a durable ItemInstance;
- typed bounded capability composition for stack quantity, charges, durability, temporal/decay state, equipment, containers, binding/transfer restrictions and upgrade/modifier state;
- rejection of arbitrary authoritative JSON/EAV/free-form script state;
- server-authoritative equipment patterns with atomic multi-slot/exclusion occupancy claims and typed requirements;
- server-authoritative bounded acyclic container legality, with missing absolute security/resource ceilings treated as an implementation blocker rather than unlimited;
- deterministic item modifier contribution ordering requirements while SIM/ruleset owners retain exact numeric arithmetic/rounding;
- explicit item-definition revision compatibility classes and deterministic migration/backfill requirements; no silent reinterpretation under an unchanged stable ItemTypeKey;
- explicit separation of world scope, binding/restrictions, current location, authorization and presentation ownership;
- physical item currency distinguished from non-item ledger/value state;
- `PROD-ENTITLEMENTS-01` remaining separately gated: Platform-owned entitlement facts may be consumed only through a separately accepted Oteryn-v2 consumer/enforcement contract.

Preserved downstream authority:

- accepted `DUR-03` owns create/destroy/split/merge/transform ItemInstanceId transitions, atomic semantic location, idempotency, stale-writer/session rejection, retry/crash/partial-failure recovery and item/currency/value conservation/provenance/anti-duplication;
- DUR-02 owns the common persistence/transaction substrate rather than item semantics;
- DUR-04 owns concrete item source/bundle/compiler/scripting choices;
- ANL-01 owns event/audit identity and durable evidence semantics;
- client/wire presentation never becomes item legality authority.

GAME-ITEM and DUR-03 architecture acceptance together close the paper-only item semantic/conservation prerequisites, but do not authorize any durable item/currency/value runtime mutation.

## `DUR-03` — Item Transaction and Anti-Duplication Invariants

- DecisionStatus: **`ACCEPTED`**.
- DeliveryStatus: **`LIFECYCLE_CLOSED`**.
- ImplementationStatus: **`NOT_STARTED`**; runtime/client/DDL/migration/production authority **`NONE`**.
- Canonical sources: `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_ANALYSIS.md` and `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md`; delivery PR #207 exact final head `a1d949362e219373a5d314c0e9ddf8de110362dd`, squash merge `63380bcba469027e90677aaf4db571fa941be2f4`.

Accepted semantic closure includes:

- exactly one typed immediate semantic location per live durable ItemInstance, with runtime projection/checkpoint and durable recovery state forbidden from becoming peer authorities;
- separation of FND-03 runtime simulation ownership from durable recoverability;
- non-blocking runtime reservation/PREPARE -> asynchronous one-game-DB durable linearization -> normalized runtime completion/reconciliation for ground/instance ↔ durable Character/value operations;
- crash/recovery rule that committed DUR-03 receipt/location state is reconciled before an older runtime checkpoint/ground projection can authorize another interaction;
- durable-drop recoverability and durable-pickup same-ItemInstance movement;
- an explicitly downstream-owned alternative where runtime-only loot becomes durable only through one idempotent materialization `MINT` occurrence;
- transaction-scoped fresh ItemInstanceIds for new concrete lifecycles, stable through physical retry and never reassigned to another logical transaction;
- explicit split/merge survivor/retirement and internal Oteryn transform `PRESERVE_INSTANCE`/`REPLACE_INSTANCE` lifecycle policy, without pretending Global exposes Oteryn UUIDs;
- explicit conservation classes `TRANSFER`, `SPLIT_MERGE_QUANTITY`, `STATE_MUTATION`, `MINT`, `BURN`, `TRANSFORM`, `CONVERSION`, with exact units/asset lines and complete source/sink/input/output lineage rather than market-price equality;
- FND-02 CommandRef, ANL OperationId/TransactionId separation, durable receipts and exact known-abort versus ambiguous-commit rules;
- current GameSession/CharacterLease/runtime ownership-generation fencing without treating ItemInstanceId, binding, NodeId or an old connection generation as authorization;
- one current game-owned PostgreSQL transaction as the v1 durable value linearization boundary, explicitly excluding implicit Platform/game distributed 2PC and mirrored cross-database authority;
- typed custody for multi-transaction workflows with independently conservation-safe steps and explicit compensation;
- bounded mandatory ANL-compatible security/value audit, allowing aggregation and preserving downstream concrete event/resource-limit registration;
- fail-closed restore reconciliation for item identity/location/custody, receipts/source causes, required audit sets, non-item assets and recovery fences;
- direct cross-world value transfer forbidden by default, including attempts to disguise it as burn+mint;
- read-only Game Intelligence investigation, never automatic authoritative repair.

Preserved downstream authority:

- exact loot generation/materialization timing belongs to combat/loot/content owners;
- trade consent/lifecycle, market pricing/order-book state, bank/depot/mail policy, rewards eligibility, houses, crafting/ruleset formulas and entitlement activation remain separately owned;
- exact unevidenced Reference source/sink/transform/loot/business behavior remains `PARITY_PENDING_EVIDENCE`;
- physical SQL schema/index/lock details, concrete Rust APIs, ANL event IDs/payloads and numeric transaction/resource ceilings remain implementation/evidence work.

DUR-03 architecture acceptance is not runtime proof. A future implementation must separately prove the exact concurrency, crash-window, stale-owner, idempotency, receipt, audit, resource-bound and restore scenarios named by the contract.

## `GAME-CHANNEL-01` — Multichannel Product Policy

- DecisionStatus: **`ACCEPTED`**.
- DeliveryStatus: **`LIFECYCLE_CLOSED`**.
- ImplementationStatus: **`NOT_STARTED`**; runtime/client/Platform/DDL/production authority **`NONE`**.
- Canonical sources: `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_ANALYSIS.md` and `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md`; delivery PR #209 exact final head `ca1112191ede7d316c874189f3053ad7f8247579`, squash merge `54dddbcfcb64baaf66e6cfd9fcd2cc3e1b4f04f1`.

Accepted semantic closure includes:

- canonical `ChannelRef=WorldId+ChannelId` separate from presentation labels/ordinals;
- current eligible-set plus recommendation/explicit selection with FND-04 retaining final target-bound admission;
- bounded pre-admission queue semantics and privacy-bounded party/friend co-location without party-owned Channel authority;
- fresh-session Channel switching with durable `CharacterId+WorldId` anti-hopping/prior-Channel state that survives logout/relog/restart;
- recovery-safe different-Channel admission + guard advancement and no mandatory new `ChannelSwitchId`;
- fail-closed explicit value-source multiplicity classes `CHANNEL_LOCAL_REPEATABLE`, `CHANNEL_LOCAL_SHARED_ELIGIBILITY`, `WORLD_SCOPED_UNIQUE`, `EXPLICIT_EVENT_POLICY_REQUIRED` with simulation scope separate from durable eligibility scope;
- qualitative lifecycle trigger vocabulary `DEMAND_PRESSURE`, `RECOVERY_PRESSURE`, `LOW_LOAD_CONSOLIDATION_CANDIDATE`, `CHANNEL_UNHEALTHY`;
- product predicates for legitimate new public Channel creation, low-load drain, drain abort/hold and terminal retirement while PERF/OPS retain numeric windows/thresholds/headroom/hysteresis/timers and orchestration;
- same-ChannelId recovery first, no silent relocation into another simulation, durable PvP/reward/value consequences and one-World economy/community identity.

GAME-CHANNEL acceptance does not accept downstream PvP/boss/reward/economy/social/house/instance business rules and does not authorize runtime implementation.

## `DUR-04` — Content, World Detail and Scripting Contract

- DecisionStatus: **`ACCEPTED`**.
- DeliveryStatus: **`LIFECYCLE_CLOSED`** after lifecycle closeout PR #213 merges.
- ImplementationStatus: **`NOT_STARTED`**; runtime/client/compiler/loader/Studio/WIT-host/DDL/content-import/production authority **`NONE`**.
- Canonical sources: `DUR-04_CONTENT_WORLD_AND_SCRIPTING_ANALYSIS.md` and `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`; delivery PR #212 exact final head `77e68ffb9e9e0e31ca751d42ff5f7c03466b2b23`, squash merge `568236c33cd23da017bca1dbd1ed98afc8da71f4`.

Accepted semantic closure includes:

- typed semantic content graph independent from final YAML/RON/JSON5/custom serializer choice;
- stable namespaced `PackageKey`, immutable `PackageRevision`, stable `ContentKey`, exact immutable Content Lock and revision-scoped compact IDs;
- distinct content/map/ruleset/world-policy/compiler/script-execution-profile/artifact identities;
- deterministic source/import -> typed model -> validation -> dependency/alias resolution -> normalization -> lowering -> client/server allowlist projection -> immutable artifact -> isolated staging -> explicit activation;
- bounded fail-closed loader with checked allocation/decompression/integrity/version/dependency/index/semantic validation and no partial authoritative publication;
- explicit activation/rollback and durable migration classes preventing silent persisted-state reinterpretation;
- exact external-source provenance/conversion dispositions with LIR quarantined to importer boundary;
- target WebAssembly Component Model + project-owned versioned WIT capability ABI, with Wasmtime only an implementation candidate;
- no ambient script filesystem/network/process/environment/SQL/global-Game authority;
- snapshot-bound authoritative reads and proposal-only extension-state/domain mutations;
- authority-scoped `ProposedActionPlan`; scripts cannot manufacture cross-owner/distributed atomicity;
- deterministic logical time, invocation-local RNG, stable query ordering, deterministic numeric/NaN policy and bounded fuel/resource semantics under `script_execution_profile_revision`;
- typed/versioned bounded persistent extension state, never VM-memory persistence;
- explicit GAME-CHANNEL multiplicity/eligibility classification for relevant value-producing sources;
- mandatory Resource Limits Registry entries before implementation acceptance and a reversible non-canonical physical-format spike before final serializer/container/chunk/floor/compression choices.

DUR-04 deliberately leaves exact physical source/bundle encoding, chunk/floor packing, compression, exact WIT function inventory, exact Wasmtime version/features and numeric limits downstream. Acceptance does not authorize executable content infrastructure or broad import/production activation.

## `SIM-DETERMINISM-01` — Authoritative Simulation Determinism Contract

- DecisionStatus: `PROPOSED`.
- DeliveryStatus: `PLANNED`.
- ImplementationStatus: `NOT_STARTED`.
- Selected as the next bounded paper-only architecture gate after DUR-04 lifecycle closeout.

Must decide:

- authoritative numeric representation boundaries and where integer/fixed-point/decimal/floating semantics are permitted;
- rounding, overflow/underflow, saturation/error and invalid numeric-state behavior;
- deterministic RNG identity, seed derivation, stream/substream ownership and consumption ordering;
- simulation logical time, tick/order and simultaneous-event tie-break semantics;
- external nondeterminism capture required for deterministic replay;
- replay input contract, checkpoint/revision requirements and formula/ruleset compatibility;
- state hashing and first-divergence localization without inventing one false global order;
- supported-target/cross-platform determinism expectations and comparison evidence;
- boundary between core simulation determinism and DUR-04 `script_execution_profile_revision`;
- deterministic fixtures required before broad combat/AI formula freeze or unresolved Character arithmetic can be `PARITY_CONFIRMED`.

Must preserve:

- FND-03 one-writer/order/generation authority;
- GAME-VISION Reference evidence discipline and fail-closed unknowns;
- GAME-CHAR formula-neutral durable facts until formula ownership/evidence is accepted;
- GAME-ITEM/DUR-03 value integrity and exact conservation;
- DUR-04 script execution profile as a nested execution-environment identity rather than a competing core-simulation authority;
- no runtime/combat/AI/script implementation authority in this paper-only gate.

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

1. `GAME-CHAR-01` semantic architecture, Character persistence partial baseline, whole `DUR-02 — Persistence v1`, `GAME-ITEM-01`, `DUR-03`, `GAME-CHANNEL-01` and `DUR-04` are accepted/lifecycle-closed architecture after their recorded closeouts. Common server/persistence, item/value, multichannel and content runtime implementation still require separate explicit implementation authority and exact implementation evidence.
2. GAME-ITEM + DUR-03 close the paper-only native item semantic, location, identity-transition, conservation, idempotency and anti-duplication prerequisites. Downstream trade/market/bank/depot/mail/reward/house/crafting/entitlement policy remains separately unaccepted, and runtime mutation is not authorized by architecture acceptance.
3. `DUR-04` is accepted/lifecycle-closed architecture. The selected next bounded paper-only gate is `SIM-DETERMINISM-01`, while Reference evidence/parity tooling remains separately ownable parallel work under independent path ownership.
4. The foundation vertical slice may use bounded minimal movement/combat/creature/interaction contracts, but Playable Alpha requires `GAME-ABILITY-01`, `GAME-AI-01` and `GAME-INTERACTION-01`.
5. `PROD-COMPAT-01` must precede production release-train and updater compatibility claims.
6. `SEC-CLIENT-01`, `DATA-PRIVACY-01`, `UX-I18N-A11Y-01`, `OPS-GM-01` and `PROD-LIVEOPS-01` are required before Playable Alpha is declared operationally complete.
7. `PROD-ENTITLEMENTS-01` remains deferred unless the owner explicitly chooses monetization or paid entitlements.
8. Expansion gates do not block foundation work when their extension points and invariants remain safe.
9. Existing gates for social, economy, houses, events, updater, operations, observability and scaling remain authoritative for their named scopes.
10. Any future implementation package must update this horizon, the global register and the corresponding dedicated contract status.
11. Accepted `GAME-VISION-01` must preserve ADR-0010 and may not turn Reference/Evolved profiles into protocol, engine, client or repository forks; the accepted 2026-07-28 first Reference target is binding across downstream Reference-sensitive work unless explicitly superseded/scoped, and evidence gaps remain fail-closed rather than guessed.
12. `SIM-DETERMINISM-01` is the one selected next programme-order paper-only architecture action after DUR-04 lifecycle closeout. Reference evidence/parity tooling may proceed in parallel only with explicit path/contract ownership; any executable implementation programme still requires separate owner authority.
13. The accepted DUR-02 Character persistence sub-baseline applies only to the profile-neutral core. Profile-specific PvP/world-policy Character facts remain blocked until their owning profile/channel semantics are accepted; persistence cannot invent them through schema convenience.
14. Later Global changes are candidate evidence for a later explicit Reference revision and never silently mutate the accepted first target.
15. Every remaining GAME-CHAR, GAME-ITEM or downstream Reference-sensitive `UNKNOWN/CONFLICT` exact value/formula/content/profile rule remains a hard parity/implementation gate where exercised even though Character/item/value semantic architecture is accepted.
16. Whole-DUR-02 acceptance does not authorize DDL/runtime and does not accept destination gates from historical reconciliation; `MOVED` means ownership moved, not that destination business behavior is accepted.
17. GAME-ITEM/DUR-03/GAME-CHANNEL/DUR-04 acceptance does not accept `PROD-ENTITLEMENTS-01`, downstream economy/social/house/reward business policy or any runtime item/value/multichannel/content implementation.

# Explicitly not decided here

This horizon does not select:

- final Reference revision identifier/naming syntax;
- remaining exact GAME-CHAR Reference values/formulas/content/profile-specific rules that are still `PARITY_PENDING_EVIDENCE` under the accepted Stage-B owner baseline;
- exact PostgreSQL DDL, migration/ORM/Rust database technology or production persistence configuration;
- exact DUR-03 SQL/index/constraint/lock implementation, Rust transaction/runtime APIs, concrete ANL event IDs/payloads or numeric transaction/resource ceilings;
- numeric RPO/RTO/backup cadence/retention or partitioning/sharding choices;
- gameplay formulas or balance values beyond separately accepted/evidenced rules;
- exact Reference item values/formulas/edge-case mechanics still `PARITY_PENDING_EVIDENCE` under accepted GAME-ITEM semantics;
- exact loot materialization timing and downstream trade/market/bank/depot/mail/reward/house/crafting behavior under their owning gates;
- exact economy rates/prices/drop tables/scarcity thresholds beyond accepted/evidenced Reference rules;
- numeric alpha/release KPI thresholds;
- exact first Evolved feature inventory beyond the accepted reliability/UX-first strategy;
- final DUR-04 authoring serializer/file extensions, World Bundle container, chunk/floor packing, compression codec, exact WIT function inventory/lowering, exact Wasmtime version/features or numeric content/script resource ceilings;
- AI framework or pathfinding algorithm;
- anti-cheat vendor or invasive client technology;
- payment provider or monetization model;
- final supported platforms/locales/accessibility scope;
- deployment topology, messaging technology or service decomposition;
- LiveOps cadence or automatic economy-control policy;
- modding support.

Those choices require dedicated evidence, contracts and owner acceptance at the appropriate gate.
