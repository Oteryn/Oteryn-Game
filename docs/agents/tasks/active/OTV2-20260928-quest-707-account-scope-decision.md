# OTV2-20260928-quest-707-account-scope-decision

```yaml
task_id: OTV2-20260928-quest-707-account-scope-decision
title: "#707 disposition and account-scoped progress decision (D44-D49)"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 707
pr: 1102
base_sha: 800e3eb6ad410442c5ec9f7c9fd2c361201d10cf
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md
  - docs/architecture/MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260928-quest-707-account-scope-decision.md
  - docs/agents/tasks/active/OTV2-20260928-spell-decisions-record.md   # archive move after #1089
  - docs/agents/tasks/archive/OTV2-20260928-spell-decisions-record.md
public_contracts:
  - MULTICHANNEL_SYSTEM_SCOPE_MATRIX
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records owner decisions D44-D49 (posted on #707 as 5867311210 and on #162 as 5867312726):

- the #707 disposition: principles accepted, the graph and shared progress superseded, the rest deferred;
- account quest completion facts;
- account cosmetics, Store purchases and achievements;
- portability rules.

The scope matrix gains four account-scope rows. The quest progress row stays Character.

## Architecture and source of truth

- `PROVEN`: scope matrix quest row; D34, D35 and D42; account guard keyed by `account_id`
  (`0002_fresh_admission_authority.sql:70`); ADR-0010 §6; product profile scope baseline §3.2;
  DUR-03 §5.2; `PROD-ENTITLEMENTS-01`; Store catalog owner decision §1 and §3; TibiaWiki "Achievements" (secondary evidence).
- `DERIVED`: append-only account facts need no cross-character ordering.
- `UNKNOWN`: gameplay effects of Reference-target cosmetics; the Achievement domain owner.

## High-risk authority/recovery qualification

Applicable at design level. §4.6 of the decision defines which fence authorizes an account-fact
insert. The negative cases below bind the first implementing allocation.

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - K1 an account fact is inserted only inside the DUR transaction of a fenced character event (full session-generation fence)
  - K2 an account fact is write-once; a duplicate insert changes nothing and keeps the first earner
  - K3 a condition accepts character completion or the account fact of the world's profile family, never another family
  - K4 per-character requirements (level, vocation, premium, items) are still checked for the acting character
  - K5 an exclusive-choice quest cannot grant account completion
  - K6 a reader accepts an account fact only while the world's active ruleset enables the policy and the quest's active revision declares grant
consumer_boundaries:
  - quest completion commit
  - achievement grant
  - cosmetic unlock
  - condition evaluation (doors, teleports, travel, NPC services, prerequisites)
mutation_operators:
  applicable:
    - stale generation (stale character fence)
    - mismatched identity or binding (other account, other profile family)
    - revoked or changed policy (quest switched to none, policy disabled on the world)
    - replay and concurrency (duplicate insert, two characters of one account)
  considered_not_applicable:
    - "time: facts carry no time-based authority"
one_invariant_per_negative_case: true
negative_cases_required_of_implementation:
  - K1 stale character fence -> no account fact written
  - K2 duplicate insert -> one row, first earner kept
  - K3 fact of another profile family -> condition not satisfied
  - K4 account completion with the acting character below the level requirement -> condition not satisfied
  - K5 exclusive-choice quest declaring account completion -> content compiler rejects
  - K6 existing fact after the quest switches to none, or on a world with the policy disabled -> condition not satisfied; the fact row is unchanged
positive_cases_required_of_implementation:
  - a character completes a quest; a second character of the account passes its door
independent_current_fact_sources:
  - character session fence rows
  - account fact table
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "P03 and #1033 fences are the reference"
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: "insert-if-absent inside the triggering transaction only"
  fenced_durable_writes: "account fact rows only"
  restart_retry_replay_concurrency_pg_reload: "covered by K1 and K2"
  evidence:
    - apps/game-server/migrations/0002_fresh_admission_authority.sql
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - "Codex P1 4121971783 (4b67b20): §4.5 and the scope matrix made the Store inbox Game-owned, contradicting the Store catalog owner decision §1/§3. Repaired: delivery ownership stays open under gap register §32; D47/D49 fix scope and portability only; the item still enters the world only through a DUR-03 MINT"
  p0_p1_rejected_with_exact_evidence: []
  p2_fixed_accepted_or_deferred:
    - "Codex P2 4121915460 (d7f8834): an opt-out did not stop old facts from satisfying conditions. Fixed: readers apply the current world and quest policy (§4.2); K6 added."
```

## Acceptance criteria

- [ ] The decision document and scope matrix rows are on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- The quest authoring format, the Achievement contract, the appearance owner, the Store inbox
  implementation, physical schema, protocol and client.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.
- The #1089 task record is archived with terminal integration evidence (`800e3eb`).

## Context checkpoint

```yaml
last_progress: authored; PR #1102 open
status: validating
branch: claude/gifted-rubin-a0axzx
pr: 1102
owner_action_required: null
blocker: null
next_action: exact-head review and Merge Queue integration of #1102
```
