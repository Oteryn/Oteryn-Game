# Oteryn v2 Architecture Analysis Gap Register

- Status: Active analysis coverage register
- Date: 2026-08-06
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Applies to: product vision, gameplay, client, content, creator tooling, operations, security, production process, community and long-term sustainability
- Reconciled: 2026-10-06 (owner ruling of 2026-10-06)

**Reconciled 2026-10-06.** Coverage statuses were reconciled against accepted or merged decisions only; a decision whose status line is `CANDIDATE` does not count, and partial coverage stays open with a note of what remains. Every changed entry carries a "Reconciled 2026-10-06" note with its locator (path, plus PR where available); previous statuses are kept in the status line, and no entry was removed. Pointers to `ARCH-ALPHA-OPS-0`, `ARCH-LIVE-READINESS-0` and `ARCH-I18N-A11Y-CREATIVE-0` name owner decisions authored on 2026-10-06 under `docs/architecture/reviews/`; a pointer is not acceptance, and each decision's contract amendments to this file apply only through its own packets. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` is a historical 2026-08-24 snapshot, not current lifecycle authority. Live programme order is issue #162 and its `STATE` comment, not this register.

## Purpose

Preserve the complete set of material areas identified during the 2026-08-05 architecture review that are not yet analysed to contract depth.

This document distinguishes three states:

- **ACCEPTED** — an ADR or accepted contract already freezes the relevant boundary;
- **PARTIALLY_ACCEPTED** — an ADR freezes a high-level direction, while a registered gate still must resolve measurable scope and detailed behavior;
- **REGISTERED_UNRESOLVED** — an existing gate names the area, but its detailed contract is not accepted;
- **NEWLY_IDENTIFIED** — the review found an important area that is not yet represented precisely enough by an existing gate.

This register does not select technologies, formulas, schemas, algorithms, service boundaries, monetization, art direction or final gameplay rules. It does not authorize implementation. Accepted decisions remain in ADRs and dedicated contracts.

The immediate programme action is the source-only historical marker in `blakinio/otclient` for the completed destination cutover. After that closeout, `FND-ID-01` is the next architecture gate. Product and creative analysis may proceed in parallel when it does not redefine accepted foundation boundaries or delay that ordered foundation sequence.

Reconciled 2026-10-06: the paragraph above is historical. The `blakinio/otclient` source-only marker and lifecycle closeout are complete (`blakinio/otclient#274`, #275) and `FND-ID-01` is accepted and merged (`2c584543cd1e3758958755478a6cc6ed3d39a8a9`), both recorded in `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`. The current next action is derived from issue #162, not from this paragraph.

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
- the accepted 19-member canonical Rust workspace, ADR-0011 `pre-native-protocol` client state and completed atomic destination cutover merged as `78988f72a80cc904aa9176ae850c50d4efa0b0f0`.

The exact implementation contracts beneath these foundations remain gated where listed below.

# A. Product identity and creative direction

## 1. Product vision and core gameplay loop

- Coverage status: **PARTIALLY_ACCEPTED** by ADR-0010 and `PRODUCT_DIRECTION_BASELINE.md`
- Registered gate: `GAME-VISION-01` — Product Vision, Parity Scope and World Profile Contract
- Priority: parallel analysis with `FND-ID-01`; must be accepted before broad gameplay and content production

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

Reconciled 2026-10-06: status unchanged (PARTIALLY_ACCEPTED). `GAME-VISION-01` is accepted at minimum gate scope by `docs/architecture/GAME-VISION-01_MINIMUM_OWNER_BASELINE.md` (2026-08-11) and the further `docs/architecture/GAME-VISION-01_*_OWNER_BASELINE.md` files (historical snapshot `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` (issue #97): ACCEPTED / LIFECYCLE_CLOSED). That baseline explicitly defers the exact Reference revision baseline, the revision naming scheme, the pillars/anti-pillars catalogue, the first Evolved feature inventory and branding, so this entry stays open for those items.

Risk if omitted: the project can produce a strong generic engine without a coherent game identity, causing incompatible mechanics, content workflows and client UX to be designed independently.

## 2. Creative, visual, audio and readability direction

- Coverage status: **NEWLY_IDENTIFIED** — owner decision in progress: `ARCH-I18N-A11Y-CREATIVE-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_I18N_A11Y_CREATIVE_DIRECTION_2026-10-06.md`)
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

Reconciled 2026-10-06: status unchanged; no accepted decision covers this gate. Pointer to `ARCH-I18N-A11Y-CREATIVE-0` added per the owner ruling of 2026-10-06; the pointer is not acceptance.

Risk if omitted: renderer, asset format, authoring tools and content budgets may be frozen before the intended presentation is known.

# B. Foundation state and unresolved contracts

## 3. Workspace, migration and dependency structure

- Coverage status: **ACCEPTED** (reconciled 2026-10-06; was **ACCEPTED / CROSS_REPOSITORY_CLOSEOUT_PENDING**)
- Completed gates: `FND-01`, `VSL-02` and the atomic destination cutover
- Canonical destination merge: `78988f72a80cc904aa9176ae850c50d4efa0b0f0`
- Remaining action: source-only historical marker in `blakinio/otclient`

Accepted evidence:

- every existing Rust client subsystem was inventoried and assigned an explicit migration disposition;
- the accepted 19-member workspace has immediate consumers, pinned toolchain/lockfile policy and machine-enforced dependency boundaries;
- provenance, exact source SHA, path mapping, transformation evidence and rollback are retained;
- `protocol-canary` is absent from the production workspace graph and release path;
- the migrated client is explicitly `pre-native-protocol` and fails closed before gameplay credential consumption, routing or transport.

Still unresolved only as cross-repository closeout:

- merge a source-only `blakinio/otclient` marker pointing to the exact destination merge and preventing the old path from becoming a second canonical product line.

Reconciled 2026-10-06: the cross-repository closeout above is complete. `blakinio/otclient#274` and #275 delivered the source-only marker and lifecycle closeout (`docs/architecture/FOUNDATION_DECISION_BACKLOG.md`, "Already accepted" item 2 and item 12). The remaining-action lines in this entry are historical.

## 4. Identifier, protocol, runtime and admission contracts

- Coverage status: **ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
- Existing gates: `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`

Still unresolved:

- complete identifier vocabulary, storage/wire forms, generation and validation;
- full `protocol-oteryn` v1 framing, messages, capabilities, errors and limits;
- command ordering, ticks, clocks, scheduling, bounded queues and overload behavior;
- snapshot, delta, reconciliation and reconnect semantics;
- session issuance, admission, character lease, generation fencing and duplicate-login behavior;
- deterministic execution and failure scenarios at transport/runtime boundaries.

Reconciled 2026-10-06: all four gates are accepted. `FND-ID-01`: `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`, merge `2c584543cd1e3758958755478a6cc6ed3d39a8a9`, lifecycle PR #87. `FND-02`, `FND-03`, `FND-04`: accepted and lifecycle-closed per `docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md` (PR #303) and historical snapshot `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` (issue #97); contracts `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`, `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md` and `docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md` (canonical-on-merge headers); bounded runtime primitives merged in PR #59. The question list above is historical. Adjacent, not absorbed: the measured tick and performance budget is owner decision in progress: `ARCH-ALPHA-OPS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md`); hard numeric limits stay with registry, `PERF-01` and DUR evidence.

## 5. Durable gameplay and content foundations

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
- Existing gates: `DUR-01` through `DUR-04`

Still unresolved:

- durable identifier representation and migration;
- character persistence, transactions, revisioning, backup and restore;
- item/currency conservation and single-location invariants;
- native World Project, compiler, World Bundle and runtime loader details;
- scripting/capability boundary, deterministic execution and resource limits;
- content revision compatibility and safe migration of live durable state.

Reconciled 2026-10-06: `DUR-01`, `DUR-02` and `DUR-03` are accepted and lifecycle-closed per `docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md` (PR #303) and historical snapshot `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` (issue #97); `docs/architecture/DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` is OWNER_ACCEPTED. ADR-0021 world-map runtime loading is accepted by `docs/architecture/reviews/OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md` (PR #1771). Still open: the exact World Project and World Bundle encoding (`docs/architecture/OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md` (PR #311) leaves it to the `DUR-04` format spike); backup/restore and safe migration of live durable state, now owner decision in progress: `ARCH-ALPHA-OPS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md`). CONFLICT: the snapshot records `DUR-04` as ACCEPTED / LIFECYCLE_CLOSED, while `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` still reads `PROPOSED / IN_REVIEW / NOT_STARTED`; this entry does not count `DUR-04` as accepted until the owner resolves the conflict.

## 6. Analytics event contracts

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
- Existing gates: `ANL-01` through `ANL-04`

Still unresolved:

- canonical event envelope and versioning;
- transactional outbox, ordering, idempotency and replay;
- operational versus best-effort versus durable audit pipelines;
- anomaly/economy/integrity projections and evidence quality;
- investigation access, retention, pseudonymization and human-review workflows.

Reconciled 2026-10-06: `ANL-01` is accepted per `docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md` (PR #303); `ANL-02` and `ANL-03` are accepted by `docs/architecture/OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md` (PR #309). Still open: `ANL-04` (read-only investigation access and human-review workflow; expansion gate, not accepted); the `ANL-03` retention ceilings and enforcement/GM contract left open by PR #309. Retention and pseudonymization lifecycle is owner decision in progress: `ARCH-LIVE-READINESS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md`) (`DATA-PRIVACY-01`).

# C. Core gameplay domain

## 7. Character lifecycle and progression

- Coverage status: **ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
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

Reconciled 2026-10-06: `GAME-CHAR-01` is accepted (`docs/architecture/GAME-CHAR-01_STAGE_A_OWNER_BASELINE.md`, `docs/architecture/GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md`; `docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md` (PR #303); historical snapshot `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` (issue #97)). Adjacent questions preserved: exact formulas and values stay profile/ruleset content; the death refinement `docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md` and the stamina/offline-training decision `docs/architecture/reviews/OTERYN_GAME_OFFLINE0_STAMINA_AND_OFFLINE_TRAINING_DECISION_2026-10-01.md` are CANDIDATE and do not count.

## 8. Item model, equipment and transformations

- Coverage status: **ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
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

Reconciled 2026-10-06: `GAME-ITEM-01` is accepted per `docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md` (PR #303) and historical snapshot `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` (issue #97). Adjacent consumers stay with their own entries: item use (`ITEM-USE-0`) and bank (`BANK-0`) are accepted by `docs/architecture/reviews/OTERYN_GAME_ACCEPT_ITEMUSE_BANK0_ACCEPTANCE_DECISION_2026-10-04.md` (PR #1750); houses by `docs/architecture/reviews/OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md` (PR #1771); market, direct trade, mail and depot remain CANDIDATE (entry 17).

## 9. Movement, collision and visibility

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
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

Reconciled 2026-10-06: `VSL-MOVE-01` is accepted as a bounded minimal gate by `docs/architecture/OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md` (PR #311). Still open: exact movement, LOS and timing values (evidence-gated by that acceptance); the visibility refinement `MOVE-RL-11` (D84-D87, `docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md`, PR #1141) is CANDIDATE and does not count; bounded prediction stays with `ALPHA-CLIENT-01`. Bounded adjacent input: `docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md` (PR #1864).

## 10. Combat, abilities, spells and conditions

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
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

Reconciled 2026-10-06: `GAME-ABILITY-01` is accepted (`docs/architecture/GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md`; `docs/architecture/OTERYN_V2_POST_GAME_ABILITY_ACCEPTANCE_RECONCILIATION_20260816.md`, PR #306) and `VSL-COMBAT-01` is accepted as a bounded minimal gate by `docs/architecture/OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md` (PR #311). Parties and PvP (`PARTY-PVP-0`) are accepted by `docs/architecture/reviews/OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md` (PR #1771). Still open: `ALPHA-RULESET-01` is not accepted; auto-attack `docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`, conditions `docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md` and multi-hit damage `docs/architecture/reviews/OTERYN_GAME_D4_MULTI_HIT_DAMAGE_RECEIPT_DECISION_2026-09-29.md` (PR #1218) are CANDIDATE and do not count.

## 11. Creature AI, spawn, NPC and pathfinding

- Coverage status: **PARTIALLY_ACCEPTED** with an NPC-specific precision gap (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
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

Amendment (pending on acceptance of CREATURE-AI-0;
`reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md`). It
resolves the representation, perception, targeting and leash, pathfinding budgets and stale-result
rejection, spawns, respawn and occupancy, channel-local spawn scope, summon ownership, attribution
and despawn, and overload items above. Boss phases and encounter recovery go to BOSS-RAID-0; NPC
behaviour stays unresolved.

Reconciled 2026-10-06: `GAME-AI-01` is accepted by `docs/architecture/OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md` (PR #309), which leaves open the FSM/framework choice, the pathfinding algorithm, numeric ceilings, the event/encounter durable owner, controlled-actor reward attribution and exact Reference values. NPC presence, walking, voices and focus are accepted by `NPC-BEHAVIOUR-0` (`docs/architecture/reviews/OTERYN_GAME_NPC_BEHAVIOUR0_NPC_PRESENCE_WALKING_VOICES_AND_FOCUS_DECISION_2026-10-01.md`, accepted via `ARCH-BATCH-ROOT-PACKETS-V1`, PR #1733); NPC service and conversation runtime (`NPC-0`) stays CANDIDATE. The `CREATURE-AI-0` amendment above stays pending: its decision header is CANDIDATE and `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_LOOP_PACKETS_2026-10-04.md` (PR #1735) keeps it a candidate. CONFLICT: `docs/architecture/reviews/OTERYN_GAME_SPAWN1A_FIXTURE_SPAWN_ROOM_R2_DECISION_2026-10-04.md` calls `CREATURE-AI-0` accepted. Boss phases go to `BOSS-RAID-0` (CANDIDATE).

## 12. World interaction and environmental mechanics

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
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

Reconciled 2026-10-06: `GAME-INTERACTION-01` is accepted by `docs/architecture/OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md` (PR #309), which leaves open the movement/relocation owner, the durable writable-text owner, the payload registry and numeric ceilings. Item use on self, target, tile, creature and item is accepted by `ITEM-USE-0` (`docs/architecture/reviews/OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md`, accepted by `docs/architecture/reviews/OTERYN_GAME_ACCEPT_ITEMUSE_BANK0_ACCEPTANCE_DECISION_2026-10-04.md` (PR #1750)). Doors, levers, fields and the world clock (`docs/architecture/reviews/OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION_2026-10-01.md`) remain CANDIDATE.

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

Reconciled 2026-10-06: status unchanged. `docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`, `docs/architecture/reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md` and `docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md` are CANDIDATE and do not count. CONFLICT: the archived task record `docs/agents/tasks/archive/OTV2-2026100x-quest-state-1.md` states "QUEST-STATE-0 accepted (#1661)" while the decision header still reads CANDIDATE.

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

Reconciled 2026-10-06: status unchanged. `docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md` and `docs/architecture/reviews/OTERYN_GAME_ENCOUNTER_RT0_ENCOUNTER_RUNTIME_AND_BOSS_LEVERS_DECISION_2026-10-01.md` are CANDIDATE and do not count; `GAME-AI-01` (PR #309) leaves the event/encounter durable owner open.

# D. Client and player experience

## 15. Native client architecture

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
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

Reconciled 2026-10-06: `ALPHA-CLIENT-01` is accepted by `docs/architecture/OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md` (PR #309), which leaves open the UI, render, network, audio, updater and installer library choices, transport/runtime, Tier evidence and numeric ceilings. `docs/architecture/ADR-0020-native-client-gameplay-entry.md` has a Candidate header and does not count. Crash reporting has an owner-accepted pre-contract baseline only (`docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`). Client version and update is owner decision in progress: `ARCH-ALPHA-OPS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md`).

## 16. Localization, onboarding and accessibility

- Coverage status: **REGISTERED_UNRESOLVED** — owner decision in progress: `ARCH-I18N-A11Y-CREATIVE-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_I18N_A11Y_CREATIVE_DIRECTION_2026-10-06.md`)
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

Reconciled 2026-10-06: status unchanged; no accepted decision covers this gate. Pointer to `ARCH-I18N-A11Y-CREATIVE-0` added per the owner ruling of 2026-10-06; the pointer is not acceptance.

# E. Economy, social systems and world topology

## 17. Economy, trade and value stability

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
- Existing gates: `EXP-ECONOMY-01`, `GAME-ITEM-01`, `DUR-03`

Still unresolved:

- direct trade, offers, escrow, cancellation and partial completion;
- market, bank, depot, mail and offline delivery;
- currency/item sources and sinks;
- fees, taxes, inflation controls and economy-health targets;
- crafting and upgrade economy;
- concurrency, reconciliation, fraud and duplication resistance;
- rollback and support correction without violating conservation.

Reconciled 2026-10-06: `GAME-ITEM-01` and `DUR-03` are accepted per `docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md` (PR #303); the account bank balance (`BANK-0`) is accepted by `docs/architecture/reviews/OTERYN_GAME_ACCEPT_ITEMUSE_BANK0_ACCEPTANCE_DECISION_2026-10-04.md` (PR #1750); the character gold fee boundary is accepted (`docs/architecture/reviews/OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md`, PR #1313). `EXP-ECONOMY-01` itself is not accepted. Still open: market, direct trade, mail and depot (`docs/architecture/reviews/OTERYN_GAME_MARKET0_WORLD_MARKET_DECISION_2026-09-30.md`, `docs/architecture/reviews/OTERYN_GAME_PLAYER_TRADE0_DIRECT_PLAYER_TRADE_DECISION_2026-09-30.md`, `docs/architecture/reviews/OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md`, `docs/architecture/reviews/OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md`: all CANDIDATE); sources, sinks and economy-health targets. Economy rollback and support correction is owner decision in progress: `ARCH-LIVE-READINESS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md`).

## 18. Party, guild, chat, friends and presence

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED**)
- Existing gate: `EXP-SOCIAL-01`

Still unresolved:

- world/channel/cross-world scope;
- party membership, leadership and shared experience;
- guild identity, roles, membership and durable history;
- chat channels, moderation and offline messages;
- friends, blocks and presence privacy;
- consistency during reconnect, channel switch and world transfer;
- social abuse limits and audit needs.

Reconciled 2026-10-06: parties and shared experience (`PARTY-PVP-0`) and guilds (`GUILD-0`) are accepted by `docs/architecture/reviews/OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md` (PR #1771). Still open: chat (`docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md`) and VIP/friends (`docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md`) are CANDIDATE; presence and contact consent have an owner-accepted pre-contract baseline only (`docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md`); cross-world scope and moderation remain open.

## 19. Houses

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED / DEFERRED**)
- Existing gate: `EXP-HOUSES-01`

Still unresolved:

- final channel topology and access model;
- one-state-per-world simulation owner;
- rent, auctions, ownership and transfer;
- doors, guest lists, beds and persistence;
- crash recovery and anti-duplication;
- relationship to instances, channels and world lifecycle.

Reconciled 2026-10-06: `EXP-HOUSES-01` whole-gate semantic architecture is owner-accepted (`docs/architecture/EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md`, DecisionStatus ACCEPTED, 2026-09-08, issue #220); house ownership, item custody and interior runtime (`HOUSE-OWN-0`, `HOUSE-CUSTODY-0`, `HOUSE-RUNTIME-0`) are accepted by `docs/architecture/reviews/OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md` (PR #1771). Still open: beds (`docs/architecture/reviews/OTERYN_GAME_BED0_HOUSE_BEDS_DECISION_2026-10-01.md`, PR #1621) are CANDIDATE; numeric rent, auction and grace values, physical schema and the `ResidenceId` representation stay deferred by that baseline.

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

Reconciled 2026-10-06: status unchanged. `docs/architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md` is an owner-accepted pre-contract baseline (2026-08-06) that states it does not complete any named gate.

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

Reconciled 2026-10-06: status unchanged; no accepted decision covers this gate. Backup/restore is owner decision in progress: `ARCH-ALPHA-OPS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md`); the rest stays EXPANSION.

# F. Live operation, release and infrastructure

## 22. LiveOps and runtime configuration

- Coverage status: **REGISTERED_UNRESOLVED** — owner decision in progress: `ARCH-LIVE-READINESS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md`)
- Existing gate: `PROD-LIVEOPS-01`

Still unresolved:

- static versus runtime configuration;
- feature flags, staged rollout, maintenance mode and kill switches;
- event/rate schedules and bounded overrides;
- ownership, validation, signatures and audit;
- propagation ordering, stale-state behavior and rollback;
- safe defaults and fail-closed behavior;
- separation from content bundles and executable deployment.

Reconciled 2026-10-06: status unchanged; no accepted decision covers this gate. Pointer added per the owner ruling of 2026-10-06; the pointer is not acceptance.

## 23. GM, support and moderation operations

- Coverage status: **REGISTERED_UNRESOLVED** — owner decision in progress: `ARCH-LIVE-READINESS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md`)
- Existing gate: `OPS-GM-01`

Still unresolved:

- role and capability boundaries;
- mute, ban, kick, teleport, inspect and recovery actions;
- audited domain transactions for corrections;
- impersonation/account-access policy;
- case management, evidence, appeals and review;
- dual control for high-risk mutations;
- immutable administrative audit and emergency rollback.

Reconciled 2026-10-06: status unchanged; no accepted decision covers this gate. `ARCH-LIVE-READINESS-0` also takes bug reporting and player support. The pointer is not acceptance.

## 24. Compatibility, release train, launcher and updater

- Coverage status: **REGISTERED_UNRESOLVED** — owner decision in progress: `ARCH-ALPHA-OPS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md`)
- Existing gates: `PROD-COMPAT-01`, `EXP-UPDATE-01`

Still unresolved:

- compatibility matrix among client, protocol, server, Gateway, content, assets, ruleset and database revisions;
- minimum/maximum clients and forced-update policy;
- mixed-version deployment and capability negotiation;
- release channels, staged rollout and rollback order;
- signed manifests, delta updates, CDN/mirror behavior and archive safety;
- immutable release evidence and cross-repository contract locks.

Reconciled 2026-10-06: status unchanged; no accepted decision covers these gates. `ARCH-ALPHA-OPS-0` takes client version and update; the pointer is not acceptance.

## 25. Performance, capacity and overload behavior

- Coverage status: **REGISTERED_UNRESOLVED** — owner decision in progress: `ARCH-ALPHA-OPS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md`)
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

Reconciled 2026-10-06: status unchanged; `PERF-01` is PLANNED in historical snapshot `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` (issue #97). `ARCH-ALPHA-OPS-0` takes time/tick and the performance budget; the pointer is not acceptance. Bounded adjacent input only: `docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_VIEWPORT_BUDGET_2026-10-06.md` (PR #1864).

## 26. Deployment, orchestration, recovery and observability

- Coverage status: **REGISTERED_UNRESOLVED** — owner decision in progress: `ARCH-ALPHA-OPS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_ALPHA_OPERABILITY_2026-10-06.md`)
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

Reconciled 2026-10-06: status unchanged; `OPS-CHANNEL-01` is PLANNED in historical snapshot `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` (issue #97). `ARCH-ALPHA-OPS-0` takes observability, logging, metrics and SLOs, schema migration, and backup and DR; the pointer is not acceptance. Owner-accepted partial inputs only: GameNode boot composition (`docs/architecture/reviews/OTERYN_GAME_NODE_BOOT_COMPOSITION_DECISION_2026-09-24.md`, PR #830) and process registration/bootstrap auth (`docs/architecture/reviews/OTERYN_GAME_NODE_PROCESS_REGISTRATION_BOOTSTRAP_AUTH_DECISION_2026-09-22.md`).

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

Reconciled 2026-10-06: status unchanged. `docs/architecture/reviews/OTERYN_GAME_SEC_CLIENT01_CLIENT_INTEGRITY_AND_ANTI_BOT_DECISION_2026-10-01.md` is CANDIDATE and does not count. The threat model is owner decision in progress: `ARCH-LIVE-READINESS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md`).

## 28. Product privacy and data lifecycle

- Coverage status: **REGISTERED_UNRESOLVED** — owner decision in progress: `ARCH-LIVE-READINESS-0` (`docs/architecture/reviews/OTERYN_GAME_ARCH_LIVE_READINESS_2026-10-06.md`)
- Existing gate: `DATA-PRIVACY-01`

Still unresolved:

- data classification across Platform, gameplay, analytics, support and security;
- retention, deletion, anonymization and legal hold;
- account deletion consequences for characters/world records;
- export/access requests and provenance;
- consent and optional telemetry;
- pseudonymous analytics and re-identification controls;
- backup/audit exceptions and privacy-safe diagnostics.

Reconciled 2026-10-06: status unchanged; no accepted decision covers this gate, and the pointer is not acceptance. Fragments only: `docs/architecture/reviews/OTERYN_CHARACTER_DURABLE_AUDIT_RETENTION_DECISION_2026-09-22.md` (owner-accepted), `docs/architecture/reviews/OTERYN_DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_DECISION_2026-09-27.md` (owner-selected, canonical acceptance pending) and `docs/architecture/CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md` (pre-contract).

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

- Coverage status: **PARTIALLY_ACCEPTED** (reconciled 2026-10-06; was **REGISTERED_UNRESOLVED / DEFERRED**)
- Existing gate: `PROD-ENTITLEMENTS-01`

Still unresolved:

- Platform/payment versus game-delivery ownership;
- entitlement identity, expiry and revocation (Store purchase scope: see below);
- idempotent purchase delivery, refunds and chargebacks;
- premium/account benefits versus character/world grants;
- real-money versus in-game economy separation;
- fraud, audit, support correction and failure behavior.

No monetization choice is implied.

Partially resolved (2026-09-28): Store catalog authorship only — see `docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md`.
Coin balance, payment, purchase delivery, entitlement lifecycle, refunds and fraud
remain open here.

Partially resolved (2026-09-28): Store purchase scope and portability only (owner decisions D47,
D49) — see `docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md`
§4.5. Store purchases belong to the account. A cosmetic unlock applies on every world while its
entitlement is usable. A purchased item is claimable on any compatible world of the profile family
it was bought for. Delivery ownership, entitlement identity, expiry, revocation and refunds remain
open here.

Reconciled 2026-10-06: the game-side `PROD-ENTITLEMENTS-01` consumer contract is accepted (merge PR #20, `d40a225e`; closeout PR #27; recorded in historical snapshot `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` (issue #97) §8), which also ends the DEFERRED state. Still open: payment, the runtime product catalogue, refunds, chargebacks, fraud and delivery activation, which remain separately governed; Premium activation (`docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md`, PR #1118, and the later `OTERYN_GAME_PREMIUM_ACTIVATION*` files) is CANDIDATE and does not count.

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

1. Complete the source-only `blakinio/otclient` marker for the merged destination cutover.
2. `FND-ID-01` — foundation identifier vocabulary.
3. `GAME-VISION-01` — refine ADR-0010 into measurable parity scope, product pillars and launch strategy in parallel without blocking identifier analysis.

Reconciled 2026-10-06: Track 0 is historical. Items 1 and 2 are complete (`blakinio/otclient#274`, #275; `FND-ID-01` merge `2c584543cd1e3758958755478a6cc6ed3d39a8a9`), and item 3 is accepted at minimum scope (entry 1). For the state of Tracks 1-3, see the reconciled entries above; current order is issue #162.

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
5. derive the immediate next action from merged task/ADR evidence and update this register whenever a completed gate advances the programme.
