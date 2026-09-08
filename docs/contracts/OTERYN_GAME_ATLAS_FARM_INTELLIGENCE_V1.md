# Oteryn Game → Atlas Farm Intelligence v1

- Contract ID: `oteryn-game-atlas-farm-intelligence-v1`
- Schema version: `1`
- Producer owner: `Oteryn/Oteryn-Game`
- Consumer: `Oteryn/Oteryn-Atlas`
- Producer revision: `farm-intelligence-v1`

## 1. Authority and scope

This contract is a deterministic, immutable, public-safe Game availability read
model for facts Atlas may use downstream. Game owns identity, loot/task
semantics, capability states, provenance, bounds, and publication. Atlas may
join and derive presentation or estimates, but is not Game truth authority.
Game does not publish farm time, KPH, route/spatial clustering, or target
hitting-time answers.

The producer may consume only an admitted, provenance-qualified Game-owned
publication whose authenticity is established outside caller-authored fields.
It never executes or parses legacy scripts, XML, OTBM, websites, browser state,
live servers, or private player data. Reference repositories are migration
evidence, not current runtime authority. Default publication is deny.

No authenticated admitted publication is available in this checkout. V1
therefore exposes no production input adapter and emits only the exact canonical
`BLOCKED_NO_ADMITTED_SOURCE` product. A repository label, syntactically valid
revision/digest/generation, or caller-declared capability is not source proof.

## 2. Source qualification at v1 admission

| Family | Classification | Capability | Proven boundary / blocker |
|---|---|---|---|
| Item identity | PROVEN schema; source UNKNOWN | `UNSUPPORTED` | Stable `oteryn:item.*` semantics exist, but no authenticated admitted publication is present. Unresolved labels stay unresolved and names never create identity. |
| Creature identity | PROVEN schema; source UNKNOWN | `UNSUPPORTED` | `monster-entity:<32 hex>` is the existing join seam, but no authenticated admitted corpus is present. |
| Loot probability/context | PROVEN representation; source UNKNOWN | `UNSUPPORTED` | Existing contract documentation describes integer `chance_ppm`; the authoritative product and exact live ruleset/profile/modifier and per-kill roll proof are unavailable. |
| Loot quantity | PROVEN bounds representation; source UNKNOWN | `UNSUPPORTED` | Unequal `min_count`/`max_count` would remain `BOUNDED_UNKNOWN`; equality cannot become `FIXED` without roll proof. No rows are admitted. |
| Placement/farm supply | UNKNOWN | `UNSUPPORTED` | Current evidence does not prove stable spawn groups, alternative/conditional activation, and complete capacity together. |
| Tasks/grouping/credit | UNKNOWN | `UNSUPPORTED` | No accepted authoritative task catalogue proving stable tasks, grouped requirements, or credit semantics is available. |
| Weekly classification | UNKNOWN | `UNSUPPORTED` | No accepted explicit weekly authority is available. |
| Respawn | UNKNOWN | `UNSUPPORTED` | Existing static `spawn_time_seconds` provenance does not prove live/current cadence or modifiers. |
| Provenance/completeness | UNKNOWN | all `UNSUPPORTED` | Caller-authored repository/revision/digest/generation/capability fields are never accepted as proof. |
| Resource bounds | UNKNOWN | publication blocked | The exact admitted corpus is unavailable to census, so v1 freezes no numeric public/production ceiling. Synthetic fixture limits are separate and test-only. |

Switching a family to `PARTIAL` or `COMPLETE`, or supplying any fact row,
requires a reviewed revision and an authenticated admitted Game publication.
Missing proof is `UNSUPPORTED`/`UNKNOWN`, never empty success.

## 3. Canonical blocked product

The product is one UTF-8 JSON file with sorted keys, compact separators, one
trailing LF, no floats, and these semantics:

- exact contract/schema/producer revisions;
- source state `UNAVAILABLE` and closed blocker reason
  `AUTHORITATIVE_NORMALIZED_GAME_PUBLICATION_UNAVAILABLE`;
- independent `UNSUPPORTED` capabilities for every family with that reason;
- empty creature, item, loot, and task arrays which are not absence claims;
- publication state `BLOCKED_NO_ADMITTED_SOURCE` and a semantic digest.

`semantic_digest` is SHA-256 over canonical product semantics with that field
omitted. The validator regenerates the internally defined product and rejects
any byte-level semantic disagreement; it does not validate or bless caller
facts.

## 4. Reserved identity and relation semantics

A future reviewed source-bearing revision must use creature IDs
`monster-entity:<32 lowercase hex>` and stable `oteryn:item.<key>` item IDs.
Display names remain display facts only. Unresolved item rows retain
`item_id: null` and a non-`RESOLVED` state. Duplicate identities/relations,
dangling references, conflicting resolution, and mixed source generations must
fail closed. None of these prospective shapes authorizes a v1 fact row.

## 5. Reserved probability and quantity semantics

A future probability is an integer rational with `0 <= numerator <=
denominator`, a positive denominator, and exact source-proven context. The
blocked production product admits no probability context or relation. Synthetic
semantic tests use only `TEST_ONLY_STATIC_NOT_AUTHORITY`; adding production
`EXACT_RULESET_PROFILE_BASE`, live/current, or other context requires a reviewed
revision and authenticated source proof.

Future quantity classification remains one of `FIXED`, `EXACT_PMF`,
`BOUNDED_UNKNOWN`, or `UNSUPPORTED`. Fixed and exact-PMF classifications require
proof of the underlying roll process; bounds do not authorize a distribution or
expected-kill calculation. A zero-yield PMF point must be preserved if an exact
PMF is ever proven. Game publishes no Atlas calculation.

## 6. Capability semantics

Capability states are `COMPLETE`, `PARTIAL`, `UNSUPPORTED`, or `UNKNOWN` with
closed reason codes; every non-complete state requires a reason. An empty array
means absence only under `COMPLETE`. V1's blocked arrays are all
`UNSUPPORTED`, and a sibling family never upgrades another family implicitly.

## 7. Resource-bound gate and rejection

V1 publishes no numeric production ceiling: an exact admitted-source census is
not available, and conservative guesses are not contract authority. Production
generation takes no input. The validator reads only a file whose byte length
exactly equals the internally generated blocked product before comparing its
exact bytes. It rejects every enriched or caller-authored product, unsafe output
paths, corrupt digests, and non-canonical bytes.

Generated unit-test fixtures have small, explicitly named test-only limits and
use contract ID `oteryn-game-atlas-farm-intelligence-test-fixture-v1`, fixed
marker `SYNTHETIC_TEST_FIXTURE_NOT_SOURCE_AUTHORITY`, and publication state
`TEST_ONLY_NON_PUBLISHABLE`. The production validator always rejects them.
Their probability and quantity cases test prospective validation only; they
make no source, corpus, or production capability claim.

Canonical output excludes timestamps, machine paths, runner IDs, secrets,
source code, and live/private state.

## 8. Compatibility and activation

Schema changes require reviewed compatibility. Consumers reject unknown versions
and retain their previous known-good product. The blocked product must keep
Atlas farm-intelligence facts unavailable. Runtime gameplay E2E is not
applicable because this static availability export changes no gameplay behavior.
Real producer/validator execution against every future admitted authenticated
source generation remains required before any family becomes source-bearing.
