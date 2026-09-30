# OTV2-20260930-premium-delivery0

```yaml
task_id: OTV2-20260930-premium-delivery0
title: "PREMIUM-DELIVERY-0 Premium evidence from Platform to Game"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-premium-delivery-0
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: ccf5f723
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_DELIVERY0_PREMIUM_EVIDENCE_TRANSPORT_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-premium-delivery0.md
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_DECISION_2026-09-28.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: OTV2-PREMIUM-DELIVERY
external_repositories: []
```

## Outcome

PREMIUM-DELIVERY-0 decides how Premium evidence reaches Game (owner direction, 2026-09-30:
"no to wydaj takie decyzje i przygotuj zeby to ruszylo").

- **Transport:** Game pulls an account's Premium snapshot from a private Platform endpoint over
  mutual TLS, at admission, reconnect and refresh, with a nonce and the consumer fence.
- **Message:** `oteryn.premium_snapshot.v1` with the consumer contract §5 fields.
- **Requested product policy** (Platform decides): 60-minute lease, refresh after 40 minutes,
  5-second skew, no stale use.
- **Children:** PREM-P in Oteryn-Platform and PREM-1 here, in parallel, meeting in one end-to-end
  test.

No code, migration or Platform change is made by this task; the Platform lane is started
separately under Platform's own governance.

## Architecture and source of truth

- `PROVEN`: PROD-ENTITLEMENTS-01 consumer contract (accepted); PREMIUM-ACTIVATION-V1 (candidate);
  Platform's entitlement delivery contract and project state (read-only, `c914564`).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. PREM-1 needs security review; the end-to-end activation needs its own
production authority.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (security, integration).
- [ ] Protected Merge Queue integration.
- [ ] The Platform side accepted by PREM-P in Oteryn-Platform.

## Excluded scope

- Payment, prices, the Store, VIP, push delivery; any Platform write.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Review of `bec97ab9` (#1369 5913683692: 2 HIGH, 3 MEDIUM, 3 LOW) and of PREMIUM-ACTIVATION-V1
  (#162 5913685128: 2 MEDIUM, 1 LOW), all answered in one push: the fence per (account,
  entitlement) with an account high water, the producer refresh point at the lease end (no stale
  contradiction), a strictly higher `authority_revision` on renewal, the `account_id` binding, a
  `producer_profile` field and compatibility records, the clock source, the restrictive-state rule,
  credential rotation; and the activation amendments (policy revision and product binding,
  degraded-behaviour owners, relocation only on `EXPIRED`/`REVOKED`/none, the §5 rows).
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-premium-delivery-0
owner_action_required: null
blocker: null
next_action: null
```
