# OTV2-20260928-use-wire-m1

```yaml
task_id: OTV2-20260928-use-wire-m1
title: USE-WIRE-V1 M1 - registries and codecs
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: 5864914163
base_branch: main
branch: claude/use-wire-m1
pr: 1066
base_sha: 45b6cc73d8dbd2988ec0153cfa5ad03318367d51
head_sha: bd8cd3f43a47ee41b6abcbebd6a1630c274c51b6
final_head_sha: bd8cd3f43a47ee41b6abcbebd6a1630c274c51b6
final_head_frozen_at: 2026-09-28T07:26:10Z
owner: "Oteryn: impl server seam" (Claude Code)
created_at: 2026-09-28T06:57:13Z
updated_at: 2026-09-28T07:50:00Z
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

## Shared-lease extension: world_spatial.rs (resolved)

Registering command 2 / domain 2 broke `world_spatial.rs`'s
`registries_bind_the_accepted_first_control_ids_and_limits` test, which hard-asserted the
registry's `command_types`/`state_domains` arrays were exactly length 1. This task initially
stopped at `SHARED_LEASE_REQUIRED`; the control plane then granted a bounded extension scoped to
that one test. Fix applied: the two cardinality asserts are now an id-filtered lookup of command id
1 / domain id 1, keeping every existing field/limit assertion for id 1 unchanged (at least as
strict as before — no more index-0 assumption). No other line in that file changed. PR #1066
opened on head `505ccb5`.

## Repair: Codex review findings (PR #1066, return to AUTHORING)

Two review findings on `world_object.rs`, both accepted and repaired in this commit:
- **P1** (r4119513870): decoders rejected an omitted proto3-default scalar/bytes field (e.g. an
  object at its initial revision 0, which a standard encoder omits). Fixed: `placement`/
  `expected_revision` in `WorldObjectTarget`, and `placement`/`state`/`revision` in
  `WorldObjectOverlayEntry`, now default to empty/0 when absent from the wire; `content_generation`
  keeps no default (absent stays `Malformed`, must be exactly 32 bytes), the missing oneof
  `world_object`, a missing delta entry, unknown/repeated fields and `disposition` 0/absent all
  still fail closed. New test:
  `omitted_proto3_defaults_are_accepted_but_required_fields_stay_malformed`.
- **P2** (r4119513878): the overlay-entry/delta/snapshot encoders and `encode_use_intent` emitted a
  `placement`/`state` over `MAX_KEY_BYTES` unchecked. Fixed: all four now return
  `Result<_, WorldObjectError>` and refuse (`LimitExceeded`) before emitting any bytes when a key
  exceeds `MAX_KEY_BYTES`; call sites (tests) updated accordingly. New test:
  `encoders_refuse_to_emit_an_oversized_key` (512 B OK, 513 B refused).

Both review threads were replied to before this repair was pushed. `world_object.rs` now has 7
tests (was 5); `world_spatial.rs` is unchanged from the shared-lease fix above.

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
- result (post-repair): fmt PASS; clippy PASS (no warnings in this task's code); `world_object`
  7/7 PASS (2 new tests for the repair above); `world_spatial` 4/4 PASS; governance validator PASS
  on this record; `validate_repository_policy.py` PASS. No dedicated protocol/resource-registry
  validator script exists beyond `validate_governance.py` (checked `tools/` and
  `.github/workflows/` by content grep).

### Component/integration

- command/run: `NOT_APPLICABLE` — M1 has no composition.
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no end-to-end use path before M2.
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending (this repair commit not yet pushed at record-write time; see PR #1066 for
  current head/checks)
- trigger source: push to `claude/use-wire-m1`; workflow/runner/classification/result: pending

## Self-review

- exact head: pending (not self-referenced in this commit)
- method/reviewer: implementing agent (this session)
- material findings: r4119513870 (P1) and r4119513878 (P2) from the Codex review of PR #1066 —
  see Repair section above; both accepted and repaired
- verdict: ready to re-freeze

## Independent review

- required: YES — public wire (`docs/contracts/**`) and registry change, per the allocation's
  `review: required` and root governance.
- exact head: `505ccb5` (superseded by this repair commit)
- method/auditor: Codex, automated PR review (not triggered by this worker)
- material findings: P1 r4119513870, P2 r4119513878 — both accepted and repaired (see above)
- verdict: findings addressed; a fresh review of the repaired head is for the control plane to
  request, not this worker (no `@codex` trigger from this task)

## PR and closeout

- changed-file review: 7 files, all inside owned_paths (world_spatial.rs under the control-plane
  shared-lease extension); verified by the control plane at freeze.
- independent review: `@codex review` on `505ccb5` raised P1 r4119513870 and P2 r4119513878; both
  repaired at `bd8cd3f43a47ee41b6abcbebd6a1630c274c51b6`; the second `@codex review` on that exact head reported no findings; both
  threads resolved.
- exact-head CI: green on `bd8cd3f43a47ee41b6abcbebd6a1630c274c51b6`; enqueued via protected auto-merge (squash) and merged through the
  Merge Queue as PR #1066, merge commit `79b85ec6ce3e0ecc34eaf753b524cd8ea61157a4` (protected main readback).
- ownership release: all owned paths released; M2 (Server Seam composition) is a separate allocation.
- related/superseded PRs: none

## Context checkpoint

```yaml
last_progress: PR #1066 merged through the Merge Queue as 79b85ec; task terminal
status: completed
branch: claude/use-wire-m1
head_sha: bd8cd3f43a47ee41b6abcbebd6a1630c274c51b6
pr: 1066
final_head_sha: bd8cd3f43a47ee41b6abcbebd6a1630c274c51b6
final_head_frozen_at: 2026-09-28T07:26:10Z
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
next_action: none (terminal); successor M2 allocated separately on #162
```
