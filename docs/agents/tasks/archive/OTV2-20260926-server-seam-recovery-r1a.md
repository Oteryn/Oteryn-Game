# OTV2-20260926-server-seam-recovery-r1a

```yaml
task_id: OTV2-20260926-server-seam-recovery-r1a
title: Registered Recovery V2 credential verification from current S2 owners
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/server-seam-recovery-r1a
pr: 936
base_sha: 91fb3135a8a67d012bfe048b230f3501780d244b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: server-seam-control-plane/recovery-liveness
created_at: 2026-09-26T10:43:06Z
updated_at: 2026-09-26T10:43:06Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/recovery_evidence_composition.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/native_admission_source.rs
  - apps/game-server/tests/native_admission_source_postgres.rs
  - apps/game-server/tests/wp5_s3b_composition.rs
  - tools/qualification/wp5_s3b/run.sh
  - .github/workflows/wp5-s3b-composition.yml
  - docs/agents/tasks/active/OTV2-20260926-server-seam-recovery-r1a.md
public_contracts: []
depends_on: []
blocks: [822]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

Allocation: [#162 comment5845487696](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5845487696).
Owner authorized authoring, testing and required review; governed integration is
OWNER-PERFORMED. The worker does not enqueue, merge or trigger funded review.

PROVEN architecture authority: native-source evidence decision (2026-09-06),
Recovery transport amendment (2026-09-06), FND-04/FND-04B and the existing sealed
Foundation V2 verifier. Platform qualification source is pinned to
9147bfd3a771762a6646fd87b9172cdb3a6c9a01; fixture keys/accounts are ephemeral.

DERIVED implementation interpretation for independent review: one local scoped
Recovery publication per authenticated acknowledged S2 source ordinal. Numeric
projection preserves separate source/publication meanings. Equal replay keeps
identity, body, time and uncertainty; newer owner ordinal yields newer publication.
Recovery V2 does not require the independent +1 Fresh guard counter. No DDL.

The private non-Clone source is resolved and consumed before locks are released.
Node current/proof -> current installed source registration -> Account floor ->
set-wide Recovery trust floor matches S2 ordering. Installed descriptor/history
must match independently issued authorization. A later issuance alone does not
invent revocation of a still-installed descriptor. Floor/history/body/request
must agree; greater history, missing facts or purpose/key substitution fail closed.
Server time is sampled after asynchronous owner resolution, at synchronous verify.

`RecoveryCurrentEvidence` supplies inert expected credential bindings. Returned
verified facts are historical authentication evidence, never actor authority.
Every later authority consumer must resolve sufficient current owners at its own
atomic boundary. Own-database consistency cannot certify full snapshot rollback:
fresh owner/high-water reconciliation remains required by the S2 restore contract.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - current Node custody and installed authenticated source registration
  - shared Account purpose ordering and set-wide Recovery signing ordering
  - exact original response provenance and immutable history equality
  - current trusted server time and conservative accepted source deadline
consumer_boundaries: [registered_verify, registered_revalidate]
mutation_operators:
  applicable: [missing, stale, denial, binding_mismatch, purpose_substitution, key_replacement, original_time_mismatch, expired, future, uncertainty_overflow, replay, lock_wait, reload, custody_revocation]
  considered_not_applicable: [PREPARE, COMMIT, controller_adoption, control_loss, grace, protection, ClientResume, full_database_restore_certification]
one_invariant_per_negative_case: true
independent_current_fact_sources: [current_Node_registration, installed_S2_registration, acknowledged_Account_floor, acknowledged_Recovery_trust_set_floor, immutable_S2_history, strict_S1_body_decode, server_clock]
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: registered_verify_and_revalidate
  protocol_versions: fresh_account_supersedes_recovery_on_shared_floor
  direct_and_reconciled_paths: direct_verification_and_current_revalidation
  fenced_durable_writes: NOT_APPLICABLE_credential_reads_do_not_grant_authority
  restart_retry_replay_concurrency_pg_reload: configured_PostgreSQL_cases
  evidence: [PR_936_exact_head_qualification_packet]
finding_dispositions:
  p0_p1_accepted_and_repaired: []
  p0_p1_rejected_with_exact_evidence: [latest_issued_descriptor_does_not_revoke_installed_descriptor_contract]
  p2_fixed_accepted_or_deferred: [fixed_pre_clone_bounds_for_subject_and_current_bindings]
```

## Acceptance and validation

- Registered verify/revalidate use independently current owners under the same locks.
- Private source/context cannot escape; no public arbitrary callback/cache framework.
- Configured PostgreSQL proves denial, purpose/key replacement, body/history/time
  negatives, exact replay/reload, custody revocation and clock sampling after wait.
- Existing S3-B proves real Platform mTLS V2 -> acknowledged S2 -> sealed verifier
  using a separate Recovery key, preserved provenance and unchanged existing
  session/lease/controller state plus an occupied source-publication slot.
- Compiler, formatting, strict Clippy, affected/workspace tests, governance,
  architecture, aggregate game-gate and required exact-head independent review pass.

Focused RED: test-first head 1e5b527ceff9cc69113dc998e61921e1c8f2a825,
merge-gate run36236631296 / Linux job108389531680: all-targets compile exit101,
E0432/E0599 (registered Recovery module/API absent). This is compile-linkage evidence;
it does not prove SQL/currentness semantics. GREEN execution must prove those.
AUTHORING diagnostics are not final candidate qualification.

Local staging is isolated under `.codex/authoring/server-seam-recovery-r1a`.
Tracked workspace/checkout remains unchanged. Windows has the pinned compiler but
the unchanged Unix server cannot compile there; no Windows port or new harness.
Configured Linux CI supplies PostgreSQL17.6 and real Platform routes.

All known metadata is prepared before final freeze. Final returned remote SHA,
head equality, complete owned-path delta, diagnostics, self-review and candidate
qualification are recorded in an immutable PR comment/check evidence after the
last authoring commit, avoiding a self-referential SHA/status commit.

## Review and closeout

Self-review: implementing worker's whole-diff/finding-family sweep before freeze;
parallel read-only AUTHORING audit is advisory and does not replace worker review.
Independent review: YES, authority/provenance/high-risk; standing_required_review.
Unique #162 root control plane owns the funded exact-head trigger after live
same-head de-duplication and required deterministic qualification.
PR remains draft until coordinator confirms READY_FOR_INTEGRATION. No DONE claim
before owner-performed MQ, actual merge_group game-gate and protected readback.
#822 remains open: playable control-loss/liveness policy and reconnect adoption,
positive ClientResume qualification and measured finite timing are separate work.

```yaml
last_progress: bounded_implementation_and_registered_tests_prepared
status: implementing
branch: codex/server-seam-recovery-r1a
pr: 936
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner_action_required: owner_performed_MQ_after_coordinator_readiness
blocker: null
next_action: qualify_frozen_exact_candidate_and_return_review_packet
```
