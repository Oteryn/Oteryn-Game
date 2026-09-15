# Native Game → Platform Game Catalog producer

This directory contains the schema-v1 deterministic producer/validator for `docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md`.

It consumes normalized native catalogue input. It does **not** parse Canary, CrystalServer, Atlas, third-party Wiki data, legacy numeric IDs or mutable runtime state as native authority.

## Produce

```powershell
python tools/game-platform-catalog/producer.py produce `
  tools/game-platform-catalog/fixtures/unsupported-native-input.json `
  game-platform-catalog.json
```

The producer writes:

- canonical `game-platform-catalog.json`;
- `game-platform-catalog.json.sha256` over the exact artifact bytes.

## Verify

```powershell
python tools/game-platform-catalog/producer.py verify game-platform-catalog.json
```

Verification recomputes the semantic digest and validates the complete v1 envelope. The `.sha256` sidecar remains an independent transport-byte integrity value.
## Tests

```powershell
python -m unittest discover -s tools/game-platform-catalog -p "test_*.py" -v
```

The tests cover deterministic ordering, semantic and artifact digests, stable native identity, manifest alignment, capability taxonomy, duplicate/dangling/contradictory records, tombstone completeness, strict JSON, hard bounds and provenance integrity.

## Current content status

The committed unsupported fixture is intentionally non-activatable. At its pinned Game revision, this task does not claim a complete native item, creature, loot, NPC/shop, spell, quest or achievement inventory.

A later Game-owned adapter may populate supported capabilities only from canonical native content evidence. Changing a capability from `unsupported/unknown` to `supported/complete` requires task-specific proof; this producer never infers that transition from an empty or legacy dataset.

## Production boundary

This tool emits offline/local artifacts for compatibility and inactive-consumer work. It does not publish, deploy or activate production content and does not define an authenticated production transport.