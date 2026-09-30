# OTV2-20260930-reward-claim-achievement-grant

```yaml
task_id: OTV2-20260930-reward-claim-achievement-grant
title: ACHIEVEMENT step 4 - the reward-claim MINT grants the chest's achievement in its fenced transaction
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/reward-claim-achievement-grant
issue: 162
lane_id: ACHIEVEMENT
pr: null   # opened by the lead; recorded in the FREEZE_SHA packet on #162
base_sha: a4f61f39
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "reward-claim achievement worker (claude-code-session-01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/src/durability/account_achievement.rs   # visibility and doc text only
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260930-reward-claim-achievement-grant.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md §3, §5 step 4"
  - "docs/architecture/OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md D40-D42, §5.1, §6"
  - "ACHIEVEMENT step 3: migration 0021 and durability::account_achievement (#1314)"
blocks: ["ACHIEVEMENT: display of account achievements"]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner authorization: 2026-09-30, this session ("tak" to "robić krok 4 ze skrzynią?").

- `RewardClaimMintRequest` gains `achievement: Option<RewardClaimAchievement>`: the chest's
  `placement.achievement` key and the caller's `AchievementCatalogueLookup` (`Earnable { revision }`,
  `Retired`, `Absent`). The frozen candidate carries it inside its request; no runtime loader for
  `content/achievements/` exists and none is added.
- The achievement is part of the intent binding, appended only when present. A claim without one
  keeps its pre-achievement binding (pinned by a unit test against an independent reconstruction),
  so in-flight reservations stay resumable. The same CommandRef with another key or lookup, or
  without the achievement, is `ConflictingCause`.
- `commit_reward_claim_mint` calls `record_achievement_grant` in its own DUR-03 transaction, after
  `admit` (the complete B3-1 fence ending with the `character_root` row lock) and before the item,
  entry, audit event, RewardClaim and receipt are written. Any grant error drops the transaction.
- Source event: kind `oteryn:reward-claim`, id = Character id (16 bytes) followed by SHA-256 over
  the length-prefixed claim family and production key (the RewardClaim's unique key; the claim
  revision is excluded, D40). A replay or reconcile returns the receipt before reaching the grant;
  a second USE of a `once` chest is `AlreadyClaimed` before it.
- `Absent` is refused as `RewardClaimMintError::UnknownAchievement` by input validation, before any
  database work: nothing is reserved, minted, claimed or granted. The commit keeps the step-3 check
  as a second line (mutation evidence below). `Retired` commits the item and claim with no
  achievement rows. No `CharacterRevision` change, no migration, no audit, protocol, client,
  content or ranking change.
- `account_achievement.rs`: `valid_key` becomes `pub(super)`; the `dead_code` expectation on
  `FencedGrantingCharacter::after_fence` is removed now that production uses it; doc text only
  otherwise.

## Architect choices for review

- No migration: the achievement rides on the request and the intent binding already fixes it in
  the durable reservation, so the reward-claim record does not need to store the key or revision.
  The grant request row is its durable provenance.
- `Absent` fails at freeze rather than only at commit, which is stricter than contract §3.3
  ("nothing is written") because not even the reservation is written.
- `AchievementGrantError::ConflictingSourceEvent` maps to `ConflictingCandidate`. It cannot occur
  through this path: the RewardClaim row and the request with its source event commit together.
- The item audit event does not record the grant; its schema is unchanged.

## High-risk authority qualification

```yaml
applicable: true
authority_invariants:
  identity_binding: [grant source event -> (Character, claim family, claim production key), achievement -> intent binding, request account -> locked root (step 3)]
  current_liveness: [B3-1 item fence at commit: recovery fence, FND-04 session/connection/lease/scope, scope assignment + node incarnation, character_root row lock]
  temporal_provenance: [first committed fact keeps its request; a second Character's claim adds a request only]
consumer_boundaries: [freeze_reward_claim_mint (validation), commit_reward_claim_mint (grant in the DUR-03 transaction), reconcile_reward_claim_mint (read-only)]
mutation_operators:
  applicable: [stale connection generation at commit, absent key, retired key, exact replay, reconcile, second USE of a once chest, same command with changed/removed achievement, second Character of the account, malformed key or revision]
  considered_not_applicable: [expired/future time - no time-bounded input; other fence operators - unchanged fence, covered by the existing every_fence_operator case]
one_invariant_per_negative_case: true
record_derived_matching_helper: none
```

## Validation (local)

- `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`:
  pass.
- `cargo test --locked -p oteryn-game-server --lib`: 1064 passed (2 new unit tests).
- PostgreSQL 17.6 (`postgres:17.6-bookworm` at the CI digest):
  - `reward_claim_mint_postgres`: 635 passed, including the 2 new cases;
  - `character_authority_postgres`: 733 passed (the protected lane; it includes both new cases);
  - `account_achievement_postgres`: 634 passed;
  - `check_function_privileges_postgres`: 1 passed;
  - `durability_postgres`: 716 passed;
  - `item_mint_postgres`: 649 passed.
- RED (mutation):
  - skipping the grant call in the commit fails the earnable case (`RowNotFound`);
  - removing the freeze-time `Absent` refusal fails the absent case: the reservation is written,
    while the item, claim, request and fact are not (the commit's step-3 check still holds);
  - dropping the achievement from the intent binding fails the `ConflictingCause` assertions.
- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: pass.

## Closeout

- Review: independent exact-head review routed by the lead on the frozen head. The worker
  triggered no owner-funded review.
- Merge commit/result: squash merge of the PR (resolve with `git log --grep`).
