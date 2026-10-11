# OTERYN-20261011-decanary-game-1

```yaml
task_id: OTERYN-20261011-decanary-game-1
title: "GAME-DECANARY-1 neutral wording for engine-name references in workflows and docs"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
lane_id: decanary
base_branch: main
branch: claude/decanary-game-1-20261011
pr: 0
base_sha: 3ce431413
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-11
updated_at: 2026-10-11
packet: "owner decision: remove Canary/OpenTibiaBR/Crystal naming; Oteryn-branded wording only"
owned_paths:
  - .github/workflows/charm-authoring-schema.yml
  - .github/workflows/item-authoring-schema.yml
  - .github/workflows/spell-wiki-capture.yml
  - .github/workflows/store-authoring-schema.yml
  - apps/game-server/AGENTS.md
  - tools/game-platform-catalog/README.md
  - tools/qualification/spells/README.md
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-game-1.md
public_contracts: []
external_repositories: []
```

## Outcome

First, behaviour-neutral slice. Workflow comments, step names and shell variable names, and three
README/AGENTS lines, now say "reference-engine" or "legacy-engine". No tool flag, pin key, path,
digest, golden or fixture changed.

## Left in place, with reason (follow-up PRs)

Inventory on `origin/main`: about 5000 files mention the names; 1101 file paths contain `canary`.

- Live couplings that need tool and data renames together: `--canary-source`/`--canary` flags and
  `ENGINES["canary"]` in `tools/content-schema` (the pins feed `steps.pins.outputs.canary_*` in
  `spell-wiki-capture.yml`), the workflow path triggers and `samples/spell-census-canary-*.json`
  comparison, `imports/canary/`, `imports/crystalserver/`.
- Pinned or digest-bearing data: `content/**` provenance, `imports/**`, `rulesets/**`,
  `apps/game-server/src/*_fixture.json`/`*_test_data.json`, `docs/reference/spells/**`
  (2074 files, 1101 paths named canary), `docs/agents/evidence/**`.
- Guards that must keep the word: `forbidden_package_fragments` in `workspace-boundaries.toml`,
  the `forbidden_fragments` entry in `tools/architecture-check/src/lib.rs`, the `protocol-canary`
  reintroduction gate in merge-gate/merge-group-gate/rust.yml and `validate_governance.py`.
- Rust identifiers and comments in `apps/game-server/src` and `tests` (about 110 files):
  need a compile check.
- Vendored `sqlx-postgres` (third-party, unrelated meaning).
- Not touched: `content/world/pins/oteryn.json`, `classify_pr_test_lanes.py`, toolchain pins.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (59 of 60; 1 error is a missing experiments/ file in the sparse checkout)
- workflow YAML parse (PyYAML): pass

## Review

No paid review requested.
