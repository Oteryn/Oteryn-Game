# QUEST-XP-1

```yaml
task_id: QUEST-XP-1
title: "QUEST-XP-1 quest XP obligation and its XP writer path"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
lane_id: quest
base_branch: main
branch: claude/quest-xp-1
pr: 1724
base_sha: 53a60a6
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01GiZEBS5mFj87BsA5Y3PKHS (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
packet: "docs/architecture/reviews/OTERYN_GAME_QUEST_GATE0_QUEST_GATES_AND_NPC_QUESTS_DECISION_2026-09-30.md row QUEST-XP-1, §5.5 and §15 (XP chain); QUEST-STATE-0 §13.1; owner decision D477"
leases: migration 0069 (control plane lease)
owned_paths:
  - apps/game-server/migrations/0069_character_quest_xp_obligations.sql
  - apps/game-server/src/quest/mod.rs
  - apps/game-server/src/durability/quest_state.rs
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/durability/character_revision_sequencer.rs
  - apps/game-server/tests/support/quest_state_postgres_cases.rs
  - apps/game-server/tests/support/quest_xp_postgres_cases.rs
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
  - docs/agents/tasks/archive/QUEST-XP-1.md
depends_on:
  - "QUEST-STATE-1 merged (#1684)"
  - "CHAR-REV-SEQ-1 merged (#1663)"
public_contracts: []
external_repositories: []
```

## Outcome

QUEST-GATE-0 §5.5 on the QUEST-STATE-1 store and the CHAR-REV-SEQ-1 sequencer.

- **Content** (`src/quest/mod.rs`): `QuestTransition::experience: Option<i64>`;
  `QuestStateCatalogue::new` refuses zero, a negative value or a value over
  `QUESTGATE0_RL_05` (100,000,000) with `InvalidExperience`, so the quest is not admitted. The
  reward is not in the definition hash: a reward edit never blocks players in progress, like
  journal text. `QUESTGATE0_RL_10` (16) bounds pending XP obligations. QUEST-LOWER-1's loader sets
  the field (control plane QUESTION, option a).
- **Migration 0069** (`game_character_quest_xp_obligations`): one row per XP-bearing receipt
  (Character, receipt key, amount 1..RL-05, UUIDv7 reward occurrence, the quest's pinned content
  revision as provenance). Guards: inserted only with its quest receipt (same key and pin, same
  physical transaction), for an occurrence no XP receipt names, at most 16 per Character; never
  updated; deleted only with the XP receipt of its occurrence for the same Character and amount;
  an XP receipt of an occurrence that names an obligation commits only with its delete; no
  truncate. The shared consistency guard (eight kinds plus the quest receipt) is unchanged.
- **Quest writer** (`durability/quest_state.rs`): an XP-bearing transition locks and counts the
  Character's pending XP obligations before any write and is refused whole as `OUT_OF_RANGE` at
  RL-10 (a claim obligation then closes `REFUSED` with that code); otherwise it inserts the
  obligation in its own transaction. The receipt (`CommittedQuestTransition::experience`) names the
  obligation while it is pending, also on a replay.
- **XP writer** (`durability/character_progression.rs`): `commit_character_quest_experience`, the
  same fenced writer with one extra step: it locks the obligation after the root and progression
  state, requires the Character and amount, and deletes it in the award's transaction. The plain
  writer is unchanged; the guard refuses it for a quest occurrence.
- **Sequencer**: `RevisionSlot::commit_quest_experience` (fail closed on a mismatch, the XP
  binding includes the revision) and `commit_quest_transition_with_experience`, which submits the
  award in the same slot after the transition. The award's context, `policy_revision` and
  `reward_revision` come from the active `FiniteProgressionPolicy`, never from the quest content
  revision. The structural gate lists the quest XP writer.
- **Replay**: `read_character_quest_state` loads pending XP obligations (failing closed over
  RL-10) and `request_pending_quest_experience` requests each again in the revision slot; a
  refused award keeps its obligation and is reported as a defect, an unknown outcome asks for a
  retry. No production caller holds a progression policy yet (the transport has none), so wiring
  it into `ComposedFreshAdmission` waits for the caller that brings the active policy.

## Tests

- Unit: `experience` bounds at and over RL-05, the reward outside the definition hash.
- PostgreSQL 17.6 (`character_authority_postgres`, `quest_state_postgres_cases::quest_xp_postgres_cases`):
  - a transition and its award commit in one slot (r2 quest, r3 XP, reward revision from the
    policy), replay submits no second award, a plain transition writes no obligation;
  - a pending obligation after a stop before the award is named by the replay, loaded at
    admission and awarded once; the exact award replays; nothing is awarded twice;
  - fencing and refusals keep the obligation: another amount, another reward revision, an
    occurrence without obligation, a stale connection generation, the plain XP writer, and a
    calculation refusal reported without retry;
  - RL-10: the seventeenth XP-bearing transition is refused writing nothing, a plain one commits,
    an award frees a slot;
  - every 0069 guard branch by SQL.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test -p oteryn-game-server` with PostgreSQL 17.6: pass (all targets, 64 test binaries; new `quest_xp_postgres_cases` 5).
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass (54 tests).

## Review

Independent persistence review on the final frozen head; the control plane requests it. `experience` is kept out of `definition_hash` (QUEST-STATE-0 §6 and QUEST-GATE-0 are silent on reward fields; control plane option a), stated in the PR body for review.
