# OTV2-20260809-fnd04a-authority-fresh-admission

```yaml
task_id: OTV2-20260809-fnd04a-authority-fresh-admission
title: FND-04A authority and fresh-admission bounded contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd04a-authority-fresh-admission
issue: 113
programme_issue: 112
pr: 114
supersedes_evidence_pr: 109
base_sha: 27f7f647f04e3b1a4151f9b124401986910f03d8
historical_candidate_sha: bf82e392d6ef8b1e627849cdc7383af9a7c987ae
owner: GPT-5.6 Sol architecture continuation session
created_at: 2026-08-09T12:16:00+02:00
updated_at: 2026-08-09T12:39:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260809-fnd04a-authority-fresh-admission.md
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
public_contracts:
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
repair_cycles_for_current_gate: 3
max_repair_cycles_for_current_gate: 3
final_head_sha: null
final_head_frozen_at: null
owner_action_required: null
blocker: null
```

## Goal

Deliver only bounded FND-04A authority + fresh admission from replacement programme #112. Reconstruct useful reviewed semantics from superseded #109 on trusted main without importing its reconnect/recovery/integration monolith. No runtime implementation is authorized.

## Trusted inputs

- `main@27f7f647f04e3b1a4151f9b124401986910f03d8`;
- accepted FND-04 analysis/reconciliation baselines;
- ADR-0003/0012; FND-ID-01; FND-02; accepted FND-03;
- `FOUNDATION_ERROR_VOCABULARY.md`;
- replacement programme #112; gate #113; delivery PR #114;
- superseded #109 `bf82e392...` as historical evidence only;
- pinned Platform native pre-admission/runtime-status contracts as read-only reconciliation evidence.

## Scope

Included: fresh authority layers, Platform/game boundary, presence/lease admission semantics, strict fresh grant, AdmissionAttemptRef vs GrantNonce, security/trust freshness/provenance/anti-rollback, route/runtime and independent authoritative revisions, ownership-safe CharacterId->WorldId binding, atomic admission, duplicate-login no-preemption, complete A-error vocabulary and fresh-admission race evidence.

Excluded: reconnect/recovery/PREPARE-COMMIT, liveness/grace/ControlLossEpoch, post-grace recovery, handoff/GameNode continuity, complete final FND-04 integration, runtime/protocol/persistence/Platform/key/deployment/production implementation.

## Carried #109 P1 acceptance

Both public contracts prove AccountId->CharacterId ownership before world classification, then CharacterId->WorldId/world eligibility, and repeat that ordering at final atomic admission. Valid ownership + stale world -> `ADMISSION_GRANT_WORLD_STALE`; invalid ownership -> account/character conflict without a world oracle. No candidate nonce/authority mutation and no grant retarget.

## Repair history

### Cycle 1 — self-review

1. ownership-before-world ordering, including final revalidation;
2. removed reconnect-proof initialization from FND-04A scope;
3. removed raw scope-ownership generation from diagnostics.

### Cycle 2 — automated pre-freeze review

1. replaced opaque `compatibility_revision` with independent mandatory `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision` alongside protocol/transport; FND-02 diagnostic `schema_revision` is deliberately not an exact admission gate;
2. replaced impossible instantaneous-revocation fixture with explicit <=5s bounded residual detection semantics;
3. added full `ADMISSION_GRANT_BINDING_MISMATCH` progression for correctly signed but wrong `iss`/`aud`/`typ`/`purpose`.

### Cycle 3 — deep security self-review

Freshness alone was insufficiently specified: a cache could theoretically re-age old evidence, and an older allow/trust snapshot still younger than 5s could arrive after a newer deny/revoke.

Both Platform-security and signing-key/profile trust evidence now require authenticated semantics sufficient to prove:

- source authority/purpose/scope;
- authenticated `source_observed_at` or equivalent source-time provenance;
- monotonic/comparable `source_revision` or equivalent non-rollback decision fence;
- current decision facts.

Accepted age is a conservative upper bound from authenticated source observation to trusted game-server time, including known clock uncertainty; cache receive/refresh/store/re-read time never resets age. If provenance or upper-bound age <=5s cannot be proved, fail `ADMISSION_GRANT_SECURITY_EVIDENCE_STALE`.

For each comparable security/trust scope, evidence older than the highest accepted source revision/fence cannot authorize even if source age remains <=5s; equal revision with contradictory authenticated content is invalid. Newer Platform deny/generation floor and newer key/profile revoke fence older allows/trust. Restart/recovery must reconstruct a current non-rollback floor from authoritative evidence or preserved trusted state before fresh admission; inability fails closed.

Residual unseen revocation remains bounded by the <=5s **source-age** ceiling and ends earlier when a newer restrictive revision is accepted. Cache refresh cannot extend it.

This is repair cycle `3/3`. No material repair budget remains for this gate. A new material finding now requires `BLOCKED`/owner-governance action; no task-local exception is authorized.

## Error-vocabulary discipline

Every FND-04A cross-component error defines stable code/category, disposition, exact retry authority, redacted diagnostic, credential-free correlation, mutation/idempotency and public class. Diagnostics expose no credential, Platform security-generation value or private fencing generation.

## Validation plan

- verify exact three-path scope against trusted main;
- verify no reconnect/recovery semantics;
- verify ownership-before-world ordering in both contracts;
- verify separate ruleset/content/map/world-policy/offer claims and FND-02 schema_revision remains diagnostic metadata;
- verify wrong-bound credential classification;
- verify source-age cannot be reset by cache and evidence ordering cannot roll back newer restrictive decisions;
- verify both security evidence scopes have restart/recovery fail-closed behavior;
- verify every A-error against Foundation Error Vocabulary;
- run full current-head architecture/security self-review;
- if and only if zero material findings, treat that exact head as frozen in immutable PR review evidence (do not create a self-referential task commit);
- run exact-head Agent Governance, Dependency review and CodeQL;
- one terminal independent exact-head review;
- zero material findings/unresolved threads;
- squash merge on unchanged accepted head only.

Runtime/browser E2E: `NOT_APPLICABLE` for docs-only architecture. Future implementation executes named fixtures.

## Current checkpoint

```yaml
status: validating
last_progress: Final allowed repair cycle 3 completed. FND-04A now requires authenticated source observation provenance and monotonic/non-rollback source ordering for both Platform-security and admission key/profile trust evidence; source-age <=5s cannot be reset by cache, older allow/trust cannot roll back newer deny/revoke, and restart without a provable current evidence floor fails closed. Separate gameplay revisions, ownership-before-world, wrong-bound credential handling and explicit residual revocation semantics remain intact.
repair_cycles_for_current_gate: 3
next_action: perform the final full current-head three-path architecture/security self-review. If any material finding exists, stop BLOCKED. If zero, record freeze/self-review on the immutable PR surface, run exact-head CI, resolve only demonstrably repaired outdated review threads, then invoke one terminal exact-head independent review.
```
