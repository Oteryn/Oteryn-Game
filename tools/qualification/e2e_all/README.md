# E2E-ALL-1 (incomplete)

Run from the Game worktree:

```sh
bash tools/qualification/e2e_all/run.sh
```

This reuses `node_boot/run.sh` unchanged, including its prerequisite pinned
Platform checkout, privileged disposable topology, PostgreSQL version and real
world compilation. It also retains node_boot's fixture pass. It is not a way
to run against a production server. See that runner for its host prerequisites.
The absolute cargo wrapper is used for the client tests; node_boot owns the
cargo commands and PostgreSQL environment inside its topology.

Evidence stays in a private `.e2e-all.*` directory inside this worktree. The
quest capture contains raw domain payloads and revision metadata, no admission
grant, database URL or private key. The client replay test must read that run's
captures; missing captures fail. Its ordinary unit mode uses an explicit
synthetic fixture and does not claim node qualification.

The summary has every DONE/PARTIAL mechanic from `SCENARIO_MATRIX.md`, plus
bounded scenario results. A full mechanic requires its own `<row>.full`
evidence. The current suite implements no full mechanic row, so the command
**exits nonzero**, even when its existing journeys and new slices succeed.
Legacy `skipped=` messages are not interpreted as missing content or a pass.
Unimplemented scenarios are FAIL, not MISSING-SERVER. SKIPPED and MISSING-SERVER
require explicit content/source evidence and also leave the suite incomplete.

MAPBOT-1 adds `client_phases/map_journey.rs` to the real-world journey. Its
MapModel uses `world::load_world` (the production boot loader), the existing
world-bundle visitor, production collision/door facts and the floor resolver.
WorldObject/Terrain catalogue classifications supplement the bundle; rope
spots use the served spell parameters and shovel targets use the server's
terrain-tool bindings. It retains house IDs, PZ membership, field content
keys, ladder classification and teleport destinations. It never parses OTBM.

The deterministic seed is 1. Candidates are sorted by kind and native position,
prefer reachable approaches, use a seeded selection capped at five per kind,
then are sorted by position. A kind with no reachable approach retains samples
so its unavailable routes are reported explicitly. The
town window is 160 tiles in x/y and two floors from the phase start. Paths
use A* with the server's cardinal cost of 10; automatic floor/teleport edges
use a zero heuristic to avoid overestimating shortcuts. The current server
admits four directions, so eight-direction routes cannot be sent through its
protocol. `stairs_up`/`stairs_down` denote automatic floor transitions; the
catalogue does not distinguish stair shapes from ramps. `hole_down` is limited
to states identified by the server's shovel transformation bindings.

Every selected sample emits `SEAM_EVIDENCE MAP_CHECK <kind> <x,y,z>
PASS|FAIL|SKIPPED <reason>` (z is the protocol's positive depth). All samples
continue after a failure. A final `return` check restores the starting tile;
`MAP_SUMMARY kind=<kind> pass=<n> fail=<n> skipped=<n>` follows for every
emitted kind. Any failure fails the phase after reporting all results. The
Python report lists the checks and verifies summary counts, duplicates and
coordinate bounds; bounded map passes never qualify a `<row>.full` mechanic.

MAPBOT-1 unfinished prerequisites, reported explicitly instead of inferred:

- The production bundle collision adapter does not expose `block_pathfind`.
  The model uses the server's enterability, including ground, solid items and
  closed doors. Locked/level/quest predicates remain private in DoorBook and
  are not yet represented separately in the model.
- Catalogue ladders whose floor-change fact is UNKNOWN have no registered
  ladder USE handler. They are approached and reported SKIPPED; no synthetic
  ascent is invented. Tool-operated ladder/rope route edges and ascent unit
  tests remain unfinished.
- The bootstrap starter template grants one backpack (`starter_grant.rs`),
  with no rope/shovel grant. Rope spots and shovel targets are approached and
  reported SKIPPED. The journey does not mint items through direct SQL.
- Map/vitals frames carry no PZ flag. Crossing a mapped border checks movement
  but reports SKIPPED for the unavailable flag assertion. VIS-2 actors carry
  appearance references rather than content IDs: spawn checks require a nearby
  visible actor of the correct kind, then report SKIPPED for exact content ID.
- Field samples currently verify movement onto the authored tile; elemental
  damage is outside this slice. Field element classification beyond its content
  key remains unavailable from bundle terrain semantics.
- Node boot is deliberately left to the owner; local tests do not establish
  running-node qualification of this new phase.

Later-slice TODO (MAPBOT-1): combat/spells/runes journeys; NPC dialogue journeys.

Unfinished: full server caller tracing and concrete content routes for the
UNTRACED/UNRESOLVED rows; floor/tool/item/combat/NPC/social/progression/death
scenarios for the complete row requirements; captured client tests beyond the
quest journal; full node execution (deliberately not run during authoring).
