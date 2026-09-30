# OTV2-20260930-prem-request0

```yaml
task_id: OTV2-20260930-prem-request0
title: "PREMIUM-DELIVERY-0 §3.1 request format (PREM-1b escalation)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-prem-request-0
pr: 1397
base_sha: 840092f3
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_DELIVERY0_PREMIUM_EVIDENCE_TRANSPORT_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-prem-request0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: OTV2-PREMIUM-DELIVERY
external_repositories: []
```

## Outcome

Answers PREM-1b's `ARCHITECTURE_ESCALATION_REQUIRED` (#162 5916078350) with PREMIUM-DELIVERY-0
§3.1:

- `POST /v1/premium/snapshot` over mutual TLS, JSON body with a request schema, canonical lowercase
  hyphenated `account_id` and a 32-hex `nonce`, at most 256 bytes;
- 200 with the §4 body only; an account without an entitlement is `NONE`, never a separate answer;
- anything else (status, redirect, content type, 5-second timeout, TLS failure) is unavailable: no
  new evidence, the fence keeps the last, Premium is denied at once as `AUTHORITY_UNAVAILABLE`
  until a pull succeeds, retry with backoff, login unaffected;
- an authenticated same-revision contradiction or unsupported/downgraded semantics is
  `INVALID_OR_CONFLICTING` with a durable conflict marker; V1 declares its reconciliation deferred;
- the in-process test producer serves exactly this exchange.

No code, migration or content change is made. Platform accepts or amends its side in PREM-P.

## Architecture and source of truth

- `PROVEN`: PREMIUM-DELIVERY-0 §3-§6; the consumer contract §4-§8.
- `DERIVED`: PREM-1a's recommendation in the escalation.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. PREM-1b needs security review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (security, cross-repository integration).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code; the Platform endpoint implementation (PREM-P).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.
- Codex round 4 (#1397, 1 P1): a failed admission, reconnect or refresh pull now classifies the
  account `AUTHORITY_UNAVAILABLE` at once until a pull succeeds, keeping the fenced evidence; the
  lease cutoff stays the separate `EXPIRED` transition (§3, §3.1, §5; consumer contract §8.3).
- Codex round 5 (final batched round; #1397, 1 P1, 2 P2):
  - 4149454824 (P1) fixed: an authenticated same-revision contradiction or unsupported/downgraded
    `schema`, `producer_profile`, `product_id` or `product_version` is `INVALID_OR_CONFLICTING`
    (consumer contract §8.1 rules 2 and 4), recorded as a durable conflict marker with bounded
    revision fingerprints (`PREMDEL0-RL-04`, 30 days); only transport, malformed and stale
    responses are `AUTHORITY_UNAVAILABLE`; reconciliation is a declared V1 deferral (§3.1, §4, §6).
  - 4149454831 (P2) fixed: a failed pull or semantic failure leaves `premium_entitlement_ended`
    unchanged (§3.1, §6).
  - 4149454835 (P2) fixed: `pr: 1397`.

This record was archived in the final authoring commit of its PR.
