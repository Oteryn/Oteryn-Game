# Oteryn Game Post-Blocker Work Orchestration Architecture

- Status: owner-directed execution architecture; canonical only after merge to protected `main`
- Date: 2026-08-25
- Repository: `Oteryn/Oteryn-Game`
- Governing Issue: #154
- Coordination task: #155
- Authoring base: `main@2d9625a97d3e172303108aff21fcc61dae3e74fe`
- Purpose: move from the terminal next-wave blocker programme to a real gameplay vertical slice using ChatGPT Work as the bounded execution coordinator and an owner-designated Supervising Architect as the architecture escalation authority.

## Decision timing

**Must decide now?** `YES`.

**Concrete downstream work blocked without this decision:** exact allocation and coordinated execution of Durability, Ability, Interaction, AI, Server Seam, Client, Movement and Combat after the blocker programme closed.

**What becomes harder later if left ambiguous:** parallel workers can acquire overlapping paths, treat stale status prose as authority, invent cross-domain semantics to unblock themselves, or accidentally turn implementation convenience into protocol/schema/persistence/resource architecture. Those failures are expensive to unwind after several dependent PRs exist.

**Evidence that may justify superseding this architecture:** a later accepted repository/META orchestration contract that natively provides the same coordinator/subagent isolation and architecture-escalation semantics, a changed owner decision about execution hierarchy, or measured delivery evidence proving a different serialization/parallelism boundary is necessary.

**Deliberately not decided here:** production deployment topology, ports/certificates/secrets, permanent World Project/Bundle format, Reference parity/gameplay formulas, new protocol/stable numeric IDs, new cross-repository authority, final renderer/asset architecture, Movement-only hard maxima before #139 is activated, and later Channel/Analytics product scope.

## Verified starting state

The coordinator MUST re-read live GitHub before every allocation. These facts describe the authoring base and are not a substitute for live preflight.

- `PROVEN`: Bootstrap, bounded Simulation, Foundation and Domain are merged.
- `PROVEN`: FND-04 production verifier/consumer is merged through PR #151; the blocker programme #131 is terminal.
- `PROVEN`: #93, #115, #116 and #123 are completed; canonical first-slice registry delivery is PR #144 / `c1020b2db62ecfa18c411bee56fa004430b28923`.
- `PROVEN`: stale tracking Issue #141 was closed on 2026-08-25 because PR #144 already fulfilled its exact registry goal.
- `PROVEN`: Ability, Interaction and AI first-slice resource gates are closed, but implementation authority is not automatically allocated.
- `PROVEN`: first journal-only Durability resource gate is closed; the accepted topology selects SQLx 0.9.0 and a game-server-local Durability module, but runtime/DDL/migrations/Cargo writes still require an exact fresh allocation.
- `PROVEN`: Server Seam preparation #96 is complete and blocker Issues #115/#116 are closed; no production gameplay listener/client-entry implementation is merged yet.
- `PROVEN`: QA evidence shell Issue #91 is completed and PR #98 merged as `dc22e0da8efcc6f4458416191261063b295af5b4`; focused shell evidence is real, while physical gameplay Tier 1/Tier 2 remain `NOT_EVALUATED` until required production boundaries exist.
- `PROVEN`: Content evidence seam and its activation-fence repair are merged; production Content activation remains separately gated by Issue #54 and is not silently promoted by this architecture.
- `PROVEN`: Movement resource successor #139 is intentionally non-current until an exact Movement child plan plus Interaction, compatible Client and real QA integration readiness exist.
- `PROVEN`: Combat remains downstream of merged Movement plus Ability, Interaction, Durability, Client and QA readiness. AI may integrate when ready but is not a hard prerequisite for the first Combat slice under the current programme DAG.

## Execution hierarchy

```text
Owner
  |
  +-- Supervising Architect
  |     owns architecture interpretation and material architecture decisions
  |     does not perform routine lane implementation
  |
  +-- Work Delivery Coordinator (ChatGPT Work)
        owns execution coordination inside existing repository authority
        |
        +-- bounded lane subagents
        +-- review/integration queue
        +-- durable architecture-escalation packets
```

### Supervising Architect

The Supervising Architect is the owner-designated architecture session. For the current owner workflow this is the main ChatGPT architecture conversation, but the durable repository contract is role-based rather than tied to one transient chat.

The role owns only decisions that require architecture authority: accepted-contract interpretation, cross-domain ownership, public API/schema/protocol changes, persistence semantics, trust/security boundaries, resource-model decisions outside already accepted values, permanent Content format decisions and cross-repository boundary changes.

The role does not become a routine worker, CI poller or merge bookkeeper.

### Work Delivery Coordinator

Work is a stricter execution profile of the existing `OTV2_IMPLEMENTATION_COORDINATOR`; it does **not** receive additional repository, production or cross-repository authority.

Work may:

- resolve live GitHub state and current architecture/contracts;
- create exact bounded Game Issues/tasks/branches/PR allocations inside authority already granted by repository policy and owner direction;
- dispatch path-disjoint subagents with self-contained instructions;
- serialize shared paths, registries, stable IDs and Cargo/workspace mutations;
- review worker output, run required validation, integrate dependency-aware PRs and perform lifecycle closeout when existing policy permits;
- continue unrelated safe lanes while one lane is waiting for an architecture decision.

Work may not:

- change accepted architecture to make implementation easier;
- interpret missing resource maxima as permission to invent values;
- invent protocol/stable IDs, Reference formulas, persistence/value semantics or permanent Content format;
- authorize production/protected/live-data/secrets/deployment work;
- write Platform/Atlas/META or other repositories without separate exact owner authority;
- treat a worker's unmerged output as a dependency unless an explicit serialized predecessor relationship exists;
- claim that an architecture escalation has been delegated/resolved unless a durable response from the Supervising Architect is recorded.

### Lane subagents

Each mutating subagent receives exactly one independently reviewable domain/task. It owns one branch/worktree and only the paths in its merged allocation. A worker must never broaden its own scope because a dependency is inconvenient.

Every worker instruction must include:

- repository and exact admission `main` SHA;
- governing Issue/task and lane ID;
- branch and exact owned paths;
- accepted contracts/ADRs and exact prerequisite merge SHAs;
- explicit excluded scope;
- required focused/component/E2E/security/replay/fencing evidence;
- expected output: root cause/design implemented, changed paths, tests/checks, exact head/PR and any blocker/escalation packet.

## Architecture escalation contract

A worker or Work MUST emit `ARCHITECTURE_ESCALATION_REQUIRED` and stop the affected mutation before choosing a new architecture when any of these is true:

1. accepted ADRs/contracts conflict or do not define a material required semantic;
2. implementation requires changing a public API, wire/schema compatibility rule or stable protocol/event/resource identity beyond an already allocated compatible change;
3. persistence ownership, durable transaction semantics, migration guarantees or value/item semantics must change;
4. an externally influenced resource requires a new/changed hard maximum not already accepted for the exact slice;
5. authentication/session/reconnect/fencing/crypto/trust authority would move or weaken;
6. a cross-repository contract or write is required;
7. production port/certificate/key/secret/deployment topology must be chosen;
8. permanent Content/world-bundle format or Reference/gameplay product semantics must be selected;
9. two valid lane designs create a semantic ownership conflict that cannot be solved by ordinary path serialization;
10. resolving a blocker would require weakening a test, fail-closed behavior, provenance rule, review gate or safety boundary.

Routine compiler errors, ordinary implementation bugs, test failures with a local root cause, formatting/lint failures, authorized merge conflict resolution and transient CI/API failures are **not** architecture escalations.

### Required escalation packet

Persist one durable GitHub Issue or task comment containing:

```yaml
classification: ARCHITECTURE_ESCALATION_REQUIRED
repository: Oteryn/Oteryn-Game
main_sha: <verified live protected main>
issue: <affected task issue>
lane_id: <affected lane>
branch: <branch or null>
head_sha: <exact current head or null>
pr: <PR or null>
facts:
  proven: []
  derived: []
  unknown: []
  conflict: []
blocking_decision: <one precise decision>
governing_authority: []
affected_paths: []
affected_contracts: []
options_within_current_authority: []
options_rejected_by_authority: []
smallest_architect_decision_required: <one bounded question>
holding_action: <fail-closed reversible state>
paused_lanes: []
independent_lanes_continuing: []
```

No timestamp, run ID or narration may substitute for material evidence.

### Handoff mechanics

No automatic cross-conversation invocation is assumed. If the Work environment can directly invoke the owner-designated Supervising Architect through an authorized product mechanism, it may do so using the durable packet as the entire task input. Otherwise Work MUST persist the packet, mark the affected lane `WAITING_ARCHITECTURE`, surface the exact escalation ID to the owner, and stop that lane. It must not claim the Supervising Architect received or resolved the task until the resulting decision is durably recorded in GitHub.

This fallback is intentional: truthful durable handoff is preferred over pretending that two independent chat sessions can call each other.

## Post-blocker delivery DAG

The target is a first real vertical slice, not maximum module count.

```text
BASELINE RECONCILIATION
  -> verify merged QA shell and current live allocations

PARALLEL WAVE A (only after exact separate allocations)
  -> INTERACTION --------------------------+
  -> ABILITY ------------------------------+---------------------+
  -> AI (non-critical-path)                |                     |
  -> DURABILITY ---------------------------+------------------+  |
  -> SERVER SEAM -----------------> CLIENT + physical QA -----+--+--> MOVEMENT gate
                                                               |       |
                                                               |       +--> MOVEMENT
                                                               |              |
                                                               +--------------+--> COMBAT

Before MOVEMENT implementation:
  exact Movement child plan
    -> activate/close #139 resource rows
    -> serialized registry mutation if required
    -> Interaction + Client + real QA readiness
```

### Phase 0 — truthful baseline

Work first reconciles live Issue/PR/task state. The merged QA shell is not an active implementation branch. Physical Tier 1/Tier 2 are a future evidence obligation and require fresh exact allocations/journeys; they are not proof gaps to hide with synthetic tests.

### Phase 1 — parallel Wave A

Work may prepare and dispatch at most five substantial implementation workers concurrently when exact path allocations are disjoint:

- `OTV2-IMPL-GAME-INTERACTION` — critical prerequisite for Movement;
- `OTV2-IMPL-GAME-ABILITY` — required before Combat;
- `OTV2-IMPL-GAME-AI` — useful but not a first Movement/Combat hard dependency;
- `OTV2-IMPL-DURABILITY` — required before Combat;
- `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM` — critical prerequisite for Client and real server-side Tier 1.

The coordinator remains separate from the worker count. Shared/Cargo/registry/stable-ID mutations are serialized even when primary implementation paths are disjoint.

### Phase 2 — Server Seam -> Client -> physical QA

Client implementation is not released before the compatible production Server Seam is merged and exact-head validated. After Server Seam, QA may execute the real server-side Tier 1 connect/admit/reconnect/resync boundary. After the native Client boundary is merged, Tier 2 may execute through the native client. Neither tier may be marked PASS from the evidence shell alone.

### Phase 3 — Movement gate

Only when Interaction, compatible Client and real QA integration readiness exist does Work bind the exact first Movement child slice and activate Issue #139. Every Movement-exercised resource row must then be accepted/registered or explicitly excluded fail-closed before Movement implementation receives write authority.

Movement is a serial integration gate; do not run competing Movement implementations.

### Phase 4 — Combat gate

Combat is allocated only after merged Movement and integration-ready Ability, Interaction, Durability, Client and QA evidence. Foundation/SIM/Domain/Content predecessors are revalidated against live main. AI may be integrated if ready but does not block the first Combat slice solely for symmetry.

Combat is also a serial integration gate.

### Later/non-critical lanes

Production Content Issue #54, permanent format selection, Channel, Analytics and Game-owned Atlas farm-intelligence export #75 continue under their own authority/readiness. They do not become hidden prerequisites for the first gameplay vertical slice unless a live accepted contract proves the slice actually consumes them.

## Shared mutation and integration rules

The following are never parallel free-for-all surfaces:

- root/app Cargo manifests and `Cargo.lock`;
- `workspace-boundaries.toml` and architecture-check policy;
- stable protocol/event/resource registries;
- shared composition roots such as `apps/game-server/src/lib.rs`;
- public contracts/ADRs shared by multiple active lanes;
- workflow/protection/governance files.

Work allocates one serialized lease for an exact mutation, integrates it, releases it, then advances the next owner. A primary worker that discovers a need for an unowned shared file reports it; it does not grab the path.

Worker completion order never determines merge order. Work integrates only after prerequisite merges, exact-head refresh, invalidated validation reruns, whole-diff review and required independent review.

## Failure and waiting behavior

- A material architecture gap -> `ARCHITECTURE_ESCALATION_REQUIRED` / affected lane `WAITING_ARCHITECTURE`.
- External CI/reviewer/dependency with unchanged candidate -> `WAITING_EXTERNAL` when the central bounded-execution policy is canonical; until provider adoption merges, use existing repository fail-closed waiting semantics and never create no-op/retrigger commits.
- Missing permission/owner authority -> `BLOCKED_OWNER_AUTHORITY`.
- Repairable test/review finding -> keep task active and repair within allocation.
- Identical no-progress retries -> bounded; never loop by changing Git identity.

Unrelated lanes may continue only when their authority, files, dependencies and semantics are genuinely independent of the blocked lane.

## Completion criteria for the coordinated vertical slice

Work may report the post-blocker delivery programme complete only when:

1. all allocations used by the slice are terminally merged/closed and ownership released;
2. Server Seam and compatible native Client boundaries are merged;
3. real applicable Tier 1/Tier 2 evidence is recorded truthfully;
4. Interaction and the exact Movement resource gate #139 are terminal before Movement merge;
5. Movement is merged through required tests/review/exact-head CI;
6. Ability + Durability + Interaction + Client + QA prerequisites are integration-ready before Combat;
7. Combat is merged through required tests/review/exact-head CI;
8. no unresolved architecture escalation is silently carried as an assumption;
9. post-merge repository state and live allocation/status overlays are reconciled.

This is not a claim of Reference parity, production readiness or live deployment authority.