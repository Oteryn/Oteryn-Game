# OTV2-20260929-a13-character-build-state

```yaml
task_id: OTV2-20260929-a13-character-build-state
title: "A13 Character build state: vocation and magic level (D150-D151)"
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: null
base_sha: 48de3868
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md
  - docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md
  - docs/agents/tasks/active/OTV2-20260929-a13-character-build-state.md
  - docs/agents/tasks/archive/OTV2-20260929-a13-character-build-state.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records architect item A13 (SPELL-CASTER-FACTS, #162 5896414182), with owner decisions
D150 and D151. The ruling is 5896480875.

- **Storage:** Game-owned build state (vocation, magic level, mana spent), one row per Character.
- **Receipts:** a new build receipt kind in the `0009` chain, joining the DEATH-0, STANCE-0 and
  H-1 guard chain.
- **Acquisition:** the vocation is chosen on Dawnport, as in Global. Magic-level training from mana
  spent ships in V1, with a bounded checkpoint.
- **Spell contract:** §10 now points to A13 for magic-level training.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: #162 5896414182; migrations `0001`-`0015`; `CasterState`; the spell cast contract §10.
- `UNKNOWN`: the Dawnport choice details, the magic-level formula and multipliers, and the death
  loss amounts. These are for the implementation lanes.

## High-risk authority/recovery qualification

Not applicable to this docs-only task. CHAR-BUILD-1 changes persistence and needs its own
persistence review.

## Acceptance criteria

- [ ] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Migrations, runtime code and content.

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `git diff --check`

## Context checkpoint

```yaml
last_progress: decision authored
status: implementing
branch: claude/gifted-rubin-a0axzx
head_sha: null
pr: null
owner_action_required: null
blocker: null
next_action: "open the PR, request review, archive this record in the final authoring commit"
```
