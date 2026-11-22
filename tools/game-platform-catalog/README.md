# Native Game → Platform Game Catalog producer

This directory contains the schema-v1 deterministic producer/validator for `docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md`.

`build.py /tmp/game-platform-catalog.json` invokes the shared Rust CONTENT
loader/view and this validator without retaining a normalized input file.
The build emits the revision 1 public WorldProject payloads described in
[the export contract](../../docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md#public-worldproject-payloads-revision-1).
Support follows the records actually emitted. Completeness follows the admitted
source coverage, including omissions for visibility and unsupported row shapes.
The artifact pins `source_revision` and the UTC commit timestamp to HEAD.
The Rust filesystem adapter runs on Linux, matching the existing CONTENT
capture boundary. Test discovery requires Cargo and the repository's existing
`jsonschema` schema-tool dependency; the Python producer itself remains portable.

It consumes normalized native catalogue input. It does **not** parse legacy-engine sources, Atlas, third-party Wiki data, legacy numeric IDs or mutable runtime state as native authority.

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

## Fixtures

`unsupported-native-input.json` is the negative-semantics fixture.
`supported-native-input.json` samples admitted public records using the exact
payload revision and declares partial coverage for the sampled capabilities.
Build the full artifact with `build.py` against the desired Game commit.

## Production boundary

This tool emits offline/local artifacts for compatibility and inactive-consumer work. It does not publish, deploy or activate production content and does not define an authenticated production transport.
