# Oteryn Game Terra-Control-Plane + Sol Parallel Execution Design

## Status

Owner-requested execution architecture for Issue #213. This document refines the already accepted Sol-lead direction for the specific deployment where the central ChatGPT Work session runs on Terra High and therefore must not exercise technical or architecture discretion.

Design admission: protected `main@4c395ece416c3c56aed5607653a0730c52dcb3fd` on 2026-08-27. Live GitHub always outranks this provenance snapshot.

This design changes execution/governance only. It does not change Game runtime architecture, accepted contracts, resource values, production topology or product semantics.

## Goal

Complete the current native gameplay vertical slice and then the remaining accepted Oteryn Game programme as quickly as safely possible by separating:

- deterministic scheduling/state reconciliation in ChatGPT Work / Terra High;
- deep lane-local technical reasoning and implementation in independent GPT-5.6 Sol Extra High chats;
- material cross-lane architecture decisions in a dedicated Sol Supervising Architect chat;
- independent read-only verification in the existing Work auditor;
- serialized ownership of every shared repository surface.

The optimisation target is useful throughput, not maximum simultaneous writers.

## Existing foundation

This design specializes, rather than replaces, the direction captured by:

- `docs/superpowers/specs/2026-08-26-oteryn-game-sol-lead-selective-codex-execution-design.md`;
- `docs/superpowers/plans/2026-08-26-oteryn-game-sol-lead-selective-codex-execution.md`;
- `docs/superpowers/plans/2026-08-25-oteryn-game-work-delivery-orchestration.md`;
- `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md`.

At admission, Durability Issue #167 has draft PR #212 and remains the live critical-path implementation lane. Server Seam, Client/QA, Movement and Combat remain downstream. These numbers are provenance only; every invocation resolves live state again.

## Execution hierarchy

```text
Owner
  |
  +--> Sol Supervising Architect (Extra High)
  |      material/cross-lane architecture decisions
  |
  +--> exactly one active Work control-plane profile
  |      deterministic scheduler and release executor
  |        |
  |        +--> Sol Durability Lead (Extra High)
  |        +--> Sol Server Seam Lead (Extra High)
  |        +--> Sol Client/QA Lead (Extra High)
  |        +--> Sol Movement Lead (Extra High)
  |        +--> Sol Combat Lead (Extra High)
  |        +--> later exact post-VSL lane leads
  |
  +--> Work Independent Auditor
         read-only, independent
```

## Control-plane activation and transfer

`OTV2_WORK_DELIVERY_COORDINATOR` and `OTV2_TERRA_GAME_CONTROL_PLANE` remain reusable profiles, but **reusable is not the same as active**. One programme lifecycle may have exactly one mutating control plane.

The selector is resolved only from durable GitHub state:

1. If the current coordinator Issue/task contains `active_control_plane_profile`, that exact profile is the only mutating control plane.
2. A legacy coordinator lifecycle that predates the selector keeps the profile already named as its canonical coordinator prompt/owner. Other reusable control-plane profiles are `RECOVERY_READ_ONLY`.
3. A switch requires a dedicated docs/governance transition merged to protected `main` that updates the current coordinator Issue/task and releases the previous profile before the new profile mutates.

Alias invocation, chat instruction, model selection, `reusable` registry status, tool availability or urgency never performs this transfer. If exactly one profile cannot be proven, control-plane mutation fails closed as `POLICY_CONFLICT`.

The existing #162 Work lifecycle therefore remains owned by `OTV2_WORK_DELIVERY_COORDINATOR` until a later durable transition explicitly selects Terra. Merging this package does not silently seize or restart #162. The Terra profile is immediately reusable for read-only recovery/transfer preparation and for future or explicitly transferred programme lifecycles.

An inactive control-plane profile may resolve live GitHub state and prepare a recovery/transfer packet, but it may not create allocations, grant shared leases, integrate/merge, mutate coordinator status or perform lifecycle closeout for that programme.

## Core invariant: Terra has zero technical discretion

The Terra control plane MUST NOT decide whether a technical change is correct or desirable. It may only execute deterministic consequences of already-canonical authority and exact GitHub evidence.

Terra may autonomously, only when it is the uniquely selected active control plane:

- resolve current `main`, Issues, tasks, branches, PR heads, checks and reviews;
- compare exact SHAs and changed paths;
- apply explicit dependency predicates from canonical programme documents;
- detect path/lease overlap;
- classify lane state using this design's state machine;
- dispatch a canonical Sol alias when all explicit release predicates are `PROVEN`;
- hold/release a shared lease according to an already-approved exact allocation;
- return a PR to its owning Sol lead when a required gate is not satisfied;
- execute a merge only when every deterministic merge predicate is satisfied and the owning Sol lead has returned `READY_FOR_INTEGRATION`;
- update coordinator-owned lifecycle/status metadata within exact governance authority.

Terra MUST NOT:

- select or redesign APIs, schemas, data models, persistence semantics or protocol behavior;
- decide a new resource limit or reinterpret an existing limit;
- decide that a worker should broaden scope or take another lane's path;
- decide between materially different implementation strategies when the choice affects contracts or ownership;
- resolve a review finding by technical judgment;
- modify product/runtime code;
- declare a technical deviation acceptable because CI is green;
- invent a new architecture decision to keep the critical path moving.

## Decision routing

Use these exact classes.

### `LANE_DECISION_REQUIRED`

Use when a bounded technical choice is inside one lane's existing architecture and ownership but requires expert judgment. Route to that lane's Sol lead.

Examples: local algorithm choice, path-local refactor, test strategy, handling a compile/runtime defect inside allocated semantics.

### `ARCHITECTURE_ESCALATION_REQUIRED`

Use before mutation when the decision affects public API/wire/schema/stable IDs, persistence/value ownership, trust/session/fencing authority, cross-lane semantic ownership, unaccepted resource maxima, permanent Content/world semantics, or another material architecture boundary. Route to `Oteryn: sol supervising architect`.

### `OWNER_DECISION_REQUIRED`

Use when the choice changes product priority, accepted scope, execution authority, production authority, funding/cost policy, or when the Supervising Architect proves that existing owner authority cannot choose among valid options.

### `POLICY_CONFLICT`

Use when current canonical repository instructions or allocations conflict, or when the active control-plane profile is not uniquely resolvable. Stop affected mutation until the conflict is reconciled durably.

## Sol lane-lead authority

A Sol lane lead is the reasoning owner for one bounded lane. It may make ordinary implementation decisions only when all of the following are true:

- a current exact merged allocation grants write authority;
- changed paths remain inside that allocation;
- the choice preserves accepted contracts/ADRs/resource limits;
- no sibling branch is consumed as implicit authority;
- the choice does not require a serialized shared surface that is not currently leased;
- the choice is not a material architecture decision under the escalation rules.

A Sol lead owns one canonical branch/worktree and one PR at a time. It does not merge its own implementation PR under this execution profile; it returns `READY_FOR_INTEGRATION` and the uniquely active control plane performs only the deterministic integration gate.

## Supervising Architect authority

The Sol Supervising Architect resolves durable `ARCHITECTURE_ESCALATION_REQUIRED` packets. It is not a routine coding lane.

It may:

- inspect all relevant live repository evidence;
- compare accepted ADRs/contracts and lane boundaries;
- choose a material architecture option only where existing owner-approved architecture authority permits;
- create/update the bounded architecture decision/contract lifecycle required to make the decision durable;
- return an exact implementation boundary to the affected lane.

It must return `OWNER_DECISION_REQUIRED` if owner authority is required. Architecture resolution never silently grants runtime write authority; the affected implementation lane still needs its exact allocation.

## Independent auditor

`Oteryn: work auditor` remains read-only and independent. It audits the active control plane and Sol lead evidence. It is never counted as a mutating lane and cannot author a fix for code it is auditing.

## Lane state machine

Canonical states:

```text
READ_ONLY_PREPARATION
WAITING_DEPENDENCY
WAITING_ALLOCATION
READY_TO_IMPLEMENT
IMPLEMENTING
LANE_DECISION_REQUIRED
SHARED_LEASE_REQUIRED
WAITING_ARCHITECTURE
WAITING_EXTERNAL
READY_FOR_INTEGRATION
REVIEW_RECONCILIATION_REQUIRED
COMPLETED_RELEASED
```

`UNKNOWN`, `CONFLICT`, `POLICY_CONFLICT` or an unresolved architecture escalation blocks mutation.

## Deterministic release predicate

Terra may release a Sol lane from preparation to mutation only when all required facts are directly `PROVEN`:

```text
Terra is the uniquely active control plane
+ current main resolved
+ governing Issue/task exists
+ exact merged allocation exists
+ exact owned paths are known
+ prerequisite merges are terminal
+ no path/shared-lease overlap
+ no unresolved architecture escalation
+ no policy conflict
= READY_TO_IMPLEMENT
```

If any operand is not proven, Terra does not infer readiness.

## Deterministic integration predicate

Terra may integrate only when:

```text
Terra is the uniquely active control plane
+ Sol lead says READY_FOR_INTEGRATION
+ exact PR head is unchanged
+ changed paths fit allocation/lease
+ all required focused/component/E2E evidence is present
+ all required exact-head CI passes
+ required independent review passes
+ zero unresolved required review threads
+ no unresolved architecture/policy conflict
+ current integration main is reconciled without invalidating evidence
= merge permitted
```

Any technical review finding requiring judgment returns to the owning Sol lead or Supervising Architect. Terra does not adjudicate it.

## Concurrency model

Default useful concurrency:

- exactly one active mutating control-plane profile per programme; the inactive Work/Terra profile is read-only recovery;
- one independent auditor;
- up to five active Sol chats;
- normally at most two mutating Sol lanes;
- a third mutating lane only when exact paths and every shared surface are proven disjoint and the scheduler records a concrete throughput reason;
- preparation/read-only lanes may run concurrently without consuming writer capacity.

The critical path is mostly serial, so downstream chats should prepare exact dependencies/tests while the current critical writer finishes rather than speculatively modifying runtime code.

## Serialized shared surfaces

At minimum:

- root/app Cargo manifests and `Cargo.lock`;
- `workspace-boundaries.toml` and architecture-check policy;
- `apps/game-server/src/lib.rs` and equivalent composition roots;
- stable protocol/event/resource registries and numeric IDs;
- shared ADRs/contracts consumed by multiple active lanes;
- workflows, branch/protection/governance files.

A worker encountering a shared requirement returns `SHARED_LEASE_REQUIRED` with the exact path and reason. The active control plane may grant/execute the shared turn only if an already-approved allocation mechanism deterministically authorizes it. Any scope/ownership ambiguity escalates.

## Current vertical-slice waves

Live GitHub always wins, but the expected dependency chain is:

```text
Durability
  -> Server Seam
  -> Client/QA
  -> Movement resource gate #139
  -> Movement
  -> Combat
  -> vertical-slice closeout
```

### Wave V0 — now

- Durability Lead: mutating only if the live #167/#212 allocation remains valid; otherwise reconcile, do not restart.
- Server Seam Lead: read-only preparation until the durable adapter is terminally merged and fresh allocation exists.
- Client/QA Lead: read-only preparation until a compatible production Server Seam is merged.
- Auditor: read-only continuous checkpoint review.
- Active control plane: scheduling, ownership, shared leases and deterministic integration only.

### Wave V1 — after Durability

Server Seam becomes the primary mutating lane after fresh allocation. Client/QA continues read-only preparation. Movement may inspect #139 readiness only.

### Wave V2 — after Server Seam

Client/QA becomes mutating after exact allocation and must obtain truthful Tier 1/Tier 2 evidence. Movement prepares the exact #139 gate.

### Wave V3 — Movement

After Client/QA readiness and terminal #139 closure, Movement becomes the primary mutating gameplay lane. Combat remains preparation-only.

### Wave V4 — Combat

After Movement and all current prerequisites merge, Combat becomes the primary mutating lane. Persistence/value/item gaps outside accepted authority escalate before code changes.

## Post-VSL full-project expansion

The first Movement+Combat VSL is not the whole Game project. After terminal VSL closeout, `Oteryn: sol post-vsl expansion` reconstructs the remaining accepted backlog and decomposes it into exact dependency-aware waves.

Expected lane families, subject to live accepted architecture and Issues, include:

- World/Content: OTBM migration, canonical world, full content pipeline, asset/presentation/runtime bundle work;
- NPC/AI: full spawn, path, perception, NPC interaction and behavior breadth;
- Player Systems/Economy: itemization, quests, rewards, crafting/economy and later social/product systems where accepted;
- Native Client/Renderer: complete map/world presentation, HUD/interaction breadth and performance;
- Tooling/Operations: authoring/migration tools, observability, deployment-readiness and operational evidence under separate production authority.

These are decomposition families, not write authority. Four reusable preparation profiles are packaged now because Issue #213 requires them explicitly: `Oteryn: sol world content prep`, `Oteryn: sol npc ai prep`, `Oteryn: sol systems economy prep`, and `Oteryn: sol tooling ops prep`. They remain read-only after VSL until an exact later allocation is merged; they may prepare allocation proposals but may not create their own write authority. The expansion lead creates exact child Issues/plans/implementation prompts only after current architecture proves scope and ownership. Terra never invents the next technical lane from this list.

## Evidence contract

Every Sol lead returns:

```yaml
lane:
issue:
task_id:
admission_main_sha:
integration_main_sha:
branch:
pr:
final_head_sha:
changed_paths: []
shared_lease_used: null
state:
focused_validation: []
component_validation: []
e2e: PASS | FAIL | BLOCKED | NOT_APPLICABLE
self_review:
independent_review:
architecture_escalation: null
unresolved_findings: []
recommended_control_plane_action: integrate | return_to_lane | wait | escalate
next_action: <exactly one concrete action>
```

The active control plane verifies every factual field against GitHub before acting.

## Prompt aliases

Control and architecture:

```text
Oteryn: terra game coordinator
Oteryn: sol supervising architect
Oteryn: work auditor
```

Current critical-path Sol leads:

```text
Oteryn: sol durability lead
Oteryn: sol server seam lead
Oteryn: sol client qa lead
Oteryn: sol movement lead
Oteryn: sol combat lead
```

After VSL:

```text
Oteryn: sol post-vsl expansion
```

Direct legacy `Oteryn: impl ...` aliases remain bounded executor/recovery prompts and do not gain authority from this design.

## Success criteria

This execution architecture is ready for canonical use only when:

- the Terra coordinator prompt encodes zero technical discretion;
- Work and Terra reusable control-plane profiles are mutually exclusive for programme mutation and transfer is durable/fail-closed;
- the Sol Supervising Architect and all five current lane-lead prompts are registered;
- the scheduler makes current and dependency-triggered launches explicit;
- Sol leads cannot write without exact merged allocation;
- Sol leads cannot seize shared surfaces;
- Terra cannot resolve technical or architecture ambiguity;
- the independent auditor remains separate;
- exact-head governance/prompt evaluation passes;
- this authority-boundary change receives genuinely independent exact-head review before merge.

Project completion remains evidence-based: each accepted product lane must be terminal on protected `main` with applicable tests/E2E and lifecycle closeout. Production/live deployment and Reference parity remain separate claims requiring their own authority and evidence.