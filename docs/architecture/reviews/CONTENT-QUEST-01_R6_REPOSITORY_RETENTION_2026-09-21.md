# CONTENT-QUEST-01 r6 repository retention

Status: PROPOSED_NONCANONICAL
Date: 2026-09-21
Repository: Oteryn/Oteryn-Game
Base main: e4da44a86d006df623b7b234d7b3a879516b86cd
Tracking issue: #707
Runtime authority: NONE
DDL authority: NONE
Production authority: NONE

## Purpose

This tracked document preserves the current complete CONTENT-QUEST-01 architecture/evidence state reached through r1-r6 so the work is no longer retained only in Issue comments and local recovery packages.

It is a retention/index document. It does not itself promote the proposed Quest/Encounter contracts to accepted architecture, does not authorize runtime implementation, and does not promote Crystal/Canary/community observations to Global Tibia truth.

## Current closure state

Architecture semantics resolved in the candidate include:

- typed finite QuestDefinition / QuestGraph / QuestNode / Objective / Transition / Reward references;
- immutable content revisions and revision-bound durable QuestState;
- CHARACTER and ACCOUNT_WITHIN_WORLD progress scopes;
- reward claim scope independent from progress scope;
- ACCOUNT_ONCE, CHARACTER_ONCE and EACH_CHARACTER_ON_ACCOUNT_ONCE claim policies;
- explicit reward binding composed into GAME-ITEM/DUR-03 rather than Quest-owned item mutation;
- durable source-occurrence consumption and replay/idempotency rules;
- graph validation, deterministic transition ordering and bounded loops;
- repeat/cooldown/daily/weekly/server-save/time-window semantic separation;
- abandon/retry, exclusive dependencies, migration, client projection, journal, Studio and support-repair boundaries;
- Encounter as specialization of the existing Event/runtime role rather than a second global runtime;
- typed encounter triggers/actions with proposal-only foreign-owner mutations;
- party/admission/participation separation;
- public/shared vs exclusive reservation vs InstanceRuntime isolation;
- explicit participation/offline eligibility;
- checkpoint/recovery/cleanup semantics;
- bounded WASM proposal-only scripting under DUR-04;
- existing game-gate only; no new required GitHub status.

The candidate preserves these owner boundaries:

- Quest does not directly mutate combat, AI, map/collision, items/value, Character death, leases or persistence.
- Encounter is not a second global runtime/process.
- AI is not quest/world/reward authority.
- scripts/WASM are proposal-only.
- GAME-ITEM/DUR-03 remain value/materialization/anti-duplication owners.
- reconnect/disconnect never resets or rolls back committed gameplay.
- clients remain non-authoritative.
- Crystal/Canary/legacy Lua/storage remain importer/provenance evidence only.
- missing Global facts remain EVIDENCE_REQUIRED.

## r2 precision closures

The first bounded Annihilator source witness exposed and closed three candidate precision gaps.

### Shared entitlement across reward alternatives

A RewardKey denotes one grant opportunity/entitlement slot, not one physical chest or one output item. ONE_OF_N alternatives share one ClaimKey:

(WorldId, QuestKey, RewardKey, claim subject, ClaimCycleRef)

Concurrent alternative selectors cannot consume the entitlement more than once. Pending/ambiguous accepted choice freezes alternative/output identity across retry, Channel/session change, restart and lost acknowledgement.

### Reward-dependent completion without circular prerequisite

A reward may be anchored either to terminal Quest completion or to an explicitly declared durable nonterminal qualification milestone. Both acyclic forms are supported:

qualified objective -> Quest COMPLETED -> claim -> delivery

qualified milestone -> claim -> committed receipt -> Quest COMPLETED

The compiler must reject an unseeded hard dependency cycle where the reward requires the same completion whose only prerequisite is that reward receipt.

### Qualified legacy identifier kinds

Importer references are qualified by repository, source revision, data-pack/variant, identifier kind and value/symbol. Numeric equality does not merge storage IDs, mission IDs, UIDs, action IDs, item types or unrelated data packs.

The Annihilator source uses the same legacy storage as reward-consumed evidence and later cosmetic/access state. Oteryn must split such overloaded scalar meaning into typed facts under correct owners rather than port the scalar as canonical state.

## r3 startup/reset source witness

Pinned Crystal source demonstrated that:

- data-global startup loads tables and then map attribute stamping;
- UID 30025 is assigned to the Annihilator lever at the expected source coordinate;
- ChestUnique reward entries are stamped and quest_reward_common registers the reward UID ranges;
- daily=true in the lever script prevents manual lever reversal on the alternate item state; it does not by itself prove a calendar reset;
- sequential source teleports/spawns are not proof of atomic Oteryn admission.

No Crystal source rule is promoted into Oteryn reset semantics.

## r4 operational server-save/restart witness

For the pinned Crystal repository-default data-global/config/Compose profile, source inspection showed:

- globalServerSaveTime = 06:00:00;
- globalServerSaveShutdown = true;
- the global server-save path transitions to GAME_STATE_SHUTDOWN;
- saveAll persists players/guilds/map/KV, while inspected Map::save persists house info/items rather than arbitrary public-map lever transforms;
- Docker Compose uses restart: unless-stopped;
- startup reloads the static map and reapplies startup map attributes.

Strongest safe source inference:

default Crystal deployment global-save shutdown -> process reload from static map state may practically reset this public-map lever.

This remains OTS_HYPOTHESIS_ONLY. In Oteryn:

process restart != quest reset
GameNode replacement != Encounter retry
InstanceRuntime recovery != new reward cycle
server startup != ClaimCycleRef advance

Only a trusted semantic reset/world-policy occurrence may advance resettable Quest/Reward state.

## r5 official-first Reference evidence

Official evidence retained in #707 includes:

- CipSoft 2009-12-22 historical statement: Annihilator again required exactly four people;
- official Character Trade / current Achievement material establishing Annihilator achievement existence before/current around the target window;
- CipSoft 2012-11-21 historical relationship between Annihilator, Demon Helmet, Demon Oak and full Demon Outfit.

These are field-level evidence only. They do not independently prove the complete 2026-07-28 target quest.

Still EVIDENCE_REQUIRED from official/accepted target evidence:

- minimum level 100;
- exact lever/room coordinates;
- exact monster count/spawn positions/stat block;
- kill-all vs survive/reach-reward qualification;
- exact four reward alternatives/item IDs;
- exact ONE_OF_N target semantics;
- door/access state after reward/outfit;
- reset cadence;
- death/disconnect/re-entry/retry;
- reward/achievement/Quest completion commit relationship;
- QuestLog text/state.

## r6 secondary corroboration

Secondary community sources converge on a plausible Annihilator shape including:

- level 100;
- four participants;
- ONE_OF_4 reward shape;
- Demon Armor, Magic Sword, Stonecutter Axe, Annihilation Bear/Present;
- six Angry Demons;
- descriptions compatible with survival/reaching reward rather than an assumed generic KillAll(6).

This is SECONDARY_COMMUNITY_ONLY and may generate investigation fixtures/questions. It cannot promote target Reference truth.

Historical secondary server-save/reset wording is versioned evidence and cannot silently define current target reset semantics or ClaimCycleRef.

## Validation state

Latest structural package validation:

- classified architecture areas: 48/48 traced;
- required test corpus: 71 cases;
- resource dimensions: 26;
- adversarial threat classes: 14;
- runtime/gameplay tests executed: false;
- hosted game-gate executed for this candidate: false;
- independent review: not performed;
- Reference quest parity: not qualified;
- production/runtime/DDL implementation: none by this architecture task.

Required cases T01-T71 remain REQUIRED_NOT_EXECUTED.

The r6 additions are:

- T68 secondary corroboration cannot promote Reference truth;
- T69 encounter population does not imply kill-all qualification;
- T70 ONE_OF_4 community convergence remains fixture-only until accepted target proof;
- T71 temporal secondary reset drift remains versioned evidence and cannot silently define reset/claim-cycle semantics.

## Prepared candidate paths

The local r6 package contains prepared versions of:

- docs/architecture/CONTENT-QUEST-01_QUEST_CONTRACT_CANDIDATE.md
- docs/architecture/CONTENT-QUEST-01_ENCOUNTER_CONTRACT_CANDIDATE.md
- docs/architecture/CONTENT-QUEST-01_VALIDATION_REQUIREMENTS.md
- docs/architecture/CONTENT-QUEST-01_AUDIT_AMENDMENTS_AND_READINESS.md
- docs/architecture/CONTENT-QUEST-01_PROTECTED_AMENDMENT_PATCH.md
- docs/architecture/reviews/CONTENT-QUEST-01_ANNIHILATOR_IMPORT_WITNESS_2026-09-21.md
- docs/architecture/reviews/CONTENT-QUEST-01_ANNIHILATOR_STARTUP_RESET_WITNESS_2026-09-21.md
- docs/architecture/reviews/CONTENT-QUEST-01_ANNIHILATOR_SERVER_SAVE_RESTART_WITNESS_2026-09-21.md
- docs/architecture/reviews/CONTENT-QUEST-01_ANNIHILATOR_REFERENCE_EVIDENCE_WITNESS_2026-09-21.md
- docs/architecture/reviews/CONTENT-QUEST-01_ANNIHILATOR_SECONDARY_CORROBORATION_WITNESS_2026-09-21.md

Package:
CONTENT-QUEST-01-closure-r6-2026-09-21.zip

Package SHA-256:
85f80992c7a56b2c2b9c0609608cdce05936a0f9178fc4c22b64c275dc1de822

Package bytes:
137011

## Exact repository evidence

Issue #707:
https://github.com/Oteryn/Oteryn-Game/issues/707

r1:
- Quest contract: issuecomment-5757124375
- Encounter contract: issuecomment-5757138722
- Validation requirements: issuecomment-5757155473
- Audit/amendments/readiness: issuecomment-5757196578

r2:
- Annihilator precision-gap closure: issuecomment-5757437963
- Verified r2 recovery index: issuecomment-5757596108

r3:
- startup/reset witness 1/2: issuecomment-5757817611
- startup/reset witness 2/2: issuecomment-5757821111
- exact protected amendment input: issuecomment-5757824106
- publication manifest/validation: issuecomment-5757831227

r4:
- server-save/restart witness: issuecomment-5757949428
- validation/publication manifest: issuecomment-5757954514

r5:
- official-first evidence: issuecomment-5758003735

r6:
- secondary corroboration witness: issuecomment-5758068456
- validation/publication manifest retained in #707 after r6 package validation.

## Required protected amendments

Before account-wide Quest execution is accepted, MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md must explicitly permit Quest progress scope CHARACTER or ACCOUNT_WITHIN_WORLD and must keep progress scope separate from claim scope, binding scope and encounter/runtime multiplicity.

Encounter wording must preserve:
- actor-local AI state under GAME-AI;
- multi-actor bounded orchestration under Encounter specialization of existing Event/runtime ownership;
- ChannelRuntime for local public occurrence;
- InstanceRuntime for private instance occurrence;
- existing world Event owner for world-unique durable occurrence/facts;
- independent reward eligibility scope.

README.md should list the candidate as proposed/noncanonical until protected acceptance.

No second Reference evidence registry, permanent serializer, generic workflow engine, global Quest process, reward ledger, Party QuestState or new CI gate is introduced.

## Current status

CONTENT-QUEST-01 architecture/evidence state:
PROPOSED_NONCANONICAL

Tracked retention:
PRESENT_ON_TASK_BRANCH

Protected architecture acceptance:
NOT_YET_GRANTED

Implementation authority:
NONE

Next safe investigation:
use the existing Oteryn map/OTBM producer/parser boundary to confirm the physical Annihilator lever/reward-room/chest bindings without creating a second parser and without promoting Crystal coordinates to Reference truth.
