# OTV2-20260928-dev-client-step-use-c2

```yaml
task_id: OTV2-20260928-dev-client-step-use-c2
title: dev client C2 - step and USE commands, decoded results and deltas, stage=dev_client door scenario
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1166
allocation_comment: control-plane allocation on #162 (alias "impl interaction")
base_branch: main
branch: claude/dev-client-step-use-c2
base_sha: ebc86db54a8733da6c7756a267bf8d038f42a794
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl interaction" (Claude Code)
created_at: 2026-09-28T00:00:00Z
updated_at: 2026-09-28T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/dev-client/**
  - crates/protocol-oteryn/**  # additive only
  - apps/game-server/src/gameplay_transport/qualification.rs  # dev_client stage + minimal plumbing
  - docs/agents/tasks/active/OTV2-20260928-dev-client-step-use-c2.md
  - docs/agents/tasks/archive/**  # git mv of the three merged C1a/C1b/repin-v2 records
public_contracts: []
depends_on:
  - OTV2-20260928-dev-client-join-c1b
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

The dev/qualification-only client (`tools/dev-client`, merged in C1b) can now play, not only join.
`connect_session` keeps the admitted connection as a `DevClientSession` (`connect_and_join` stays,
as the join-only wrapper). `step(direction)` sends the FND-02 `ClientCommand` type 1
(`WORLD_ACTOR_STEP_INTENT`, the command the `admission` seam stage sends); `use_object(placement,
expected_revision)` sends type 2 (`USE_INTENT`, USE-WIRE-V1, as `use_wire` sends). Each returns a
`CommandOutcome`: `command_id`, `status`, the typed disposition, the result's `server_sequence`
and the one server-sequenced delta the disposition promises (`Moved` -> domain 1 `WORLD_SPATIAL`,
`Committed` -> domain 2 `WORLD_OBJECT_OVERLAY`), already validated and applied to the session's
mirrored state.

Discipline, all fail-closed (any violation, timeout or I/O error makes the session
`SessionUnusable`, so nothing more is sent on a connection whose state is unknown):
- `CommandId` starts at `ServerAccepted.next_command_id`, strictly increases, and is consumed
  before the write; a locally unencodable command (placement over the key bound) is refused
  before anything is sent and consumes nothing.
- Every inbound frame passes `WireEnvelopeView::validate(ServerToClient, admitted)` and must
  carry the admitted `connection_generation`.
- `CommandResult` and `StateDelta` must arrive at exactly the previous applied `server_sequence`
  plus one (start: `SnapshotBegin.target_server_sequence`); a `CommandResult` must carry the id
  just sent.
- A `StateDelta` must name the promised domain, the registered `delta_type` (1), be based on
  exactly the last applied domain revision (start: the join snapshot's domain revisions, now
  exposed on `JoinSnapshot`), and carry the loaded `content_generation`.
- A `LivenessProbe` between frames is answered with a `LivenessAck` (last applied sequence) and
  otherwise ignored, so the 5 s idle cadence cannot desynchronise a slow client.

`crates/protocol-oteryn` gained only additive client-direction codecs, each round-tripped
against the existing server encoder: `encode_client_command` (+ `ClientCommandValue`),
`encode_liveness_ack`, `decode_liveness_probe`, `decode_command_result` (+ `CommandResultView`),
`decode_state_delta` (+ `StateDeltaView`), and `SnapshotBeginFields.target_server_sequence`.
`tools/dev-client` holds no codec of its own.

`stage=dev_client` (`qualification.rs`) now, after the unchanged join assertions, drives with the
dev client itself: east (next to the door), USE open (COMMITTED, revision +1), north (through the
doorway), south (out), USE close (COMMITTED, closed, revision +1). Every expectation is exact and
derived as `use_wire_frames` derives its own (same room, cells, door key/state keys, ids and
sequences from a fresh session, spatial revision chain from the join baseline, door revision
chain from the joined door revision). The stage is one code path shared by both seam entry points
(`server_seam_real_owners_over_tcp_tls`, `node_boot_seam_against_running_node`) and uses no
`runtime` handle. Admission count (4) and release choreography are unchanged: the client is
dropped after the last command, so the connection closes silently exactly as before, then the
same control-loss (60 s) and grace-release (100 s) waits run.

The three merged records (C1a #1131, C1b #1147, repin-v2 #1155) are archived by `git mv`, each
`status: completed` with `merge_pr`/`merge_sha`.

## Architecture and source of truth

- `PROVEN`: merged C1b code (`tools/dev-client`), FND-02 `foundation.proto`, USE-WIRE-V1
  `world_object_v1.proto`, FIRST-CONTROL-WIRE-V1 `world_spatial_v1.proto`.
- `PROVEN`: server behaviour read in `gameplay_transport/connection.rs::serve_admitted` and
  `mod.rs::use_object`: result then at most one delta; spatial base = own per-actor revision
  (fresh 1); overlay delta base = committed revision - 1; snapshot overlay domain revision = door
  revision.
- `DERIVED`: door end state of the preceding `use_wire` stage (closed, revision 2), hence the
  dev client's open -> 3 and close -> 4.
- `UNKNOWN` (not exercised): real seam E2E locally, see Validation/E2E.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: >
  No production code changes. The dev client is a dev-dependency tool admitted through the same
  fresh-admission path as every other seam client; the stage adds client commands on an already
  admitted session. No session, lease, generation, authority or persisted-recovery semantics are
  introduced or interpreted (the client only checks the generation it was given).
```

## Validation

### Focused

- run: `cargo +1.94.0 fmt --all --check`; `cargo clippy --offline --workspace --all-targets -- -D warnings`;
  `cargo test --offline -p oteryn-protocol-oteryn -p oteryn-dev-client`;
  `cargo test --offline -p oteryn-game-server --lib`; `cargo check -p oteryn-game-server --tests`;
  `cargo run --locked -p oteryn-architecture-check -- workspace .`;
  `python3 tools/agents/validate_governance.py`;
  `python3 tools/repository/validate_repository_policy.py`; `git diff --check`
- result: all PASS. protocol-oteryn 57/57 (+8: client command, liveness ack/probe, command
  result, state delta, snapshot target sequence, each with negatives); dev-client 25/25 (+13:
  full door scenario with every disposition, rejected step, liveness, unencodable use, one
  negative test per client check, stall timeout); game-server --lib 695/0/2-ignored.

### Component/integration

- run: `cargo test --offline -p oteryn-dev-client`
- result: PASS. Fake-TLS-server tests: the server decodes every client command through
  `WireEnvelopeView::client_command` and answers with the crate's server encoders; the join
  starts away from fresh-admission constants (first CommandId 7, target sequence 40, spatial
  revision 5, door revision 2) to prove the client derives them from the join.

### E2E

- scenario: `server_seam_real_owners_over_tcp_tls` `stage=dev_client` and
  `node_boot_seam_against_running_node` (same stage code).
- result: NOT RUN LOCALLY - needs PostgreSQL 17.6 and the exact Platform checkout
  (`tools/qualification/wp5_s3b/run.sh`); the sandbox has PostgreSQL 16 only. Type-checked by
  `cargo check -p oteryn-game-server --tests`. CI job `Merge gate / Server Seam over TCP+TLS`
  and the node-boot job run it for real on the frozen final head of the PR.

### Exact-head CI

- candidate: the frozen final head of the PR; its live checks govern.
- result: pending

## Self-review

- exact head: the frozen final head of the PR
- method/reviewer: implementing agent (this session)
- material findings: none open; expectations cross-checked line by line against
  `use_wire_frames` and `serve_admitted`.
- verdict: ready for independent review

## Independent review

- required: YES - extends a shared-lease protocol crate (additive) and the seam qualification.
- verdict: pending (requested through the control plane, no `@codex` from this worker)

## Excluded scope

`.github/**`, `tools/repository/**`, `apps/client/**`, production `apps/game-server/src/**`
other than `qualification.rs`, registries and proto files, `wp5_s3b/run.sh`: unchanged. No
`@codex`, no auto-merge, no review-thread resolution by this agent.

## Context checkpoint

```yaml
last_progress: implementation and local validation complete; PR #1166 opened
status: ready
branch: claude/dev-client-step-use-c2
pr: 1166
final_head_sha: the frozen final head of the PR
owner_action_required: null
blocker: null
next_action: await required checks and independent review on PR #1166
```
