# OTV2-20260808-fnd03-runtime-ordering

```yaml
task_id: OTV2-20260808-fnd03-runtime-ordering
title: FND-03 authoritative runtime ordering analysis
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd03-runtime-ordering
pr: 98
base_sha: b85bdd3f278d9de12284eab7c6352219325b3751
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T19:06:00+02:00
updated_at: 2026-08-08T19:21:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260808-fnd03-runtime-ordering.md
  - docs/architecture/FND-03_AUTHORITATIVE_RUNTIME_EXECUTION_ANALYSIS_BASELINE.md
  - docs/architecture/FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md
public_contracts:
  - docs/architecture/FND-03_AUTHORITATIVE_RUNTIME_EXECUTION_ANALYSIS_BASELINE.md
  - docs/architecture/FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md
depends_on:
  - FND-ID-01 accepted and merged
  - FND-02 accepted and merged
  - ADR-0009 accepted GameNode execution/capacity/recovery baseline
  - merged disconnect/re-entry clarification package from PR #96
blocks:
  - FND-03 Runtime Execution Contract completion
  - authoritative runtime implementation claims
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform (read-only)
  - blakinio/Otheryn (read-only)
  - blakinio/otclient (read-only)
```

## Outcome

Persist the owner-directed FND-03 runtime-execution analysis as one bounded package with two companion architecture baselines. The first baseline resolves the authoritative execution/ordering kernel; the second extends that analysis into NodeRuntime/WorldServices responsibilities, lifecycle, timer catch-up, noisy-neighbor fairness, failure containment, dependency delay/loss, deterministic randomness/replay, event cut points and foundation failure-scenario ownership.

The package remains analysis input to the later complete FND-03 contract. It does not itself claim that FND-03 is accepted or that runtime implementation is authorized, and it deliberately avoids premature selection of benchmark-sensitive libraries, worker counts, tick rates, queue capacities, orchestration products or persistence technology.

## Architecture and source of truth

- **PROVEN:** `main` at task start is `b85bdd3f278d9de12284eab7c6352219325b3751`.
- **PROVEN:** no open pull requests existed in `blakinio/Oteryn-v2` at task start.
- **PROVEN:** FND-ID-01 and FND-02 are accepted and merged; FND-03 is the next ordered foundation gate according to `FOUNDATION_PROGRAMME_CURRENT_STATUS.md`.
- **PROVEN:** ADR-0009 requires a multithreaded GameNode with exactly one logical authoritative mutation owner per ChannelRuntime and generation/revision validation for auxiliary work.
- **PROVEN:** `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md` requires one logical authoritative mutation owner for each concrete InstanceRuntime and explicitly defers the exact ChannelRuntime/InstanceRuntime/WorldServices/GameNode relationship to FND-03.
- **PROVEN:** FND-02 freezes per-GameSession CommandId ordering and bounded ingress but does not define a complete world-simulation time model.
- **PROVEN:** the reconnect/disconnect owner baselines require server-authoritative timing and forbid trusting client-supplied timestamps for reconnect/protection decisions.
- **PROVEN:** `RESOURCE_LIMITS_REGISTRY.json` requires bounded concrete limits before implementation acceptance but does not currently provide evidence-backed FND-03 internal queue/timer/worker capacities.
- **DERIVED:** ChannelRuntime and InstanceRuntime should share one authoritative execution abstraction and differ by owned state scope rather than by scheduler semantics.
- **DERIVED:** CommandId ordering, runtime input order, authoritative mutation/commit order, state revisions, protocol server sequence, monotonic deadlines and wall-clock time must remain distinct concepts unless a later contract proves exact semantic equivalence.
- **DERIVED:** cross-session and async-completion ordering should linearize at the current authoritative owner and retain enough normalized-order evidence for deterministic replay rather than pretending OS thread wake-up order is a gameplay contract.
- **DERIVED:** NodeRuntime is process-incarnation supervision/execution hosting, while WorldServices is a typed boundary to independently owned world-shared domains; co-location does not merge semantic authority.
- **DERIVED:** lifecycle authority, timer catch-up taxonomy, bounded per-scope fairness and fail-stop behavior for unexpected authoritative invariant failure should be explicit before implementation.
- **UNKNOWN:** final runtime library, worker topology, simulation quantum, queue capacities, CPU affinity, checkpoint interval, RPO/RTO, exact runtime-local counter widths and final replay artifact/storage shape remain deliberately unresolved until their owning contract or benchmark evidence exists.

Canonical inputs:

- `docs/architecture/ADR-0001-native-rust-multichannel-platform.md`
- `docs/architecture/ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md`
- `docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md`
- `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md`
- `docs/architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`
- `docs/architecture/FND-ID-01_NODE_ID_PROCESS_INCARNATION_OWNER_BASELINE.md`
- `docs/architecture/FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md`
- `docs/architecture/LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md`
- `docs/architecture/DISCONNECT_LIVENESS_AND_CRASH_EVIDENCE_OWNER_BASELINE.md`
- `docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md`
- `docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md`
- `docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`
- `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`
- `docs/contracts/FOUNDATION_ERROR_VOCABULARY.md`
- `docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md`
- `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`

## Acceptance criteria

### Ordering and ownership baseline

- [x] Persist the current owner-directed runtime-execution conclusions in one bounded architecture baseline.
- [x] Separate CommandId order, runtime input order, authoritative mutation/commit order, monotonic deadlines and wall-clock timestamps explicitly.
- [x] Define one logical authoritative execution model usable by ChannelRuntime and InstanceRuntime.
- [x] Define auxiliary-work proposal/stale-result rules without allowing worker threads to mutate authoritative state directly.
- [x] Define ownership-generation and NodeId separation for crash/replacement safety.
- [x] Define queue/backpressure classes and fail-open/fail-closed principles without guessing benchmark-sensitive numeric capacities.
- [x] Analyse deterministic ordering for concurrent commands, timers, worker completions and system/runtime events.
- [x] Apply the mandatory architecture decision timing test to material recommendations.

### Lifecycle/failure/replay companion

- [x] Define NodeRuntime as process-incarnation supervision/execution hosting without making it a second gameplay/world authority.
- [x] Define WorldServices as typed access to explicit world-shared owners rather than an untyped process-global singleton.
- [x] Preserve and refine ADR-0009 lifecycle vocabulary with authority/readiness semantics per state.
- [x] Define an explicit bounded timer catch-up/coalescing taxonomy rather than implicit unlimited missed-tick replay.
- [x] Define non-starvation/bounded-yield requirements across hosted authoritative scopes without freezing scheduler implementation.
- [x] Define fail-stop containment for unexpected authoritative panic/invariant failure and wider GameNode failure when corruption cannot be isolated.
- [x] Define asynchronous dependency pending/revalidation direction without allowing remote/database I/O to block authoritative mutation execution.
- [x] Define deterministic-authoritative-randomness/replay separation from analytics event replay and retain Game Intelligence read-only authority boundaries.
- [x] Map applicable foundation failure scenarios to FND-03 expected behavior or the accepted downstream owner.
- [x] Enumerate FND-03 resource-limit categories that need concrete registered maxima before implementation acceptance while deliberately not guessing the numbers.

### Governance

- [x] Keep runtime implementation, library selection and production activation explicitly unauthorized.
- [ ] Complete documentation/governance validation and an independent architecture audit on the final unchanged head before merge.

## Excluded scope

This task must not:

- implement Rust runtime code, protocol listeners/codecs, Game Session admission, leases, persistence or production diagnostics;
- select Tokio or another server async runtime as a binding FND-03 choice;
- freeze worker counts, CPU affinity, exact simulation tick/quantum, queue capacities, checkpoint interval, RPO/RTO or orchestrator product without evidence;
- redefine FND-02 wire semantics or CommandId ordering;
- redefine FND-04 admission/reconnect credential or lease semantics;
- write to external repositories;
- authorize a mandatory Launcher/Guardian process or heartbeat;
- claim runtime, multichannel or E2E implementation evidence from this documentation-only package.

## Implementation / findings

The persisted analysis now establishes the recommended contract direction:

```text
multithreaded GameNode
-> multiple independent scopes may execute concurrently
-> one logical ordered owner per ChannelRuntime/InstanceRuntime
-> logical writer is not a dedicated OS-thread contract
-> NodeRuntime supervises process/runtime capacity, not gameplay semantics
-> WorldServices exposes typed world-domain owners, not shared mutable globals
-> CommandId order stays FND-02-owned
-> normalized runtime inputs are linearized at the authoritative owner
-> monotonic deadlines are distinct from wall clock and protocol sequencing
-> mutation-capable timers re-enter through the owner
-> timer families declare bounded catch-up/coalescing semantics
-> worker/service completions are proposals/results, not mutation callbacks
-> stale generation/revision/local-handle work fails closed
-> control/fence work cannot be starved by ordinary gameplay backlog
-> hosted scopes receive bounded scheduling opportunity; one noisy scope cannot starve unrelated scopes
-> all queue classes are bounded with explicit overload behavior
-> benchmark-sensitive numeric capacities wait for measured implementation evidence
-> unexpected authoritative invariant failure is fail-stop for the affected scope unless safe isolation cannot be proven, in which case the GameNode fails stop
-> external dependency waits are explicit asynchronous pending operations with generation/revision revalidation
-> replacement process gets a fresh NodeId while semantic channel/instance identity may survive under a newer ownership generation
-> deterministic simulation replay uses normalized inputs/clocks/randomness and authoritative order evidence rather than original CPU/thread interleaving
-> analytics/event replay cannot replay gameplay mutation
```

The analysis preserves a modular-monolith initial placement: one GameNode may host multiple channels/instances and in-process WorldServices integration, while semantic ownership remains explicit and independently extractable later.

A parallel final-contract PR #99 was created after this earlier task was already authoritative. It was closed unmerged as `SUPERSEDED / PREMATURE` so the final FND-03 contract cannot outrun its own analysis gate. No #99 content is canonical.

One synchronization issue was found when the lifecycle/failure/replay companion appeared on the branch: the active task still declared only the original two owned paths. This task update claims the third companion path and its public architecture artifact before final validation; no runtime scope or authority is expanded.

## Validation

### Focused

- changed-path review: pending final exact-head audit after companion-path synchronization
- accepted-input consistency review: in progress; no material architecture contradiction identified so far

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only package; no runtime component changes are authorized.
- result: `NOT_APPLICABLE`.

### E2E

- scenario: `NOT_APPLICABLE` — this task changes architecture documentation only and cannot prove executable runtime behavior.
- result: `NOT_APPLICABLE`.

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending for post-synchronization head
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending after companion-path synchronization
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending final head
- unresolved review threads: pending
- related/superseded PRs: PR #99 closed unmerged as a later duplicate/premature final-contract package
- protected auto-merge: not requested
- merge policy: squash after exact-head validation and zero open material findings
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PR #98 now owns two companion FND-03 analysis baselines: the authoritative execution/ordering kernel plus lifecycle/failure/replay continuation covering NodeRuntime/WorldServices, scope lifecycle, timer catch-up, fairness, fail-stop containment, dependency loss, replay/randomness, resource classes and failure-scenario ownership; duplicate #99 remains closed unmerged.
status: validating
branch: docs/OTV2-20260808-fnd03-runtime-ordering
head_sha: null
pr: 98
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending-final-head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze the new PR #98 head, audit all three changed paths against accepted FND-03 inputs, and require fresh exact-head repository checks before squash merge.
```
