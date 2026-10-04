# NPC data import into the native server

The existing WorldProject/v2 source remains authoritative. The server's data-only importer reads the complete project through the existing bounded filesystem capture and validates its manifest, lock, schemas and references. It retains the original NPC, Dialogue and NPC-referenced Service records, linked Presentation/Behavior profiles and source/quality metadata in an immutable catalogue. It does not import Lua or replace Oteryn protocol types.

Validate and load the catalogue without a database or world activation:

```sh
cargo run --offline --locked -p oteryn-game-server -- npc-import --project-root content/world --expected-tree-sha256 6391b6115264194d2f2bf2fcd7a9c14d2a251f2d89145755a77f1ac52bbb88ed
```

The command reports imported counts and exits. Source files already persist in `content/world`; this command does not create a database table or modify source data. For a serving process, add both explicit options to its existing launch:

```sh
oteryn-game-server serve --config /path/to/node.toml --npc-data-project /path/to/content/world --npc-data-sha256 6391b6115264194d2f2bf2fcd7a9c14d2a251f2d89145755a77f1ac52bbb88ed
```

The serving process validates the pin before connecting to the database or binding sockets and holds the catalogue until shutdown. Without these options its boot path is unchanged. A malformed pin, wrong digest, unsafe filesystem entry, invalid schema or broken reference rejects the import. Both commands use the same importer. Import consumes the project's validated canonical digest, not arbitrary JSON whitespace. Update the explicit expected digest when intentionally changing source data.

This is a bounded preproduction data reader. Its limits are qualification budgets, not registered production maxima. Provisional and uncertain authoring data remain labelled as such. Runtime eligibility is not inferred from successful data import. Existing native entry-room content and map activation remain unchanged; the data catalogue does not create NPC actors, positions, gameplay capabilities, conversations, quests, trade or travel actions. Later runtime consumers must bind their own accepted content identity and eligibility rules. No full live-node startup or production deployment is qualified by the data-import command.
