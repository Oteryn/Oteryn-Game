# OTV2-20260928-death0-character-death-receipt-decision

```yaml
task_id: OTV2-20260928-death0-character-death-receipt-decision
title: "DEATH-0 Character death receipt decision"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1148
base_sha: 7d1134f090ac249f964fede017efabba91e22b90
head_sha: 784daecad5017d1e067cefd4d61f7dd4ab0fab92
final_head_sha: 784daecad5017d1e067cefd4d61f7dd4ab0fab92
final_head_frozen_at: 2026-09-28T18:27Z
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md
  - docs/agents/tasks/active/OTV2-20260928-death0-character-death-receipt-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records the architecture decision for DEATH-0, the storage prerequisite of the Reference
first player death decision (#1132, §4.3). It adds a second receipt kind (death) to the
`0009` Character revision chain, defines the guard changes, a blessing state table, the respawn
position record and the `commit_character_death` writer contract for DEATH-1.

No migration, runtime or registry change; the migration belongs to the DEATH-0 allocation.

## Architecture and source of truth

- `PROVEN`: migration `0009_character_progression.sql`; `durability/character_progression.rs`;
  the Character/item composition decision §3.6; the first player death decision §4.3-§4.6.
- `UNKNOWN`: Character position persistence; blessing purchase receipts (DEATH-4).

## High-risk authority/recovery qualification

The decision changes the Character revision chain's admissible successors, a persistence
invariant. It keeps every existing guard property for XP receipts, adds a death kind with its own
monotonic constraint (experience never increases), requires exactly one receipt of either kind per
revision and continuity across kinds, and reuses the XP writer's fences and occurrence
idempotency. DEATH-0 and DEATH-1 carry the negative cases in the handback when allocated.

## Acceptance criteria

- [x] The decision document is on an exact frozen head with passing validators.
- [x] Independent exact-head review (one Codex review, one repair generation; owner decision to merge after green CI).
- [x] Protected Merge Queue integration (`75e502a8`).

## Excluded scope

- The migration, the writer code, Character position persistence, blessing purchases, item effects.

## Finding dispositions

Codex review of `053819d`: three P1, all ACCEPTED in repair generation 1 of 1 (#162 convergence
rule 5869165340).

- 4125646927 (respawn ignored after a restart): a `game_character_pending_respawns` obligation row
  is inserted by the death transaction and consumed at respawn, admission or recovery (§3.4).
- 4125646943 (no complete intent binding): the receipt carries `command_binding` and
  `policy_digest`; the writer compares the binding before replay (§3.1, §3.5).
- 4125646935 (no durable death cell): the receipt records the death cell for resumed DEATH-3 item
  effects (§3.1).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Terminal integration

- PR #1148 merged through the Merge Queue on 2026-09-28 as `75e502a8`.
- Review: one Codex review of `053819d` (three P1s), repaired in the single repair generation
  `784daec`; the owner decided to merge after green CI.
- Protected-main readback: the decision and this record on `75e502a8` are byte-identical to the
  frozen head `784daec`.
- The PR body predates the repair (pending respawns, intent binding, death cell); the decision is
  authoritative.
- Next allocations: DEATH-0 (migration); DEATH-1b after DEATH-0 (DEATH-1a requested on #162).
- Archived under `OTV2-20260928-spell-d7-cw1-chest-amendments`.

## Context checkpoint

```yaml
last_progress: protected-integrated as 75e502a8; archived
status: completed
branch: claude/gifted-rubin-a0axzx
head_sha: 784daecad5017d1e067cefd4d61f7dd4ab0fab92
pr: 1148
owner_action_required: null
blocker: null
next_action: null
```
