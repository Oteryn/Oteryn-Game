# OTV2-20260805-game-node-capacity-and-recovery-baseline

```yaml
task_id: OTV2-20260805-game-node-capacity-and-recovery-baseline
title: Record GameNode execution, capacity, deployment and recovery baseline
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/adr-0008-protocol-canary-reference-only
pr: 37
base_sha: 52ef04882e13771829e0159b63410a7cd9e80150
head_sha: ce4b970174911d7b785bbb68f06849ae6241c167
merge_commit: 96c605c9fabc3266eca9dd7f0010c97e88fd057c
owner: GPT-5.6-Thinking-architecture-coordinator
created_at: 2026-08-05T19:45:00+02:00
updated_at: 2026-08-05T19:57:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md
  - docs/agents/tasks/archive/OTV2-20260805-game-node-capacity-and-recovery-baseline.md
public_contracts:
  - docs/architecture/ADR-0009-game-node-execution-capacity-deployment-and-recovery-baseline.md
depends_on:
  - ADR-0001
  - ADR-0003
  - ADR-0004
  - ADR-0007
  - FND-03
  - FND-04
  - DUR-02
  - DUR-03
blocks:
  - publication of production player-capacity claims until PERF-01 is accepted with measured evidence
  - automatic production channel scaling until OPS-CHANNEL-01 is accepted
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/Oteryn-Platform
```

## Outcome

ADR-0009 records the accepted GameNode execution, capacity, deployment and recovery baseline:

- `GameNode` means one logical running game-server process identified by canonical `NodeId`;
- production defaults to one GameNode process per container;
- one GameNode may host several ChannelRuntimes, while one-channel placement remains supported;
- the server is multithreaded, but one logical writer commits authoritative mutations for each channel;
- I/O, authoritative simulation and bounded CPU-bound auxiliary work remain separated;
- capacity is measured separately per channel, GameNode and logical world;
- exact player limits require representative benchmarks and named reference hardware;
- the initial production policy targets at least 30% headroom below measured saturation;
- existing GameNodes may create channels only when accepted capacity remains;
- new or replacement processes/containers are started by an external orchestrator, never by the game process through Docker/Kubernetes APIs;
- GameNode failure uses fencing, replacement/restart, checkpoint plus bounded replay, fresh Game Sessions and a full authoritative snapshot;
- players are not silently moved to another channel or teleported solely because a process failed;
- active-channel live migration and hot standby remain deferred.

ADR-0009 also registers `PERF-01` and `OPS-CHANNEL-01` as required follow-up contracts.

## Acceptance criteria

- [x] Host, container, process, GameNode and ChannelRuntime terminology is explicit.
- [x] Canonical `NodeId` terminology is preserved pending `FND-ID-01`.
- [x] Multithreaded single-writer execution and stale-result rejection are recorded.
- [x] Capacity limits are separated for channel, GameNode and world.
- [x] Representative benchmark and saturation evidence requirements are recorded without inventing player numbers.
- [x] Dynamic channel lifecycle and external orchestrator ownership are recorded.
- [x] GameNode failure, fencing, checkpoint/replay and player reconnect behavior are recorded.
- [x] Critical durable operations remain protected by atomicity, idempotency and anti-duplication invariants.
- [x] `PERF-01` and `OPS-CHANNEL-01` are registered.
- [x] No runtime code, deployment configuration, external repository or production system was changed.
- [x] Architecture audit passed after correcting premature `GameNodeId` terminology.
- [x] Exact-head required workflows passed.
- [x] PR #37 squash-merged and the result was verified.

## Validation

- focused changed-file review: `PASS`
- component/integration: `NOT_APPLICABLE` — architecture-only package
- E2E: `NOT_APPLICABLE` — no executable product change; ADR-0009 defines mandatory future physical failure scenarios
- exact head: `ce4b970174911d7b785bbb68f06849ae6241c167`
- Agent governance run `31032227692`: `PASS`
- Dependency review run `31032226364`: `PASS`
- CodeQL run `31032226152`: `PASS`
- unresolved review threads: `0`
- independent audit: `PASS`, zero material findings after terminology correction

## PR and closeout

- PR: `#37 — docs(architecture): record protocol and GameNode runtime baselines`
- merge result: squash-merged as `96c605c9fabc3266eca9dd7f0010c97e88fd057c`
- ownership release: complete
- external repositories changed: none
- production changes: none

## Context checkpoint

```yaml
last_progress: ADR-0009 was squash-merged to main with exact-head checks passing and the task was archived.
status: completed
branch: docs/adr-0008-protocol-canary-reference-only
head_sha: ce4b970174911d7b785bbb68f06849ae6241c167
pr: 37
ci_check_generation: ce4b970174911d7b785bbb68f06849ae6241c167
ci_checks_for_current_head: 3
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 3
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
stall_warnings: 0
blocker: null
next_action: None; ownership released and later work continues under FND-01, FND-03, PERF-01 and OPS-CHANNEL-01.
```
