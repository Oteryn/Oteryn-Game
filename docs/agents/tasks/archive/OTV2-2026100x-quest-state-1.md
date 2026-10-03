# OTV2-2026100x-quest-state-1

```yaml
task_id: OTV2-2026100x-quest-state-1
title: "QUEST-STATE-1 quest tracks, transitions, receipts and obligations"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
lane_id: quest
base_branch: main
branch: quest-state-1
pr: 1684
base_sha: 98a2f95
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01B16idKhSkCFSG9Psnnvks3 (oteryn-hard-worker)
control_plane: claude-code-session-013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-03
updated_at: 2026-10-03
packet: "docs/architecture/reviews/OTERYN_GAME_QUEST_STATE0_QUEST_PROGRESS_STORE_DECISION_2026-09-30.md §14 (with §13)"
leases: migration 0056 (control plane lease)
owned_paths:
  - apps/game-server/migrations/0056_character_quest_state.sql
  - apps/game-server/src/durability/quest_state.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/quest/
  - apps/game-server/src/lib.rs
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - apps/game-server/tests/support/quest_state_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - docs/agents/tasks/archive/OTV2-2026100x-quest-state-1.md
  # Granted by the control plane (option a; D358 reply): minimal additions only, the quest
  # slot, the SEQUENCED_WRITERS entry and the ninth receipt kind in the recovery chain.
  - apps/game-server/src/durability/character_revision_sequencer.rs
  - apps/game-server/src/durability/character_authority.rs
depends_on:
  - "QUEST-STATE-0 accepted (#1661)"
  - "CHAR-REV-SEQ-1 merged (#1663)"
blocks: [QUEST-GATE-1, QUEST-TRIGGER-1, NPC-QUEST-1, QUEST-XP-1, QUEST-PRED-1, QUEST-LOWER-1, QUEST-ACCOUNT-1]
public_contracts: []
external_repositories: []
```

## Outcome

QUEST-STATE-0 §3-§6 and §13 as packet §14 states them, with account completion deferred
(§13.3).

- **Migration 0056.**
  - Four tables: `game_character_quest_tracks`, `game_character_quest_states`,
    `game_character_quest_receipts` and `game_character_quest_obligations`.
  - A receipt is keyed by (Character, cause id, cause ordinal, transition key). A CommandRef cause
    is (GameSessionId, CommandId) under `command`, `use` or `claim_obligation`; a creature-death
    occurrence is (occurrence, 0) under `creature_death`. The receipt keeps the request-only
    binding, the quest's pin (content revision and definition hash), `completes`, and every
    effect's track, value before and value after, with the chain columns of the `0020` siblings.
  - Deferred guards: the receipt is at the root revision, names distinct tracks of its own quest,
    each written by it, and a quest state written by it with its pin; a track row equals its
    receipt's effect and an update continues from the stored value; a quest state is written
    only with a receipt of its quest and completed only by a completing one. Row guards: tracks
    and states are never deleted, rekeyed, rewritten at their revision, repinned or uncompleted;
    receipts are immutable; nothing is truncated.
  - Obligations: born `PENDING` only as the companion of its claim's MINT receipt in the same
    physical transaction; `PENDING -> REFUSED` (terminal, with its code) or
    `PENDING -> WAITING_MIGRATION` and back; deleted only while `PENDING` with the quest receipt
    that names it, inserted by the same physical transaction.
  - The shared consistency guard is replaced by the `0032` body with a ninth arm (§13.1); the
    `0012` claim guard `game_reward_claim_mint_consistency_guard` gets a new body with one arm for
    the obligation companion. No applied migration is edited.
- **Catalogue** (`src/quest/mod.rs`, path-loaded as `durability::quest_state::quest` so every
  crate that path-loads `durability` compiles it): `QuestStateCatalogue` with tracks,
  transitions (at most 8 effects on the quest's own tracks), each quest's `definition_hash` over
  its tracks and transitions only, and the pure §4 evaluation (`NOT_SUPPORTED`, then
  `STAGE_MISMATCH`, then `OUT_OF_RANGE`).
- **Writer** (`durability/quest_state.rs`): `commit_character_quest_transition`, fenced like the
  XP writer, with the §5.3 lock order, the request-only binding (replay or conflict), one revision
  advance and one receipt per transition, and `CAPACITY_EXCEEDED` at RL-01 and RL-05. A
  completed quest keeps its pin and no longer blocks (§6). `RevisionSlot::commit_quest_transition`
  retries once on a mismatch (§5.2); the structural gate lists the writer.
- **Obligations** (§5.4): `RewardClaimMintRequest::quest_transition` is part of the intent
  binding (a claim without one keeps its binding); the claim inserts the `PENDING` row with its
  receipt, or refuses with `ObligationsFull` at RL-07 before anything is written. `chest_use`
  passes the chest's transition; Content declares none until QUEST-LOWER-1.
- **Admission** (§7, §12.3): `read_character_quest_state` loads tracks, states and open
  obligations, failing closed over RL-01, -05, -07 or -08. `admit_character_quest_state` loads
  the copy and requests each `PENDING` obligation again through the revision slot. The
  production `ComposedFreshAdmission` runs it at fresh admission and at resume, keeps the copy per
  session (a failed load fails quest actions closed, not login), and requests a failed attempt
  again after 60 seconds on the owner cadence. That cadence runs beside the listener in
  `serve_gameplay` for every session, and a committed chest USE whose chest names a transition
  requests its obligation in the same session. No quest catalogue is loaded yet
  (`quest_catalogue: None`), so obligations stay pending until QUEST-LOWER-1.
- **Recovery integrity**: `verify_character_integrity` counts quest receipts in the chain.

## Tests

- Unit: catalogue validation, every comparison and effect kind, bounds and checked `ADD`, the
  definition hash; the request binding; the cause encoding; the copy; the claim intent binding.
- PostgreSQL 17.6 (`character_authority_postgres`):
  - `quest_state_postgres_cases`: commit, replay, conflict, reconcile and each refusal code
    writing nothing; the chain across XP, stance and quest receipts with the integrity check; a
    bypass writer caught (slot retry once, fence mismatch, guard); a same-GameSession reconnect
    commits and a replaced or ended session is refused; every guard branch; RL-01, -02, -04,
    -05, -06 at and over the bound, and the load failing closed over RL-01.
  - `reward_claim_mint_postgres_cases::quest_obligations`: a claim with a transition commits
    the items and the `PENDING` row together, a claim without one writes none; the delete,
    insert and state guards; admission without and with the catalogue; `REFUSED` terminal;
    `WAITING_MIGRATION` kept, counted and not retried; RL-07 at and over the bound.
  - `chest_use_postgres_cases`: a chest USE records no obligation.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test --locked -p oteryn-game-server` with PostgreSQL 17.6: pass (all targets; new suites `quest_state_postgres_cases` 7, `quest_obligations` 2).
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass (54 tests).

## Review

Independent persistence review (Codex) on the final frozen head; the control plane requests it.

- Round 1 on `a3a5e44`: two P1s, both accepted and fixed in the repair candidate.
  - 4174607861: the obligation retry depended on the vitals-only Serene tick. It now runs on an
    unconditional owner cadence in `serve_gameplay`, which reloads the session copy and also
    retries a failed load.
  - 4174607858: a chest claim's new obligation waited for the next admission. A committed chest
    USE whose chest names a transition now refreshes the session at once; an unproven outcome is
    refreshed on the cadence after the backoff.

## Deviations and open points

- The ComposedFreshAdmission path is proved through its shared admission step
  (`admit_character_quest_state`, which `admit` and `resume` both call) on PostgreSQL; no test
  drives `ComposedFreshAdmission` itself against a database, since no harness for it exists
  outside the WP5 topology.
- `src/lib.rs` is unchanged: the catalogue is path-loaded under `durability`.
