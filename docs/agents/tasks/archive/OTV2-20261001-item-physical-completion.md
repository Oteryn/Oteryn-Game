# OTV2-20261001-item-physical-completion

```yaml
task_id: OTV2-20261001-item-physical-completion
title: Qualify source-supported pickupability, Rune stack facts and weight literals
mode: DATA_ENRICHMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/item-physical-completion-20261001
pr: 1481
base_sha: 5cb90c84013faa4c07fc35e3f660e67db6d691d2
owner: owner-directed Codex session
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/item_physical_promotion.rs
  - apps/game-server/src/content/item_stats_promotion.rs
  - apps/game-server/src/content/item_stack_false_promotion.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-schema/item-authoring/{lower_client_physical_packet.py,test_lower_client_physical_packet.py,lower_wiki_stats_packet.py,test_lower_wiki_stats_packet.py}
  - content/items/definitions/**
  - content/**/index.json
  - content/content.lock.json
  - content/world/{project.json,manifest.json,content.lock.json,definitions/reference.json}
  - docs/agents/evidence/OTV2-20261001-item-physical-*.json
  - docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json
  - docs/agents/evidence/OTV2-20261001-item-stack-false-promotion-v1.json
  - tools/content-schema/item-authoring/{lower_wiki_stack_false_packet.py,test_lower_wiki_stack_false_packet.py}
  - docs/agents/evidence/OTV2-20261001-item-physical-completion.md
  - .github/workflows/item-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20261001-item-physical-completion.md
public_contracts: []
depends_on: [1467]
```

## Outcome

6,756 explicit positive pickupability facts, 39 coupled Rune stackable/max 100 facts and
three exact weight normalizations and one explicit negative stack fact add 6,838 leaves on 6,756 distinct Items. Exact comparison
preserves every other field, class, identity, materializable flag and destination.
The source qualification binds exact 15.30 client bytes, two explicit Crystal binding
revisions/parser bridges and official current/archive Rune-manual quotes.
Missing flags and unbound/map-owned objects remain held. No charge-origin repair is made.

66 already admitted Items receive pickupability facts through existing readers; the admitted
Sudden Death Rune i3155 receives stackable=true/max 100. The other 38 Rune admission states
remain unchanged. This distinction avoids describing data enrichment as having no possible
reader effect.

## Validation and lifecycle

1,293 game-server library tests passed (2 ignored), 3 repository tests and packet atomicity,
conflict/blocked-state, idempotence and identity-class guards pass. Migration validation,
97/97 materialized validation, deterministic source packet rebuilds, Ruff and whitespace
pass. Clippy --all-targets with warnings denied also passes; exact successor-head CI
is retained externally after freeze.

Draft #1481 remains integration-pending; the active control plane owns external review and
protected integration. Exact head is in the publication FREEZE_SHA record, not a self-reference
in this commit. Legacy Rune charges, movable/ownership holds and broader Item completion
remain separate work. This archive closes this bounded authoring batch only.
