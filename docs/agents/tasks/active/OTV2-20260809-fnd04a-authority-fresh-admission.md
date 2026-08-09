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
updated_at: 2026-08-09T12:27:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260809-fnd04a-authority-fresh-admission.md
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
public_contracts:
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
repair_cycles_for_current_gate: 1
max_repair_cycles_for_current_gate: 3
final_head_sha: null
final_head_frozen_at: null
owner_action_required: null
blocker: null
```

## Goal

Deliver only the bounded FND-04A authority + fresh-admission contract from replacement programme #112. Preserve accepted analysis and reviewed useful semantics from superseded PR #109 without importing its monolithic reconnect/recovery/error-integration surface.

FND-04A is architecture/documentation only. It does not authorize runtime implementation.

## Trusted inputs

- `main@27f7f647f04e3b1a4151f9b124401986910f03d8`;
- accepted FND-04 analysis/reconciliation baselines on main;
- ADR-0003 and ADR-0012;
- FND-ID-01, FND-02, accepted FND-03;
- `docs/contracts/FOUNDATION_ERROR_VOCABULARY.md`;
- replacement programme Issue #112;
- owning Issue #113;
- delivery PR #114;
- superseded PR #109 exact head `bf82e392...` as historical reviewed evidence only.

Do not treat any unmerged #109 file as canonical merely because it was reviewed.

## Bounded scope

### Included

- fresh-admission authority layers and Platform/game authority split;
- AccountPresenceClaim/CharacterLease admission boundary;
- strict fresh-entry signed profile;
- AdmissionAttemptRef vs GrantNonce;
- Platform-security and key/profile trust freshness;
- route/runtime/compatibility/current target applicability;
- current ownership-safe `CharacterId -> WorldId` / world eligibility;
- atomic final admission revalidation/commit;
- fresh-admission duplicate-login/no-preemption rules;
- fresh-admission error subset with complete Foundation Error Vocabulary fields;
- fresh-admission/world-transfer TOCTOU evidence.

### Excluded

- reconnect secret/proof and PREPARE/COMMIT;
- reauthenticated recovery;
- liveness/same-session grace/ControlLossEpoch;
- post-grace recovery;
- Channel/Instance continuity and GameNode replacement;
- complete FND-04 shared error/failure integration and final FND-04 index/status;
- runtime, protocol-codec, persistence schema, Platform implementation, KMS/HSM, deployment, production traffic.

Those are FND-04B/FND-04C under #112.

## Carried P1 acceptance

The final review of superseded #109 found missing current `CharacterId -> WorldId` / world-eligibility validation.

FND-04A closes it only if both public contracts require:

1. prove current `AccountId -> CharacterId` ownership/lifecycle before any world-state classification;
2. then evaluate current CharacterId->WorldId/world eligibility;
3. repeat ownership first and world applicability second immediately before/atomically with authority creation;
4. valid ownership + world mismatch/change-before-commit -> `ADMISSION_GRANT_WORLD_STALE`;
5. invalid ownership -> account/character conflict without world-state oracle;
6. no GrantNonce or candidate presence/lease/session/transport mutation;
7. no silent retarget after legal world transfer;
8. independent initial-mismatch, invalid-ownership and transfer-before-commit fixtures.

## Error-vocabulary discipline

Every FND-04A-owned cross-component error defines stable code/category, disposition, retry authority, redacted diagnostic, credential-free correlation fields, mutation/idempotency outcome and bounded public class. Diagnostics never expose credentials, Platform security-generation values or private fencing generations; match/stale classes are used where needed.

FND-04C may integrate accepted rows but must not silently alter them.

## Repair cycle 1 — pre-freeze self-review

Self-review found three material scope/security inconsistencies and repaired them before final-head freeze:

1. world-state evaluation originally preceded AccountId->CharacterId ownership, risking a world-state classification oracle for a producer-invalid/non-owned CharacterId; both contracts now require ownership first, then world relation, including at final atomic revalidation;
2. FND-04A atomic effects mentioned reconnect-proof initialization despite reconnect being explicitly out of scope; all reconnect secret/proof semantics were removed from A and left to FND-04B;
3. diagnostics correlation exposed raw `scope_ownership_generation` despite forbidding private fencing data; it now records only safe match/stale relation classes plus non-secret revision context.

These changes are one coherent repair hypothesis and count as `repair_cycles_for_current_gate: 1`.

## Validation plan

Before readiness:

- inspect full three-path diff against trusted main;
- confirm no reconnect/recovery semantic duplication;
- verify both public docs have identical ownership-before-world and final-linearization semantics;
- verify every FND-04A error row satisfies Foundation Error Vocabulary;
- verify `ADMISSION_GRANT_WORLD_STALE` public mapping leaks no transfer detail;
- run exact-head Agent governance, Dependency review and CodeQL;
- full exact-head architecture/security self-review;
- freeze exact head without a later checkpoint-only commit;
- one independent exact-head review;
- zero material findings/unresolved threads;
- maximum three repair cycles;
- squash merge only on unchanged accepted head.

Runtime/component/browser E2E: `NOT_APPLICABLE` because this delivery is architecture/contracts only. Future implementation must execute the specified fixtures.

## Current checkpoint

```yaml
status: validating
last_progress: Repair cycle 1 completed before freeze. Both public contracts now prove AccountId->CharacterId ownership before current world classification, repeat that ordering at atomic final admission, keep ADMISSION_GRANT_WORLD_STALE for an owned character whose signed world is stale, exclude reconnect-proof semantics from FND-04A, and remove raw fencing generation from diagnostics. PR #114 remains exactly three bounded documentation paths.
repair_cycles_for_current_gate: 1
next_action: perform a fresh full three-path architecture/security self-review of the repaired head; if zero material findings, freeze that exact head and run exact-head CI before the single independent terminal review.
```
