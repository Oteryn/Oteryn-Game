# OTV2-20261005-ops-assign-report-1

```yaml
task_id: OTV2-20261005-ops-assign-report-1
title: OPS-ASSIGN-REPORT-1 ReportScopeAssignmentV1 from oteryn-game-ops
mode: IMPLEMENTATION
status: complete
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/ops-assign-report-1-20261005
issue: 1622
pr: 1822
created_at: 2026-10-05
updated_at: 2026-10-05
packet: docs/architecture/reviews/OTERYN_GAME_ARCH_LOGIN_FIRST_PACKETS_2026-10-05.md §2.3 @ 4b15cb4f701a90c024fa145f386434d9be79fcfe
owned_paths:
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - apps/game-server/src/native_admission_source/
  - apps/game-server/tests/native_scope_assignment.rs
  - apps/game-server/tests/native_admission_source_transport.rs
  - docs/agents/tasks/active/OTV2-20261005-ops-assign-report-1.md
  - docs/agents/tasks/archive/OTV2-20261005-ops-assign-report-1.md
public_contracts:
  - docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md §3, §5, §6, §10
depends_on: [P1 ARCH-LOGIN-FIRST-PACKETS-V1]
external_repositories: []
```

## Scope

`oteryn-game-ops` reports each committed scope assignment to Platform
(`ReportScopeAssignmentV1`, `POST /internal/v1/game-auth/native-scope-assignments`) with the
ownership-authority client certificate. `assignment assign|replace` send it after the commit;
`assignment report --world --channel` re-sends the current durable assignment. The epoch is
never raised here (U16).

## Design

- `native_admission_source/scope_assignment.rs`: exact §5 encoder and success decoder,
  ownership-authority descriptor (`OTERYN_GAME_SCOPE_OWNERSHIP_AUTHORITY`, purpose-separated
  key), one bounded exchange, and `report` that encodes once and re-sends the identical bytes
  with bounded backoff (250 ms doubling to 4 s, 8 attempts) until `accepted`/`superseded` or a
  definite `400`/`401`/`409`. `429`, `503`, transport failure and timeout retry.
- Reporting channel is a separate root-owned TOML file (`--report-config`): endpoint, peer
  name, trust roots, client certificate and key, `node_certificate_files` (the runtime-status
  certificate of every node-host identity named in any scope: exactly that set, each one
  required, and each leaf's complete subject, rendered as RFC 4514 short names such as
  `CN=node-a`, must equal its identity), optional further producer certificates (account characters), declared
  `assignment_epoch` (never raised, U-RS5 storage pending) and the node-host identities
  configured for each scope. Absent, the tool behaves as before (§15 rollback). It requires
  `--node-config`, whose native-evidence and runtime-status certificates (both required) are
  always compared with every node certificate: the authority key is refused if any producer
  leaf shares its public key (contract §3). The endpoint, peer name and trust roots (by
  content) must equal the node configuration's `[platform]` ones and the declared epoch its
  `platform.runtime_status.assignment_epoch`, or the tool refuses before anything is sent.
- `assign|replace --node-identity` validate the identity against the scope before anything is
  written and stage it with the operation; only a committed receipt (submit or `reconcile`)
  binds it to the target `(node_id, registration_revision)` in the state directory, so a
  rejected or failed operation binds nothing;
  the report body comes from the durable row (`ownership_generation`, `decided_at`), the bound
  identity and the declared epoch, so `assignment report --world --channel` re-sends it
  byte-identically. `reconcile` reports a committed assignment too. A failed report leaves the
  Game assignment authoritative. A revoke is not reported (CP D745): contract §5 gives no
  holder-less `node_identity`, and any configured identity in a revocation report could be
  matched by that host (§7). `revoke --node-identity` and `assignment report` on a revoked
  scope refuse and name the follow-up; a committed revoke logs `report=not_sent`. The
  transport bounds every response body to 256 bytes (`NRS-RESPONSE-BYTES`), not only `200`,
  whether framed by length, chunked or close-delimited.
  Logs carry scope ids, result class, attempts and elapsed time only (§10).

## Validation

`cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
`cargo test -p oteryn-game-server` (incl. `tests/native_scope_assignment.rs`: exact wire,
accepted, superseded, `400`/`401`/`409` stop, wrong-purpose identity, timeout/`503`/`429`
then success with identical bytes, exhausted retries, oversized response refused for every status
and framing, identical re-send, config parsing, a certificate for every node identity, a certificate whose subject is not its
configured identity refused, including extra subject attributes, Platform
channel and epoch match) and the `oteryn-game-ops` unit tests (identity staged until commit,
promotion binds once): pass.

- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.

## Open items

- REVOKE-REPORT-CONTRACT-1: a contract operation (or a §5 form) that reports a revocation
  without a matchable `node_identity`, owned by the contract owner and Platform. Until it
  lands, a still-credentialed revoked holder can keep its last generation routable until the
  next assign/replace report; an honest holder stops heartbeats and goes stale within F (§8.3).

## State

Complete with the PR.
