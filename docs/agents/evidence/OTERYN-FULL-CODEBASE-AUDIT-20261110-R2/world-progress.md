# World — partial source review

Immutable input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **IN PROGRESS**. Exact full-file and selected-range coverage is recorded in the central ledger; unreviewed movement, floor, spell-tile, runtime and reversion owners remain explicit.

The lead manually reviewed the complete manifest, exports, house-scope declarations, observability, World clock, map-view differ, map core, admission-zone tests, daily projection, map view, overlay and pickup owner, all base map facts and their three test modules, quest callbacks, World-change phases, door owner and registry, and the complete 2,667-line map bootstrap including every original test and fixture body. Six previously completed movement helpers retain their original attribution. Reading a test body does not mean executing it.

## Reviewed invariants and actual boundaries

- Bootstrap verifies the complete bundle and current content before composing map owners. It validates the start cell, exact served Item identity, native signed-floor decoding and finite floor chains. Daily projection precedes map, facts, spell and floor owners. Layer admission and epoch binding remain explicit. Ordinary activation and boss placement have separate qualification paths.
- Compact map lookup binds sector masks, rank, sorted coordinates, palette bounds and current scope. Activity regions require a bounded subset of the existing map, a distinct Instance scope and fresh mutable state. Collision queries validate current scope and bounds before using immutable facts, callback state and doors.
- The World clock binds its secret and tokens to one World, anchors UTC once to monotonic time and refuses pre-epoch arithmetic. Qualification-only time retention remains feature-bound. Client view diffs require compatible generation, bundle, epoch, floor and one-step movement; otherwise the producer requests a snapshot. Changes and clears share a bounded budget.
- Views preserve exact native layer semantics, roof/stack selection and current ground-entry eligibility. Overlay admission accounts for memory before mutation, binds scope and map revision, and validates globally unique identities when rebuilding. Pickup retains quantity, attributes and custody restrictions; unresolved durable outcomes do not prematurely reveal a hidden item.
- Map facts preserve ambiguous-identity refusals, House boundaries and independent callback/channel state. Terrain transformations require the exact current state and explicitly qualified target. Unknown collision facts refuse passage outside the closed qualified exceptions.
- Quest callbacks prepare a cloned projection before publication, bound effect and speech sizes, pair portals, and bind expiration to the exact revision. World-change and callback namespaces remain separate. The actual movement caller and the bundle signed-byte decoder were checked; a tentative negative-floor encoding concern was refuted by the complete representation path.
- Doors bind current pose and revision with atomic updates. Direct changes check occupancy; key use requires the exact qualified key and matching action and does not grant quest/level bypass. Instance copies restore the authored pose. Explicit irregular registry triples take precedence over ordinary pairs.

## Validation and limits

Three original unchanged `door_registry.rs` tests passed through a direct `rustc --test` wrapper that imports the exact source file. The source SHA-256 matched the immutable Git blob; no implementation was copied or replaced. See `world-door-registry-unchanged-tests.json`. All other new World test bodies were inspected without execution.

No new confirmed finding is established by this partial pass. The Activity condition-persistence integration candidate remains private and unconfirmed because a maintained qualified room and reachable production entry are still missing. No full Cargo, SQL, live map, native session or deployment test was run. The package is neither REVIEWED in full nor runtime VERIFIED.
