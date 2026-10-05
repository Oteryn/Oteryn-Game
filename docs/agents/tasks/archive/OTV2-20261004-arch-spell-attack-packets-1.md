# OTV2-20261004-arch-spell-attack-packets-1

```yaml
task_id: OTV2-20261004-arch-spell-attack-packets-1
title: "ARCH-SPELL-ATTACK-PACKETS-1: full spell book activation, SPELL-TARGET-1 packet, rune order"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-spell-attack-packets-20261004
issue: 162
pr: 1787
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01YL1cQaLL3BquJajKivZVhw (Sol Supervising Architect)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_SPELL_ATTACK_PACKETS_2026-10-04.md
  - docs/agents/tasks/archive/OTV2-20261004-arch-spell-attack-packets-1.md
public_contracts: []
depends_on: []
blocks: [SPELL-BOOK-ACTIVATE-1, SPELL-TARGET-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **D577 `b`:** the 246-spell book reaches the default node through
  `OTERYN_NATIVE_GAMEPLAY_MANIFEST` plus a matching content-activation issuance. There is no code
  default, path fallback or embedded catalogue (§1.1). The owner-side steps are listed in §1.3.
- **SPELL-BOOK-ACTIVATE-1** (§2.1): a whole-book disposition sweep with goldens, area and
  directional damage evidence, refusal fixes only, and the node-boot qualification default moved
  to `content/spells.manifest.json`.
- **Damage spells** (§1.4): area and directional damage already run through ordinary combat.
  **SPELL-TARGET-1** (§2.2) replaces the two `AttackTarget` arms after ATTACK-1b and SPELL-LOCK-1
  merge.
- **RUNE-USE-0** stays a candidate. It is accepted after ITEM-USE-1 is packeted, and its children
  keep their order (§1.5).
- **#1787 P1s 4179125682, 4179125690, 4179125692** (§1.3):
  - deploy the manifest tree with `imports/spells/r25/source-world.json` and
    `rulesets/progression/wheel-of-destiny/spell-profile.json`, keeping repository-relative paths;
  - the real command is `content activate ... --request <file>`, with a named request file;
  - no rollout or availability promise: a mismatched node refuses readiness until its
    configuration and the activation match, and rollback is a baseline issuance at the next
    sequence.
- **#1787 round 2, 4179150924, 4179150930 and 4179150933:**
  - the activation command carries the required `--config <ops-config>`, in §1.1 and §1.3;
  - SPELL-BOOK-ACTIVATE-1 changes the node-boot stager to hash-bound staging from the repository
    root, keeping repository-relative paths, because the canonical manifest's locators leave
    `content/` (§2.1).
- No code, contract or wire change.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
