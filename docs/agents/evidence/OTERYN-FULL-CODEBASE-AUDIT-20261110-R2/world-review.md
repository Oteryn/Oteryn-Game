# World — completed source review

Immutable input: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **REVIEWED**, not runtime VERIFIED. All **73 tracked package files / 28,966 physical lines** have complete substantive coverage. This comprises **72 manual source reviews and one explicitly mixed semantic catalogue review**. Six earlier movement helper reviews retain their original attribution through identical Git blobs; the lead does not claim a second manual review of those helpers. Exact Git/worktree hashes and full range endpoints passed in `world-root-validation.json`.

## Map construction and mutable owners

The complete bootstrap, map, facts, floor, spell-tile, callback, overlay, pickup, view, door, clock and movement sources and their original test bodies were read. Bootstrap validates the complete bundle and current content before composing map owners, including exact served Item identity, current start ground, signed native floor decoding and bounded floor chains. Daily projection precedes map, facts, spell and floor owners. Ordinary activation and boss placement use separate qualification paths.

Compact map lookup binds sector masks, rank, coordinate order, palette bounds and current scope. Activity regions require a bounded subset of the existing map, a distinct Instance scope and fresh mutable state. Collision combines immutable facts, current callback state and doors only after current scope and bounds validation. Unknown facts retain refusal behavior.

Views preserve native layer and roof semantics and current ground-entry eligibility. View diffs require compatible generation, bundle, epoch, floor and one-step motion; otherwise they request a snapshot. Overlay memory admission precedes mutation, rebuilt identities are globally unique, and pickup retains quantity, attributes and custody restrictions. An unresolved durable outcome does not reveal a hidden item prematurely. Quest callbacks prepare a cloned projection, bound effects and speech, pair portals and bind expiration to exact revisions. World-change and callback namespaces remain separate.

The World clock binds its secret and tokens to one World, anchors UTC to monotonic time and refuses pre-epoch arithmetic. The signed bundle floor decoder was traced; the tentative negative-floor encoding concern was refuted by the complete representation path. Doors validate current pose and revision. Keys require exact identity and action and grant no quest/level bypass. Instance copies restore authored pose; irregular registry triples precede ordinary pairs.

## Movement, floor changes and current occupancy

Movement commits bind exact actor, session, snapshot, scope and content. Source stepping retains native floor conventions, checked arithmetic, bounded probes and a final ground/solid landing check. A solid intermediate source stair probe is intentional and does not authorize a solid final landing. Current Item reads are collected within one physical transaction; binding rechecks `pg_current_xact_id`, current position and recomputed destination. Unknown nonempty captured air refuses, and current stack height requires exact policy revisions.

Bundle floor changes, rope targets and callback relocation bind current context, object placement revision or added identity, current actor position, doors, protection zones and occupancy. Activity paths clip both origin and destination to the active region. Rope landing order and all candidate refusals were traced.

The movement batch and path budgets are finite. Current-map caches are scoped to the physical owner transaction and invalidated on aborted work. Search kernels apply bounded work and path length, retain exact chase radii and charge expanded work on asynchronous cancellation. Occupancy contributes path cost; it does not grant movement authority. Creature speed uses current active conditions and exact content/profile identity. The actual MonsterMap caller supplies current movement proofs; historical comments about unactivated helpers were not accepted as present caller evidence.

Interest selection is canonical by own actor, floor distance, planar distance and identity. Candidate and result ceilings are separately enforced, rejected candidates consume examination budget, and oversized deltas request a resynchronization. The same native floor convention is used through the geometry helpers.

## Object transitions and scheduled reversion

Local object generation includes lock, provenance, World, frame and event-owner identity. Binding reruns the semantic linker and validates exact active scope, declared states, supported capability, unique transitions, destination relations and paired event-owned inverse transitions. Synthetic placement evidence remains explicitly nonpromotable.

Session commands check current authority before duplicate classification. Retained outcomes replay without rolling back current object state. Prepared state, revision and collision publish together only after ingress terminalization or successful scope staging. Stale revision precedes USE selection; absent and ambiguous player transitions refuse. Event-owned transitions cannot be invoked by a player session. Timed scope transitions require the private scheduler capability.

The actual legacy native door caller in `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/objects.rs` collects **all committed player positions** under the same runtime lock as the operation. This supersedes the older World comment describing issuer-only occupancy. `apps/game-server/src/node/serve/channel_boot.rs` constructs the door fence from the committed activation scope and exact content; Activity registration uses its own qualified assignment and runtime pin.

The reversion helper retains canonical nested occurrence identity, bounded key size and depth, exact scope/content/incarnation/state/revision, stable due order and terminal duplicate outcomes. A timed forward stages its record only with a mutating publish. Open CREATE rearm requires exactly one matching Pending record and current scope; TRANSFORM rearm is excluded. Timer-origin execution never schedules another record. Occupied inverse refusal is terminal, and ordinal exhaustion stops the scope. Current Rust references did not establish a live production registration of `ScopeRevertDriver`; these reviewed helper and test contracts are not presented as live integration evidence.

## Complete catalogue method and validation

`map/spell_tiles/immovable_catalogue.rs` is the one mixed review. The lead manually reviewed its control context, actual consumer and generators, interpreted every one of the 14 semantic classes, and exhaustively checked all **54,267 positions**, including **42,107 known IDs** and **12,160 unknown positions**. Known membership agrees with the complete committed appearance membership set. The exact pinned XML supplied **32,938 members**; every explicit collision/movability override and all **45 magic-field identities** were compared with the encoded catalogue. No individual repeated character is represented as manually read. See `world-immovable-review-method.md` and `world-immovable-semantics.json`.

The original `appearances.dat` is unavailable. Re-derivation of its base protobuf solid/pathfinding/movable bits and complete external donor fidelity remain **UNKNOWN**. Supporting canonical files receive no automatic full-review credit. The 287 object and 158 terrain floor-change rows were also compared against the corresponding captured facts, with family separation and uniqueness checked. The original ladder catalogue `--check` passed without source modification.

**29 original unchanged-source Rust tests passed**: 3 door registry, 21 interest and 5 terrain tools. They ran with direct `rustc --test`, importing or compiling the exact source; no implementation substitutes or stubs were used. Their separate JSON records retain commands, hashes and output. All other World test bodies were inspected without execution.

No new confirmed finding is established by this package review. Unconfirmed cross-package candidates remain private with missing maintained producer/reachability evidence stated explicitly. No full Cargo build, PostgreSQL, migration, live map, native session, TLS integration or deployment was executed.
