# OTV2-20260930-house-own0

```yaml
task_id: OTV2-20260930-house-own0
title: "HOUSE-OWN-0 house ownership"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-house-own-0
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: 9acef5cc
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_OWN0_HOUSE_OWNERSHIP_DECISION_2026-09-30.md
  - docs/architecture/EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md
  - docs/architecture/reviews/OTERYN_GAME_HOUSE_CUSTODY0_HOUSE_ITEM_CUSTODY_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20260930-house-own0.md
public_contracts:
  - docs/architecture/EXP-HOUSES-01_OWNER_ACCEPTANCE_BASELINE.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

HOUSE-OWN-0 decides ownership of ordinary physical houses under the owner-accepted EXP-HOUSES-01
(owner direction, 2026-09-30).

- **Property, tiles and slot** tables; owner `HouseId -> CharacterId`; one slot per (Account, World).
- **Auction:** proxy bids, each escrowed (max + snapshotted rent), 7 days from the first bid, a
  15-minute extension, settlement over the effective valid-bid set, bounded release steps.
- **Rent** from the bank every 30 days, 7-day grace, eviction and a 30-day ban; move-out 1-30 days.
- **Disposition** under a database-enforced content fence with a fenced content set, items to each
  reclaim subject's Inbox in bounded steps.
- **Ground on house tiles** (HOUSE-CUSTODY-0 §3.5 gate), **ACL** lists with revisions, **wire**.
- **Owner questions** H1 (price and rent as D178 sinks) and H2 (houses before Premium exists; §10
  holds and bidding fails closed until answered).

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: EXP-HOUSES-01; the House catalogue contract; HOUSE-CUSTODY-0; the scope matrix; the
  gold fee decision (D178).
- `DERIVED`: BANK-0, MARKET-0 (candidates); the Tibia manual.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. HOUSE-1 needs persistence, economy and security review; HOUSE-WIRE-1
protocol review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence, economy, security, protocol).
- [ ] Protected Merge Queue integration, after #1357 and #1367.

## Excluded scope

- Code, migrations and content; interior runtime, beds, Residence, guildhalls, Bazaar, ACL
  patterns, direct transfer.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only): 11 material and 4 minor findings, all fixed:
  escrow on every bid, an escrow column with a guard and a rent snapshot, escrow returns past the
  bank limit, a bidder cap with paginated release, the clock after the lock, root locks before the
  property in settlement, Premium failing closed (now owner question H2), Inbox deliveries (MARKET-0
  §5 amended), the HOUSE-CUSTODY-0 gate, order and grants, the fence set on leaving OWNED with a
  fenced set and a trigger, the §24 evidence and §25 scenario map, doors by position, grace and
  move-out ordering, the tie time, and the declared differences. Its runtime-gating question
  follows HOUSE-CUSTODY-0 §4; its ban-scope and reserve-price questions are Global-parity rulings.
- Amendments follow the control-plane rule (#162 5912405163): pending on acceptance.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-house-own-0
owner_action_required: "H1 and H2 in the decision §14"
blocker: null
next_action: null
```
