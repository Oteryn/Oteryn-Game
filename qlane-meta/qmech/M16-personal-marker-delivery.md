# M16 — personal NPC quest marker delivery

Scope: Sterling's Targuna treasure confirmation uses the existing reward callback
and migration 2070 writer. This change delivers its committed personal marker in
capability 16's current-World quest-log snapshot/delta and draws a Cross in the
native minimap, including the label in its marker list. It does not qualify the
other Targuna stages.

## Parity

| Rule | Reference | Implementation |
|---|---|---|
| Confirming the map hand-in requires Morla's report and advances mission to 4 / treasure-marked to 1 | Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/npc/sterling.lua:142-157`, OTS_HYPOTHESIS_ONLY | Existing `m16-targuna-marker.json` callback, fenced Quest reward writer and migration 2070 |
| Personal Cross at (31948,31924,7), label `Treasure` | Same donor, lines 62 and 155 | Current-World fenced read -> domain 16 -> native floor -7 minimap layer |
| Delivery survives reconnect and replay creates no second committed marker | Oteryn receipt/session contract; donor supplies the player-scoped display effect | Domain replacement snapshots/deltas, existing immutable marker/source/receipt coupling |

The wire extension is a bounded repeated field 4 on `QuestLogV1`. It carries only
coordinates/depth and label; all markers are Cross icons. No track keys or mutation
commands are exposed. The maximum domain grows from 180,268 to exactly 249,900
bytes for ten full tracked lines, a full view and 256 maximum-size markers, below
existing FND-02 bounds. The existing boundary test now exercises that full domain;
no previous limit assertion is removed.

Server markers use a separate connection-local client layer. They are not saved
in the user's World-wide exploration/marker file, so another character cannot
inherit them. A replacement snapshot replaces this layer, including an empty set.
The native marker editor displays their labels and coordinates without changing
the committed server facts.

No migration, capability or command number is allocated. The number helper was
checked against origin/main and origin/claude/happy-davinci-ijz3wy-qmech; the
existing migration 2070 is reused. No content inputs or generated pins change.

## Validation

First delivery slice: `cargo fmt --all --check`; clippy for protocol, dispatch,
client-runtime and native client with all targets/features and `-D warnings`;
13 quest-log codec tests; 5 client-runtime quest tests; and the native marker
rendering/replacement test passed. The PostgreSQL command-dispatch proof is being
qualified separately and M16 remains PARTIAL until that proof is committed.

The full protocol suite exposed the existing target-move test rejecting direction
5 although the unchanged qmech step codec accepts NorthEast=5. It is outside M16
and remains unchanged. Local node-boot is excluded by the owner's instruction;
Windows execution remains a CI check.
