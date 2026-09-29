# OTV2-20260929-n1-session-crate

```yaml
task_id: OTV2-20260929-n1-session-crate
title: N1 - transport-neutral session crate and TLS/TCP adapter extracted from tools/dev-client (ADR-0020)
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
allocation: "#162 allocation of ADR-0020 child N1 (repository worker)"
base_branch: main
branch: claude/n1-session-crate
base_sha: 3e3ed1aeac84280b91b38053383e83f395aebe19
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "repository worker (Claude Code session_01PwTJFS62J35S88Srpqnrgx)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - crates/session/**
  - crates/session-tcp/**
  - tools/dev-client/**
  - Cargo.toml                      # shared: two workspace members
  - Cargo.lock                      # shared: two new package entries, dev-client dependency list
  - workspace-boundaries.toml       # shared: members, paths, production, edges
  - docs/agents/tasks/active/OTV2-20260929-n1-session-crate.md
public_contracts:
  - FND-02
depends_on: [ADR-0020]
blocks: [N3, N4, N5]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ADR-0020 section 1 crate edge for lane N1, with no behaviour change.

- `oteryn-session` (`crates/session`): connect-agnostic session over an abstract byte stream
  (`SessionStream`, blanket over tokio `AsyncRead + AsyncWrite + Unpin`). It holds admission
  (`Session::admit`), envelope validation, sequencing, join-snapshot assembly, `step`/`use_object`, command
  status pairing, duplicate results and liveness, all moved verbatim from `tools/dev-client`. It uses
  `oteryn-protocol-oteryn` for every codec and names no TLS or TCP type. It re-exports `ALPN_OTERYN_GAME_V1`
  so an adapter needs no direct protocol edge.
- `oteryn-session-tcp` (`crates/session-tcp`): `connect` = TCP connect, rustls TLS 1.3 handshake against one
  pinned root, exact ALPN `oteryn-game/1` check before returning the stream (`TcpTlsStream`), each stage bounded
  by the deadline. Depends on `oteryn-session` only among workspace members. A compile-time assertion proves
  `TcpTlsStream` satisfies `SessionStream`.
- `oteryn-dev-client` is a thin harness: it keeps `JoinRequest`, `connect_and_join`, `connect_session`,
  `DevClientSession` and the flat `DevClientError` (a lossless `From<SessionError>`/`From<TcpAdapterError>`
  mapping with no wildcard arm, so a new session variant fails the build until mapped) and delegates to the two
  crates. Its 32 tests are unchanged in body; only the test-module import list changed because helpers moved.
  `oteryn-session` gained two tests over an in-memory duplex stream (admit + join + `step` with delta; close
  before admission fails closed), proving the boundary carries no transport.
- Naming: `oteryn-session` and `oteryn-session-tcp` contain none of the forbidden fragments `canary`,
  `protocol-core`, `transport`, `game-session`, `session-lease`, `persistence`.

## Registration

`workspace-boundaries.toml`: both crates added to `members`, `paths` (same relative position), `production`;
edges `oteryn-session = [oteryn-protocol-oteryn]`, `oteryn-session-tcp = [oteryn-session]`,
`oteryn-dev-client = [oteryn-protocol-oteryn, oteryn-session, oteryn-session-tcp]`.
`oteryn-client` and `oteryn-game-server` gain no edge, so neither production closure changes.

## CI and pins

No `.github/**`, merge-authority pin or `validate_repository_policy_core.py` hash edit is needed for N1:
the closure check in `merge-gate.yml`, `merge-group-gate.yml` and `rust.yml` walks
`cargo tree --edges normal,build` from `oteryn-client` and `oteryn-game-server`; the new crates are reachable
from neither (checked locally: 0 matches for `oteryn-session` or `oteryn-dev-client` in both trees; the
game-server reaches the dev client only through its dev edge). The closure amendment
("`oteryn-client` contains `protocol-oteryn` only through the session crate") is lane N5, batched with #1083.

## Acceptance criteria

- [x] Session crate holds no codec and names no TLS/TCP type; TCP adapter implements the stream boundary.
- [x] `oteryn-dev-client` has no session logic left; public test surface kept; tests pass.
- [x] `cargo +1.94.0 fmt --all -- --check`.
- [x] `cargo clippy --all-targets -D warnings` on `oteryn-session`, `oteryn-session-tcp`, `oteryn-dev-client`,
  `oteryn-synthetic-client-harness`.
- [x] Tests: `oteryn-dev-client` 32 passed; `oteryn-session` 2 passed; `oteryn-synthetic-client-harness` 16
  passed; `oteryn-architecture-check` 14 passed; `cargo run -p oteryn-synthetic-client-harness` prints
  `synthetic-ok`; `oteryn-architecture-check workspace .` PASS.
- [x] `validate_governance.py` and `validate_repository_policy.py` pass.
- [x] `apps/game-server` tests compile (`cargo check --tests`); its PostgreSQL-backed seam qualification that
  drives `connect_session` was not run locally.
- [ ] Required checks green on the exact frozen head.
- [ ] Independent review routed by the control plane (not done by this worker).

## Deviations and gaps

- No production runtime composition uses the new crates yet; `oteryn-client` is untouched (fail-closed entry
  stays). Client wiring, the closure change and CI edits belong to N4/N5.
- `DevClientError` stays a flat enum so existing matching tests are unchanged; its Display text still lives in
  the dev client beside the session crate's own. A production client can use `SessionError` and
  `TcpAdapterError` directly.
- No PR comment, review trigger or Jira update by this worker.
