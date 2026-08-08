# OTV2-20260808-fnd03-runtime-execution-final

```yaml
task_id: OTV2-20260808-fnd03-runtime-execution-final
title: Finalize FND-03 authoritative runtime execution contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd03-runtime-execution-final
pr: 102
base_sha: a931c54e7e32b2cea370317ce88896a18eed8ccb
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T19:28:00+02:00
updated_at: 2026-08-08T19:35:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md
  - docs/agents/tasks/active/OTV2-20260808-fnd03-runtime-execution-final.md
public_contracts:
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
depends_on:
  - docs/architecture/FND-03_AUTHORITATIVE_RUNTIME_EXECUTION_ANALYSIS_BASELINE.md
  - docs/architecture/FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md
  - docs/architecture/ADR-0001-native-rust-multichannel-platform.md
  - docs/architecture/ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md
  - docs/architecture/FND-ID-01_FOUNDATION_IDENTIFIER_CONTRACT.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md
  - docs/architecture/DISCONNECT_LIVENESS_AND_CRASH_EVIDENCE_OWNER_BASELINE.md
  - docs/architecture/DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md
  - docs/architecture/DISCONNECT_CLIENT_OS_FORENSICS_PRIVACY_TIMING_REFINEMENT.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
  - docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
blocks:
  - authoritative Rust GameNode runtime implementation claims
  - runtime-dependent vertical-slice implementation packages
  - FND-04 final integration where session/reconnect semantics depend on runtime liveness and fencing interfaces
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Deliver the complete architecture-only `FND-03 Runtime Execution Contract` from the two accepted analysis baselines now canonical on `main`.

The final contract freezes the runtime semantics required before implementation: ownership/lifecycle boundaries, authoritative input linearization, clocks/timers, auxiliary-work revalidation, bounded overload behavior, recovery/fencing/checkpoint cuts, liveness integration, deterministic randomness/replay, failure/error disposition and downstream ownership. It deliberately does not choose benchmark-sensitive runtime technology or numeric capacities without evidence.

Acceptance of this contract completes the FND-03 architecture gate only. It does not itself authorize Rust runtime implementation, production traffic or deployment.

## Architecture and source of truth

- **PROVEN:** `main@a931c54e7e32b2cea370317ce88896a18eed8ccb` contains both merged FND-03 analysis baselines and their lifecycle closeout.
- **PROVEN:** no open pull requests existed when this final package began; later duplicate lifecycle PR #101 was closed unmerged as superseded by canonical closeout #100.
- **PROVEN:** closed PR #99 is a non-authoritative premature draft and is not accepted FND-03 text.
- **PROVEN:** FND-02 owns per-GameSession `(GameSessionId, CommandId)`, connection generation, server sequence and snapshot/delta/resync semantics; this contract consumes rather than redefines them.
- **PROVEN:** one logical mutation owner per ChannelRuntime/InstanceRuntime is already accepted; process/thread placement is not semantic authority.
- **DERIVED AND NOW FROZEN BY CANDIDATE CONTRACT:** one owner-scoped `RuntimeExecutionOrdinal` is sufficient as the mandatory runtime input linearization/evidence value when one normalized authoritative input resolves without interleaving; domain/state revisions remain committed-state authority, so separate mandatory RuntimeInputOrdinal/RuntimeCommitOrdinal counters are not required.
- **DERIVED AND NOW FROZEN BY CANDIDATE CONTRACT:** scope ownership generation is distinct from NodeId and cannot be self-granted by a newly started GameNode.
- **UNKNOWN / deliberately deferred:** concrete async runtime, worker topology/counts, CPU affinity, global tick/quantum, benchmark-sensitive queue/timer/worker limits, checkpoint storage/RPO/RTO and orchestrator product.

## Acceptance criteria

### Final ownership and ordering semantics

- [x] Freeze final responsibilities for NodeRuntime, WorldServices, ChannelRuntime and InstanceRuntime.
- [x] Freeze canonical identity versus scope-ownership generation versus NodeId placement separation.
- [x] Define who may establish a newer ownership generation semantically without selecting an orchestrator product: only the accepted scope-assignment/ownership authority may establish a grant; NodeRuntime cannot self-promote.
- [x] Freeze one owner-scoped `RuntimeExecutionOrdinal` and supersede analysis-only dual RuntimeInputOrdinal/RuntimeCommitOrdinal terminology.
- [x] Define non-interleaved owner resolution for commands, timers, control/fencing events, service completions and auxiliary results.
- [x] Define non-reuse/exhaustion behavior for runtime ordering and scope ownership generation without inventing an unnecessary public wire encoding.

### Time, timers and liveness

- [x] Freeze wall-clock versus process-local monotonic time versus authoritative execution-order semantics.
- [x] Prohibit persistence/comparison of opaque process-local monotonic instants across NodeId replacement.
- [x] Define durable/recoverable timers as domain semantic state owned by gameplay/DUR contracts and reconstructed onto a fresh local monotonic schedule.
- [x] Freeze owner-scoped timer scheduling, equal-deadline ordering, stale/cancellation behavior and explicit bounded catch-up taxonomy.
- [x] Freeze the FND-03/FND-04 liveness split: FND-04 decides sufficient current-generation evidence/session eligibility/15-second semantics; FND-03 measures monotonic elapsed time and executes accepted 2-second, 5-second and 4-second runtime effects.

### Queues, dependencies and recovery

- [x] Freeze required bounded queue/pending/work classes and post-reservation no-silent-drop rule.
- [x] Require concrete numeric runtime limits in `RESOURCE_LIMITS_REGISTRY.json` before implementation acceptance, backed by protocol/safety/benchmark/spike evidence rather than guessed in this architecture contract.
- [x] Freeze bounded slow-client resync/transport-close direction, scope fairness and control/fence non-starvation.
- [x] Freeze asynchronous dependency pending/revalidation behavior and prohibit blocking remote/database/expensive CPU work inside the authoritative mutation lane.
- [x] Freeze lifecycle activation/drain/checkpoint/fence/recovery cuts while leaving persistence encoding/journal/RPO/RTO to DUR/OPS owners.
- [x] Preserve same semantic ChannelId/eligible InstanceId across recovery while NodeId changes and scope ownership generation advances.

### Determinism, failure and downstream boundaries

- [x] Freeze deterministic authoritative gameplay RNG requirements and separation from cryptographic/security randomness.
- [x] Freeze deterministic simulation replay evidence and prohibit analytics/event replay from replaying authoritative gameplay mutation.
- [x] Produce complete FND-03 disposition for every foundation failure scenario with `PASS`, `DEFERRED_BY_ACCEPTED_GATE` or `NOT_APPLICABLE` and explicit later owner.
- [x] Map runtime-facing failures to `FOUNDATION_ERROR_VOCABULARY.md` without exposing implementation strings as public behavior.
- [x] Freeze downstream ownership with FND-04, DUR-*, ANL-*, PERF-01, OPS-CHANNEL-01 and gameplay/client contracts.
- [x] Synchronize `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` transition-safely for PR #102 and post-merge interpretation.

### Governance

- [x] No runtime code/workspace/production implementation is introduced by this package.
- [ ] Full final changed-path architecture review finds zero unresolved material conflicts.
- [ ] Exact-head Agent governance and applicable repository checks pass.
- [ ] Independent exact-head architecture/security audit passes with zero open material findings.
- [ ] Squash merge only after all final-head gates pass; archive/release ownership in a separate closeout PR.

## Excluded scope

This task does not:

- implement GameNode/ChannelRuntime/InstanceRuntime Rust code or create speculative runtime crates;
- select Tokio/another runtime, worker/thread counts, CPU affinity, fixed global tick frequency or scheduler quantum;
- guess numeric queue/timer/worker capacities;
- define PostgreSQL schema, transaction isolation, checkpoint encoding, journal technology, RPO/RTO or backup policy;
- define FND-04 credentials/tokens/lease storage/duplicate-login state machine;
- define ANL-01 event schema/outbox implementation;
- select Kubernetes/Docker/systemd or another orchestrator product;
- define gameplay-specific combat/movement/AI tie-break formulas beyond the generic owner execution framework;
- authorize client diagnostics, Launcher/Guardian implementation, production traffic/deployment or external-repository writes.

## Implementation / findings

The candidate final contract resolves the material analysis ambiguities as follows:

1. **Runtime order:** one `RuntimeExecutionOrdinal` linearizes every normalized authoritative input accepted by one scope ownership generation. It is distinct from CommandId, server_sequence and domain state revisions.
2. **Commit evidence:** one owner input resolves non-interleaved; resulting zero-or-more deterministic domain mutations advance their existing state revisions. A second mandatory runtime-wide commit counter adds no required safety invariant and is rejected.
3. **Ownership authority:** a new NodeId is process identity only. NodeRuntime cannot self-grant channel/instance authority; an accepted scope ownership authority establishes a newer non-reused generation, with physical grant/fence distribution owned later by OPS/DUR.
4. **Durable time:** process monotonic instants never cross process lifetime. A timer that survives restart is represented as owning domain semantic state with explicit downtime/catch-up semantics and reconstructed after recovery.
5. **Liveness split:** FND-04 decides which current-generation evidence is sufficient and owns logical reconnect/15-second semantics; FND-03 timestamps accepted progress locally and executes 2-second PvE disconnect protection, five-second stale concrete transport cleanup timing and four-second post-authorized re-entry protection.
6. **Overload:** every runtime queue/pending set is bounded. Numeric hard maxima are a mandatory implementation-acceptance artifact in the resource registry, not guessed architecture constants.
7. **Failure containment:** unexpected authoritative panic/invariant failure is fail-stop for the affected scope; failure escalates to GameNode replacement when safe isolation cannot be proven.
8. **Replay:** deterministic simulation replay reconstructs accepted normalized order/clocks/RNG/revisions and never relies on original CPU/thread scheduling; analytics replay cannot execute gameplay effects.

No material contradiction with FND-02 or the two merged FND-03 analysis baselines has been identified in the pre-freeze review.

## Validation

### Focused

- accepted-analysis reconciliation: `PASS` at pre-freeze review; final exact-head diff audit still required after task/status synchronization.
- contract scope: exactly one new FND-03 contract plus current-status and active-task synchronization.

### Component/integration

- result: `NOT_APPLICABLE` — architecture-only contract delivery.

### E2E

- result: `NOT_APPLICABLE` — no executable capability introduced; implementation evidence requirements are defined in the contract for the later implementation task.

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending after final synchronization
- result: pending

## Independent audit

- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- delivery PR: 102
- changed-file review: pending final exact head
- unresolved review threads: pending
- related/superseded PRs: #99 and #101 closed unmerged as superseded; neither is authority
- merge policy: squash after exact-head validation
- merge commit/result: pending
- ownership release: pending separate closeout

## Context checkpoint

```yaml
last_progress: Final FND-03 contract and transition-safe foundation status are now drafted in PR #102; all remaining semantic analysis items are resolved without selecting runtime technology or guessed numeric capacities.
status: validating
branch: docs/OTV2-20260808-fnd03-runtime-execution-final
head_sha: null
pr: 102
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
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Freeze PR #102 head, perform full three-path exact-head architecture audit and require fresh Agent governance/Dependency review/CodeQL before squash merge.
```
