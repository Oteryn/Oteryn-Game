# OTV2-20260930-skills0-weapon-skills

```yaml
task_id: OTV2-20260930-skills0-weapon-skills
title: "SKILLS-0 weapon skills, shielding and fishing"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-skills-0
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: 09ccf36b
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_SKILLS0_WEAPON_SKILLS_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md
  - docs/agents/tasks/archive/OTV2-20260930-skills0-weapon-skills.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

SKILLS-0 extends the A13 build state from magic level to all eight Tibia skills (architect
programme plan, #162 5910870596, M1).

- **Storage.** Seven (`level`, `tries`) pairs on `game_character_build_state`, in the same
  CHAR-BUILD-1 migration and build receipt kind; no second table or receipt kind.
- **Chain.** Training, promotion and death directions cover every family; the seed becomes
  (`none`, 0, 0, 7 x (10, 0)); the binding covers the skills.
- **Vocation choice.** Keeps cumulative progress and re-levels it under the new vocation, magic
  level included (architect decision; replaces A13's "equal" rule).
- **Death.** Per-skill loss of cumulative tries by the ratio of the XP DEATH-0 takes to the XP
  before the death.
- **Formulas.** Canary constants, multipliers and training rules as one content table, each
  `PARITY_PENDING`.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: A13 (D150, D151); DEATH-0 and D58 (`death.rs`); GAME-CHAR-01 Stage B; the Tibia
  manual (`characters.md`); `actor_spell_v1.proto` (no skill fields).
- `DERIVED`: Canary `04b83b51` (`OTS_HYPOTHESIS_ONLY`) for formulas, multipliers, training and
  conversion.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only. CHAR-BUILD-1 needs persistence review.

## Acceptance criteria

- [ ] Decision on an exact frozen head with passing validators.
- [ ] Independent exact-head review (persistence).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations and content; offline and exercise training; equipment skill boosts; the skill
  wire.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review (`oteryn-hard-worker`, read-only) of the draft: 3 material findings, 4 evidence
  gaps, 3 hardening, all fixed (the A13 §4.2 amendment with seed, CHECK bounds and binding;
  vocation-choice conversion; an exact loss ratio from DEATH-0; target-level formula; cumulative
  tries; multipliers and the elite knight row; Canary training triggers; the progression marker;
  no skill wire; multi-level receipts).
- Its two owner questions were architect applications of owner rule 5905825574 and D58, and are
  recorded as architect decisions (§3.3, §3.6).
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-skills-0
owner_action_required: null
blocker: null
next_action: null
```
