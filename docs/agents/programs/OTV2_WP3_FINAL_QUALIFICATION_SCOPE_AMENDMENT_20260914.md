# WP3 exact final-qualification scope amendment

```yaml
allocation_id: OTV2-WP3-FINAL-QUALIFICATION-SCOPE-20260914
allocation_state: NOT_ACTIVE
repository: Oteryn/Oteryn-Game
coordinator: 162
issue: 351
pr: 356
worker_branch: agent/sqlx-driver-budget-351
allocation_base_main_sha: 2260308087c69be89e7b7f15a959550802e16e4f
source_wp3_head: 587cbccd0b5c3c25152aa39054bd1d02dca3a1e7
frozen_inventory: issue_162_comment_5655890678
risk: HIGH
```

This is a prospective scope-only proposal for the already-frozen E02 cell.
It creates no material writer, replacement implementation, numeric limit or
present source/workflow authority. Existing M03 implementation remains with
the sole canonical writer under #162/5657013898. M05 remains subject to frozen
order and its existing six-file admission. B01 audit rotation is separate.

## Concrete evidence requiring this scope

The canonical workspace excludes vendored unit targets. Its current PR Linux
job explicitly executes four Tokio targets and configured durability_postgres,
but neither a workspace PASS nor the direct SQLx TLS helper supplies complete
core/PostgreSQL unit or registered-production-root qualification.

The proven external SQLx-core manifest uses the authored core source and
vendored Rustls/Tokio patches. The imported core lock instead resolves older
registry Rustls/Tokio. A versioned helper manifest and lock are needed for
reproducible exact-source hosted execution without changing production Cargo.

M03's newly executable standalone Rustls test build exposes two stale fixture
constructors in the previously unallocated msgs/handshake_test.rs. Their
Arc<PayloadU16> arguments no longer match the already-admitted TicketPayload
representation. This is a test-source compilation failure, not authority to
change ticket semantics or reopen protected retained-session proofs.
The separate verify.rs macro import and three handy.rs fixture arguments stay
under their existing focused-test grants; they are not new production scope.

M05's explicit literal-IP/separate-DNS/inline-CA construction and mutable
acknowledgement custody require adapting the existing registered-runtime tests.
A URL-only fixture cannot silently stand in for the selected production root.
The six production files remain governed by #162/5653206745; this amendment
does not allocate any additional production path or generic pool symbol.

## Exact additional scope after protected activation

1. `vendor/rustls-0.23.43/src/msgs/handshake_test.rs`:
   only `sample_new_session_ticket_payload` and
   `sample_new_session_ticket_payload_tls13`, replacing their ticket fixture
   construction with the existing ordinary `TicketPayload::from_unowned`
   representation and the minimum required test-only import. Preserve sample
   bytes, expected wire encodings, every assertion and all production code.

2. `tools/qualification/sqlx-core/Cargo.toml` and `Cargo.lock`:
   an isolated test package whose library path resolves to the canonical
   `vendor/sqlx-core-0.9.0/src/lib.rs`, with relative patches to the existing
   vendored Rustls/Tokio and only required existing SQLx dependency composition.
   Preserve selected production versions/features and upstream attribution.
   No root Cargo, imported crate manifest/lock, dependency upgrade or new
   production feature authority follows.

3. `.github/workflows/merge-gate.yml::rust_linux`:
   add the final explicit PostgreSQL unit/strict-Clippy and SQLx-core
   unit/strict-Clippy commands alongside existing Tokio and configured PG
   execution. Consolidate them once after material test interfaces stabilize.
   Preserve job identity, permissions, event/target authority, checkout and
   source pins, matrix, failure propagation, existing steps and fan-in.

4. `tools/repository/validate_pr_gate_pg_sim.py` and
   `tools/repository/test_validate_pr_gate_pg_sim.py`:
   exact structural assertions for those commands and the corresponding
   `EXPECTED_EVIDENCE_JOB_SHA256['rust_linux']` value. Preserve all existing
   assertions and other fingerprints. Review actual final workflow content
   before computing its fingerprint; never accept arbitrary drift.

5. `apps/game-server/tests/durability_postgres.rs`:
   adapt only
   `registered_process_restart_reconciles_real_originals_without_releasing_custody`
   and
   `registered_runtime_shares_custody_and_retains_originals_across_all_handles`
   to the explicit production-profile and bounded recovered-custody interfaces.
   Add one private `wp3_registered_root_qualification` module for the frozen
   M05/E02 root construction, ready-demand, return/finality, cancellation,
   reaping/recovery, sequential acknowledgement and ambiguous-slot witnesses,
   with minimum in-file typed-profile/isolated-PKI/subprocess support.
   Preserve complete original-operation, fencing, restart and comparison
   assertions. Do not redirect production-root cases through LegacyFixture.

Existing vendor PostgreSQL resource/TLS fixtures may be reused only within
their already-admitted qualification purpose and custody. No new
`tests/support/postgres.rs`, public fixture API, fixture directory or generic
test-file lease is created. Report any concrete additional necessary symbol
before mutating it.

## Required qualification and review

Execute exact locked source graphs on Rust 1.94. The minimum added commands
are PostgreSQL `--lib` tests and strict `--lib --tests` Clippy, plus equivalent
core commands through the new helper with
`_rt-tokio,_tls-rustls-aws-lc-rs`. Final feature coverage follows the affected
resolved graph; ordinary Any/offline compatibility remains required where
the authored shared paths make it applicable. Do not duplicate focused core
filters when the full target already subsumes them. M03's isolated provider
tests permit ordinary parallel execution once exact-head evidence confirms it;
do not impose serialization as an unexplained substitute for fixing races.

Retain explicit Tokio targets, affected Rustls tests and ordinary/no_std proof.
Any upstream fixtures absent from the published crate must be obtained from
its exact VCS-pinned revision in isolated test staging, with source/fixture
digests and authored-source parity. Such staging is not permission to add
unallocated files to the canonical vendor tree or claim a modified test
overlay as exact-head qualification.

Configured PostgreSQL 17.6 must execute the actual registered production root,
including funded positive and phase-specific denial witnesses. Verify the
isolated admin target and pinned container identity before fixture mutation,
clean up partial failures, and report execution/skips honestly. The new inline
typed ResourceDenied family must be recognized where the actual denial phase
returns it; do not accept generic timeout as a substitute.

Require producer full-diff review and genuinely independent HIGH-risk review
of the exact QA/runtime-fixture changes and workflow/fingerprint changes.
Preserve all tests, security checks and compatibility; no ignored/suppressed
case, no-op/retrigger commit or missing-environment success can supply proof.

## Exclusions and activation

The four B01 paths and protected merge-group blob remain unchanged. This
proposal neither authorizes their audit rotation nor chooses the eventual
queue workflow blob. Final B01 content still requires explicit human-owner
authorization and independent deep review of its concrete rotation.

Activation requires independent exact-candidate review, required exact-head
CI, explicit human-owner authorization for this exact scope candidate,
authorized native exact-head Merge Queue, real merge_group game-gate success,
protected-main readback and explicit fresh #162 application to SAME #351/#356.
A generic continuation instruction or merged document alone does not activate
the source/workflow lease. Afterwards Work serializes the narrow fixture
repair with M03 as needed and the coherent final E02 routing after M03/M05;
it does not create a replacement cell or bypass their source/review gates.

No architecture, registry maxima, production topology, provider implementation,
generic pool, credentials/protection, external repository, WP4/WP5 or Server
Seam authority follows. Existing proven scopes remain retained by reference.
