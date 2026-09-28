# OTV2-20260928-use-wire-m1

```yaml
task_id: OTV2-20260928-use-wire-m1
title: USE-WIRE-V1 M1 - registries and codecs
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5864914163
base_branch: main
branch: claude/use-wire-m1
pr: null
base_sha: 45b6cc73d8dbd2988ec0153cfa5ad03318367d51
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl server seam" (Claude Code)
created_at: 2026-09-28T06:57:13Z
updated_at: 2026-09-28T07:35:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto
  - apps/game-server/src/gameplay_transport/world_object.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - docs/agents/tasks/active/OTV2-20260928-use-wire-m1.md
  - apps/game-server/src/gameplay_transport/world_spatial.rs  # shared-lease extension granted by control plane (stale cardinality assertion)
public_contracts:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/protocol-oteryn/v1/world_object_v1.proto
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

M1 of USE-WIRE-V1: registries, proto and strict codecs only; nothing composed into the connection
(M2). Adds command type 2 `USE_INTENT` and state domain 2 `WORLD_OBJECT_OVERLAY` (delta type 1,
snapshot type 1) to `PROTOCOL_OTERYN_V1_REGISTRY.json`, both free on base `45b6cc7`; adds
`WOBJ-RL-01`/`02`/`03` to `RESOURCE_LIMITS_REGISTRY.json` in the `MOVE-RL-02`/`MOVE-RL-11` row
format; adds `world_object_v1.proto`; adds strict encode/decode for the intent, result, overlay
delta and snapshot in `gameplay_transport/world_object.rs` (`#[allow(dead_code)]` outside tests,
mirroring `world_spatial.rs`), with round-trip, exactly-one-target, zero/unknown-enum,
unknown/repeated-field, over-bound and non-32-byte-`content_generation` tests plus a
registry-binding test; adds the `world_object` module declaration to `gameplay_transport/mod.rs`.

Excluded (M2): Server Seam composition (dispatch, transition-selection kernel, scope
broadcast/snapshot) and the end-to-end use/occupied/stale-state proof.

## Architecture and source of truth

- `PROVEN`: #162 comment 5864914163 (`USE-WIRE-V1`), owner acceptance of design A2', supersedes
  packet 5860722313; this task implements its M1 allocation verbatim.
- `PROVEN`: precedent followed exactly — `docs/agents/tasks/archive/OTV2-20260927-first-control-wire-m1.md`,
  `gameplay_transport/world_spatial.rs`, `world_spatial_v1.proto`.
- `PROVEN`: `foundation/protocol.rs` `MAX_SNAPSHOT_CHUNK_BYTES = 524_288` (registered
  `FND02-SNAPSHOT-CHUNK-BYTES`) is the existing single-chunk snapshot bound `WOBJ-RL-03` derives
  from (read-only; `foundation/**` excluded).
- `PROVEN`: `content/production.rs` `FIRST_PRODUCTION_MAX_KEY_BYTES = 512` is the existing
  accepted key maximum, reused unmodified for `placement`/`state` (read-only; `content/**` excluded).

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Pure registries, a proto schema and strict encode/decode with no I/O, session, lease,
  generation or authority evidence and no PREPARE/COMMIT. Nothing is composed into the
  connection, runtime or durability seam; M2 (server composition) is a separately allocated task.
```

## Acceptance criteria

- [x] Protocol registry: `USE_INTENT` (id 2) / `WORLD_OBJECT_OVERLAY` (id 2, delta 1, snapshot 1),
      `owner_decision` citing #162 comment 5864914163, schemas + byte bounds.
- [x] `world_object_v1.proto`: `UseIntentV1` (oneof `target`, reserved 2-4), `WorldObjectTargetV1`,
      `UseResultV1`/`UseDispositionV1` (0 UNSPECIFIED..6 REJECTED), `WorldObjectOverlayEntryV1`
      (32-byte `content_generation`, `placement`, `state` key, `revision`), USE-WIRE-V1 semantics
      in comments.
- [x] Resource registry: `WOBJ-RL-01`/`02`/`03` in the `MOVE-RL-02`/`MOVE-RL-11` row format,
      `WOBJ-RL-03` arithmetic shown in `notes`, key bounds from `FIRST_PRODUCTION_MAX_KEY_BYTES`.
- [x] `world_object.rs`: strict codecs + the listed test coverage + registry-binding test.
- [x] `gameplay_transport/mod.rs`: module declaration only.
- [x] Full required-validation suite green on one head.

## Shared-lease extension: world_spatial.rs

Registering command 2 / domain 2 grows `PROTOCOL_OTERYN_V1_REGISTRY.json`'s arrays from length 1
to 2. `gameplay_transport/world_spatial.rs`'s existing test
`registries_bind_the_accepted_first_control_ids_and_limits` asserted exact global cardinality
(`assert_eq!(commands.len(), 1)` / `domains.len(), 1)`, was lines 366/371) instead of filtering by
id/name, so it broke — exactly as #162 comment 5864914163 anticipated would happen to whoever
registers the next command/domain. This task initially stopped at `SHARED_LEASE_REQUIRED`; the
control plane then granted a bounded lease extension for exactly this one test. The fix (this
lease, applied): the two cardinality asserts are replaced with an id-filtered lookup of command id
1 / domain id 1 (`.find(|x| x["id"] == ...)`), keeping every existing field and limit assertion for
id 1 unchanged and not weakening the test — it is at least as strict as before (it also stops
assuming id 1 sits at array index 0). No other line in `world_spatial.rs` changed. All 4
`world_spatial` tests pass on this head.

## Excluded scope

`connection.rs` dispatch, `world_runtime.rs`, `foundation/**`, `lib.rs`, client, Cargo files,
workflows, `content/**`, `tools/**`, the spell-cast candidate (#1042), Server Seam composition
(M2). No `@codex review` trigger, no comment on #162, no merge/enqueue.

## Implementation / findings

- IDs re-verified free immediately before authoring: `command_types`/`state_domains` each had only
  id 1. No open PR at authoring time touched the six owned paths (`#1060` touches Cargo only).
- `WOBJ-RL-03` arithmetic (also in the registry row's `notes`): one `WorldObjectOverlayEntryV1` as
  a repeated-field element is at most 1,078 bytes — outer tag+length (1+2=3) wrapping an entry of
  at most 1,075 bytes (`content_generation` 1+1+32=34; `placement` 1+2+512=515, using
  `FIRST_PRODUCTION_MAX_KEY_BYTES`; `state` 1+2+512=515; `revision` 1+10=11; sum 1,075).
  `floor(524,288/1,078)=486` (486*1,078=523,908; remainder 380). Delta max 1,078 B
  (`WOBJ-RL-02`=1), snapshot max 523,908 B.
- `MAX_USE_INTENT_BYTES=529`: `WorldObjectTargetV1` (`placement` 515 + `expected_revision` 11 =
  526) wrapped as the `world_object` submessage field (1+2=3); 529.
- `MAX_USE_RESULT_BYTES=4`: single small enum field (2 B actual), same slack as
  `MAX_STEP_RESULT_BYTES` so repeated-field vs. over-bound stay distinguishable in tests.

## Validation

### Focused

- command/run: `cargo fmt --check -p oteryn-game-server`; `cargo clippy -p oteryn-game-server
  --all-targets -- -D warnings`; `cargo test -p oteryn-game-server world_object`; `cargo test -p
  oteryn-game-server world_spatial`; `python3 tools/agents/validate_governance.py`; `python3
  tools/repository/validate_repository_policy.py`
- result: fmt PASS; clippy PASS (no warnings in this task's code); `world_object` 5/5 PASS;
  `world_spatial` 4/4 PASS (id-filtered lookup fix under the shared-lease extension above);
  governance validator PASS on this record; `validate_repository_policy.py` PASS (23 files, 47
  workflows). No dedicated protocol/resource-registry validator script exists beyond
  `validate_governance.py` (checked `tools/` and `.github/workflows/` by content grep).

### Component/integration

- command/run: `NOT_APPLICABLE` — M1 has no composition.
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no end-to-end use path before M2.
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending (not frozen; blocked)
- trigger source / workflow / runner / classification / result: pending

## Self-review

- exact head: pending (recorded once frozen/pushed; not self-referenced in this commit)
- method/reviewer: implementing agent (this session)
- material findings: none outstanding; the `world_spatial.rs` shared-lease item above is resolved
  under the granted extension
- verdict: ready to freeze

## Independent review

- required: YES — public wire (`docs/contracts/**`) and registry change, per the allocation's
  `review: required` and root governance.
- exact head / method-auditor / material findings / verdict: pending (not frozen; the control
  plane triggers one `@codex review` after freeze, not performed by this worker)

## PR and closeout

- changed-file review / unresolved threads / protected auto-merge / merge commit / ownership
  release: pending (recorded once the PR is opened and reviewed; not tracked in this file per the
  no-self-referential-freeze rule — see the live PR and #162 for current status)
- related/superseded PRs: none known; overlap check at allocation time found none touching these
  paths

## Context checkpoint

```yaml
last_progress: shared-lease extension applied to world_spatial.rs's id-filtered registry test;
  full required-validation suite green on all seven owned files on claude/use-wire-m1
status: implementing
branch: claude/use-wire-m1
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
next_action: commit, push claude/use-wire-m1 and open the PR to main
```
