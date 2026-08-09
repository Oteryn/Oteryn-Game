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
updated_at: 2026-08-09T12:21:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260809-fnd04a-authority-fresh-admission.md
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
public_contracts:
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
repair_cycles_for_current_gate: 0
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

- fresh-admission authority layers;
- Platform/game authority split;
- AccountPresenceClaim/CharacterLease admission boundary;
- strict fresh-entry signed profile;
- AdmissionAttemptRef vs GrantNonce;
- Platform-security and key/profile trust freshness;
- route/runtime/compatibility/current target applicability;
- current `CharacterId -> WorldId` / world-eligibility binding;
- atomic final admission revalidation/commit;
- fresh-admission duplicate-login/no-preemption rules;
- fresh-admission error subset with full Foundation Error Vocabulary fields;
- fresh-admission/world-transfer TOCTOU evidence requirements.

### Excluded

- reconnect secret and PREPARE/COMMIT;
- reauthenticated recovery;
- liveness/same-session grace/ControlLossEpoch;
- post-grace recovery;
- Channel/Instance continuity and GameNode replacement;
- complete FND-04 error catalogue/shared failure integration;
- final FND-04 index/programme status;
- runtime, protocol-codec, persistence schema, Platform implementation, KMS/HSM, deployment, production traffic.

Those are FND-04B/FND-04C under #112.

## Carried P1 acceptance

The final review of superseded #109 found that final fresh admission did not explicitly prove the current `CharacterId -> WorldId` / world-eligibility relation matched the signed `world_id`.

FND-04A closes that finding only if both public contracts require:

1. initial current character-world eligibility evaluation;
2. the same check again immediately before and atomically with authority creation;
3. mismatch or changed-world-before-commit -> `ADMISSION_GRANT_WORLD_STALE`;
4. no GrantNonce consumption or candidate presence/lease/session/transport authority mutation;
5. no silent grant retarget after legal world transfer;
6. independent initial-mismatch and change-before-commit/world-transfer fixtures.

## Error-vocabulary discipline

FND-04A does not defer the completeness of its own fresh-admission errors. Every FND-04A-owned public/cross-component error must have:

- stable code and Foundation category;
- `RETRYABLE` / `TERMINAL` / `SECURITY_TERMINAL`;
- exact retry/new-authority rule;
- redacted diagnostic message;
- credential-free correlation/trace fields;
- idempotency/partial-mutation outcome;
- bounded public class.

FND-04C will integrate accepted rows but must not silently alter FND-04A semantics.

## Decision timing

Authority split, atomic final admission, current character-world binding, strict profile and security/trust freshness are decide-now. Physical transaction technology, tables, caches, KMS vendor, production lease timing and capacities remain deferred to evidence-owning downstream gates.

## Validation plan

Before readiness:

- inspect full three-path diff against trusted main;
- confirm no reconnect/recovery semantic duplication;
- verify both public docs have identical current-world and final-linearization semantics;
- verify every FND-04A error row satisfies Foundation Error Vocabulary;
- verify `ADMISSION_GRANT_WORLD_STALE` public mapping does not expose private transfer details;
- run exact-head Agent governance, Dependency review and CodeQL;
- full exact-head architecture/security self-review;
- freeze exact head without a later checkpoint-only commit;
- one independent exact-head review;
- zero material findings and unresolved threads;
- no more than three repair cycles;
- squash merge only on unchanged accepted head.

Runtime/component/browser E2E: `NOT_APPLICABLE` because this delivery is architecture/contracts only. Future implementation must execute the specified fault/interoperability fixtures.

## Current checkpoint

```yaml
status: validating
last_progress: Superseded monolithic #109 after owner-approved final review found P1 current-character-world omission and P2 incomplete error diagnostics. Replacement programme #112 and bounded gate #113 created. PR #114 now carries one bounded three-path FND-04A package reconstructed from trusted main with explicit CharacterId->WorldId/world-eligibility final revalidation, ADMISSION_GRANT_WORLD_STALE, world-transfer race fixtures and complete fresh-admission diagnostic/correlation fields.
repair_cycles_for_current_gate: 0
next_action: validate full PR #114 three-path diff against trusted main and FND-04A acceptance; repair only material findings, otherwise freeze exact head and run exact-head CI/self-audit/independent review.
```
