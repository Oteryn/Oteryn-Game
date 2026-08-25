# Oteryn Game -> Atlas Creature Gameplay Profiles v1

- Parent contract: `oteryn-game-atlas-export-v1`, semantic revision `1`
- Capability: `creature-gameplay-profiles-v1`
- Profile schema version: `1`
- Producer owner: `Oteryn/Oteryn-Game`
- Consumer: `Oteryn/Oteryn-Atlas`
- Coordination: `ATLAS-CREATURE-GAMEPLAY-PROFILES`
- Migration evidence revision: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`

## 1. Purpose and authority

This capability publishes a deterministic immutable public-safe read model for static NPC and monster gameplay facts used by Atlas creature Gameplay profiles.

Game is the only authority for the facts, public allowlist, identity, completeness state, reason codes, deterministic extraction, bounds, provenance and product digest. Atlas owns bounded consumption, presentation and joins to existing placement records. Platform is not an Atlas runtime data source.

This capability is export/read-model only. It does not authorize or change gameplay runtime behavior.

## 2. Forbidden authority shortcuts

Neither producer nor consumer may make gameplay facts look complete by executing Lua, using `eval`, booting a Game server for introspection, inspecting live GameNode state, parsing comments/file names as facts, scraping wikis, consulting Platform as a repair path, or promoting display-name equality to canonical identity.

Legacy Crystal/Canary Lua/XML is migration/reference evidence only inside the Game-owned static importer boundary. Unsupported dynamic constructs fail the affected subsection closed.

## 3. Identity

A profile uses the same stable creature `entity_id` namespace as `static-creatures-v1`:

- NPC: `npc-entity:<32 lowercase hex>`
- monster: `monster-entity:<32 lowercase hex>`

The shared producer helper is `stable_creature_entity_id(kind, normalized_name)`. Factoring this helper must not change any existing placement entity ID.

Profiles and placements are joined only by `entity_id`. Names are display facts, never join authority.

Referenced items use a Game-owned public export identity when resolved. If only a truthful label is established, the relation retains the label with `item_ref: null` and non-`RESOLVED` item resolution. Atlas must not create an authoritative item link in that case.

## 4. Completeness vocabulary

Every gameplay subsection has exactly one state:

- `COMPLETE`
- `PARTIAL`
- `UNRESOLVED`
- `AMBIGUOUS`
- `UNKNOWN`
- `NOT_APPLICABLE`

An empty collection proves absence only under `COMPLETE`. Unsupported syntax must never be silently omitted while the subsection remains `COMPLETE`.

`reason_codes` are producer-owned bounded strings from the closed set defined by the producer version. Unknown future reason codes may be displayed generically by Atlas but may not be reinterpreted.

## 5. NPC profile

An NPC profile has these subsection families:

```text
shop.state
shop.sells[]
shop.buys[]
services.state
services.values[]
travel.state
travel.destinations[]
```

A trade row may contain only proven public fields:

```json
{
  "item_ref": "item:<stable-export-id> or null",
  "item_name": "Health Potion",
  "item_resolution_state": "RESOLVED|UNRESOLVED|AMBIGUOUS|UNKNOWN",
  "unit_price": 50,
  "currency": "gold",
  "amount": 1
}
```

`unit_price` and `amount` are non-negative integers. `currency` is emitted only when currency semantics are explicit. The first public service taxonomy is closed to `bank`, `blessing`, `trainer`, `shop`, `travel`, `quest`.

A travel destination may expose a label, stable destination reference/position when proven, non-negative integer price/currency, and bounded static conditions only when their semantics are explicit. Dynamic callbacks or computed conditions remain non-complete.

## 6. Monster profile

A monster profile has:

```text
loot.state
loot.entries[]
stats.state
stats.health / experience / armor / defense / speed
resistances.state
resistances.elements[]
resistances.immunities[]
```

A loot row contains:

```json
{
  "item_ref": "item:<stable-export-id> or null",
  "item_name": "Gold Coin",
  "item_resolution_state": "RESOLVED|UNRESOLVED|AMBIGUOUS|UNKNOWN",
  "chance_ppm": 800000,
  "min_count": 1,
  "max_count": 100
}
```

`chance_ppm` is authoritative integer parts-per-million in `[0, 1000000]`. Floating-point probability authority is forbidden. Counts are non-negative integers with `min_count <= max_count`.

Stats are integers where source semantics are explicit. Unsupported/null fields are not coerced to zero. Resistances preserve explicit signed integer percentages and explicit immunity identifiers only.

Spawn placement facts are not duplicated into this product; Atlas obtains Spawns/Locations from its already-authoritative placement product using `entity_id`.

## 7. Static extraction boundary

The v1 extractor recognizes only deliberately supported literal/static configuration shapes covered by producer tests. It may structurally parse literals and known table constructors, but it must never execute source code.

Examples that force a non-complete state for the affected subsection include callbacks, computed table mutation, loops, function results, unknown helper semantics, executable expressions, and unsupported nested loot structures.

Proven static rows may remain visible under `PARTIAL`; unsupported content is represented by a reason code rather than silently dropped as complete.

## 8. Product layout

The immutable product contains one manifest plus deterministic shards and a deduplicated referenced-item table. The manifest exposes semantic equivalents of:

```json
{
  "contract_id": "oteryn-game-atlas-export-v1",
  "semantic_revision": 1,
  "capability": "creature-gameplay-profiles-v1",
  "profile_schema_version": 1,
  "producer_repository_sha": "<40 lowercase hex>",
  "source_evidence": {
    "repository": "blakinio/Otheryn",
    "sha": "e417c5e7c22986bf4acef0495eb47f7b72c97cce"
  },
  "counts": {
    "npc_profiles": 0,
    "monster_profiles": 0,
    "referenced_items": 0
  },
  "shards": [],
  "semantic_digest": "sha256:<64 lowercase hex>"
}
```

Shard selection is derived from stable entity identity, not display name. Every shard descriptor contains a safe relative path, canonical byte count, SHA-256 digest and profile count. Paths containing traversal, absolute roots or platform-specific escape semantics are invalid.

Canonical JSON uses UTF-8, sorted object keys, compact separators and deterministic record ordering. Wall-clock timestamps, machine paths and runner IDs are excluded from canonical digests.

`semantic_digest` binds the canonical manifest semantics excluding the digest field itself and binds every shard descriptor/digest. Any byte/count/digest mismatch is corruption and fails closed.

## 9. Resource bounds

Unbounded parsing or rendering is forbidden. Before merge, the producer freezes numeric hard limits from real evidence census with safety margin for at least:

- manifest bytes;
- shard bytes;
- profiles per shard;
- total/referenced items;
- string UTF-8 bytes and nesting depth;
- shop rows per profile;
- loot rows per profile;
- travel destinations;
- resistance and immunity entries.

The manifest declares the producer limit profile/revision. Exceeding a hard bound produces no publishable product. Atlas enforces independent equal-or-stricter consumer bounds and bounded cache residency.

Numeric v1 limits are part of this contract once recorded in the producer `LIMITS` table and readiness evidence on the merged implementation; changing a limit incompatibly requires explicit review and a compatible capability/schema decision.

## 10. Public safety

Default deny applies. The product contains no credentials, private paths, admin/editor metadata, unreleased content, anti-abuse internals, arbitrary source code, live mutable player/world state or proprietary pixels.

An optional `appearance_ref` on a referenced item is allowed only when an existing publication-safe Game/Atlas asset reference is already authorized. Lack of a proven icon never suppresses a truthful row.

All source labels are data, not HTML. Consumers must render them as text.

## 11. Failure and compatibility

A consumer rejects unknown contract/capability/schema revisions, malformed identities, invalid subsection states, unsafe paths, invalid integers, duplicate identities, broken references, digest mismatch and resource-limit violations.

Gameplay-profile failure is isolated from the base Atlas map, creature rendering/search and Semantic inspector. The previous known-good publication remains independently rollbackable.

Mixed placement/gameplay generations are accepted only when their compatibility/provenance tuple proves they use the same creature identity seam and declared source evidence. Atlas may not heuristically repair a mismatch.

## 12. Producer verification requirements

The producer must retain deterministic tests for at least:

- complete NPC sells/buys and static travel;
- complete-empty shop and non-complete empty distinction;
- partial dynamic NPC constructs;
- complete monster loot and complete-empty loot;
- unsupported/dynamic/nested loot becoming non-complete;
- health, experience, armor, defense and speed;
- resistances/immunities;
- unresolved item identity;
- duplicate identities;
- invalid price/chance/count;
- deterministic ordering/bytes/digests;
- corruption and hard-limit rejection;
- proof that arbitrary source script is never executed.

The exact implementation head also requires repository governance/merge checks and an independent audit appropriate to this public contract/parser boundary before squash merge.