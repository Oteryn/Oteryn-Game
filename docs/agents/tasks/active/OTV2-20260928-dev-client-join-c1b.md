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
head_sha: a61536b2940360b8bc544d3d18c4ac843b21f670
owner: "Oteryn: impl interaction" (Claude Code)
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/dev-client/**
  - Cargo.toml
  - Cargo.lock
  - workspace-boundaries.toml  # shared lease: member registration + [dev_edges] table
  - apps/game-server/Cargo.toml  # [dev-dependencies] only
  - apps/game-server/src/gameplay_transport/qualification.rs  # shared lease: one stage + plumbing
  - crates/protocol-oteryn/**  # shared lease (coordinator resume): additive client codecs
  - tools/architecture-check/**  # shared lease (coordinator resume): kind-aware dev edges
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
native client (ADR-0011 §6). Connects over rustls TLS 1.3 (ALPN `oteryn-game/1`), admits with the
WP5 fixture grant mechanism the existing seam E2E already uses, and decodes the join snapshot's
domains 1 `WORLD_SPATIAL` and 2 `WORLD_OBJECT_OVERLAY`, asserting the native entry door overlay by
its wire identity: placement `oteryn:cell/entry-door` and its state key (`oteryn:local-object/
entry-door` is the content definition key, never placed on the wire). Holds no codec of its own:
TLS transport + frame length-prefixing + dispatch over `crates/protocol-oteryn`'s codecs only.

Proved E2E via `stage=dev_client` added to `qualification.rs`'s
`server_seam_real_owners_over_tcp_tls` (existing CI job `Merge gate / Server Seam over TCP+TLS`,
`wp5_s3b/run.sh` unmodified): admits `characters[1]` (free: released by `use_wire` above), asserts
the decoded snapshot against the door/channel's own live state, lets the connection close (same
"silent" pattern `concurrent[0]`/`use_session` use), waits through the same control-loss (60s) ->
grace-release (100s) cycle, so `committed_players == 1` / `committed_admissions` (4 -> 5) keep
holding. Moved `accepted_session()`'s fixed-offset `ServerAccepted` hand-parse onto
`decode_server_accepted`; the hand-rolled `bootstrap()`/`envelope()` negative-test helpers stay
(they must emit deliberately non-conforming frames the strict `encode_client_bootstrap` cannot).

Coordinator resume (owner: option 1, #162 comment 5875470550) resolved this task's own initial
`DESIGN_DECISION_REQUIRED` stop over a reproduced `tools/architecture-check` gap (a dev-dependency
path edge from a production root onto a non-production tool package failed
`validate_production_closure` however declared): `internal_edges` now splits by `cargo metadata`
`kind` into `normal` (walked by `validate_acyclic`/`validate_production_closure`, unchanged) and
`dev` (new `[dev_edges]` table, exact-equality checked, never closure-walked); 4 new tests.
`crates/protocol-oteryn` gained additive-only client-direction codecs (wire format unchanged):
`encode_client_bootstrap`, `decode_server_accepted`, `decode_snapshot_chunk`, `decode_snapshot_id`,
each round-tripped against the crate's existing server-side encoders/ingress validation.

## Fix round (Codex P2 review on `1efc029`, plus a CI failure on the same head)

Four P2 findings, each fixed with a dedicated test:

1. `decode_snapshot_chunk` (protocol-oteryn) now enforces `MAX_STATE_DOMAINS_PER_SYNC` before
   pushing an entry and rejects a duplicate `domain_id` (`FND02-STATE-DOMAINS-PER-SYNC`,
   `RESOURCE_LIMITS_REGISTRY.json`: 256 accepted / 257 rejected / duplicates rejected) - 2 tests.
2. `connect_and_join` (dev-client) now correlates every join-snapshot frame: `SnapshotBegin`'s
   `snapshot_id` is read via the new `decode_snapshot_id` (protocol-oteryn, additive), and
   `SnapshotChunk`'s/`SnapshotCommit`'s `snapshot_id` and all three frames' envelope
   `connection_generation` must match it/the admitted session - 1 test (mismatched chunk id).
3. `JoinRequest` gained a `deadline: Duration` field; the TCP connect, TLS handshake and every
   frame read are now bounded by it (`DevClientError::Timeout(stage)` on expiry) - 1 test (server
   stalls after admission).
4. Domain dispatch now matches `(domain_id, snapshot_type)`; only `snapshot_type` 1 is registered
   for domains 1/2 (`PROTOCOL_OTERYN_V1_REGISTRY.json`), anything else on a known domain is
   `DevClientError::UnregisteredSnapshotType` - 1 test.

Separately, CI (`Merge gate / Server Seam over TCP+TLS` on `1efc029`) failed inside
`stage=dev_client` itself: connect/admission/join-snapshot decode all worked, but the assertion
wrongly assumed a *fresh* door (closed, revision 0); `use_wire` runs immediately before this stage
and already opened+closed the door, leaving it closed at revision 2. Fixed by deriving every
expected value live instead of assuming freshness: `SeamClients` gained a `door: Option<&'a
Mutex<LocalObjectRuntime>>` field (`None` only for the unrelated, already-`#[ignore]`d
`node_boot_seam_against_running_node`, which has no local door object); `stage=dev_client` now
reads `door.lock().await`'s actual `placement_key()`/`state_key()`/`revision()` and
`runtime.content_pin().client_artifact_digest()` - the same two sources
`observe_world_object_overlay` reads for every real join - right after the join, and compares the
decoded snapshot against those live values, not a hardcoded assumption.

## Architecture and source of truth

- `PROVEN`: #162 allocation 5875272684; coordinator resume 5875470550; CI failure on `1efc029`
  (`Merge gate / Server Seam over TCP+TLS`, `stage=dev_client`, door-revision mismatch).
- `PROVEN`: ADR-0011 §3/§5/§6 - production client stays fail-closed; §6 allows this dev harness.
- `DERIVED`: door overlay wire identity - `gameplay_transport/mod.rs::observe_world_object_overlay`
  through `world_runtime.rs:441` (`placement_key` = `DOOR_CELL.0`) vs `:448` (`DOOR_DEFINITION`,
  content-resolution only, not wire); its live revision after `use_wire` is 2, not 0.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No production mutation, session, lease, generation or authority evidence introduced or changed.
  The dev client is admitted through the exact same PREPARE/COMMIT-free fresh-admission path every
  other seam client already exercises. All four fixes and the door-state repair are decode-side
  correlation/bounds/dispatch hardening and a test-assertion correction, not new authority surface.
```

## Validation

### Focused

- run: `cargo +1.94.0 fmt --all --check`; `cargo clippy --offline --workspace --all-targets -- -D warnings`;
  `cargo test --offline -p oteryn-architecture-check -p oteryn-protocol-oteryn -p oteryn-dev-client`;
  `cargo test --offline -p oteryn-game-server --lib`; `cargo run --locked -p oteryn-architecture-check -- workspace .`;
  `cargo tree --locked -p oteryn-client --edges normal,build` / `-p oteryn-game-server --edges normal,build`
  (both: `oteryn-dev-client` absent) / `-p oteryn-game-server --edges dev` (present);
  `python3 tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`
- result: fmt PASS; clippy PASS (0 warnings, whole workspace); architecture-check 14/14;
  protocol-oteryn 46/46 (43 prior + 3 new: 2 snapshot-domain-bound, 1 snapshot-id round trip);
  dev-client 5/5 (2 prior + 3 new: mismatched snapshot id, stalled-server timeout, unregistered
  snapshot type); game-server --lib 669/0/2-ignored (unchanged); both `cargo tree` runs confirm
  `oteryn-dev-client` outside both production closures; `workspace-boundaries: PASS`; governance
  and repository-policy validators PASS.

### Component/integration

- run: `cargo test --offline -p oteryn-dev-client`
- result: PASS - 5 same-process fake-TLS-server tests: the happy-path join (door overlay present),
  server-closes-before-admission, mismatched `SnapshotChunk` id, stalled-server timeout, and an
  unregistered `snapshot_type` on a known domain.

### E2E

- scenario: `server_seam_real_owners_over_tcp_tls`, `stage=dev_client`, against the composed WP5
  S3-B owners (Platform + PostgreSQL 17.6) over real loopback TCP + TLS 1.3.
- result: local run `#[ignore]`d (no exact Platform checkout / PostgreSQL 17.6 service available
  locally); compiles/type-checks cleanly. The prior candidate (`1efc029`) DID run for real in CI
  and reached `stage=dev_client` (every earlier stage, through `use_wire released=terminal
  admissions=3`, passed) before failing on the fresh-door assumption fixed above; this candidate
  fixes that failure and the 4 Codex findings, restarting exact-head CI.

### Exact-head CI

- candidate: the frozen final head of PR #1147 (last functional change `a61536b`); its live checks govern.
- result: pending (superseding the failed run on `1efc029`, which is no longer the tested head)

## Self-review

- exact head: `a61536b` (PR #1147)
- method/reviewer: implementing agent (this session)
- material findings: 4 Codex P2 findings (protocol-oteryn snapshot-domain bounds; dev-client frame
  correlation, deadlines, snapshot-type dispatch) plus one self-discovered-via-CI finding (the
  `stage=dev_client` door assertion assumed a fresh door instead of `use_wire`'s live end state).
  All five repaired test-first with a dedicated regression test each; verified with the full
  validation list above.
- verdict: ready for independent review

## Independent review

- required: YES - touches two shared-lease files outside the original owned paths
  (`tools/architecture-check/**`, `crates/protocol-oteryn/**`), changes the production-closure
  checker's semantics, adds a workspace member wired as a dev-dependency of a production root.
- exact head: the frozen final head of PR #1147
- method/auditor: Codex via control-plane trigger on that exact SHA (4 threads on `1efc029` replied naming `a61536b`)
- material findings: the 4 P2 findings above; disposition `fixed` for each (see Fix round)
- verdict: pending

## Excluded scope

`.github/**`, `tools/repository/**` (unchanged); `apps/client/**`/`oteryn-client` closure
(unchanged, verified via `cargo tree`); production `apps/game-server/src/**` other than
`qualification.rs` (unchanged); protocol registries/proto files (unchanged); `wp5_s3b/run.sh`
(unchanged). No `@codex` post, no auto-merge enabled, review threads not resolved by this agent.

## Context checkpoint

```yaml
last_progress: fix round pushed (a61536b); replied on the 4 Codex threads naming the fixing commit
status: ready
branch: claude/dev-client-join-c1b
head_sha: a61536b2940360b8bc544d3d18c4ac843b21f670
pr: 1147
final_head_sha: a61536b2940360b8bc544d3d18c4ac843b21f670 (supersedes 1efc029, which failed CI at stage=dev_client)
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
identical_failure_retries: 1
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: await new exact-head CI and review on PR #1147 (a61536b)
```
