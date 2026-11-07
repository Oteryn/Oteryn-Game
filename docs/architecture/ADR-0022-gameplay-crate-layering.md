# ADR-0022: Gameplay crate layering

- Status: **Accepted direction; existing edge violations require follow-up**
- Date: 2026-10-21
- Decision owner: repository owner
- Applies to: the current `crates/gameplay/*` workspace packages
- Refines: ADR-0015's open physical decomposition and ADR-0019's logical modularity
- Preserves: one-process GameNode authority, multichannel WorldId/ChannelId separation, session-generation fencing, accepted durability contracts and `protocol-oteryn`
- Does not authorize: gameplay behavior, SQL, wire-format changes or production activation

## Context

Gameplay now has real crate boundaries. Pure rules need to remain usable without
PostgreSQL, asynchronous runtime orchestration or the actor carrier. Shared actor
state must not accumulate boss-specific mechanics. A checked dependency direction
makes these boundaries reviewable while code moves preserve behavior and tests.

## Decision

Each directory below is a package named `oteryn-gameplay-<directory>`. The layer
table lives in `workspace-boundaries.toml`, rather than in checker source code.

| Layer | Directories | Responsibility |
| --- | --- | --- |
| L0 | `domain` | Shared domain values and contracts |
| L1 | `content` | Authored definitions and content interpretation |
| L2 | `foundation`, `entities` | Kernel primitives and the actor carrier |
| L3 | `characters`, `progression`, `items`, `economy`, `quests`, `housing`, `combat-rules`, `spell-rules`, `encounter-rules`, `social-rules` | Pure gameplay rules |
| L4 | `durability` | PostgreSQL adapters and durable operations |
| L4 | `world`, `combat`, `spell`, `monster-ai`, `npc`, `encounters`, `social` | Runtime orchestration |
| L5 | `dispatch` | Gameplay command dispatch and composition |

The existing supporting directories also have explicit assignments: `weather`,
`world-change`, `source-critical-baseline`, `character-recovery-fence`,
`admission-evidence` and `node-files` are L0 leaves with no gameplay dependencies;
`native-admission-source` is L1 and consumes `admission-evidence`. Together with
the table, these list every current directory under `crates/gameplay/`.

Normal and build dependencies between gameplay packages must go to a strictly
lower layer. The explicit `[gameplay_same_layer_edges]` table preserves the
existing directed L3 rule edges and L4 runtime edges, and permits
`entities -> foundation` at L2.
It is separate from `[edges]`: adding a declared dependency alone cannot bypass
the layer rule. Peer exceptions are not a general permission for peer coupling;
the graph must still be acyclic. L4 peers have no implicit exception. Dependencies
on packages outside `crates/gameplay/` remain subject to the existing exact-edge
and production-closure checks.

L3 rules have no IO, SQL or actor-carrier ownership. The dependency guard rejects
normal/build dependencies on `sqlx`, `tokio`, `entities`, or any L4/L5 gameplay
package. Dev dependencies remain subject to the existing exact `[dev_edges]`
allowlist and do not enter production dependency walks.

`entities` carries shared actor state and depends on domain, content, foundation
and simulation determinism. `foundation` owns kernel primitives. Boss-specific
runtime logic belongs to `encounters`; its pure decisions belong to
`encounter-rules`. Boss identifiers do not belong in the carrier or kernel.
`dispatch` is the only L5 package and composes the lower-layer owners.

## Enforcement

`cargo +1.99.1 run -p oteryn-architecture-check -- workspace .` reads the layer,
peer-edge and gameplay-rule sections alongside the existing membership/edge
policy. It requires one layer assignment for every gameplay member, checks
normal/build dependency direction, and inspects Cargo metadata for prohibited
L3 dependencies, including renamed and target-specific declarations.

Package names and all directories below `crates/gameplay/`, including directories
that are not Cargo members, must not contain `persistence`. The PostgreSQL owner
is named `durability`.

The configurable boss denylist is `welter`, `smelly_cheese`, `bone_overlord`,
`boreth`, `rum_barrel`, `crystal_tentacle`, `haragoth` and `icicle`. The checker
examines Rust file names and function, struct, enum and module declarations
under `entities` and `foundation`, including methods and nested declarations.
It normalizes case and underscores so Rust type names are covered. Comments
and string literals are not declarations.

The boss guard is configured as `error`: boss source ownership findings fail the
workspace gate now that the boss-code move to `encounters/src/bosses/` has landed.
Both error and warning modes retain checker unit tests. Layer, purity and naming
failures are also errors.

## L4 intra-layer order

The current L4 dependencies form a DAG with the order
`durability < world < combat < spell < monster-ai < encounters`.
`npc` sits above `world`; `social` sits above `world` and `combat` and also
consumes `durability`. The exact direct peer edges are:

| Source | L4 dependency targets |
| --- | --- |
| `durability` | None |
| `world` | `durability` |
| `combat` | `durability`, `world` |
| `spell` | `combat`, `durability`, `world` |
| `monster-ai` | `combat`, `durability`, `spell`, `world` |
| `encounters` | `combat`, `durability`, `monster-ai`, `spell`, `world` |
| `npc` | `world` |
| `social` | `combat`, `durability`, `world` |

These are the existing manifest dependencies, deliberately declared in
`[gameplay_same_layer_edges]` using the same mechanism as the L3 rule DAG.
Any new same-layer edge must be added to that table deliberately, must match
the manifest and `[edges]`, and must preserve the acyclic dependency order.

## Remaining gap

`content -> foundation` (L1 to L2) remains a layer violation; a parallel job is
removing it. It has no same-layer exception and still fails the workspace gate.

## Consequences

Rule crates remain independently testable and cannot acquire runtime dependencies
through an ordinary Cargo edit. Actor state and boss behavior have explicit
owners. New gameplay packages require an intentional layer assignment. Existing
graph violations remain visible, so this refactor does not claim a green
architecture gate before the dependency graph satisfies the accepted direction.

Moves retain implementation names, behavior, SQL and wire bytes; tests move with
their owner. This decision adds guards and documents ownership without changing
the accepted authorities or creating a new service boundary.
