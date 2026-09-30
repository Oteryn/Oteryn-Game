# OTV2-20260930-chest-content-2a

```yaml
task_id: OTV2-20260930-chest-content-2a
title: CHEST-CONTENT part 2a - reward count link rule and the deferred D39 re-review LOW items
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: null
allocation: "#162 5909237761 (part 2), scope ruling 5911004459, split 5911566009 / 5911586050"
base_sha: 93982dfa
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: CHEST-1/D39 worker (claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_playable.rs  # shared: RewardClaim count rule only
  - apps/game-server/tests/content_reference_playable.rs  # shared: one case appended
  - apps/game-server/src/durability/reward_claim_mint.rs  # comment only
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs  # two cases appended
  - docs/agents/tasks/archive/OTV2-20260930-chest-content-2a.md
public_contracts: []
depends_on: [CHEST-CONTENT part 1 (#1334), #1320]
blocks: [CHEST-CONTENT part 2b (loader and 231 claims; shape asked in 5911586050)]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **Link count rule** (carried LOW, review 5911004459). A RewardClaim reward whose Item has
  known stack facts must fit them:
  - a NonStackable Item takes exactly one;
  - a StackCapable Item with a known `stack_max` takes at most that maximum.

  Unknown facts stay with the MINT admission, which fails closed on them (D82). One case covers
  single and over for both kinds, plus unknown.
- **D39 re-review LOW items** (5907886746), deferred from #1334 until #1320 merged:
  - The `admit` comment now names the right lock: `lock_admission_relations`' EXCLUSIVE table
    locks serialize the pending read, not the `character_root` row lock.
  - New PG case with a real state-3 session:
    - a reservation stranded in an ended GameSession no longer blocks the claim for the
      Character's new session;
    - that reservation itself is rejected at commit.

    Mutation evidence: dropping the session-state condition from the pending query fails this
    case with `ClaimPending`.
  - New PG case for the commit-time re-check: a reservation of the same claim that appears after
    this candidate froze refuses the commit with `ClaimPending`, and the footprint is unchanged.
- **Nit** `chest_use.rs:232`: stays as is. That branch cannot pass link, and it fails closed
  with a comment saying so.

## Validation

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `--lib`: 1142. `content_reference_playable`: 46. `interaction_workflow`: 15.
- PostgreSQL 17.11 (local; CI pins 17.6): `reward_claim_mint_postgres` (688) and
  `chest_use_postgres` (752) pass, including the 2 new cases.

## Closeout

- merge commit/result: squash merge of this PR (pending)
- review: independent exact-head review after freeze (pending)
- follow-up: CHEST-CONTENT part 2b (loader and 231 claims), once the control plane answers
  5911586050
