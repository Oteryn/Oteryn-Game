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
existing directed L3 rule edges, and permits `entities -> foundation` at L2.
It is separate from `[edges]`: adding a declared dependency alone cannot bypass
the layer rule. L3 peer exceptions are not a general permission for peer coupling;
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

`cargo +1.95.0 run -p oteryn-architecture-check -- workspace .` reads the layer,
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

The boss guard is temporarily configured as `warn`: findings are printed and
do not cause a failing exit status. Once the concurrent boss-code move lands,
set `boss_identifier_mode = "error"`. Both modes have checker unit tests.
Layer, purity and naming failures are errors in either mode.

## Current migration gaps

The graph at authoring time contains `content -> foundation` (L1 to L2) and
L4-to-L4 dependencies, including runtime consumers of `durability` and `world`.
These are real violations of the strict layer direction above. They are reported
by the checker; this ADR does not silently grandfather them or change their code
or edge declarations. Their owning refactor must remove the edges or obtain a
separate accepted architecture amendment before the complete workspace gate can
pass. The warning mode applies only to boss source ownership.

All 20 existing offending dependency declarations remain in place because their
crate code and dependency edges are outside this guard/documentation job:

| Manifest | Dependency targets and declaration lines |
| --- | --- |
| `crates/gameplay/combat/Cargo.toml` | `durability` (line 25), `world` (line 28) |
| `crates/gameplay/content/Cargo.toml` | `foundation` (line 19) |
| `crates/gameplay/encounters/Cargo.toml` | `combat` (line 18), `durability` (line 22), `monster-ai` (line 26), `spell` (line 29), `world` (line 31) |
| `crates/gameplay/monster-ai/Cargo.toml` | `combat` (line 21), `durability` (line 23), `spell` (line 26), `world` (line 27) |
| `crates/gameplay/npc/Cargo.toml` | `world` (line 24) |
| `crates/gameplay/social/Cargo.toml` | `combat` (line 25), `durability` (line 28), `world` (line 31) |
| `crates/gameplay/spell/Cargo.toml` | `combat` (line 25), `durability` (line 28), `world` (line 31) |
| `crates/gameplay/world/Cargo.toml` | `durability` (line 24) |

## Consequences

Rule crates remain independently testable and cannot acquire runtime dependencies
through an ordinary Cargo edit. Actor state and boss behavior have explicit
owners. New gameplay packages require an intentional layer assignment. Existing
graph violations remain visible, so this refactor does not claim a green
architecture gate before the dependency graph satisfies the accepted direction.

Moves retain implementation names, behavior, SQL and wire bytes; tests move with
their owner. This decision adds guards and documents ownership without changing
the accepted authorities or creating a new service boundary.
