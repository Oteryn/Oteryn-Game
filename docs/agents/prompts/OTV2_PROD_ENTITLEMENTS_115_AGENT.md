# Oteryn-v2 PROD-ENTITLEMENTS-01 Issue #115 Architecture Agent

Alias: `OTV2-ENTITLEMENTS-115`

## 1. Role and mode

```text
ROLE: SECURITY-SENSITIVE DOMAIN ARCHITECTURE DESIGN AGENT
MODE: CONTRACT / ANALYSIS / EVIDENCE
ISSUE: #115
MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY
IMPLEMENTATION_AUTHORITY: NONE
```

Your task is to advance the still-open Oteryn-v2 consumer/enforcement side of `PROD-ENTITLEMENTS-01` after the Platform producer prerequisite has been satisfied. This is architecture/security work only.

## 2. Repository authority

Writable repository: `blakinio/Oteryn-v2` only and only within a bounded issue #115 task/branch/draft PR with explicit non-overlapping owned paths.

`blakinio/Oteryn-Platform` is read-only. All other repositories are read-only unless the owner separately authorizes exact writes.

No runtime/client/server/protocol implementation, entitlement activation, payment/order mutation, PostgreSQL DDL/migrations, production/protected-environment changes, secrets, live account/session/data mutation or deployment is authorized.

No Codex/OpenAI/owner-funded AI review without explicit authorization for the exact PR/use.

## 3. Mandatory startup and truth reconstruction

Do not trust old issue prose or agent summaries without verification.

Read:

1. root `AGENTS.md`, `AGENTS.override.md`;
2. `docs/agents/AGENTS.md`;
3. `docs/agents/MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md`;
4. `docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`;
5. `docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md`;
6. `docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md`;
7. `docs/agents/PROMPTING_STANDARD.md` and `PROMPT_EVAL_STANDARD.md`;
8. live main/open PRs/active tasks/reviews/CI and path ownership;
9. issue #115 and all comments;
10. `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md`;
11. existing `PROD-ENTITLEMENTS-01` dependency/authority documents and relevant FND-04/Platform boundary contracts;
12. exact producer remediation evidence in `blakinio/Oteryn-Platform` at the pinned revision already consumed by Oteryn-v2.

Known baseline to verify:

- Platform remediation prerequisite `OPA-SEC-0007` is satisfied and an exact Platform merge revision is pinned in Oteryn-v2;
- issue #115 intentionally remains open;
- Oteryn-v2 consumer/enforcement contract is not accepted;
- Premium/VIP and other game-consumed entitlement activation is `NOT_AUTHORIZED`;
- this gate is independent of current GAME-ABILITY owner-decision priority and must not absorb it.

Use `PROVEN`, `DERIVED`, `UNKNOWN`, `CONFLICT`, `RECOMMENDATION` truth labels.

## 4. Exact outcome

Produce one bounded draft architecture package for the Oteryn-v2 consumer/enforcement contract, without duplicating Platform commercial authority.

The package must freeze or explicitly leave unresolved, with rationale, at least:

- Platform producer-owned entitlement lifecycle/revision authority;
- Oteryn-v2 game-consumer/enforcement authority boundary;
- finite authority validity (`valid_until`, lease expiry, finite `max_stale`, refresh deadline or equivalently strong mechanism);
- producer/source authentication and revision ordering;
- anti-rollback/out-of-order fencing;
- typed/equivalent `CURRENT`, `STALE_WITHIN_BOUND`, `AUTHORITY_UNAVAILABLE`, `EXPIRED`, `REVOKED` semantics;
- clock/skew or producer-issued validity semantics that cannot be extended by client/GameNode local clocks;
- new admission, reconnect and already-running-session behavior boundaries;
- explicit offline/degraded policy with no implicit infinite grace;
- idempotent game-affecting grant/delivery semantics consistent with Platform/game authority split;
- producer/consumer contract revision pinning and compatibility rules;
- rollout, rollback and fail-closed behavior;
- observability/audit evidence required to diagnose stale/revoked authority without leaking sensitive data.

Apply the architecture decision timing test to every material decision. Do not freeze unrelated commerce features merely for completeness.

## 5. Security acceptance scenarios

Define deterministic negative-path/security acceptance for at least:

- Platform outage before authority expiry;
- outage after authority expiry;
- revocation during partition;
- reconnect/restart using cached evidence;
- delayed/out-of-order producer revisions;
- projection rollback;
- clock skew/clock rollback;
- duplicate/replayed grant delivery;
- downgrade to an older producer/consumer contract revision;
- inability to refresh authority;
- activation/rollback across mixed compatible revisions.

Do not claim these scenarios PASS unless an implementation is later separately authorized and executed. At architecture stage define the required evidence and expected fail-closed outcomes.

## 6. Ownership and cross-domain discipline

Prefer a new bounded `PROD-ENTITLEMENTS-01` consumer/enforcement candidate-contract artifact plus the lane's own task record. Do not edit coordinator-only programme overlays unless the coordinator explicitly delegates that exact file.

Do not rewrite Platform contract text as competing authority. Pin and consume the exact producer contract/revision.

If FND-04, persistence, gameplay, analytics or Platform boundaries expose a missing decision owned elsewhere, record a typed `CROSS_DOMAIN_FINDING` with `worker_action: REPORT_ONLY` rather than editing foreign ownership.

## 7. Independent review requirement

Treat this lane as security/authorization-sensitive. Before any candidate is promoted to canonical acceptance, require a genuinely independent exact-head review by a qualified non-authoring reviewer under trusted-base policy.

A worker self-review is mandatory but is not independent. A coordinator who materially co-authors the proposal cannot relabel its own final review as independent.

Codex is optional and owner-funded; do not invoke it without exact owner authorization. If no independent mechanism is available when required, stop `BLOCKED` rather than weakening the gate.

## 8. Validation and draft-PR handoff

Before integration-ready handoff:

- full changed-file/diff inspection;
- verify exact owned paths and no coordinator/sibling overlap;
- applicable governance/link/schema checks;
- exact-head full-diff self-review;
- explicit security threat/failure-path review;
- ordinary exact-head repository CI;
- zero unresolved review threads;
- live-main drift check;
- draft PR remains draft with `MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`;
- runtime/component/E2E = `NOT_APPLICABLE` because no executable entitlement path is authorized.

Do not mark the whole gate `ACCEPTED` merely because the candidate PR merges. Owner/coordinator acceptance is separate.

## 9. Stop conditions and completion

Stop with an exact blocker if:

- pinned producer evidence cannot be verified;
- accepted cross-repository contracts conflict materially;
- owned paths overlap another task;
- the design would require Platform mutation or runtime implementation to resolve;
- a required independent reviewer is unavailable at acceptance time.

Successful worker completion is:

```text
INTEGRATION_READY — DRAFT PR — COORDINATOR/OWNER ACTION REQUIRED
```

Durable checkpoint must record branch, PR, exact head, producer revision consumed, owned paths, findings, validation/review state and exactly one `next_action`.
