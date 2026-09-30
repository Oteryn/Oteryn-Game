# OTV2-20260930-achievement-account-facts

```yaml
task_id: OTV2-20260930-achievement-account-facts
title: ACHIEVEMENT step 3 - grant requests and AccountAchievement facts (migration 0021)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/achievement-account-facts
issue: 162
lane_id: ACHIEVEMENT
pr: null   # opened by the lead; recorded in the FREEZE_SHA packet on #162
base_sha: cf251bb2
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "achievement account-facts worker (claude-code-session-01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0021_account_achievements.sql
  - apps/game-server/src/durability/account_achievement.rs
  - apps/game-server/src/durability/mod.rs      # registration and linkage block only
  - apps/game-server/tests/account_achievement_postgres.rs
  - apps/game-server/tests/support/account_achievement_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # protected PostgreSQL lane: one #[path] include only
  - docs/agents/tasks/archive/OTV2-20260930-achievement-account-facts.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md §3, §4, §5 step 3 (#1287)"
  - "account-progress decision 2026-09-28 §4.4, §4.6, §7 required_revalidation"
blocks: ["ACHIEVEMENT step 4: first granter wired end to end"]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner authorization: 2026-09-30, this session ("1 i 2 jest ok, skończ to zadanie").

- Migration 0021:
  - `game_account_achievement_grant_requests`: one immutable row per (source event, key) with
    account, earning Character, catalogue revision and server time (`earned_at`).
  - `game_account_achievements`: the write-once fact, primary key (account, key). Its only payload
    is the request it was derived from (composite foreign key), so the first committed insert keeps
    its provenance.
  - A deferred guard rejects, at commit, a request whose Character is not a live root of its
    account, or a request that was not consumed into a fact in its own transaction.
  - Update, delete and truncate are rejected. The runtime role gets SELECT and INSERT; the control
    role gets SELECT. No existing object changes, and `CharacterRevision` is not advanced.
- `durability::account_achievement::record_achievement_grant` runs inside the granter's
  transaction:
  - the caller passes a `FencedGrantingCharacter`, which only durability writers can create after
    their fence;
  - it re-takes the `character_root` row lock (the lock the XP, reward-claim and Bestiary writers
    use) and reads the account from the root;
  - it inserts the request, then inserts the fact with `ON CONFLICT DO NOTHING`, and returns
    `Granted` or `AlreadyHeld` with the fact's first provenance.
  - A replayed request is idempotent. A source event reused with other facts conflicts.

## Architect choices for review

- Catalogue check as input (packet item 3): the granter passes `AchievementCatalogueLookup`
  (`Earnable { revision }`, `Retired`, `Absent`). No runtime loader for `content/achievements/`
  exists. `Absent` returns `UnknownAchievement` before any write, and the granter drops its
  transaction. `Retired` writes nothing and returns `Retired`, so the granting transaction goes on.
- `source_event` is `(source_kind, source_event_id)`: a token for the granting domain plus its own
  event id of 1 to 64 bytes. It is not a foreign key, because the granting kinds (quest, reward
  claim, interaction, counter) keep different receipt keys. The step-4 granter fixes its kind.
- No granter exists yet. The Postgres cases use a `#[cfg(test)]` granting transaction
  (`commit_test_achievement_grants`). It runs the XP writer's complete gameplay fence
  (`assert_gameplay_fence`), then the grants in order, all committed together or not at all.
  Production has no standalone grant entry, because the contract puts the request inside the
  earning event's own transaction.
- No audit-outbox event: the immutable request row is the durable provenance. Bestiary and charm
  receipts follow the same pattern.
- Points: not implemented. No caller or test needs them, and they are a read over facts and a
  world catalogue (contract §4).

## High-risk authority qualification

```yaml
applicable: true
authority_invariants:
  identity_binding: [request -> root's account (deferred guard), fact -> its request (composite FK), (source_kind, source_event_id, key) -> one request]
  current_liveness: [granter's fence: recovery fence, FND-04 session/connection/lease/scope, scope assignment + node incarnation, root row lock]
  temporal_provenance: [first committed fact keeps its request; earned_at is server statement time]
consumer_boundaries: [record_achievement_grant, SQL writes under 0021]
mutation_operators:
  applicable: [stale connection generation, ended session (other Character took presence), duplicate key other source, duplicate key other Character, exact replay, source reuse with other revision, fact without request, request without fact, request bound to another account, concurrent same key, unknown key, retired key, update/delete/truncate, runtime-role grants]
  considered_not_applicable: [expired/future time - no time-bounded input]
one_invariant_per_negative_case: true
record_derived_matching_helper: none
```

## Validation (local)

- `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`:
  pass.
- `cargo test --locked -p oteryn-game-server --lib`: 1062 passed.
- PostgreSQL 17.6 (`postgres:17.6-bookworm` at the CI digest):
  - `account_achievement_postgres`: 632 passed, including the 6 new cases;
  - `character_authority_postgres`: 729 passed (the protected lane, including the new cases);
  - `check_function_privileges_postgres`: 1 passed;
  - `bestiary_progress_postgres`: 631 passed;
  - `charm_state_postgres`: 634 passed;
  - `durability_postgres`: 714 passed.
- RED:
  - without the guard's consumption arm, the unconsumed-request case fails;
  - without the runtime INSERT grant, the runtime-role case fails.
- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: pass.

## Closeout

- Review: the step-3 persistence review (contract §5), independent and on the exact head, routed by
  the lead on the frozen head. The worker triggered no owner-funded review.
- Merge commit/result: squash merge of the PR (resolve with `git log --grep`).
