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
native client (ADR-0011 §6). Connects over rustls TLS 1.3 (ALPN `oteryn-game/1`, rejected if not
exactly that, including no ALPN negotiated), admits with the WP5 fixture grant mechanism the
existing seam E2E already uses, and decodes the join snapshot's domains 1 `WORLD_SPATIAL` and 2
`WORLD_OBJECT_OVERLAY`, asserting the native entry door overlay by its wire identity: placement
`oteryn:cell/entry-door` and its state key. Every frame of the transfer (`SnapshotBegin`,
`SnapshotChunk`(s), `SnapshotCommit`) is checked against `SnapshotBegin`'s full declaration
(`chunk_count`, in-order `chunk_index`, summed `data` bytes equal `total_encoded_bytes`) and the
admitted session's `connection_generation` before being trusted. Holds no codec of its own: TLS
transport + frame length-prefixing + dispatch over `crates/protocol-oteryn`'s codecs only.

Proved E2E via `stage=dev_client` added to `qualification.rs`'s
`server_seam_real_owners_over_tcp_tls` (existing CI job `Merge gate / Server Seam over TCP+TLS`,
`wp5_s3b/run.sh` unmodified) and exercised (compile/type-check only, since it is `#[ignore]`d
locally) by `node_boot_seam_against_running_node` (`Merge gate / Node boot against the real
Platform`): admits `characters[1]` (free: released by `use_wire` above), asserts the decoded
snapshot, lets the connection close (same "silent" pattern `concurrent[0]`/`use_session` use),
waits through the same control-loss (60s) -> grace-release (100s) cycle, so
`committed_players == 1` / `committed_admissions` (4 -> 5) keep holding.

Coordinator resume (owner: option 1, #162 comment 5875470550) resolved this task's own initial
`DESIGN_DECISION_REQUIRED` stop over a reproduced `tools/architecture-check` gap: `internal_edges`
now splits by `cargo metadata` `kind` into `normal` (walked by
`validate_acyclic`/`validate_production_closure`, unchanged) and `dev` (new `[dev_edges]` table,
exact-equality checked, never closure-walked).

## Fix rounds (Codex review + CI on the frozen final head of PR #1147, each superseded in turn)

Round 1 (4 P2 findings): `decode_snapshot_chunk` (protocol-oteryn) enforces
`MAX_STATE_DOMAINS_PER_SYNC` and rejects a duplicate `domain_id`; `connect_and_join` correlates
`SnapshotChunk`/`SnapshotCommit` `snapshot_id` and `connection_generation` against
`SnapshotBegin`/the admitted session (new additive `decode_snapshot_id`); `JoinRequest` gained a
`deadline` bounding the connect/handshake/every frame read; domain dispatch matches `(domain_id,
snapshot_type)`, rejecting an unregistered type on a known domain. Plus a CI failure at
`stage=dev_client` itself: the door-overlay assertion wrongly assumed a fresh door (closed,
revision 0) when `use_wire` runs immediately before and leaves it closed at revision 2 — first
fixed with a live `door`/`runtime` query (superseded below).

Round 2 (2 more P2 findings, plus a second CI failure): (1) the TLS handshake now checks
`stream.get_ref().1.alpn_protocol()` and rejects (sending nothing) unless it is exactly
`oteryn-game/1`, including when none was negotiated. (2) `SnapshotBegin`'s full declaration is now
enforced: a new additive `decode_snapshot_begin` (protocol-oteryn) extracts `chunk_count` and
`total_encoded_bytes`; a new additive `decode_snapshot_chunk_framing` extracts each chunk's
`chunk_index` and raw `data` length; `connect_and_join` reads exactly `chunk_count` chunks in
order, sums their `data` length against `total_encoded_bytes`, and accepts the commit only once
both match. Separately, CI (`Merge gate / Node boot against the real Platform`, which explicitly
runs the `#[ignore]`d `node_boot_seam_against_running_node`) failed: that mode's `SeamClients`
has no local `door`/`runtime` object (the node is a separate process), so the round-1 live-query
fix could not run there. Fixed by dropping the live query entirely: the expected door
placement/state/revision (closed, revision 2) and content generation are now derived the same way
in both modes, from the same construction `use_wire_frames`'s own final overlay delta already
uses — proven to match the real wire by `use_wire`'s own byte-for-byte frame comparison just
above, not a live runtime read. `SeamClients.door` was removed; both seam functions compile and
run the `stage=dev_client` logic identically.

## Architecture and source of truth

- `PROVEN`: #162 allocation 5875272684; coordinator resume 5875470550; two Codex P2 review rounds
  plus two CI failures on PR #1147's frozen final head, each repaired in turn.
- `PROVEN`: ADR-0011 §3/§5/§6; FND-02 §16 (`foundation.proto` `SnapshotBegin`/`SnapshotChunk`);
  `RESOURCE_LIMITS_REGISTRY.json` `FND02-STATE-DOMAINS-PER-SYNC`; `PROTOCOL_OTERYN_V1_REGISTRY.json`
  (snapshot type 1 only, domains 1/2).
- `DERIVED`: door overlay wire identity and its post-`use_wire` end state (closed, revision 2) -
  `gameplay_transport/mod.rs::observe_world_object_overlay`, `world_runtime.rs:441/448`, and
  `use_wire_frames`'s own final `overlay_delta`, which the `use_wire` stage proves matches the wire.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No production mutation, session, lease, generation or authority evidence introduced or changed.
  The dev client is admitted through the exact same PREPARE/COMMIT-free fresh-admission path every
  other seam client already exercises. All fixes across both rounds are decode-side
  correlation/bounds/dispatch/ALPN hardening and test-assertion corrections, not new authority
  surface.
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
  protocol-oteryn 47/47 (43 prior round + 4 new: begin/chunk-framing round trip plus round-1's
  domain-bound tests); dev-client 9/9 (5 prior round + 4 new: no-ALPN rejection, short transfer,
  wrong chunk index, assembled-length mismatch); game-server --lib 669/0/2-ignored (unchanged);
  both `cargo tree` runs confirm `oteryn-dev-client` outside both production closures;
  `workspace-boundaries: PASS`; governance and repository-policy validators PASS.

### Component/integration

- run: `cargo test --offline -p oteryn-dev-client`
- result: PASS - 9 same-process fake-TLS-server tests, including a trusted server with no
  negotiated ALPN, a `SnapshotBegin` declaring 2 chunks with only 1 sent, a wrong `chunk_index`,
  and an assembled-length mismatch, alongside the prior round's coverage.

### E2E

- scenario: `server_seam_real_owners_over_tcp_tls`'s `stage=dev_client` (real composed WP5 S3-B
  owners, real loopback TCP+TLS) and `node_boot_seam_against_running_node`'s use of the same
  stage logic (externally-running node).
- result: both `#[ignore]`d locally (no exact Platform checkout / PostgreSQL 17.6 service, and no
  running node-boot topology, available locally); both compile/type-check cleanly. The frozen
  final head of PR #1147 DID run both for real in CI across the two prior rounds and reached
  `stage=dev_client` each time (every earlier stage passed) before the two failures fixed above;
  this candidate fixes both and restarts exact-head CI on both jobs.

### Exact-head CI

- candidate: the frozen final head of PR #1147 (after this round's push); its live checks govern.
- result: pending (supersedes the two prior failed runs, neither of which is the tested head)

## Self-review

- exact head: the frozen final head of PR #1147
- method/reviewer: implementing agent (this session)
- material findings: 6 Codex P2 findings across two review rounds, plus two self-discovered CI
  failures (round 1: fresh-door assumption; round 2: the round-1 fix's live-query door check
  could not run in `node_boot_seam_against_running_node`, which has no local door object). All
  repaired test-first with a dedicated regression test each where applicable; the round-2 door
  fix replaces the round-1 live query with a derivation that holds in both entry points.
- verdict: ready for independent review

## Independent review

- required: YES - touches two shared-lease files (`tools/architecture-check/**`,
  `crates/protocol-oteryn/**`), changes the production-closure checker's semantics, adds a
  workspace member wired as a dev-dependency of a production root.
- exact head: the frozen final head of PR #1147
- method/auditor: Codex via control-plane trigger; round-2 threads replied naming the fixing
  commit, not resolved by this agent
- material findings: 6 P2 findings across two rounds; disposition `fixed` for each
- verdict: pending

## Excluded scope

`.github/**`, `tools/repository/**` (unchanged); `apps/client/**`/`oteryn-client` closure
(unchanged, verified via `cargo tree`); production `apps/game-server/src/**` other than
`qualification.rs` (unchanged); protocol registries/proto files (unchanged); `wp5_s3b/run.sh`
(unchanged). No `@codex` post, no auto-merge enabled, review threads not resolved by this agent.

## Context checkpoint

```yaml
last_progress: fix round 2 (2 Codex P2 + node-boot CI failure) pushed; replied on the 2 new threads
status: ready
branch: claude/dev-client-join-c1b
head_sha: 97c4641e52f3ca4f11eeb28a80fa69a856352396
pr: 1147
final_head_sha: the frozen final head of PR #1147 (supersedes both prior failed CI runs)
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
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: await new exact-head CI and review on PR #1147
```
