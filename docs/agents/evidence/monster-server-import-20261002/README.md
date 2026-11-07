# Monster server data import — 2026-10-02

This successor imports the prepared #1594 population into the server-owned `content/world` package and regenerates its existing successor tree. The current native filesystem loader accepts **1763 Creature, 104 Encounter and 1609 Document** declarations and resolves every exact encyclopedia reference. The canonical package still has eleven files; Document is an existing declaration family, not a new playable record family.

The importer is pinned to `creature-admission-stage.json` and the prepared index. The original snapshot remains immutable. Actual native validation found one rejected inspection string: Energy Pulse had a leading space. `stage_monster_optional_documents.py` now trims inspection text reproducibly; only that cell differs in the imported stage. Existing protected Item aliases and the established Crystal creature-batch revision mapping remain unchanged.

`qualification.json`, `native-loader-result.json` and `native-loader-proof.json` record actual execution, exact source hashes, counts and scope. `qualify_native_loader.rs` reproduces filesystem capture, canonical rewrite/parse, exact staged subset comparison and two rejection controls; compile against a freshly built `oteryn-game-server` library and its dependencies. The recorded library is source-bound to this repository, replacing the older snapshot’s unbound-cache limitation for this import proof.

Reproduce the package with:

```sh
cargo run --locked -p oteryn-game-server --example materialize_content_world_project_v2 -- --output-root /tmp/new-monster-world
# Atomically install the eleven output files into content/world; preserve adjacent world catalogues.
python tools/content-migration/world_project_v2_to_tree.py
python tools/content-migration/validate_world_project_v2_to_tree.py
python tools/content-migration/test_world_project_v2_to_tree.py
cargo test --locked -p oteryn-game-server --test content_world_project_repository
```

The copied package is data-import evidence. Live server deployment, gameplay execution, effect playback, conditional mechanics, reward pools and respawn were not tested or authorized by this change. Existing source hypotheses, accepted custom mitigation estimates and unresolved owner work remain explicit in #1594/#162. Protected-main reconciliation and deployment stay with the coordinator.
