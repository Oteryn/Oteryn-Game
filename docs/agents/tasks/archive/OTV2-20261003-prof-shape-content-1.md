# OTV2-20261003-prof-shape-content-1

```yaml
task_id: OTV2-20261003-prof-shape-content-1
title: "PROF-SHAPE-CONTENT-1: ProficiencyShaping schema, validator and operation admission (schema only)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prof-shape-content-1
pr: 1670
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session-01QFRdKvFbNsNiCzMyR75FrC (second architect lane, content packet)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/proficiency-authoring/shaping.schema.json
  - tools/content-schema/proficiency-authoring/shaping_authoring.py
  - tools/content-schema/proficiency-authoring/samples/shaping-candidate.json
  - tools/content-schema/proficiency-authoring/test_proficiency_authoring.py
  - tools/content-schema/proficiency-authoring/README.md
  - docs/agents/tasks/archive/OTV2-20261003-prof-shape-content-1.md
depends_on: ["PROF-CONTENT-1 (#1342, merged)", "PROFICIENCY-1B (#1662, merged)"]
blocks: [PROF-SHAPE-1]
```

## Outcome

Allocation: D376 (#1622), the fallback of D371, option a: schema and validator only.

- `shaping.schema.json`: the `ProficiencyShaping` catalogue of PROFICIENCY-1B §3. Every value is a
  cell, `UNKNOWN` or `KNOWN` with an evidence class from the closed list (official, owner-verified
  TibiaPal or tibiatools.io, English TibiaWiki). OTS sources and gold costs cannot be expressed.
- `shaping_authoring.py`: the semantic rules, and `admitted()`, which implements PROFICIENCY-1B
  §3.3 including the #1662 fixes (`RESHAPE_OFFER` needs every entry's value at the row's rank;
  `RESHAPE_CHOOSE` follows the pending offer).
- `samples/shaping-candidate.json`: one fixture definition with the evidenced `MODIFY` costs
  (250 and 1,000 dust); everything else `UNKNOWN`.
- No `content/` change. The shaping family enters `content/` only when evidence fills a pool;
  TibiaWiki was unreachable from this container (HTTP 402), so no further value was promoted.

## Validation

- `python test_proficiency_authoring.py` (all tests, 4 new); `python proficiency_authoring.py build --check`;
  `python shaping_authoring.py`; `ruff check .` and `ruff format --check .` (pinned 0.16.1)
- `python tools/agents/validate_governance.py`; `python -m unittest discover -s tools/agents/tests`
