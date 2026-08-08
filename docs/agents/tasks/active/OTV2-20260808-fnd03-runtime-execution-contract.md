# OTV2-20260808-fnd03-runtime-execution-contract

```yaml
task_id: OTV2-20260808-fnd03-runtime-execution-contract
title: Define FND-03 authoritative runtime execution contract
mode: CONTRACT
status: investigating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd03-runtime-execution-contract
pr: null
base_sha: b85bdd3f278d9de12284eab7c6352219325b3751
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: OpenAI architecture continuation agent
created_at: 2026-08-08T19:05:00+02:00
updated_at: 2026-08-08T19:05:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/agents/tasks/active/OTV2-20260808-fnd03-runtime-execution-contract.md
public_contracts:
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
depends_on:
  - ADR-0001-native-rust-multichannel-platform.md
  - ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md
  - FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
  - FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md
  - FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md
  - FND-ID-01_NODE_ID_PROCESS_INCARNATION_OWNER_BASELINE.md
  - DISCONNECT_LIVENESS_AND_CRASH_EVIDENCE_OWNER_BASELINE.md
  - DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md
  - DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md
  - DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md
  - DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md
  - RESOURCE_LIMITS_REGISTRY.json
  - FOUNDATION_ERROR_VOCABULARY.md
  - FOUNDATION_FAILURE_SCENARIOS.md
blocks:
  - authoritative Rust game runtime implementation claims
  - safe integration of FND-02 command ingress with channel/instance execution
  - FND-04 final reconnect/admission state machine where it depends on runtime liveness and ownership boundaries
  - runtime-dependent vertical-slice implementation packages
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Freeze the minimum runtime execution semantics needed to implement the authoritative Rust GameNode safely after FND-02 without selecting benchmark-sensitive implementation details prematurely.

The contract must define logical ownership and mutation boundaries for `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime`; command/timer/lifecycle ordering; clock authority; bounded queue and overload semantics; auxiliary parallel-work return; stale-result fencing; liveness/disconnect timer integration; draining/checkpoint/recovery cuts; deterministic evidence/replay requirements; and the exact boundary between FND-03 and FND-04/DUR/ANL/OPS/PERF work.

No runtime code, protocol implementation, persistence schema, Platform change or production deployment is authorized by this task.

## Architecture and source of truth

- `PROVEN` — `main@b85bdd3f278d9de12284eab7c6352219325b3751` has completed FND-02 and the reconnect/re-entry clarification lifecycle; live open-PR scan found no competing PR before this task began.
- `PROVEN` — ADR-0001 and ADR-0009 require one logical authoritative mutation owner per channel while permitting multithreaded auxiliary work and several channels per GameNode.
- `PROVEN` — the instance baseline requires one logical authoritative `InstanceRuntime` owner after committed admission/handoff and forbids source channels from retaining mutation authority.
- `PROVEN` — FND-02 fixes `(GameSessionId, CommandId)` ordering/idempotency, `connection_generation`, `server_sequence`, snapshot/delta/resync and the 64-command outstanding window; FND-03 must consume rather than redefine those wire semantics.
- `PROVEN` — disconnect protection uses server-authoritative liveness and monotonic elapsed-time boundaries; client/OS diagnostics remain corroborating only.
- `PROVEN` — current resource policy requires bounded resources but does not yet contain numeric FND-03 runtime queue/worker/timer ceilings.
- `DERIVED` — queue topology and overload semantics must be fixed now, while benchmark-sensitive numeric internal capacities should be registered before implementation acceptance from bounded spike/performance evidence rather than guessed in this architecture package.

## Acceptance criteria

### Runtime ownership

- [ ] Define `NodeRuntime`, `WorldServices`, `ChannelRuntime` and `InstanceRuntime` responsibilities without making process placement semantic ownership.
- [ ] Preserve exactly one logical authoritative mutation owner for each active channel and concrete instance.
- [ ] Separate canonical identities (`NodeId`, `ChannelId`, `InstanceId`) from runtime-local handles and ownership/fencing generations.
- [ ] Define ownership activation, invalidation and stale-generation rejection boundaries.

### Ordering and execution

- [ ] Consume FND-02 per-session `CommandId` ordering without introducing a competing client-command sequence.
- [ ] Define the authoritative channel/instance commit boundary for commands, timers, lifecycle events, service completions and auxiliary results.
- [ ] Define cross-session ordering evidence without pretending operating-system thread arrival order is deterministic gameplay semantics.
- [ ] Prevent later reserved commands from committing authoritative mutation ahead of earlier reserved commands where FND-02 forbids it.
- [ ] Define deterministic timer ordering, cancellation and stale-timer behavior.

### Clocks and liveness

- [ ] Separate wall-clock correlation, monotonic elapsed time and authoritative simulation/order domains.
- [ ] Ensure mutable wall-clock/NTP changes cannot alter gameplay deadlines, reconnect protection or liveness timers.
- [ ] Preserve accepted `2 s` disconnect-protection, `5 s` stale-transport cleanup, `15 s` logical reconnect-grace input and `4 s` re-entry protection boundaries without moving FND-04-owned eligibility semantics into FND-03.
- [ ] Define deterministic/virtual test clock requirements.

### Parallelism and queues

- [ ] Forbid network/database/service I/O and blocking auxiliary work from directly blocking or mutating the authoritative writer.
- [ ] Define immutable auxiliary-work input, source revision/generation/deadline tagging and writer-side revalidation.
- [ ] Require bounded queues/executors and deterministic overload/backpressure behavior for every work class.
- [ ] Preserve accepted/reserved commands from silent loss while allowing explicitly best-effort telemetry to drop only under ADR-0006 policy.
- [ ] Define slow-client behavior that prevents unbounded outbound state growth and converges to bounded resync/disconnect behavior.
- [ ] State which numeric internal capacities are benchmark-sensitive and therefore require registered concrete limits before implementation acceptance rather than guessed architecture constants.

### Lifecycle and recovery

- [ ] Define startup/readiness/drain/checkpoint/fence/recovery execution cuts consistent with ADR-0009.
- [ ] Require fresh `NodeId` per process incarnation and separate current channel/instance ownership generation.
- [ ] Define safe checkpoint ordering boundary without selecting DUR-02 storage format/RPO/RTO.
- [ ] Define stale worker/timer/transport/session evidence rejection after recovery generation changes.
- [ ] Preserve same-channel recovery and no silent channel hopping after failure.

### Evidence and downstream boundaries

- [ ] Define deterministic replay/evidence requirements so live thread scheduling is never the only explanation of an authoritative result.
- [ ] Classify applicable foundation failure scenarios and explicitly defer those owned by FND-04, DUR, ANL, PERF or OPS.
- [ ] Map public/cross-component runtime failures to the foundation error vocabulary without inventing unstable implementation messages.
- [ ] Keep Launcher/Guardian topology, concrete heartbeat cadence, persistence schema, async runtime/library, worker counts, CPU affinity, orchestrator product, tick frequency and production capacity outside this contract unless concrete downstream safety requires otherwise.
- [ ] Synchronize `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` transition-safely without claiming runtime implementation.

### Governance

- [ ] Full changed-path architecture review finds zero unresolved material contradictions with accepted ADRs/contracts.
- [ ] Exact-head Agent governance and applicable repository checks pass.
- [ ] Independent exact-head architecture/security audit passes with zero open material findings.
- [ ] Squash merge only after all required gates pass; task lifecycle is archived separately after delivery merge.

## Excluded scope

This contract does not:

- implement Rust runtime code or create speculative workspace crates;
- choose Tokio or any other concrete async/runtime/worker library;
- choose OS thread count, CPU pinning/affinity, worker count or fixed tick frequency;
- define FND-04 token/session/lease/reconnect credential construction or final state-machine ownership;
- define PostgreSQL schema, transaction isolation, checkpoint encoding, RPO/RTO or durable replay technology;
- define ANL-01 event schemas or production telemetry backend;
- define PERF-01 player-capacity numbers or OPS-CHANNEL-01 orchestrator/deployment topology;
- define gameplay-specific movement/combat/AI tie-breaking beyond the generic authoritative execution framework;
- authorize client/OS diagnostics, Launcher/Guardian implementation, production deployment or external-repository writes.

## Implementation / findings

Initial analysis confirms that runtime correctness depends on three separate concepts that must not be overloaded:

```text
canonical identity
+ current ownership/fencing generation
+ current physical/process execution placement
```

The same semantic channel or instance may survive recovery while placement and generation change. A new `NodeId` identifies a new process incarnation but grants no channel/instance authority by itself.

FND-03 will therefore define a logical single-writer commit lane per authoritative simulation owner and make all concurrency return through that boundary, while leaving concrete executor/thread technology to later measured implementation.

## Validation

### Focused

- architecture-source review: in progress
- governance consistency: pending

### Component/integration

- result: `NOT_APPLICABLE`; architecture-only contract task changes no runtime component.

### E2E

- scenario: `NOT_APPLICABLE`; no executable runtime capability is introduced by this task.
- result: `NOT_APPLICABLE`.

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none at task start
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: FND-03 started from clean main after reconnect clarification closeout; mandatory runtime/identity/liveness/instance/failure inputs were reviewed and the bounded contract task now owns only the runtime contract, current-status synchronization and its task record.
status: investigating
branch: docs/OTV2-20260808-fnd03-runtime-execution-contract
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Draft FND-03_RUNTIME_EXECUTION_CONTRACT.md from the accepted inputs without freezing benchmark-sensitive implementation technology or guessed numeric capacities.
```
