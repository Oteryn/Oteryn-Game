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
updated_at: 2026-08-09T12:34:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260809-fnd04a-authority-fresh-admission.md
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
public_contracts:
  - docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md
  - docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md
repair_cycles_for_current_gate: 2
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
- superseded #109 `bf82e392...` as historical evidence only.

## Scope

Included: fresh authority layers, Platform/game boundary, presence/lease admission semantics, strict fresh grant, AdmissionAttemptRef vs GrantNonce, security/trust freshness, route/runtime and independent authoritative revisions, ownership-safe CharacterId->WorldId binding, atomic admission, duplicate-login no-preemption, complete A-error vocabulary, fresh-admission race evidence.

Excluded: reconnect/recovery/PREPARE-COMMIT, liveness/grace/ControlLossEpoch, post-grace recovery, handoff/GameNode continuity, complete final FND-04 integration, runtime/protocol/persistence/Platform/key/deployment/production implementation.

## Carried #109 P1 acceptance

Both public contracts must prove AccountId->CharacterId ownership **before** world classification, then prove CharacterId->WorldId/world eligibility, and repeat that ordering at final atomic admission. Valid ownership + stale world -> `ADMISSION_GRANT_WORLD_STALE`; invalid ownership -> account/character conflict without world oracle. No nonce/authority mutation and no grant retarget.

## Repair history

### Cycle 1 — self-review before freeze

1. moved AccountId->CharacterId ownership before world-state classification, including final revalidation;
2. removed reconnect-proof initialization from FND-04A scope;
3. removed raw scope-ownership generation from diagnostics, using safe match/stale classes.

### Cycle 2 — automated review of pre-cycle-1 generation

Automated review produced two P1 and one P2; all were inspected against current accepted baselines and repaired coherently:

1. **P1 independent authoritative revisions** — one opaque `compatibility_revision` conflicted with accepted requirement to keep protocol/content/ruleset/policy concepts separate and with FND-04 analysis requiring ruleset/content/map/world-policy/offer revisions. v1 now has separate mandatory `ruleset_revision`, `content_revision`, `map_revision`, `world_policy_revision`, `offer_revision`; opaque `compatibility_revision` is removed. Each dimension is independently revalidated and independently fault-tested.
2. **P1 revocation freshness semantics** — prior fixture implied instantaneous detection even though trust evidence age <=5s is accepted. Contracts now explicitly define bounded-staleness semantics: revocation already present in final accepted evidence fails authentication; a revocation after that evidence observation point may remain unseen only until newer evidence records it or the previous evidence exceeds 5s. This is an explicit maximum residual detection window, not an atomic global revocation fence.
3. **P2 wrong-bound credential** — added `ADMISSION_GRANT_BINDING_MISMATCH` for correctly signed but wrong `iss`/`aud`/`typ`/`purpose`, category `SESSION_REJECTED`, security-terminal, no mutation, bounded `RETRY_LOGIN`, redacted diagnostic and credential-free mismatch-class correlation. Unsupported profile remains revision failure; malformed structure remains malformed.

`repair_cycles_for_current_gate: 2`. One repair cycle remains; no task-local exception is allowed.

## Error-vocabulary discipline

Every FND-04A cross-component error defines stable code/category, disposition, exact retry authority, redacted diagnostic, credential-free correlation, mutation/idempotency and public class. Diagnostics expose no credentials, Platform security-generation values or private fencing generations.

## Validation plan

- full three-path diff/scope review against trusted main;
- verify no reconnect/recovery semantics;
- verify ownership-before-world ordering in both public contracts;
- verify separate revision claims and no `compatibility_revision` overload;
- verify bounded <=5s revocation model is internally consistent and does not claim instant detection;
- verify wrong-bound credential mapping and every A-error against Foundation Error Vocabulary;
- exact-head Agent Governance, Dependency review, CodeQL;
- exact-head full architecture/security self-review;
- freeze only after zero local material findings;
- one terminal independent exact-head review;
- zero material findings/unresolved threads;
- max 3 repair cycles;
- squash merge on unchanged accepted head only.

Runtime/browser E2E: `NOT_APPLICABLE` for docs-only architecture. Future implementation executes named fixtures.

## Current checkpoint

```yaml
status: validating
last_progress: Repair cycle 2 completed. FND-04A now binds protocol/transport plus ruleset/content/map/world-policy/offer revisions separately; removes opaque compatibility_revision; defines the accepted <=5s residual revocation-detection window without claiming instantaneous revocation; and adds a full ADMISSION_GRANT_BINDING_MISMATCH progression for wrong-bound signed credentials. Ownership-before-world and no-reconnect-scope fixes from cycle 1 remain intact.
repair_cycles_for_current_gate: 2
next_action: perform one fresh full-diff architecture/security self-review of the current three-path head. If zero material findings, finalize PR metadata, freeze exact head and run exact-head CI before the single terminal independent review. Any new material finding consumes the final repair cycle.
```
