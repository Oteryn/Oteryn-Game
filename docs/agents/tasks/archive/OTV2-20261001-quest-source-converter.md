# OTV2-20261001-quest-source-converter

```yaml
task_id: OTV2-20261001-quest-source-converter
title: Recover statically proven quest source facts
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/quest-data-completion-20261001
branch: codex/quest-converter-enrichment-20261001
pr: null
base_sha: e40cb5267fb9cd8bbf207cbe6f88ee4c08aafe1d
head_sha: null
owner: codex-quest-data-parent
created_at: 2026-10-01T12:45:00Z
updated_at: 2026-10-01T12:45:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/
  - docs/agents/tasks/archive/OTV2-20261001-quest-source-converter.md
public_contracts: []
depends_on: [OTV2-20261001-quest-data-completeness]
blocks: []
external_repositories: []
```

Owner scope and single writer: #162 comment 5931319908. Immutable scalar,
position and table aliases and unique XML item names recover source facts;
dynamic values, mutations, shadowing and branch-selected aliases stay unresolved.
Explicit Boolean condition grouping preserves the source expression.

Pinned Canary/Crystal facts remain OTS_HYPOTHESIS_ONLY. No runtime changes,
Global parity assertion, item subtype conversion or new vocabulary is introduced.
Focused converter regressions, existing 262 schema cases, whole-sample semantic
validation and deterministic diagnostic refresh pass. The frozen SHA, draft PR
and repository CI live on GitHub; #162 retains review routing and MQ.
