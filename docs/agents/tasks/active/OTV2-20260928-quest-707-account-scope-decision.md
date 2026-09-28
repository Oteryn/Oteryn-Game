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
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md   # §32 scope note only
  - docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md   # §3 scope note only
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
  - K1 an account fact earned by gameplay is inserted only inside the DUR transaction of a fenced character event (full session-generation fence), or for an achievement derived only from a durable grant request committed in one
  - K2 an account fact is write-once; a duplicate insert changes nothing and keeps the first committed insert's provenance
  - K3 a condition accepts character completion or the account fact of the world's profile family, never another family
  - K4 per-character requirements (level, vocation, premium, items) are still checked for the acting character
  - K5 an exclusive-choice quest cannot grant account completion
  - K6 a reader accepts an account fact only while the world's active ruleset enables the policy and the applicable quest revision (the acting character's pinned revision while the quest is active, else the active revision) declares grant
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
  - K1 stale character fence -> no account fact and no achievement grant request written
  - K2 duplicate insert -> one row, first committed provenance kept
  - K3 fact of another profile family -> condition not satisfied
  - K4 account completion with the acting character below the level requirement -> condition not satisfied
  - K5 exclusive-choice quest declaring account completion -> content compiler rejects
  - K6 existing fact after the quest switches to none, or on a world with the policy disabled -> condition not satisfied; the fact row is unchanged
  - K6 character with the quest active under a pinned grant revision after the active revision switches to none -> still evaluated against the pinned revision until an explicit migration
positive_cases_required_of_implementation:
  - a character completes a quest; a second character of the account passes its door
  - a character earns an achievement; its committed grant request is later consumed once into AccountAchievement, including after the session ended
independent_current_fact_sources:
  - character session fence rows
  - account fact table
  - durable achievement grant request rows
record_derived_matching_helper:
  allowed_for_positive_happy_path: true
  forbidden_for_negative_authority_or_provenance_cases: true
finding_family_sweep:
  sibling_apis: "P03 and #1033 fences are the reference"
  protocol_versions: NOT_APPLICABLE
  direct_and_reconciled_paths: "quest completions and gameplay cosmetic unlocks: insert-if-absent inside the triggering transaction; achievements: the triggering transaction commits a grant request, and the Achievement owner reconciles it into AccountAchievement idempotently, possibly after the session (decision §4.4)"
  fenced_durable_writes: "quest completion and gameplay cosmetic fact rows and achievement grant request rows, under the character fence; AccountAchievement rows only from a committed request"
  restart_retry_replay_concurrency_pg_reload: "covered by K1 and K2"
  evidence:
    - apps/game-server/migrations/0002_fresh_admission_authority.sql
finding_dispositions:
  p0_p1_accepted_and_repaired:
    - "Codex P1 4121971783 (4b67b20): §4.5 and the scope matrix made the Store inbox Game-owned, contradicting the Store catalog owner decision §1/§3. Repaired: delivery ownership stays open under gap register §32; D47/D49 fix scope and portability only; the item still enters the world only through a DUR-03 MINT"
    - "Codex P1 4122013017 and 4122012997 (0f588f3): Store cosmetic unlocks sat inside the write-once, character-event fact model, preselecting delivery and revocation. Repaired: that model covers gameplay-earned facts only; Store unlocks keep account scope and portability, with delivery and lifecycle under §32 and PROD-ENTITLEMENTS-01 §2.1"
    - "Codex P1 4122065056 (2234322): the policy check used the active revision, ignoring P2 pinning. Repaired: the applicable revision is the character's pinned one while the quest is active; K6 extended"
    - "Codex P1 4122065062 (2234322): cross-world Store claims lacked item compatibility. Repaired: the line records item-definition provenance; the claim validates against the target world under DUR-03 §46 and fails closed, leaving the line claimable"
    - "Codex P1 4122118645 (8af8e03): achievements were inserted directly in the earning transaction, overriding the reward chest grant-request handoff. Repaired: the event records a durable grant request; the Achievement owner derives the fact idempotently, possibly after the session; K1 extended"
    - "Codex P1 4122147295 (4c6e7dd): with later consumption the fact could not promise the earliest earner. Repaired: the fact records the provenance of the request it was derived from, not guaranteed earliest, and nothing may depend on it; K2 reworded"
    - "Codex P1 4122185020 (90634da): gap register §32 and the Store catalog decision §3 still listed Store scope as unresolved. Repaired: both records now note the scope portion as resolved by D47/D49, with delivery, identity, expiry, revocation and refunds still open"
    - "Codex P1 4122231516 (d002a3f): the Store catalog decision §3 still listed scope as open above the new note. Repaired: the §3 list and lead-in no longer list scope"
    - "Codex P1 4122277170 (ac27b40): the finding-family sweep still required every account-fact insert inside the triggering transaction. Repaired: the sweep separates direct quest/cosmetic inserts from reconciled achievement consumption and lists grant requests as a fact source"
  p0_p1_rejected_with_exact_evidence:
    - "Codex P1 4122231506 (d002a3f): a commit cannot contain its own SHA (ANTI_STALL_AND_EXECUTION_BUDGET.md:71,84); each frozen head and freeze time is recorded on #162 and the terminal archive records the final head"
  p2_fixed_accepted_or_deferred:
    - "Codex P2 4121915460 (d7f8834): an opt-out did not stop old facts from satisfying conditions. Fixed: readers apply the current world and quest policy (§4.2); K6 added."
    - "Codex P2 4122277186 (ac27b40): the rejected finding sat in the accepted bucket. Fixed: moved to p0_p1_rejected_with_exact_evidence"
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
