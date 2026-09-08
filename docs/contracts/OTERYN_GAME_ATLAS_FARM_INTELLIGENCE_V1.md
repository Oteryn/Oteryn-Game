# Oteryn Game → Atlas Farm Intelligence v1

- Contract ID: `oteryn-game-atlas-farm-intelligence-v1`
- Schema version: `1`
- Producer owner: `Oteryn/Oteryn-Game`
- Consumer: `Oteryn/Oteryn-Atlas`
- Producer revision: `farm-intelligence-v1`

## 1. Authority and scope

This contract is a deterministic, immutable, public-safe Game read model for
facts that Atlas may use downstream. Game owns identity, loot/task semantics,
capability states, provenance, bounds, and publication. Atlas may join and
derive presentation or estimates, but is not Game truth authority. Game does
not publish farm time, KPH, route/spatial clustering, or a target hitting-time
answer in this product.

The producer consumes only an admitted normalized Game read model. It neither
executes nor parses legacy scripts, XML, OTBM, websites, browser state, live
servers, or private player data. Reference repositories are migration evidence,
not current runtime authority. Default publication is deny.

## 2. Source qualification at v1 admission

| Family | Classification | Capability | Proven boundary / blocker |
|---|---|---|---|
| Item identity | PROVEN + UNKNOWN per row | `PARTIAL` | A resolved `oteryn:item.*` is stable; existing gameplay rows without such a mapping remain unresolved and names never create identity. |
| Creature identity | PROVEN | `PARTIAL` | `monster-entity:<32 hex>` is the existing static-creature/gameplay join seam; the complete authoritative corpus is not available in this checkout. |
| Loot probability/context | PROVEN + UNKNOWN | `PARTIAL` | Existing gameplay rows provide integer `chance_ppm`; exact live ruleset/profile/modifier applicability and per-kill roll process are not proven. v1 represents a reduced rational plus an explicit static/non-live context. |
| Loot quantity | PROVEN bounds; UNKNOWN distribution | `PARTIAL` | `min_count`/`max_count` are authoritative bounds. Unequal bounds are `BOUNDED_UNKNOWN`, never a uniform distribution or exact PMF. Equal bounds are not automatically fixed without roll proof. |
| Placement/farm supply | UNKNOWN | `UNSUPPORTED` | Current publications do not prove stable spawn groups, alternative/conditional activation, and complete capacity semantics together. Equal geometry is not group identity. |
| Tasks/grouping/credit | UNKNOWN | `UNSUPPORTED` | No accepted Game-owned authoritative task catalogue proving stable tasks, grouped requirements, or credit semantics was found. Empty tasks do not prove absence. |
| Weekly classification | UNKNOWN | `UNSUPPORTED` | No accepted explicit weekly authority was found; ordinary/custom kill targets are not reclassified. |
| Respawn | UNKNOWN | `UNSUPPORTED` | Existing static `spawn_time_seconds` provenance does not prove live/current cadence or modifiers. |
| Provenance/completeness | PROVEN | per-family | A source repository, exact 40-hex revision, semantic digest, and one generation bind every emitted record. Mixed generations fail closed. |
| Resource bounds | DERIVED | enforced | The normalized static product is capped conservatively; limits are not production corpus-size claims. |

Switching a family to `COMPLETE` or supplying `EXACT_PMF` requires reviewed,
revisioned Game source proof. Missing proof is `PARTIAL`, `UNSUPPORTED`, or
`UNKNOWN`, never empty success.

## 3. Canonical product

The product is one UTF-8 JSON file with sorted keys, compact separators, a
single trailing LF, deterministic record order, and no floats. It contains:

- exact contract/schema/producer revisions;
- source repository, 40-hex revision, `sha256:` semantic digest, and generation;
- independent capabilities for item identity, creature identity, loot
  probability, loot quantity, placement supply, tasks, weekly, and respawn;
- creature and resolved-item identity tables;
- loot relations, and an explicitly empty `tasks` array while tasks are
  unsupported;
- the exact limit profile and semantic digest.

`semantic_digest` is SHA-256 over canonical product semantics with that field
omitted. A validator regenerates canonical semantics and rejects any byte-level
semantic disagreement.

## 4. Identity and relation rules

Creature IDs are `monster-entity:<32 lowercase hex>`. Item IDs are stable
`oteryn:item.<key>` values. Display names are bounded display facts only.
Unresolved item rows retain `item_id: null`, their display name, and a
non-`RESOLVED` resolution state. Relations must reference a declared creature
and, when non-null, a declared item. Duplicate identities/relations, dangling
references, conflicting resolution, or mixed source generation are invalid.

## 5. Probability and quantity

Probability is an integer rational `{numerator, denominator, context}` with
`0 <= numerator <= denominator`, positive bounded denominator, and an explicit
ruleset/profile scope string. Floating-point probability authority is forbidden.
A static migration profile must not be presented as live/current probability.
V1 admits only `STATIC_MIGRATION_PROFILE_NOT_LIVE_CURRENT` and
`EXACT_RULESET_PROFILE_BASE`; adding a live/current context requires a reviewed
schema/producer revision and corresponding Game source proof.

Quantity is exactly one of:

- `FIXED` with one non-negative count, only with proven roll semantics;
- `EXACT_PMF` with unique counts and integer rational masses using one
  denominator and summing exactly to one (zero-yield mass is preserved);
- `BOUNDED_UNKNOWN` with ordered min/max and no inferred internal distribution;
- `UNSUPPORTED` with no quantity fields.

Atlas may perform bounded estimates only where the exact process needed by its
algorithm is proven. Bounds alone do not authorize expected-kill/hitting-time
semantics, and Game publishes no such calculation.

## 6. Capability/completeness semantics

Each family is `COMPLETE`, `PARTIAL`, `UNSUPPORTED`, or `UNKNOWN` with sorted,
deduplicated reason codes; every non-complete state requires a reason. An empty
array means absence only under `COMPLETE`. A supported sibling family never
upgrades another family implicitly.

## 7. Limits and rejection

The v1 producer caps input/product bytes at 4 MiB, combined records at 16,384,
strings at 512 UTF-8 bytes, PMFs at 128 points, counts at 1,000,000, and rational
denominators at 1,000,000,000. It rejects malformed/non-UTF-8/oversized JSON,
unknown keys or revisions, booleans masquerading as integers, unsafe output
paths, corrupt digests, invalid ranges/PMFs, duplicates, dangling references,
and provenance/generation mismatches. Canonical output excludes timestamps,
machine paths, runner IDs, secrets, source code, and live/private state.

## 8. Compatibility and activation

Schema changes require a reviewed compatible revision. Consumers reject unknown
versions and retain their previous known-good product. A product is not a live
source and does not activate Atlas functionality by itself. Runtime gameplay E2E
is not applicable because this delivery is a static export/read-model and does
not change gameplay behavior; real producer and validator execution remains
required for every admitted normalized source generation.
