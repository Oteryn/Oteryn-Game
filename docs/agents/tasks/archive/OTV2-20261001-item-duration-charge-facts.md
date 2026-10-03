# OTV2-20261001-item-duration-charge-facts

```yaml
task_id: OTV2-20261001-item-duration-charge-facts
mode: REPAIR
status: completed_pending_merge
repository: Oteryn/Oteryn-Game
base_branch: main
authoring_baseline_sha: cf0a4537c4f350fbd199130bb766423de8d4b57d
dependency_pr: 1449
branch: codex/item-duration-charge-facts-20261001
owner: owner-directed Codex session
created_at: 2026-10-01
owned_paths:
  - apps/game-server/src/content/item_stats_promotion.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-schema/item-authoring/{lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py}
  - docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json
  - docs/agents/evidence/OTV2-20261001-item-charge-duration-source-completion.md
  - docs/agents/tasks/archive/OTV2-20261001-item-duration-charge-facts.md
  - content/world/**
  - content/items/**
  - content/**/index.json
  - content/content.lock.json
```

The owner requested continued source-backed Item auditing and schema completion.
The retained wiki snapshot exposes a missing stat-lowering path: Wand of Darkness
(i25760) explicitly has 250 charges, and 138 single-ID-page Items declare duration.
Adds closed COUNT_U32 and DURATION_MS fields, exact rational unit conversion and
source/native qualification. Every present observation must agree; exact bindings,
map owners, source-page ID cardinality, blocked states and known-value conflicts
fence admission. All 54 variant holds keep source coordinates. The 57 qualified
existing charge counts remain idempotent; the net change is 139 fields on 138 Items.
Missing consumption, timer, decay and other companion fields remain UNKNOWN.
Identity, physical/stack classes, materialization, destinations and all other
semantics are preserved. Native totals are 126 known charge counts and 138 durations;
these totals include historical facts outside the new source packet qualification.

Validation: source qualification and unit/bounds regressions, deterministic rebuild,
native promotion guards, materialization and exact semantic comparison, repository
inventory/recapture checks, full library tests, Clippy all-targets, migration,
materialized-tree checks, Ruff and formatting. Exact-head CI is still required.
Programme control plane owns review and integration; keep draft.
