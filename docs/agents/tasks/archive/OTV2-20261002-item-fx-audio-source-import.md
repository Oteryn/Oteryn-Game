# OTV2-20261002-item-fx-audio-source-import

```yaml
task_id: OTV2-20261002-item-fx-audio-source-import
title: Import prepared Item effect and sound source data into server content
mode: DATA_ENRICHMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/item-fx-audio-raw-import-20261002
pr: 1599
base_sha: 4151e3005daa41ed848533e45b63fadc5ed78fa3
owner: owner-directed Codex session
created_at: 2026-10-02
updated_at: 2026-10-02
execution_policy: continuous_progress
depends_on: [1562]
owned_paths:
  - .github/workflows/item-authoring-schema.yml
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/examples/support/item_fx_audio_raw_import.rs
  - apps/game-server/tests/content_item_fx_audio_raw.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/content.lock.json
  - content/world/content.lock.json
  - content/world/manifest.json
  - content/world/project.json
  - content/world/provenance/imports.json
  - docs/agents/evidence/OTV2-20261002-item-fx-audio-source-import.md
  - docs/agents/tasks/archive/OTV2-20261002-item-fx-audio-source-import.md
  - docs/architecture/OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md
  - docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md
  - imports/ots-source-evidence/item-fx-audio295/import-batch.json
  - imports/ots-source-evidence/item-fx-audio295/producer/raw_fx_audio_reimport.py
  - imports/ots-source-evidence/item-fx-audio295/raw-source-staging.json
  - imports/ots-source-evidence/item-fx-audio295/retained-source-files.json
  - tools/content-schema/item-authoring/test_raw_fx_audio_reimport.py
public_contracts: []
```

Import 295 source-local Item/effect/audio observations and one context record into existing server provenance as 296 opaque Text reimport states. Preserve all previous 13 import batches and 61 states; resulting totals are 14 batches and 357 states. Preserve all Native and canonical Item records, source owners, bindings, relations and World placement. This imports source DATA and retained replay evidence; it does not implement effects, audio events, runtime assets, new typed Native bindings or establish full Item completion.

Independent review and final selected checks PASS. Publication SHA/full frozen readback are retained externally. CI/protected integration and full Item readiness remain pending. Original source checkpoints and published recovery archive are preserved.
