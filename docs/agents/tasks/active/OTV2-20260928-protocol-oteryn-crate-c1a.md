# OTV2-20260928-protocol-oteryn-crate-c1a

```yaml
task_id: OTV2-20260928-protocol-oteryn-crate-c1a
title: protocol-oteryn crate C1a - extract FND-02 wire codecs
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
allocation_comment: latest #162 comment (owner accepted A6)
base_branch: main
branch: claude/protocol-oteryn-c1a
base_sha: e1ccfd40c08c39e968d7010589e3e31cbb5e3d71
owner: "Oteryn: content world client" (Claude Code)
created_at: 2026-09-28T15:53:00Z
updated_at: 2026-09-28T16:10:00Z
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/**
  - Cargo.toml
  - Cargo.lock
  - apps/game-server/Cargo.toml
  - apps/game-server/src/foundation/protocol.rs
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - apps/game-server/src/gameplay_transport/world_object.rs
  - docs/agents/tasks/active/OTV2-20260928-protocol-oteryn-crate-c1a.md
  - apps/game-server/src/foundation/mod.rs  # shared-lease grant (#162), re-export of moved protocol error types
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

Extracts the existing FND-02 wire codecs into a new real crate `crates/protocol-oteryn`
(`oteryn-protocol-oteryn`) that both the server and the future client (C1b) can use, per ADR-0011
§2 (move exactly the real items that have a consumer) and #162 A6. No new dependencies: the moved
code is pure `std` (hand-rolled protobuf-wire parsing, no `prost`/`bytes` usage existed to move).

Moved from `apps/game-server/src/foundation/protocol.rs` into `crates/protocol-oteryn/src/lib.rs`:
framing (`decode_framed_envelope`), the top-level envelope (`MessageType`, `Direction`, `Phase`,
`Sequencing`, `WireEnvelopeView` and its decode methods, `decode_wire_envelope`), the typed IDs
(`CharacterId`/`WorldId`/`ChannelId`/`NodeId`/`GameSessionId`), ClientBootstrap/ClientResume views,
ClientCommand/CommandResult, StateDelta, snapshot chunk/commit, liveness probe/ack, protocol-error
encode, the ALPN constant, and `FoundationProtocolError`/`ProtocolDisposition`/`FrameLength`/
`MAX_WIRE_FRAME_BYTES` (originally in `foundation/mod.rs`, moved under the shared-lease grant
below since every moved codec function returns this error type). Moved from
`apps/game-server/src/gameplay_transport/{world_spatial,world_object}.rs`: the FIRST-CONTROL-WIRE-V1
and USE-WIRE-V1 typed payload codecs, verbatim (already self-contained; `world_object.rs`'s one
`crate::content::FIRST_PRODUCTION_MAX_KEY_BYTES` import became a documented literal `512`, since
the new crate cannot depend on `apps/game-server`). All 40 moved round-trip/registry-binding tests
(28 envelope + 4 `world_spatial` + 7 `world_object` + 1 renamed count check) moved with the code;
`include_str!` registry paths adjusted from `../../../../docs/contracts/...` (4 levels, under
`gameplay_transport/`) to `../../../docs/contracts/...` (3 levels, directly under `src/`).

Stayed in `apps/game-server/src/foundation/protocol.rs` (server-only, no consumer outside this
crate: `CommandRef` used only by `world_runtime.rs`/`item_mint_audit.rs` durability correlation;
`ResyncPlan`/`plan_resync` had zero consumers anywhere including their own then-test; the
connection-state trackers `StateRevisionTracker`/`ServerSequenceTracker`/the local `SnapshotBarrier`
wrapped by `snapshot_facade.rs`): 12 tests stayed with them, including one hybrid test
(`first_control_server_frames_pass_foundation_ingress_validation`) that exercises both the moved
encode/decode functions and the staying `snapshot_facade::SnapshotBarrier`.

## Architecture and source of truth

- `PROVEN`: #162 latest allocation comment, owner acceptance of A6 (extract FND-02 codecs into
  `crates/protocol-oteryn` for C1a).
- `PROVEN`: `docs/repository/PLAYABLE_FIRST_ENGINEERING_POLICY.md` / ADR-0011 §2 - move exactly the
  real items that have a consumer, no speculative API.

## Shared-lease grant: apps/game-server/src/foundation/mod.rs (resolved)

`FoundationProtocolError`/`ProtocolDisposition`/`FrameLength`/`MAX_WIRE_FRAME_BYTES` were defined
in `mod.rs`, not `protocol.rs`. Every moved codec function returns `FoundationProtocolError`, and
two unowned files depend on its *exact* type identity, not a merely-convertible copy:
`world_runtime.rs:1430` types a `map_err` closure parameter as `FoundationProtocolError`, and
`connection.rs`/`qualification.rs` construct and pattern-match it directly. This task first
stopped and reported `SHARED_LEASE_REQUIRED` naming `mod.rs`. The control plane granted a lease
scoped to exactly replacing those four definitions with `pub use oteryn_protocol_oteryn::{...};`
re-exports (plus the matching trait impls, which cannot stay in `mod.rs` once the type is foreign:
orphan rules require `Display`/`Error` for `FoundationProtocolError` to live where the type is
defined). Nothing else in `mod.rs` changed. Because the re-export makes it the same type, every
existing consumer (`connection.rs`, `qualification.rs`, `world_runtime.rs`, `snapshot_facade.rs`,
`tcp_tls.rs`, and others) compiles unchanged - verified by the full build/test/clippy run below.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  Pure code motion of existing wire codecs (encode/decode + typed IDs) plus a re-export shim.
  No new I/O, session, lease, generation or authority evidence; no PREPARE/COMMIT; no protocol,
  admission or persistence semantics changed. The server's decoded/encoded bytes are identical
  (verified by the moved round-trip tests, unchanged).
```

## Validation

### Focused

- command/run: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test -p oteryn-game-server --lib`; `cargo test -p oteryn-protocol-oteryn`; `python3
  tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`
- result: fmt PASS; clippy PASS (0 warnings across the whole workspace, including the standalone
  `apps/game-server/tests/*_postgres.rs`/`wp5_s3b_composition.rs` Foundation test binaries that
  `#[path]`-include `foundation/mod.rs` without `gameplay_transport`); `oteryn-game-server --lib`
  655 passed / 0 failed / 2 ignored (unchanged from main's total minus the 40 tests that moved:
  41 protocol.rs + 4 `world_spatial.rs` + 7 `world_object.rs` = 52 total; 12 stayed, 40 moved);
  `oteryn-protocol-oteryn` 40 passed / 0 failed. Governance and repository-policy validators PASS.
- `cargo deny`: not installed in this environment and no supply-chain script exists locally beyond
  the CI `cargo-deny-action` in `.github/workflows/{rust,merge-gate}.yml`; not run. No new
  dependency was introduced: the new crate has zero non-dev dependencies (the moved code is pure
  `std`) and its one dev-dependency (`serde_json`, for the moved registry-binding tests) already
  exists in `workspace.dependencies` at the same pinned version used elsewhere in the workspace.

### Component/integration

- command/run: `NOT_APPLICABLE` - pure code motion, no new integration surface.
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE`
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending (this record is written before the freeze/push; see the PR for current
  head/checks)
- trigger source: push to `claude/protocol-oteryn-c1a`; workflow/runner/classification/result:
  pending

## Self-review

- exact head: pending (not self-referenced in this commit)
- method/reviewer: implementing agent (this session)
- material findings: none found; the one architectural blocker (mod.rs FoundationProtocolError
  ownership) was resolved via the shared-lease grant above before authoring the rest of the move
- verdict: ready to freeze

## Independent review

- required: YES - this touches a shared-lease file (`foundation/mod.rs`) and adds a new crate to
  the workspace, per root governance.
- exact head: pending
- method/auditor: pending (no `@codex` trigger from this task per its instructions)
- material findings: pending
- verdict: pending

## Excluded scope

`apps/client`, `crates/client-*`, `platform-client`, `connection.rs` logic (re-exports keep it
unchanged), registries/proto files, CI workflows, any `mod.rs` content beyond the four re-exported
names. No `@codex review` trigger, no comment on #162.

## Context checkpoint

```yaml
last_progress: full extraction implemented and locally validated (fmt/clippy/tests/governance
  green); freezing and opening PR next
status: implementing
branch: claude/protocol-oteryn-c1a
head_sha: pending
pr: pending
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
next_action: commit, push, open PR against main referencing #162 A6 and this record
```
