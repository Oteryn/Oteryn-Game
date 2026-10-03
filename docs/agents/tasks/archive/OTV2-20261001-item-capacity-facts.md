# OTV2-20261001-item-capacity-facts

```yaml
task_id: OTV2-20261001-item-capacity-facts
mode: REPAIR
status: completed_pending_merge
repository: Oteryn/Oteryn-Game
base_branch: codex/item-equipment-facts-20261001
base_sha: abe461deeef358626f343c2d8dbb7714f4508fee
branch: codex/item-capacity-facts-20261001
owner: owner-directed Codex session
created_at: 2026-10-01
owned_paths:
  - .github/workflows/item-authoring-schema.yml
  - apps/game-server/{src/content/**,examples/materialize_content_world_project_v2.rs}
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-schema/item-authoring/*capacity*
  - docs/agents/evidence/OTV2-20261001-item-capacity-promotion-v1.*
  - content/world/**
  - content/items/**
  - content/**/index.json
  - content/content.lock.json
```

Continuation of the owner-directed audit, dependent on draft #1445. Promotes 17
previously unknown container capacities and corrects Adventurer Backpack i53074
20 to 22 with agreeing wiki pages, exact source binding and affirmative client
container evidence. The packet preserves 34 holds and pins the decoder and map
owner inputs. It prevalidates the entire packet before mutation. Equipment,
identity, admission and all other Item fields remain byte-semantically unchanged.
Validation: four source qualification tests, deterministic packet rebuild,
two native promotion tests, three repository inventory/recapture tests,
migration and materialized-tree checks, Ruff and
formatting. Programme control plane owns review and integration; keep draft.

Exact package inventory pins and promoted-atom counts are refreshed with the
regenerated data. CI exposed stale expectations; the integrity checks stay strict.
