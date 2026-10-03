# OTV2-20261003-canary-items-xml-pin

```yaml
task_id: OTV2-20261003-canary-items-xml-pin
title: "Pin Canary items.xml at 04b83b51 as an OTS_HYPOTHESIS_ONLY import (D384 a)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/canary-items-xml-pin
pr: 1671
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
owner: claude-code-session-01QFRdKvFbNsNiCzMyR75FrC (second architect lane)
created_at: 2026-10-03
updated_at: 2026-10-03
owned_paths:
  - imports/canary/items-xml/
  - docs/agents/tasks/archive/OTV2-20261003-canary-items-xml-pin.md
blocks: [TIMED-CONTENT-1]
```

## Outcome

Allocation: D384 a (#1622). `imports/canary/items-xml/` holds a byte-identical copy of Canary
`data/items/items.xml` (17,109 `<item>` elements, SHA-256 `1cf2992c…`) and its GPL-2.0 `LICENSE`
at `04b83b512114bfd888000d6e1433ed8ecaec7c5b`, with a manifest and README. Evidence class
`OTS_HYPOTHESIS_ONLY`. No content, tool or index change. A local `.gitattributes` exempts the two
files from Git's whitespace check so the bytes match the hash.

## Validation

- `sha256sum` matches `manifest.json`; `git diff --check`; `python tools/agents/validate_governance.py`
