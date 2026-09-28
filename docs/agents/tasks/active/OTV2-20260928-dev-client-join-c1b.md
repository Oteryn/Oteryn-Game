# OTV2-20260928-dev-client-join-c1b

```yaml
task_id: OTV2-20260928-dev-client-join-c1b
title: dev client join C1b - dev/qualification-only native client, joins and decodes the join snapshot
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1147
allocation_comment: 5875272684 (allocation); coordinator resume 5875470550 (owner: option 1, leases)
base_branch: main
branch: claude/dev-client-join-c1b
base_sha: 8e2e474a3d82a6bfbc21be409f444cfd64d42961
head_sha: 97c4641e52f3ca4f11eeb28a80fa69a856352396
owner: "Oteryn: impl interaction" (Claude Code)
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/dev-client/**
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml  # shared: member registration + [dev_edges]
  - apps/game-server/Cargo.toml  # [dev-dependencies] only
  - apps/game-server/src/gameplay_transport/qualification.rs  # shared: one stage + plumbing
  - crates/protocol-oteryn/**  # shared (coordinator resume): additive client codecs
  - tools/architecture-check/**  # shared (coordinator resume): kind-aware dev edges
  - docs/agents/tasks/active/OTV2-20260928-dev-client-join-c1b.md
public_contracts: []
depends_on:
  - OTV2-20260928-protocol-oteryn-crate-c1a
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

Adds `tools/dev-client` (`oteryn-dev-client`, `release-role = tool-only`), a dev/qualification-only
native client (ADR-0011 §6). Connects over rustls TLS 1.3 (ALPN `oteryn-game/1`, rejected if not
exactly that, including no ALPN negotiated), admits with the WP5 fixture grant mechanism the
existing seam E2E already uses, and decodes the join snapshot's domains 1 `WORLD_SPATIAL` and 2
`WORLD_OBJECT_OVERLAY`, asserting the native entry door overlay by its wire identity: placement
`oteryn:cell/entry-door` and its state key. Every inbound frame is checked with
`WireEnvelopeView::validate` (direction, phase, sequencing, pre-/post-admission generation
presence) before its payload is consumed, then correlated to `SnapshotBegin`'s full declaration
(`chunk_count`, in-order `chunk_index`, concatenated `data` equal `total_encoded_bytes`). Every
chunk's raw bytes are concatenated, in order, into one owned buffer; the assembled `SnapshotBody`
is protobuf-decoded exactly once, only after every chunk and the commit have validated. Holds no
codec of its own: TLS transport + frame length-prefixing + dispatch over `protocol-oteryn`'s
codecs only.

Proved E2E via `stage=dev_client` added to `qualification.rs`'s
`server_seam_real_owners_over_tcp_tls` (CI job `Merge gate / Server Seam over TCP+TLS`,
`wp5_s3b/run.sh` unmodified) and exercised (compile/type-check only, `#[ignore]`d locally) by
`node_boot_seam_against_running_node` (`Merge gate / Node boot against the real Platform`): admits
`characters[1]` (free: released by `use_wire` above), asserts the decoded snapshot, lets the
connection close (same "silent" pattern `concurrent[0]`/`use_session` use), waits through the same
control-loss (60s) -> grace-release (100s) cycle, so `committed_players == 1` /
`committed_admissions` (4 -> 5) hold.

Coordinator resume (owner: option 1, #162 comment 5875470550) resolved this task's own initial
`DESIGN_DECISION_REQUIRED` stop over a reproduced `tools/architecture-check` gap: `internal_edges`
now splits by `cargo metadata` `kind` into `normal` (walked by `validate_acyclic`/
`validate_production_closure`, unchanged) and `dev` (new `[dev_edges]` table, exact-equality
checked, never closure-walked).

## Fix rounds (Codex review + CI on the frozen final head of PR #1147, each superseded in turn)

Rounds 1-2 (6 P2 findings, 2 self-discovered CI failures, all carried forward, superseded by round
3 below where they overlap): domain count/duplicate-ID bounds and full `SnapshotBegin`/
`SnapshotChunk`/`SnapshotCommit` correlation (`snapshot_id`, `connection_generation`,
`chunk_index`, summed length); ALPN enforced exactly (`oteryn-game/1` or reject, unsent); the
door-overlay CI assertion moved off a live `door`/`runtime` query (unavailable in
`node_boot_seam_against_running_node`) onto `use_wire_frames`'s proven-correct overlay delta.

Round 3 (3 P2 findings, final round): (1) correctness bug - `connect_and_join` decoded each
chunk's `DomainSnapshot` entries independently instead of assembling first, but a multi-chunk
transfer may split a `SnapshotBody` field at any byte offset. Fixed: `decode_snapshot_chunk_framing`
now returns the raw `data` slice (same-PR-only refinement); new `decode_snapshot_body`
(protocol-oteryn) holds the domain-count/duplicate-ID checks, applied once to the assembled body;
`connect_and_join` concatenates every chunk's `data` (checked-arithmetic bounded against
`total_encoded_bytes` before each append) and decodes once, only after the length matches and the
commit validates. New test: a valid 2-chunk split-mid-field transfer succeeds.
(2) `WireEnvelopeView::validate` now runs on every inbound frame - `ServerAccepted` pre-admission,
each snapshot frame post-admission - before any payload is consumed. New tests: `ServerAccepted`
with a nonzero envelope `connection_generation`, `SnapshotBegin` with a nonzero `server_sequence`,
both rejected. (3) `snapshot_id == 0` is now absent/invalid (FND-02: "Zero is invalid") in
`decode_snapshot_id`/`decode_snapshot_begin`; `decode_snapshot_chunk` inherits the fix by reuse.
New test covers all three decode sites.

FND-02 audit (§8/§11/§12/§14/§16, all 4 consumed frame types): message-type/direction match;
pre-admission needs envelope `connection_generation == 0`, post-admission needs it equal the
admitted session's; all 4 unsequenced so `server_sequence` must be 0 - enforced by `validate`;
snapshot_id/chunk_index/chunk_count/length correlation (rounds 1-2); body decode strictly after
full assembly + commit, `snapshot_id != 0` (round 3); cumulative size checked-arithmetic-bounded
before allocation growth (hardening; same result either way, no dedicated test). §17/§18/§19 out
of scope per "do not add features beyond the join path".

## Architecture and source of truth

- `PROVEN`: #162 allocation 5875272684; coordinator resume 5875470550; 3 Codex P2 review rounds
  plus two CI failures on PR #1147's frozen final head, each repaired in turn.
- `PROVEN`: ADR-0011 §3/§5/§6; `docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` §8/§11/
  §12/§14/§16; `RESOURCE_LIMITS_REGISTRY.json` `FND02-STATE-DOMAINS-PER-SYNC`;
  `PROTOCOL_OTERYN_V1_REGISTRY.json` (snapshot type 1 only, domains 1/2).
- `DERIVED`: door overlay wire identity and its post-`use_wire` end state (closed, revision 2) -
  `gameplay_transport/mod.rs::observe_world_object_overlay`, `world_runtime.rs:441/448`, and
  `use_wire_frames`'s own final `overlay_delta`, which the `use_wire` stage proves matches the wire.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No production mutation, session, lease, generation or authority evidence introduced or changed.
  The dev client is admitted through the same PREPARE/COMMIT-free fresh-admission path every other
  seam client already exercises. All fixes across all rounds are decode-side
  correlation/bounds/dispatch/ALPN/validation hardening and test corrections, not new authority.
```

## Validation

### Focused

- run: `cargo +1.94.0 fmt --all --check`; `cargo clippy --offline --workspace --all-targets -- -D warnings`;
  `cargo test --offline -p oteryn-architecture-check -p oteryn-protocol-oteryn -p oteryn-dev-client`;
  `cargo test --offline -p oteryn-game-server --lib`; `cargo run --locked -p oteryn-architecture-check -- workspace .`;
  `cargo tree --locked -p oteryn-client --edges normal,build` / `-p oteryn-game-server --edges normal,build`
  (absent) / `-p oteryn-game-server --edges dev` (present); `cargo update --offline --workspace`
  (Cargo.lock unchanged); `python3 tools/agents/validate_governance.py`;
  `python3 tools/repository/validate_repository_policy.py`
- result: fmt PASS; clippy PASS (0 warnings, whole workspace); architecture-check 14/14;
  protocol-oteryn 48/48 (+1 new: `snapshot_id == 0` rejection); dev-client 12/12 (+3 new: 2-chunk
  split-mid-field success, `ServerAccepted` nonzero generation, `SnapshotBegin` nonzero
  `server_sequence`); game-server --lib 669/0/2-ignored (unchanged); both `cargo tree` runs confirm
  `oteryn-dev-client` outside both production closures; `workspace-boundaries: PASS`; governance
  and repository-policy validators PASS.

### Component/integration

- run: `cargo test --offline -p oteryn-dev-client`
- result: PASS - 12 fake-TLS-server tests: happy path and rounds-1-2 negatives, plus round 3's
  assemble-then-decode success case and the two `validate` negatives.

### E2E

- scenario: `server_seam_real_owners_over_tcp_tls`'s `stage=dev_client` (real composed WP5 S3-B
  owners, real loopback TCP+TLS) and `node_boot_seam_against_running_node`'s use of the same
  stage logic (externally-running node).
- result: both `#[ignore]`d locally (no exact Platform checkout / PostgreSQL 17.6 service, no
  running node-boot topology, available locally); both compile/type-check cleanly. The frozen
  final head ran both for real in CI across the two prior rounds and reached `stage=dev_client`
  each time before the failures fixed in those rounds; this candidate only changes decode/
  validation logic inside `stage=dev_client` itself.

### Exact-head CI

- candidate: the frozen final head of PR #1147 (after this round's push); its live checks govern.
- result: pending (supersedes all prior runs, none of which is the tested head)

## Self-review

- exact head: the frozen final head of PR #1147
- method/reviewer: implementing agent (this session)
- material findings: 9 Codex P2 findings across three rounds, plus two self-discovered CI failures
  in rounds 1-2 (both repaired, see Fix rounds). Round 3's 3 findings each repaired test-first,
  plus a self-driven FND-02 audit (see Fix rounds) that found one hardening, no new observable gap.
- verdict: ready for independent review

## Independent review

- required: YES - touches two shared-lease files (`tools/architecture-check/**`,
  `crates/protocol-oteryn/**`), changes the production-closure checker's semantics, adds a
  workspace member wired as a dev-dependency of a production root.
- exact head: the frozen final head of PR #1147
- method/auditor: Codex via control-plane trigger; each round's threads replied naming the fixing
  commit, not resolved by this agent
- material findings: 9 P2 findings across three rounds; disposition `fixed` for each
- verdict: pending

## Excluded scope

`.github/**`, `tools/repository/**` (unchanged); `apps/client/**`/`oteryn-client` closure
(unchanged, via `cargo tree`); production `apps/game-server/src/**` other than `qualification.rs`
(unchanged); protocol registries/proto files (unchanged); `wp5_s3b/run.sh` (unchanged). No
`@codex` post, no auto-merge, review threads not resolved by this agent.

## Context checkpoint

```yaml
last_progress: fix round 3 pushed (assemble-before-decode/validate/zero-id); replied on 3 threads
status: ready
branch: claude/dev-client-join-c1b
head_sha: 97c4641e52f3ca4f11eeb28a80fa69a856352396
pr: 1147
final_head_sha: the frozen final head of PR #1147 (supersedes all prior CI runs)
final_head_frozen_at: 2026-09-28T00:00:00Z
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 2
repair_cycles_for_current_gate: 3
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: await new exact-head CI and review on PR #1147
```
