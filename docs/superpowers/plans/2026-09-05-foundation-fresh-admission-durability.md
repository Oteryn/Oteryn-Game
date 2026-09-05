# Foundation Fresh-Admission Durability Implementation Plan

> Agent execution: use `superpowers:executing-plans` internally, subordinate to root AGENTS, the accepted design and exact coordinator allocation. One writer executes serially because all steps share Foundation API surfaces; useful independent review is read-only. No duplicate approval or planning gate.

**Goal:** Deliver Child A's complete Foundation semantic boundary for Issue #318.

**Architecture:** Foundation verifies credentials and independently current owning-source evidence, forms an immutable authorization, submits bounded persistence work and yields. Completion/reconciliation arrives as another normalized input; independently current facts still gate adoption. Owning-source publication also activates only after acknowledged/reconciled durable acceptance.

**Tech stack:** Existing Rust 1.94.0 workspace and dependencies; no Cargo changes.

**Spec:** `docs/architecture/reviews/OTERYN_GAME_FRESH_ADMISSION_DURABILITY_AUTHORITY_DECISION_2026-09-05.md`, decision `FND-DUR-FRESH-ADMISSION-V1`, integrated by #317 at `a8678d4a94e479a9aa2a92920379a4b32f95143b`; accepted FND-03/FND-04A and preserved reconnect V1/V2.

**Allocation:** `docs/agents/tasks/active/OTV2-20260905-foundation-fresh-admission-318.md`. This plan does not activate its prospective lease. Start `agent/foundation-fresh-admission-318` only from the exact protected merge SHA of that allocation, resolved on GitHub before first mutation.

## File map and limits

All runtime files below are under `apps/game-server/src/foundation/`:

| Exact file | Responsibility |
| --- | --- |
| `fnd04_verifier.rs` | Verified fresh facts and authenticated provenance |
| `admission_authority_publication.rs` (new) | Typed source capabilities, bootstrap/CAS/publication ports |
| `fresh_admission_durability.rs` (new) | Authorization, requests, outcomes, reconciliation/flow |
| `admission.rs` | Narrow checked core projection adoption |
| `admission_facade.rs` | Durable entry/adoption and compatibility disposition |
| `mod.rs` | Deliberate module/test/public export registration |
| `fresh_admission_durability_tests.rs` (new) | Independent-source regressions |

Only the worker's allocated task record is additional bookkeeping scope. No SQLx/SQL/schema/migration, Cargo/lockfile, workflow, listener/composition, source registration or `admission_recovery_inner.rs` edit. Reuse exported `AuthenticatedTransportRefV1`, `RuntimeScopeRefV1`, `AuthorityEvidenceFenceV1`, `CharacterLease` and `GameSessionAuthoritySnapshot::from_current_facts`. The two admission source files share one private module, so additive core implementations can stay in owned `admission.rs`.

Child A establishes no real producer availability or production readiness. Child B supplies physical durability; Child C registers and proves actual owning sources.

## Task 1 — Verified provenance result

**Files:** `fnd04_verifier.rs`, `mod.rs`, colocated verifier tests.

**Planned interfaces:** private-field `VerifiedFreshDurabilityFactsV1` and `verify_fresh_grant_durability_v1(token, now, durability_trust, current_authority) -> Result<VerifiedFreshDurabilityFactsV1, Fnd04ConsumerError>`. The durability trust/current context requires authenticated source capabilities, never a caller-filled `FreshCurrentEvidence`.

Retain AccountId, replay key, character/world/channel, signed account security generation, scope generation, route/runtime revisions, protocol/transport, each gameplay revision, key/profile identity and original iat/nbf/exp. The current evidence trait returns only key bytes/minimum generation: add a durability-specific extension or fail-closed unavailable-by-default methods retaining source authority/purpose/scope, comparable revision, decision identity, source time, uncertainty and accepted publication binding. Never default-forge provenance.

- [ ] Add RED tests `fresh_durability_retains_account_and_all_independent_fences`, `fresh_durability_preserves_authentication_error_precedence`, `fresh_durability_rejects_missing_provenance`, `fresh_durability_checks_account_character_before_world`.
- [ ] Run focused tests and preserve executed RED on the allocated branch.
- [ ] Share authenticated parsing/validation with the existing fresh verifier, keeping its signature and classification order. Return the richer value only after complete checks; no unauthenticated reparsing.
- [ ] Preserve strict NumericDate and conservative source-age predicates using checked math; run GREEN and publish the bounded checkpoint.

```bash
cargo +1.94.0 test --locked -p oteryn-game-server foundation::fnd04_verifier
```

Representative regression using the existing API:

```rust
#[test]
fn fresh_credential_expiry_keeps_existing_strict_boundary() {
    assert_eq!(NumericDate::validate(134, 100, 100, 130), Ok(()));
    assert_eq!(
        NumericDate::validate(135, 100, 100, 130),
        Err(NumericDateError::Expired),
    );
}
```

Current NumericDate requires lifetime <=30s, established nbf/issue-age conditions and strict `now < exp + 5`. Source-age independently requires a conservative upper bound <=5s including clock uncertainty. Derive the accepted deadline from all constraints without changing strict/inclusive endpoints. Test source age at/exceeding five seconds, future observation, absent trusted time, uncertainty exhausting the bound and arithmetic overflow separately. Preserve original times for the final L decision; cache/restart/receipt/COMMIT acknowledgment never re-age evidence.

## Task 2 — Typed publication capabilities and ports

**Files:** new `admission_authority_publication.rs`, `mod.rs`, split tests.

**Planned interfaces:** `AdmissionAuthorityPublicationV1`, explicit bootstrap/CAS operation, atomic multi-domain request, `AdmissionAuthorityPublicationReceiptV1`, bounded submission and exact-request completion/reconciliation types. Closed variants cover account/security/presence; character ownership/world/lease; runtime ownership/readiness/revisions; fixed verifier scope/key/profile trust. Typed expected guard bindings feed Task 3.

Private constructors and owning adapter capabilities prevent grants, raw facts, old receipts and arbitrary provenance structs from establishing current guard truth. Existing `AuthorityEvidenceFenceV1.source_revision` is an opaque String, not permission for lexicographic ordering: use source-defined comparable revision semantics.

- [ ] Add RED tests `publication_requires_owning_source_capability`, `publication_pending_does_not_activate_readiness`, `publication_exact_replay_preserves_source_time`, `publication_stale_cas_rejects`, `publication_equal_revision_contradiction_rejects`, `publication_missing_bootstrap_stays_closed`.
- [ ] Add compile-fail doctests preventing public grant/fact/receipt construction of publication and registered producer capabilities.
- [ ] Implement typed source/CAS/bootstrap requests, source nonrollback semantics and submit/yield/completion. No permissive Default or fixture-backed production constructor.
- [ ] Keep authority/readiness closed while publication is pending/ambiguous; activate only on exact receipt/reconciliation. Cover atomic multi-domain result, deny/tombstone retention, conflicting initialization and absent restart high-water floor with independent source state.
- [ ] Run focused/doctests, inspect capability visibility and publish GREEN.

```bash
cargo +1.94.0 test --locked -p oteryn-game-server foundation::fresh_admission_durability_tests::publication
cargo +1.94.0 test --locked -p oteryn-game-server --doc
```

These are Foundation semantic tests, not proof of PostgreSQL CAS or real source registration.

## Task 3 — Split-phase outcomes, reconcile and adoption

**Files:** new `fresh_admission_durability.rs`, `admission.rs`, `admission_facade.rs`, `mod.rs`, split tests.

**Planned interfaces:**

- `FreshAdmissionCommitAuthorizationV1`: verified facts, candidate GameSessionId and authenticated transport ref, initial connection generation exactly one, expected/proposed acquired lease generations, runtime binding, typed guard expectations and accepted deadline.
- `FreshAdmissionCommitRequestV1`: immutable retained request binding.
- `FreshAdmissionSubmissionV1::{Accepted, Unavailable}`: local bounded submission only.
- `FreshAdmissionDurableOutcomeV1::{Committed, ExistingCommitted, RejectedReplayConflict, RejectedIncumbent, RejectedStaleAuthority, AmbiguousOrUnavailable}`; closed collision variants where needed.
- `FreshAdmissionDurableReconciliationSnapshotV1`: immutable receipt plus current `GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>` from one fenced durable read.
- `FreshAdmissionDurabilityFlowV1` with `begin`, `accept_submission`, `accept_completion`, `accept_reconciliation` and independently-current adoption; bounded submit/reconcile ports with later normalized inputs and no synchronous journal supertrait.

- [ ] Add RED tests `fresh_submit_yields_without_controller`, `fresh_unavailable_submission_has_no_authority`, `fresh_completion_requires_exact_request`, `fresh_ambiguous_outcome_only_reconciles_original_binding`, `fresh_same_key_exact_retry_returns_original_commit`, `fresh_same_key_changed_binding_rejects`.
- [ ] Implement retained request and phase transitions: accepted submission -> PendingCommit/yield; committed completion -> reconciliation/current adoption; ambiguous -> ReconciliationRequired using original replay/candidate/transport. Submission or receipt alone never installs a controller.
- [ ] Reject wrong/duplicate/out-of-order completion. Exact committed retry returns original identity and audit time; changed immutable binding conflicts without new authority. Only proven noncommit permits new candidate/transport after collision, never ambiguity.
- [ ] Add independently-current adoption inputs and compare session/lifecycle, account-character/world, lease, runtime/readiness, controller/generation and applicable security/trust against expected bindings. Fail closed and clear stale process projection.
- [ ] Add isolated tests `fresh_committed_stale_scope_cannot_install_controller`, `fresh_committed_stale_lease_cannot_install_controller`, `fresh_terminal_receipt_cannot_reactivate`, `fresh_reconnected_receipt_cannot_rollback_generation`, plus one-field account/world/security/trust/readiness mutations with positive controls.
- [ ] Explicitly mark synchronous fresh/journal methods non-production compatibility; keep the production port structurally distinct and preserve reconnect V1/V2.
- [ ] Run focused GREEN, inspect all transitions/compatibility paths and publish.

```bash
cargo +1.94.0 test --locked -p oteryn-game-server foundation::fresh_admission_durability_tests
cargo +1.94.0 test --locked -p oteryn-game-server foundation::admission
```

Build independent source fixtures before expected authorization/receipt; never derive live negatives from a record matching helper. Test source capabilities cannot satisfy production owning-source registration.

## Task 4 — Regressions and exact-head qualification

**Files:** same seven runtime paths plus allocated worker task when necessary.

- [ ] Sweep AuthorityInvariant x ConsumerBoundary x MutationOperator across authorization, publication activation, direct completion and reconciled adoption; use the task's concrete invariant/operator inventory.
- [ ] Preserve existing verifier/synchronous Foundation/reconnect V1/V2 assertions. Include restart without current floor, historical receipt after reconnect/terminal, runtime replacement during pending commit and same-key exact/conflicting replay.
- [ ] Run focused/component checks actually available on the exact worker head:

```bash
cargo +1.94.0 test --locked -p oteryn-game-server foundation::
cargo +1.94.0 test --locked -p oteryn-game-server --test authority_invariants
cargo +1.94.0 test --locked -p oteryn-game-server --doc
cargo +1.94.0 fmt --all --check
cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings
```

- [ ] Whole-diff self-review checks all public constructors, retained evidence, source freshness/order, no synchronous I/O path, no grant/receipt-derived authority and exact changed paths.
- [ ] Publish the exact worker head and qualify through canonical PR CI: repository-selected locked workspace build/Clippy/tests, applicable real PostgreSQL regression and governance gates. If local Rust is absent, use canonical PR CI and state which focused checks actually ran; no Remote Desktop or scope widening.
- [ ] Obtain genuinely independent exact-head authority/security review, disposition findings and rerun only invalidated layers after material repair.
- [ ] Return the qualified head to Work for protected integration/readback. B starts after A; C after A+B; Server Seam remains held until A+B+C integration and C producer readiness. No new physical E2E or production-ready claim from A.

Source readiness prerequisite is tracked separately in Issue #319 for Child C. It does not block Child A semantic implementation or widen this allocation.
