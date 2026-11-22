# Native Game → Platform Game Catalog producer

This directory contains the schema-v1 deterministic producer/validator for `docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md`.

`build.py /tmp/game-platform-catalog.json` is the protected-main build path.
Its eight v1 capabilities remain `unsupported/unknown` while Platform’s inactive
consumer is pending. The artifact contains empty entity, relation and tombstone lists.

Run `build.py --preproduction /tmp/game-platform-catalog-preproduction.json` to
export the populated revision 1 WorldProject for offline use under
[the export contract](../../docs/contracts/OTERYN_GAME_PLATFORM_CATALOG_EXPORT_V1.md#public-worldproject-payloads-revision-1).
Support follows emitted records and completeness follows admitted source coverage.
Both builds pin the Rust projection, WorldProject, Python producer and UTC commit
timestamp to the same Git commit captured from HEAD.
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

The local output paths are symlinks to one generation under the output directory's
`.catalogue` directory. Publication requires a filesystem and account that can
create symlinks. Use a regular output basename that does not end in `.sha256`.
The producer can migrate an existing regular-file pair whose transport digest
matches. It retains published generations so a reader can resolve the JSON path
once and read the digest beside that resolved JSON, even during a subsequent
build. Copy the file contents when moving the pair outside the output directory.

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
Build the populated artifact with `build.py --preproduction` from a checkout of the desired Game commit.

## Production boundary

This tool emits offline/local artifacts for compatibility and inactive-consumer work. It does not publish, deploy or activate production content and does not define an authenticated production transport.
