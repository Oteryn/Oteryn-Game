# Admin contracts and agent — source review

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.
Auditor: lead. Both packages: REVIEWED, static source scope complete.

## Scope and result

All tracked files in `crates/admin/contracts/` and `crates/admin/agent/` were substantively reviewed, including manifests, all production functions, and all existing tests. No new demonstrated defect was established in the current reachable executor. Native crypto, HTTP/mTLS and PostgreSQL tests were inspected but not run. This is not a claim of a live Platform interoperability test or deployed configuration validation.

## Contracts

`contracts/src/envelope.rs:1-163` defines closed request/response DTOs, risk ordering, typed errors and retryability. `catalog.rs:1-290` binds every catalog operation to its name, minimum risk, permission, target and closed parameter type; list limits are enforced at runtime (not merely emitted as JSON-schema annotations), and player search requires a filter. All 32 schema mappings in `schema.rs` were checked against the catalog. `json.rs:1-67` recursively rejects duplicate decoded keys and trailing input. `lib.rs:19-71` canonicalizes signed material, removes only the signature field, limits integer interoperability, uses strict Ed25519 verification and separates digest generation. Contract tests exercise canonical number/Unicode ordering, tampering, invalid JSON and generated schema agreement; test inspection is not a passing test result.

## Actual execution path

`agent/src/main.rs:10-41` parses fixed configuration and policy, initializes the signing key, database pool and HTTPS transport, then runs the agent. `config.rs` rejects non-HTTPS origins, credentials, queries, fragments and non-root paths. `policy.rs` bounds the signer roster, validates key windows and prevents lowering catalog risk or granting unrelated permissions. Policy projection omits signer material.

`channel.rs:85-94` advertises and executes **only `admin.policy.get`**. Catalog membership alone does not make other reads or writes reachable. This counterguard prevents treating future player/item/administrative mutation behavior as an existing vulnerability. `verifier.rs:23-173` checks operation/contract/version, trusted current signer, signature, bounded lifetime, nonce, closed parameters, UUIDv7 identities, query-only kind, permissions/world enablement, risk and required controls, then idempotency. Current query IDs must match operation IDs. The accepted ADM-01 verification order is preserved.

`channel.rs:166-213` authenticates before atomically consuming a nonce, performs authorization before idempotency lookup, rechecks expiry after store I/O, and persists successful results. Rejected requests are audited separately; failure to persist audit removes the claimed ledger ID before resigning. `store.rs:39-132` implements atomic nonce claims, unique idempotency insertion, concurrent-winner readback, digest conflict rejection, and a single transaction for result plus ledger. An early return when a prior winner exists drops a transaction with no new effect. Migration `0956_admin_control_plane.sql:1-61` gives the agent no gameplay authority and enforces append-only ledger behavior. Native role/locking behavior remains unexecuted here.

Transport bounds are a 30-second HTTP timeout, 1 MiB response cap and 32 requests per batch. Redirects are disabled. Heartbeats run in a sibling future during both channel polling and result delivery; supersession or transport failure terminates for supervision as documented. Existing paused-time tests check heartbeats while delivery is blocked, but were not executed during this review.

## Refuted or bounded hypotheses

- Cross-operation idempotency reuse does not currently provide a gameplay mutation path: one read executor is advertised and query request/operation IDs must match. Future executor expansion would need a fresh review.
- A supplied preview digest cannot manufacture current R4 execution evidence: the driver does not derive such evidence from the request and no R4 effect is advertised.
- Unknown operations and malformed signatures do not reach an executor; later validation failure burns the nonce by the explicit contract.
- A database failure does not create a successful response claiming an absent audit row.

## Dependencies and remaining limits

Owning material: `docs/architecture/ADM-01_ADMIN_CONTROL_PLANE_CONTRACT.md` verification/transport/risk sections and `docs/contracts/admin-control/v1/README.md`. Cross-package source reviewed: migration 0956. The Platform implementation, provisioned identities, current signer validity and actual supervision behavior were not exercised. No security finding is inferred merely from those unavailable runtime facts.
