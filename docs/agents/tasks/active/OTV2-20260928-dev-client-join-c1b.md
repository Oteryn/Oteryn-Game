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
head_sha: fbca533c3e08b35ce27ffaf4d2cb42ecc3d055fd
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
native client (ADR-0011 §6). It connects over rustls TLS 1.3 (ALPN `oteryn-game/1`), admits with
the WP5 fixture grant mechanism the existing seam E2E already uses, and decodes the join
snapshot's domains 1 `WORLD_SPATIAL` and 2 `WORLD_OBJECT_OVERLAY`, asserting the native entry
door overlay by its wire identity: placement `oteryn:cell/entry-door` and its state key
(`oteryn:local-object/entry-door` is the content definition key, never placed on the wire -
traced through `gameplay_transport/mod.rs::observe_world_object_overlay` and
`world_runtime.rs:441/448`). It holds no codec of its own: TLS transport + frame length-prefixing
+ dispatch over `crates/protocol-oteryn`'s codecs only.

Proved E2E by adding `stage=dev_client` to `qualification.rs`'s `server_seam_real_owners_over_tcp_tls`
(existing CI job `Merge gate / Server Seam over TCP+TLS`, `wp5_s3b/run.sh` unmodified). Admits
`characters[1]` (free: released by the `use_wire` stage above), asserts the decoded snapshot, lets
the connection close (same "silent" pattern `concurrent[0]`/`use_session` use), waits through the
same control-loss (60s) -> grace-release (100s) cycle, so `committed_players == 1` /
`committed_admissions` (4 -> 5) keep holding. Moved `accepted_session()`'s fixed-offset
`ServerAccepted` hand-parse onto the new `decode_server_accepted` ("move, don't duplicate"); the
other hand-rolled `bootstrap()`/`envelope()` helpers stay (they must emit deliberately
non-conforming frames for `foundation_negatives`/`fnd04_negatives`, which the strict
`encode_client_bootstrap` cannot produce by construction).

## Coordinator-resolved finding (owner: option 1)

First pass stopped `DESIGN_DECISION_REQUIRED`: `tools/architecture-check`'s `internal_edges()`
built its graph from every `cargo metadata` `path` dependency regardless of `kind`, so any
`[dev-dependencies]` path edge from a production root onto a non-production tool package failed
`validate_production_closure` however declared (reproduced empirically, then reverted, before the
stop). Owner picked option 1 (#162 comment 5875470550): make the checker kind-aware.

- `tools/architecture-check`: `internal_edges` now splits by `cargo metadata` `kind` into `normal`
  (`null`/`"build"`) and `dev`. New `[dev_edges]` table in `workspace-boundaries.toml`, checked for
  exact equality like `[edges]` (`validate_declared_edges`) but never walked by
  `validate_production_closure`/`validate_acyclic` (both still read only `policy.edges`, per "normal
  and build rules are unchanged"). Added the 3 requested tests plus a 4th (`internal_edges` itself
  splits correctly by kind). `oteryn-game-server`'s normal `[edges]` unchanged; `[dev_edges]` adds
  `oteryn-game-server -> oteryn-dev-client` and, since the split also surfaced a pre-existing
  dev-only edge miscategorized under the old kind-blind check (`oteryn-client-simulation`'s
  `[dev-dependencies] oteryn-foundation`, already on `main`), moved that one too (no behavioral
  change - never reachable from a production root either way).
- `crates/protocol-oteryn`: additive-only, wire format unchanged. `ClientBootstrapValue`/
  `encode_client_bootstrap` (round-trips through the existing server-side
  `WireEnvelopeView::client_bootstrap()`). `ServerAcceptedFields`/`decode_server_accepted` (reuses
  the existing `validate_server_acceptance_ingress`, round-trips through the existing
  `encode_server_accepted`). `decode_snapshot_chunk` (walks a chunk body's `DomainSnapshot`
  entries via the crate's own public primitives; round-trips through the existing
  `encode_single_chunk_snapshot`). 43/43 tests pass (40 existing unchanged + 3 new).

## Architecture and source of truth

- `PROVEN`: #162 allocation 5875272684; coordinator resume 5875470550 (option 1, leases, seam-stage
  plan approval, door assertion by placement + state key).
- `PROVEN`: ADR-0011 §3/§5/§6 - production client stays fail-closed; §6 allows this dev harness.
- `PROVEN`: `docs/repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md` - minimum-sufficient; dev client
  adds exactly connect + admit + join-snapshot decode, no speculative API.
- `DERIVED`: door overlay wire identity - `gameplay_transport/mod.rs::observe_world_object_overlay`
  through `world_runtime.rs:441` (`placement_key` = `DOOR_CELL.0`) vs `:448` (`DOOR_DEFINITION`,
  content-resolution only, not wire).

## Shared-lease grants (resolved by coordinator resume)

Both opened by #162 comment 5875470550 after this task's own stop; see the finding above for
exact scope. `crates/protocol-oteryn/src/lib.rs` gained three new public items and their
round-trip tests only; no existing function's behavior or signature changed.
`tools/architecture-check/src/lib.rs` gained the kind split, a `validate_declared_edges` helper
(extracted from `validate_workspace`'s prior inline loop, behavior-preserving for every existing
`[edges]` member), and four new tests; `validate_acyclic`/`validate_production_closure` untouched.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No production mutation, session, lease, generation or authority evidence introduced or changed.
  The dev client is admitted through the exact same PREPARE/COMMIT-free fresh-admission path
  (FND-04 grant verification, S2 durable admission) every other seam client already exercises.
  tools/architecture-check and the wire-codec additions are dev/CI-time encode-decode only.
```

## Validation

### Focused

- run: `cargo +1.94.0 fmt --all --check`; `cargo clippy --offline --workspace --all-targets -- -D warnings`;
  `cargo test --offline -p oteryn-architecture-check -p oteryn-protocol-oteryn -p oteryn-dev-client`;
  `cargo test --offline -p oteryn-game-server --lib`; `cargo run --locked -p oteryn-architecture-check -- workspace .`;
  `cargo tree --locked -p oteryn-client --edges normal,build`; `cargo tree --locked -p oteryn-game-server --edges normal,build`
  (both: `oteryn-dev-client` absent); `cargo tree --locked -p oteryn-game-server --edges dev` (present);
  `python3 tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`
- result: fmt PASS (workspace-wide); clippy PASS (0 warnings, whole workspace); architecture-check
  14/14 (10 + 4 new); protocol-oteryn 43/43 (40 + 3 new); dev-client 2/2 (new; a same-process
  fake-server round trip: TLS handshake, `ClientBootstrap` admission, join-snapshot decode with
  the door overlay present, plus a server-closes-before-admission negative); game-server --lib
  669/0/2-ignored (unchanged pass count; the 2 ignored are the topology-gated seam tests); both
  `cargo tree` runs confirm `oteryn-dev-client` outside both production closures;
  `workspace-boundaries: PASS`; governance and repository-policy validators PASS.

### Component/integration

- run: `cargo test --offline -p oteryn-dev-client`
- result: PASS - same-process fake TLS server (rustls + rcgen), real `oteryn-protocol-oteryn`
  encode/decode both sides, including a `WORLD_OBJECT_OVERLAY` entry with the exact
  `oteryn:cell/entry-door` placement/closed-state wire values the real server sends.

### E2E

- scenario: `server_seam_real_owners_over_tcp_tls`, new `stage=dev_client`, against the composed
  WP5 S3-B owners (Platform + PostgreSQL 17.6) over real loopback TCP + TLS 1.3.
- result: `#[ignore]`d - neither the exact Platform checkout nor a PostgreSQL 17.6 service was
  available locally (`wp5_s3b/run.sh` needs `PLATFORM_SOURCE`/`OTERYN_TEST_POSTGRES_ADMIN_URL`).
  Compiles/type-checks cleanly (`cargo check -p oteryn-game-server --tests`, full-workspace
  clippy); every other stage of the scenario is unchanged. Runs for real in the existing CI job
  `Merge gate / Server Seam over TCP+TLS` once qualified.

### Exact-head CI

- candidate: PR #1147, head `fbca533c3e08b35ce27ffaf4d2cb42ecc3d055fd`; live exact-head checks on
  that PR govern.
- result: pending

## Self-review

- exact head: `fbca533c3e08b35ce27ffaf4d2cb42ecc3d055fd` (PR #1147)
- method/reviewer: implementing agent (this session)
- material findings: none in the final candidate. The one architectural blocker
  (architecture-check's kind-blind closure walk) was surfaced by an initial
  `DESIGN_DECISION_REQUIRED` stop with reproduced empirical evidence, resolved by the
  coordinator's option-1 lease grant before this candidate was authored.
- verdict: ready for independent review

## Independent review

- required: YES - touches two shared-lease files outside the original owned paths
  (`tools/architecture-check/**`, `crates/protocol-oteryn/**`), changes the production-closure
  checker's semantics, adds a workspace member wired as a dev-dependency of a production root.
- exact head: `fbca533c3e08b35ce27ffaf4d2cb42ecc3d055fd` (PR #1147)
- method/auditor: pending control-plane trigger
- material findings: pending
- verdict: pending

## Excluded scope

`.github/**`, `tools/repository/**` (unchanged); `apps/client/**`/`oteryn-client` closure
(unchanged, verified via `cargo tree`); production `apps/game-server/src/**` other than
`qualification.rs` (unchanged); protocol registries/proto files (unchanged - only
`crates/protocol-oteryn/src/lib.rs`'s Rust surface grew, additively); `wp5_s3b/run.sh` (unchanged
- `cargo test -p oteryn-game-server --no-run` already builds `oteryn-dev-client` transitively).
No `@codex` post, no auto-merge enabled.

## Context checkpoint

```yaml
last_progress: PR #1147 opened; full local validation green; awaiting exact-head CI and review
status: ready
branch: claude/dev-client-join-c1b
head_sha: fbca533c3e08b35ce27ffaf4d2cb42ecc3d055fd
pr: 1147
final_head_sha: fbca533c3e08b35ce27ffaf4d2cb42ecc3d055fd (unless a material repair requires a new candidate)
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
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: await required checks and independent review on PR #1147
```
