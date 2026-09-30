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
  - apps/game-server/src/durability/account_achievement.rs   # review round 1: grant token (F1), beyond doc text
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
  - apps/game-server/tests/support/account_achievement_postgres_cases.rs   # review round 1 (F1)
  - apps/game-server/src/interaction/chest_use.rs   # merge with main: `achievement: None` only
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
  `FencedGrantingCharacter::after_fence` is removed now that production uses it. Review round 1
  (below) replaces the grant token.

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

## Review round 1

Independent persistence review of `43ac86ff` (routed by the lead): FIX. Exactly-once grant,
`Absent`/`Retired`, untouched `CharacterRevision` and the lock order were confirmed.

- **F1 (required, fixed).** `FencedGrantingCharacter::after_fence` was `pub(super)` and held only a
  CharacterId: any durability module could mint the token without a fence, and
  `record_achievement_grant` never re-checked it. Fix:
  - `after_fence` now takes `&mut` the transaction and a `reward_claim_mint::RewardClaimFenceChecked`
    witness. The witness has a private field, so only `reward_claim_mint` can build it, and only
    `admit` does, right after `character_item_fence_is_current` returns true. `admit` returns the
    token with the destination; the commit passes it to the grant.
  - The token stores `pg_current_xact_id()::text`, read in that transaction when it is created. It
    is neither `Clone` nor `Copy`, and `record_achievement_grant` takes it by value.
  - The grant's root `FOR UPDATE` query adds `AND pg_current_xact_id()::text = $2`. A token from
    another transaction finds no row: `AuthorityRejected`, nothing written.
  - Test-only construction (`in_transaction`) is private to `account_achievement` and used by the
    `#[cfg(test)]` harness, which mints one token per request.
  - New PostgreSQL case `a_token_minted_in_another_transaction_writes_neither_request_nor_fact`
    (`account_achievement_postgres_cases.rs`, run by `account_achievement_postgres` and
    `character_authority_postgres`). Only the token's transaction changes: the same complete fence
    holds in both transactions and the second one commits whatever the grant wrote.
  - Mutation evidence: replacing the xact-id condition with `$2::text IS NOT NULL` fails the new
    case (`Ok(Granted(..))`); skipping the grant call in the commit still fails
    `a_chest_achievement_commits_with_its_claim_once_per_account`.
- **F2 (low, deferred).** Not added in this PR: rollback notes for migration 0021, an optional
  `FOR KEY SHARE` on the root read, and negative runtime-role `UPDATE`/`DELETE` tests.
- **Overlap (not reconciled).** #1297 (D39 chest USE) also changes `durability/reward_claim_mint.rs`:
  it moves the placement into the intent binding (v2) and makes `admit` return `ClaimPending`.
  Whichever PR merges second merges `main` and keeps the no-achievement binding pinned. #1297
  merged first; reconciled in *Merge with main* below.

Validation of the repair (PostgreSQL 17.6, same image digest as CI, pulled through
`mirror.gcr.io` after a Docker Hub 429): `cargo fmt --all --check` and `cargo clippy --locked -p
oteryn-game-server --all-targets -- -D warnings` pass; `--lib` 1065 passed; `reward_claim_mint_postgres`
636, `account_achievement_postgres` 636, `character_authority_postgres` 735,
`check_function_privileges_postgres` 1, `durability_postgres` 717, `item_mint_postgres` 650, all
passed; governance and repository-policy validators and `git diff --check` pass.

## Merge with main

The PR conflicted with `main` at `58429a26` (#1297 D39 chest USE, #1316 CHAR-NAME-1 migration
0022, #1318 GOLD-FEE-1a migration 0023 and others), so the branch head `6257ae5e` merged
`origin/main` with a normal merge commit (no rebase, amend or force-push).

- **Textual conflict (one).** `durability/reward_claim_mint.rs`, the `intent_binding` doc comment:
  both sides kept, now "the claim, its source placement, ... and, only when the chest has one, its
  achievement and catalogue lookup". The code auto-merged: the canonical bytes are main's v2
  (`INTENT_BINDING_VERSION = 2`, `source_placement` after the claim revision) followed by this PR's
  optional achievement suffix. `account_achievement_postgres_cases.rs` and
  `reward_claim_mint_postgres_cases.rs` auto-merged textually (main already named the second
  Character `Second Hero` in the former).
- **#1297 overlap resolved.** `admit` keeps main's order and behaviour: the fence, then the F1
  token (`FencedGrantingCharacter::after_fence` with the `RewardClaimFenceChecked` witness, right
  after `character_item_fence_is_current` returns true), then `already_claimed`, main's §17.2
  `claim_pending` read and the plan (`AlreadyClaimed` before `ClaimPending` before room). The
  commit grants after `admit` and before `apply_claim`, as before. `validate_request` checks both
  main's `source_placement` and the achievement (`Absent` refused, `Retired` admitted).
- **Semantic fixes the merge required.**
  - `interaction/chest_use.rs` (`prepare_chest_use`, the only new `RewardClaimMintRequest`
    constructor on main): `achievement: None`. The chest placement carries no achievement key in
    D39 and no catalogue lookup is available there, so wiring one is out of scope. **Next step:**
    resolve the placement's achievement and its `AchievementCatalogueLookup` in the chest USE path
    once a catalogue lookup exists (no loader is built here).
  - The unit-test request fixture gains main's `source_placement: "fixture:placement.chest"`.
  - `reward_claim_mint_postgres_cases.rs` `switch_to_second_character`: the direct
    `game_character_roots` insert gains the name `Second Hero` (0022 made `name` NOT NULL), as
    main did in `account_achievement_postgres_cases.rs`.
  - No migration or test registration collided: this PR adds no migration.
- **Pinned binding.** The no-achievement unit-test hex is re-pinned from the v1 value to
  `0252d6f592be772f020fb5db54324d97e34502c6980dac6371fca8fc3105cac126`, main's v2 binding for the
  same request (with `source_placement` `fixture:placement.chest`). Evidence: an independent Python
  rebuild of the canonical bytes (length-prefixed texts, the `push_facts` layout, SHA-256, version
  byte). Calibration: the same script with version 1 and without the placement reproduces the
  previously pinned v1 value `016d5194b02ecc07d47aef727e1a1077ba93027a8a51c607f5096bf95193f63429`
  byte for byte; with version 2 and the placement it gives the new pin, and the Rust test passes on
  the merged code. A claim without an achievement therefore keeps main's binding byte for byte.
- **Validation of the merge** (PostgreSQL 17.6, `postgres:17.6-bookworm`): `cargo fmt --all
  --check`; `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`; `--lib` 1132 passed
  (2 ignored); `reward_claim_mint_postgres` 676, `account_achievement_postgres` 676,
  `chest_use_postgres` 742, `character_authority_postgres` 777, `check_function_privileges_postgres`
  1, `durability_postgres` 757, `item_mint_postgres` 690, all passed; governance and
  repository-policy validators and `git diff --check` pass. Mutation re-check: replacing the grant's
  `pg_current_xact_id()::text = $2` condition with `$2 = $2` fails
  `a_token_minted_in_another_transaction_writes_neither_request_nor_fact`.

## Closeout

- Review: independent exact-head review routed by the lead on the frozen head. The worker
  triggered no owner-funded review.
- Merge commit/result: squash merge of the PR (resolve with `git log --grep`).
