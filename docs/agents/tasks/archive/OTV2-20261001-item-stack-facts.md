# OTV2-20261001-item-stack-facts

```yaml
task_id: OTV2-20261001-item-stack-facts
mode: REPAIR
status: completed_pending_merge
repository: Oteryn/Oteryn-Game
base_branch: main
authoring_baseline_sha: aba65519088dceadf0e8caa33151c63c36dec5bc
dependency_pr: 1447
branch: codex/item-stack-facts-20261001
owner: owner-directed Codex session
created_at: 2026-10-01
owned_paths:
  - .github/workflows/item-authoring-schema.yml
  - apps/game-server/src/content/{mod.rs,item_stack_false_promotion.rs}
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-schema/item-authoring/*stack_false*
  - docs/agents/evidence/OTV2-20261001-item-stack-false-promotion-v1.*
  - docs/agents/tasks/archive/OTV2-20261001-item-stack-facts.md
  - content/world/**
  - content/items/**
  - content/**/index.json
  - content/content.lock.json
```

Owner-directed audit continuation, with authoring ancestry through draft #1447.
The source-qualified packet contains 2,380 explicit non-stackability facts: 2,345
previously UNKNOWN fields and 35 idempotent known negatives, with 13 map-owner
holds. Physical class, stack class, materialization, destination admission and
stack maximum remain unchanged. The applier validates the complete packet before
mutation and runs after starter admission. All present wiki values must agree;
identity bindings, client cumulative flags and retained evidence states gate
qualification. Packet sources retain snapshot, decoder and map-owner hashes.

Validation: three source qualification tests, two native promotion tests,
deterministic packet rebuild, native materialization, exact 2,345-field semantic
comparison, migration/materialized-tree checks, three repository integration tests,
full library tests (1,284 passed, two ignored), all-target Clippy with warnings denied,
Ruff and formatting. Programme control plane owns review and integration; keep draft.
