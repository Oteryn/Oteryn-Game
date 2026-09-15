# OTV2-20260808-fnd03-runtime-execution-final — archived

```yaml
task_id: OTV2-20260808-fnd03-runtime-execution-final
title: Finalize FND-03 authoritative runtime execution contract
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260808-fnd03-runtime-execution-final
pr: 102
base_sha: a931c54e7e32b2cea370317ce88896a18eed8ccb
head_sha: b1ccc90de58052f1bb61b705250b59507792f909
final_head_sha: b1ccc90de58052f1bb61b705250b59507792f909
final_head_frozen_at: 2026-08-08T20:27:00+02:00
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-08T19:28:00+02:00
updated_at: 2026-08-08T20:29:14+02:00
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
delivery_pr: 102
delivery_exact_head: b1ccc90de58052f1bb61b705250b59507792f909
delivery_squash_merge: e72f2514924e8bbf8d1a729721cce9e67d977544
closeout_pr: 103
closeout_branch: docs/OTV2-20260808-fnd03-runtime-execution-final-closeout
completed_at: 2026-08-08T20:27:20+02:00
ownership_released: true
next_gate: FND-04 Identity, Game Session, Admission and Character Lease Contract
```

## Outcome

The complete architecture-only `FND-03 Runtime Execution Contract` is accepted and canonical on `main`.

The contract freezes the runtime semantics required before implementation: ownership/lifecycle boundaries, authoritative input linearization, clocks/timers, auxiliary-work revalidation, bounded overload behavior, recovery/fencing/checkpoint cuts, liveness integration, deterministic randomness/replay, failure/error disposition and downstream ownership. It deliberately does not choose benchmark-sensitive runtime technology or numeric capacities without evidence.

Acceptance completes the FND-03 **architecture gate only**. It does not authorize Rust runtime implementation, production traffic or deployment.

## Architecture and source of truth

- **PROVEN:** both merged FND-03 analysis baselines are canonical inputs.
- **PROVEN:** FND-02 retains ownership of per-GameSession `(GameSessionId, CommandId)`, connection generation, server sequence and snapshot/delta/resync semantics; FND-03 consumes rather than redefines them.
- **PROVEN:** one logical mutation owner per ChannelRuntime/InstanceRuntime remains accepted; process/thread placement is not semantic authority.
- **PROVEN:** one owner-scoped `RuntimeExecutionOrdinal` is the mandatory runtime input linearization/evidence value; domain/state revisions remain committed-state authority, so a second mandatory runtime-wide commit counter is not required.
- **PROVEN:** scope ownership generation is distinct from NodeId and cannot be self-granted by a newly started GameNode.
- **PROVEN:** full four-second re-entry protection includes inbound PvE suppression from all monsters, including already-targeting monsters, while effects committed before protection activation may still resolve.
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
- [x] Preserve the complete four-second inbound/outbound PvE re-entry protection semantics and require later implementation evidence for the inbound monster-attack suppression boundary.

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

- [x] No runtime code/workspace/production implementation was introduced by this package.
- [x] Final changed-path architecture review found zero unresolved material conflicts.
- [x] Exact-head Agent governance, Dependency review and CodeQL passed.
- [x] Independent exact-head architecture/security audit passed with zero open material findings.
- [x] Both material review findings were fixed and resolved before merge.
- [x] Squash merge completed only after exact-head gates passed.
- [x] Lifecycle closeout PR #103 created to archive the task and release ownership.

## Excluded scope

This task did not:

- implement GameNode/ChannelRuntime/InstanceRuntime Rust code or create speculative runtime crates;
- select Tokio/another runtime, worker/thread counts, CPU affinity, fixed global tick frequency or scheduler quantum;
- guess numeric queue/timer/worker capacities;
- define PostgreSQL schema, transaction isolation, checkpoint encoding, journal technology, RPO/RTO or backup policy;
- define FND-04 credentials/tokens/lease storage/duplicate-login state machine;
- define ANL-01 event schema/outbox implementation;
- select Kubernetes/Docker/systemd or another orchestrator product;
- define gameplay-specific combat/movement/AI tie-break formulas beyond the generic owner execution framework;
- authorize client diagnostics, Launcher/Guardian implementation, production traffic/deployment or external-repository writes.

Runtime/component/E2E proof is `NOT_APPLICABLE` because this delivery changed architecture documentation only. Executable proof remains mandatory for any later authorized implementation package.

## Material findings resolved

1. **Inbound PvE protection omission** — final review found that outgoing re-entry offense was blocked but the candidate contract did not explicitly preserve the accepted rule suppressing new inbound monster attacks for the full four-second interval. The exact final head now explicitly blocks new PvE monster attacks, including from an already-targeting monster, while permitting already-committed effects to resolve, and adds required implementation-acceptance evidence.
2. **Canonical progression synchronization** — review requested explicit protection against returning agents to an already-completed FND-03 gate. `FOUNDATION_PROGRAMME_CURRENT_STATUS.md` is in the delivery and uses transition-safe wording: PR #102 is candidate while open; after merge the main contract is authoritative; closeout follows; then FND-04 is the next architecture package.
3. **Historical stale evidence invalidation** — earlier audit/CI at pre-repair head was not reused. The repair produced final exact head `b1ccc90de58052f1bb61b705250b59507792f909`, then fresh exact-head CI and a fresh architecture/security re-audit were required before merge.

No material finding remained at delivery merge.

## Validation evidence

### Focused architecture review

- changed paths: exactly 3 declared documentation paths;
- final contract reconciled with both merged FND-03 analysis baselines, ADR-0001, ADR-0009, FND-ID-01, FND-02, instance ownership/handoff, reconnect/liveness/re-entry baselines, multichannel scope matrix, resource-limit policy, error vocabulary and foundation failure catalogue;
- result: `PASS`.

### Exact-head CI

Final exact head: `b1ccc90de58052f1bb61b705250b59507792f909`.

- Agent governance run `31271918522`: `PASS`;
- Dependency review run `31271918525`: `PASS`;
- CodeQL run `31271918531`: `PASS`.

### Independent audit

- exact head: `b1ccc90de58052f1bb61b705250b59507792f909`;
- audit review: `4889413790`;
- verdict: `PASS`;
- open material findings: `0`.

### Review threads

- inbound PvE protection P1: fixed and resolved;
- canonical foundation-status P1: fixed and resolved;
- unresolved review threads at merge: `0`.

### Delivery merge

- delivery PR: `#102`;
- final exact head: `b1ccc90de58052f1bb61b705250b59507792f909`;
- squash merge: `e72f2514924e8bbf8d1a729721cce9e67d977544`;
- merged at: `2026-08-08T20:27:20+02:00`;
- result: `PASS`.

## Canonical FND-03 decisions delivered

1. `NodeRuntime` is process-incarnation supervision/execution hosting, not gameplay/world authority.
2. `WorldServices` is typed access to explicitly owned world-scoped domains, not a mutable global Game singleton.
3. ChannelRuntime and InstanceRuntime share one correctness kernel with one logical mutation owner per semantic scope.
4. Semantic scope identity, scope ownership generation and NodeId/process placement are separate concepts.
5. Every mutation-capable event normalizes through one authoritative owner resolution boundary.
6. `RuntimeExecutionOrdinal` linearizes owner-local normalized inputs; FND-02 CommandId/server_sequence and domain state revisions retain their own meanings.
7. Wall clock, process-local monotonic time and authoritative execution order are separate.
8. Process-local monotonic instants are never durable across NodeId replacement.
9. Mutation-capable timers, worker results and service/database completions re-enter through the current owner and are revalidated.
10. Every load-amplified runtime queue/pending/executor class is bounded; concrete maxima require later registered evidence.
11. Control/fencing cannot starve behind ordinary gameplay; this is not a gameplay-priority bypass.
12. Lifecycle activation/drain/checkpoint/fence/recovery semantics preserve one-owner authority and external fencing.
13. FND-04 owns sufficient liveness/session/reconnect/15-second semantics; FND-03 executes accepted monotonic 2-second, 5-second and four-second effects.
14. Re-entry protection blocks both player outgoing PvE offense and new inbound PvE monster attacks for the complete four-second interval, including attacks from existing targets; already-committed effects may resolve.
15. Authoritative gameplay RNG is deterministic/server-controlled and distinct from cryptographic security randomness.
16. Deterministic simulation replay never depends on original CPU/thread interleaving, and analytics replay cannot execute gameplay mutation.
17. Unexpected authoritative invariant failure is fail-stop unless narrower safe isolation is proven.

## Cross-repository disposition

No external repository was modified. No Platform, Otheryn or archived-client write was authorized or performed.

## Next safe gate

The next foundation architecture package is one bounded architecture-only `FND-04 Identity, Game Session, Admission and Character Lease Contract` task from current `main`.

FND-04 must consume FND-ID-01, FND-02, accepted FND-03, Platform/Gateway admission boundaries, reconnect generation, disconnect/re-entry/liveness semantics and character/account single-online-character constraints. It must not implement runtime, persistence schemas or production traffic unless separately authorized.

## Context checkpoint

```yaml
last_progress: Final FND-03 contract passed fresh exact-head Agent governance, Dependency review, CodeQL and architecture/security re-audit at b1ccc90de58052f1bb61b705250b59507792f909 after fixing inbound PvE re-entry protection, then squash-merged as e72f2514924e8bbf8d1a729721cce9e67d977544. Closeout PR #103 archives and releases FND-03 final-contract ownership.
status: completed
branch: docs/OTV2-20260808-fnd03-runtime-execution-final
head_sha: b1ccc90de58052f1bb61b705250b59507792f909
pr: 102
final_head_sha: b1ccc90de58052f1bb61b705250b59507792f909
final_head_frozen_at: 2026-08-08T20:27:00+02:00
ci_trigger_source: pull_request
ci_check_generation: delivery-final
ci_checks_for_current_head: 3
ci_run_ids:
  - 31271918522
  - 31271918525
  - 31271918531
ci_job_ids: []
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 3
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Merge lifecycle closeout PR #103, then begin one bounded architecture-only FND-04 Identity, Game Session, Admission and Character Lease Contract task from current main.
```
