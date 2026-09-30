# OTV2-20260929-charm2-bestiary-progress

```yaml
task_id: OTV2-20260929-charm2-bestiary-progress
title: CHARM-2 - Bestiary kill progress as a creature-death descendant (migration 0019)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charm2-bestiary-progress
issue: 162
lane_id: GAME-CHAR durability (0009 guard-function chain)
pr: 1306
base_sha: 4ea220fa   # merged with main at c3263e85 (after #1278, DEATH-1)
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "CHARM-2 hard worker (claude-code-session-012nzPTz29NThWJG45F2m5fP)"
created_at: 2026-09-29
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0019_character_bestiary_progress.sql   # renumbered from 0018 (#1278 took 0018)
  - apps/game-server/src/durability/bestiary_progress.rs
  - apps/game-server/src/durability/mod.rs          # module + linkage test only
  - apps/game-server/src/durability/character_authority.rs   # one Bestiary UNION arm in verify_character_integrity
  - apps/game-server/src/combat/death_reward.rs     # the new descendant only
  - apps/game-server/src/combat.rs                  # re-exports only
  - apps/game-server/src/domain/bestiary.rs
  - apps/game-server/src/domain/mod.rs              # module line only
  - apps/game-server/tests/bestiary_progress_postgres.rs
  - apps/game-server/tests/combat_bestiary_postgres.rs
  - apps/game-server/tests/support/bestiary_postgres_harness.rs
  - apps/game-server/tests/support/bestiary_progress_postgres_cases.rs
  - apps/game-server/tests/support/combat_bestiary_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # shared: #[path] includes only
  - rulesets/progression/bestiary/**
  - docs/agents/tasks/archive/OTV2-20260929-charm2-bestiary-progress.md
public_contracts:
  - "CHARM-0 decision packet §4.1, owner answers §7 (2a, 6a) (#1295)"
  - DUR-02 §4.1, §4.6, §7
  - VSL-COMBAT-01 §24.1
depends_on: ["#1278 DEATH-1 (merged d9f74372)"]
blocks: [CHARM-3, CHARM-5]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

A persistent kill counter per character and Bestiary race: `(CharacterId, race key = Creature
definition key) -> kill_count`. It is written as one more independent descendant of a committed
creature death, after loot and XP, under the same `CurrentCharacterGameplayFence`
(`character_progression::assert_gameplay_fence`) and the same memoized (death, character) reward
occurrence.

- Only creatures whose definition carries `bestiary` count.
- The counter saturates at the final kill threshold. A kill at the bound writes nothing and advances
  no CharacterRevision.
- Stages are derived from `kill_count` and `kill_thresholds`, never stored (answer 2a).
- Credit: the principal damaged the creature at most 5 minutes (inclusive) before its death
  (answer 6a).

## Design

- **Migration 0019**:
  - `game_character_bestiary_kill_receipts`: immutable, keyed by the UUIDv7 occurrence. It carries
    CharacterRevision original/committed (+1), level and experience before = after, `race_key`,
    `race_definition_revision`, `race_digest`, `final_kill_threshold`, `kill_count_before/after`
    (CHECK `after = before + 1 <= final`) and the eight revision fields.
  - `game_character_bestiary_progress`: `(character_id, race_key)` primary key, `kill_count >= 1`,
    latest revision and occurrence. It is never deleted or reassigned.
  - The current consistency guard (0017) is replaced, with every existing arm kept. It adds the
    Bestiary receipt kind to the revision-one check, the cross-kind chain and the transition
    binding. It also binds the per-race kill chain (`before` = previous `after`, 0 first) and each
    progress row to the race's latest receipt.
  - Truncate guards, fixed `search_path`, runtime SELECT/INSERT on receipts and SELECT/INSERT/UPDATE
    on rows, control SELECT.
- **`verify_character_integrity`**: the Bestiary receipt kind joins the XP/death/stance chain, so
  `open_character_authority` stays valid after kills (restart case).
- **Writer** `durability::bestiary_progress`:
  - Replay of an exact occurrence returns the retained receipt without reacquiring authority.
    Changed semantics (character, race key, definition revision, thresholds, revisions) conflict.
  - The command binding excludes the expected CharacterRevision. The composition derives it from
    the sibling XP outcome, which can differ between a first attempt and its replay.
  - `reconcile_bestiary_kill` and `read_bestiary_progress`.
- **Composition** `settle_creature_death_rewards_with_bestiary`:
  - Loot and XP run first, unchanged.
  - The kill expects the XP award's committed revision when XP committed or replayed, otherwise the
    fence's.
  - A missing race is `NotABestiaryRace`; outside the window is `OutsideCreditWindow`; damage after
    the death fails closed (`InvalidCreditEvidence`).
- **Rules**: `rulesets/progression/bestiary/kill-progress.json`. A unit test checks that it matches
  the code.

## Assumptions and open items

- Owner Q1 (open): the credit-window evidence (principal's last damage time, death time) is
  caller-supplied. The owner carrier records no damage time. The stated assumption is (a): keep
  caller evidence now and add owner-side damage times in a later foundation task.
- CHARM-3 (0020) must base its replacement of the consistency guard on 0019's version and keep the
  Bestiary arms, and must keep the Bestiary arm in `verify_character_integrity`.
- P2 (deferred): the consistency guard scans a character's whole receipt chain on every commit,
  as XP already does. Kill receipts add to the chain, bounded by the sum of final thresholds.

## Validation (local)

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`,
  `cargo test --locked -p oteryn-game-server`: pass.
- PostgreSQL 17.6: `character_authority_postgres`, `bestiary_progress_postgres`,
  `combat_bestiary_postgres`, `character_stance_postgres`, `character_death_receipts_postgres`,
  `character_progression_postgres`, `combat_death_reward_postgres`, `durability_postgres`: pass.
  - The restart case, RED before #1278, is now GREEN.
- Negative cases, one invariant each:
  - stale connection, lease or scope generation; substituted session, character or node; stale
    CharacterRevision;
  - stale context, policy or reward revision; missing progression state; zero lease;
  - conflicting occurrence reuse (race, thresholds, definition revision);
  - concurrent distinct kills and concurrent same occurrence; saturation;
  - 13 direct-SQL bypasses, each asserted by its rejection reason.
- Composition cases: credited, replay, not a race, outside the window, future evidence, and an XP
  failure that does not block the kill.
- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: pass.

## Closeout

- Review: independent exact-head persistence review, routed by the control plane after freeze.
- Merge commit/result: squash merge of #1306 (resolve with `git log --grep "(#1306)"`).
