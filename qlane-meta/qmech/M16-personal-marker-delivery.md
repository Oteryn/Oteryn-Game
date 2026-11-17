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
rendering/replacement test passed. The PostgreSQL command-dispatch proof is described below.

The full protocol suite exposed the existing target-move test rejecting direction
5 although the unchanged qmech step codec accepts NorthEast=5. It is outside M16
and remains unchanged. Local node-boot is excluded by the owner's instruction;
Windows execution remains a CI check.

## PostgreSQL qualification

`cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib m16_sterling -- --nocapture`
executed against PostgreSQL 18.6: **1 passed**, 83.12 seconds, with
`M16_STERLING_DISPATCH_PG_EXECUTED marker_delta_reconnect_replay`.

The mounted case drives real command 7 through `serve_admitted`. It proves an
empty initial marker projection, Sterling's guarded confirmation, mission=4 and
treasure-marked=1, exactly one marker/source/receipt transaction, a domain-16
marker delta while the log is closed, a reconnect snapshot, exact-command
`DuplicateReplay`, and no second marker/receipt for a fresh repeated confirmation.
Its prerequisite setup uses the existing quest-runtime fixture transition to
establish a valid Character revision/receipt chain, then seeds later Targuna
prerequisites with fixture privileges. The marker confirmation uses the production
callback, runtime role and all original database guards.

The same case returns **Ok(())** with no PostgreSQL admin URL (1 passed, 0.00s).
The twelve existing dispatch quest-log regressions also pass. The full protocol
suite reports **724 passed, 1 unchanged target-move failure**; none is ignored or
removed. This completes the bounded M16 delivery mechanic, not the other Targuna
stages. No migration or pin regeneration is needed.

## Capability 451 integration

The centrally integrated personal-map query uses capability **451**, command 733;
diagonal movement retains **449**. The earlier domain-16 delivery remains compatible.
The dispatch proof now negotiates 451 and reads the same committed Sterling fact
through the real command-733 router, in addition to its original delta, reconnect,
replay and transaction-coupling assertions. The imported query-owner test uses the
capability constant rather than the stale literal 450.

When both projections arrive, the canonical personal-map layer owns the icon and
the legacy icon is suppressed for the same location. A new admission into the
same World clears both Character-specific layers. The native regression preserves
its original legacy-delivery assertions and additionally checks dual-delivery
rendering and independent snapshot replacement; the privacy test checks both layers.

### Integration validation and blockers

The refreshed World pin is `00bb6b2b3305d38ac347a1ba06b8b555f7d0d5a1915dd474dc4c1e5e8513f5f8`.
`pin-refresh` generated the World pin, client placement-index constant and
preproduction manifest; `pin-check` passed. No derived digest was hand-edited.
Fresh all-feature cargo check passed for dispatch, protocol, client runtime, UI,
assets and native client. Native dual-delivery (1), marker privacy/validation (2),
client-assets World (6), personal-map codec/UI (4), and all protocol tests (751)
passed. The protocol registration test first exposed the out-of-block personal-map
module declaration; moving it into sorted position made all 751 tests pass.

Full regeneration is blocked by the imported floor-assembly inputs:
`ValueError: FLOOR_ASSEMBLY_PROFILE_CONFLICT`. The relevant generator and content
inputs matched the integrated qmech tree before World pin refresh.

The PostgreSQL follow-up is **not qualified on the integrated head**. Both real
caller cases stop during migration 2020, before gameplay:
`quest reward quantity variable requires exact guard anchor: actual BIGINT;`.
The earlier 83.12-second PostgreSQL success above belongs to the pre-integration
slice. The historical migration is unchanged. A central migration-chain repair
and a rerun of the two marker cases remain necessary; neither test is ignored,
removed or weakened to bypass this failure.

Final integrated `personal_map` run: the two pure dispatch tests passed; both
PostgreSQL cases failed in migration 2020 with the anchor error quoted above
(2 passed, 2 failed). With `OTERYN_TEST_POSTGRES_ADMIN_URL` absent, all four
focused cases passed (0 ignored). The capability-closure test now uses the 451
constant and preserves both complete/incomplete dependency assertions. Final
all-feature/all-target dispatch clippy passed. These results do not supersede
the PostgreSQL blocker or claim that the integrated caller path has executed.
