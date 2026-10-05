# OTV2-20261005-g4-pins-exclude-1

```yaml
task_id: OTV2-20261005-g4-pins-exclude-1
title: "Exclude content/world/pins from the G4 tracked-legacy comparison"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 1622
base_branch: main
branch: claude/g4-pins-exclude-1-20261005
owned_paths:
  - .github/workflows/g4-canonical-worldproject-package-seed.yml
  - docs/agents/tasks/archive/OTV2-20261005-g4-pins-exclude-1.md
```

## Defect

`G4 canonical WorldProject package seed / qualify` failed with `Only in tracked-legacy: pins` on any PR touching `content/world/**`: #1805 added `content/world/pins/` (pin tables, not WorldProject package content) and the "Compare tracked package" pruning never removed it.

## Change

One step line in "Compare tracked package": `rm -r -- "${tracked:?}/pins"` before the empty-dir sweep. The whole directory is removed so later pin files need no workflow edit.

## Validation

Workflow YAML parses; the pruned copy of `content/world` contains no `pins`. The full `diff` against `world-a` runs in CI (it needs the seed generation steps).
