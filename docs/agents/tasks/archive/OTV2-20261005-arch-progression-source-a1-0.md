# OTV2-20261005-arch-progression-source-a1-0

```yaml
task_id: OTV2-20261005-arch-progression-source-a1-0
title: "ARCH-PROGRESSION-SOURCE-0 Amendment A1: character experience evidence from the public formula (D753)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-progression-source-a1-20261005
issue: 162
pr: 1838
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-05
updated_at: 2026-10-05
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_ARCH_PROGRESSION_SOURCE_PACKETS_2026-10-05.md
  - docs/agents/tasks/archive/OTV2-20261005-arch-progression-source-a1-0.md
public_contracts: []
depends_on: []
blocks: [OTV2-20261005-progression-content-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Answers CP D753. PROGRESSION-CONTENT-1 could not capture the TibiaWiki experience table,
  because every fetch returned 403.
- Amendment A1 (§5 of the decision) adds three things:
  - The experience thresholds are generated from the public closed-form formula
    (§1.1 "Formula evidence"), using exact integer arithmetic. The generator asserts
    `p % 3 == 0`.
  - Evidence uses `OTERYN_GAME_CHARACTER_EXPERIENCE_EVIDENCE/v1`, with `source_kind:
    closed_form_formula`, `last_level` 2000 and `levels_sha256`. The hash is a regression pin.
  - An owner cross-check, `--samples`, compares samples against the official table. It does not
    block. Samples are never committed, and a mismatch is a BLOCKER.
- The declared difference becomes "thresholds from the public formula, not checked against the
  official table". The formula is never runtime authority (checkpoint 2026-09-09 §9.2).
- The §1.3 evidence digest fields and the §2.1 owned_paths comments and acceptance tests are
  updated. Nothing else in the packet changes.
- Codex P2 on e4944156 is fixed. The producer holds the formula once, as structured constants.
  It renders the canonical text from them and refuses an `evidence.json` whose `formula` differs
  byte for byte, so the metadata cannot diverge from the generator. Acceptance tests cover it.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
