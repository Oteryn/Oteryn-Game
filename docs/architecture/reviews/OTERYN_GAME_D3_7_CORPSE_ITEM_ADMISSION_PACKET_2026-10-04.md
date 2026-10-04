# D3-7 packet: the rat corpse Item admission

- Packet: `D3-7-PACKET-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the packet below.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane order of 2026-10-04, item 4 (a content-lane packet for D3-7, the
  `i00005801` corpse revision, with exact owned paths).
- Builds on:
  - D3 §4.6, §4.7 and §4.8 (D137) and the D3-7 row of its packet table
    (`OTERYN_GAME_D3_CORPSE_CONTAINER_LOOT_WINDOW_DECAY_DECISION_2026-09-29.md`);
  - STARTER-CONTENT-1 (`apps/game-server/src/content/item_admission.rs`, the pinned admission
    packet `docs/agents/evidence/OTV2-20261001-starter-backpack-item-admission.json`);
  - TIMED-CONTENT-1 (`item_timed_promotion.rs`: the temporal mode mapping).
- Amends: nothing. D137 already states what the revision must prove; this packet fixes how and
  where.
- Runtime, migration, protocol and production authority: NONE. The packet needs its own #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order and shared files

- D3-7 adds only content facts. It needs no wire change, capability or migration.
- It regenerates `content/world/**`. Other open PRs regenerate the same tree (PROF-SNOWBALL-REKEY
  #1753, SPAWN-CONTENT-1 #1759, QUEST-CONTENT-2 #1764). D3-7 starts from `main` after #1753 merges,
  because #1753 rewrites the Item shards. When another regenerating PR merges first, the worker
  merges `main` and reruns the tools. Generated files are never merged by hand.
- `apps/game-server/src/combat/death_reward.rs` keeps its `i00005801` comment. DEATH-2b (#1747)
  owns that file, and D3-7 does not touch it.

## 1. Rulings

### 1.1 Which Item

D3 names `oteryn:item.registry.i00005801`. In `content/items/aliases.json` that key is an alias of
`oteryn:item.tibia.i5964`, the dead rat (Canary id 5964). `oteryn:creature.rat`
already binds `corpse_item` to `oteryn:item.tibia.i5964`, and that binding does not change (D137).
The admission therefore names the canonical key `oteryn:item.tibia.i5964`. The record is
identity-only today: `materializable: false`, `stack_class: Unknown`, no semantics.

### 1.2 Shape: a second admission packet

The revision is an Item admission, like the STARTER-CONTENT-1 backpack. The v1 shape cannot carry
it: v1 sets a container-slot equipment pattern and requires an existing capacity. So D3-7 adds a
**v2 admission packet** next to v1:

- the file `docs/agents/evidence/OTV2-20261004-d3-7-corpse-item-admission.json`, schema
  `OTERYN_ITEM_ADMISSION/v2`, one admission;
- `item_admission.rs` pins its bytes and SHA-256 and sets the item count const to 1;
- the function `apply_item_admission_v2` runs right after `apply_item_admission_v1` in
  `examples/materialize_content_world_project_v2.rs`, so after the timed promotion;
- v1, its packet and its digest stay unchanged.

The v2 admission is strict, like v1. It fails closed unless the Item exists and its record is
still identity-only: not materializable, stack class `Unknown`, and the `container`, `equipment`
and `temporal` groups all `Unknown`. The timed packet has no `i5964` row today. If a later timed
row sets `temporal` first, admission fails, and the conflict goes to the control plane.

The admitted shape (D137):

| Field | Value | Source |
| --- | --- | --- |
| `materializable` | `true` | D137 |
| `stack_class` | `NonStackable` | D137 |
| `container.capacity` | `Known(16)` | D137, which equals `GAMEITEM01-CORPSE-CONTAINER-ENTRIES-MAX` (D3 §4.2) |
| `equipment` | `NotApplicable` | D137 (no `container`-slot pattern, D3 §4.5) |
| `temporal.consumption_mode` | `Known(DurableAbsoluteDeadline)` | D137, D3 §4.6 |
| `temporal.duration` | `Known(60000 ms)` | D113, D3 §4.6 (60 s from materialization) |
| `temporal.stop_duration` | `Known(false)` | a durable deadline never pauses (D3 §4.6) |
| `temporal.decay_target` | `NotApplicable` | D3 §4.7 (decay retires the corpse and its contents; there is no successor Item) |

### 1.3 Evidence and divergence

The packet records one evidence source, Canary, pinned from the checked-in import
`imports/canary/items-xml/` (D384, `OTERYN_IMPORT_PINNED_SOURCE/v1`):

```json
"evidence": {
  "canary": {
    "repository": "opentibiabr/canary",
    "path": "data/items/items.xml",
    "revision": "04b83b512114bfd888000d6e1433ed8ecaec7c5b",
    "sha256": "1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2",
    "lines": "16141-16146",
    "attributes": {
      "containersize": "10",
      "decayTo": "3994",
      "duration": "10",
      "fluidsource": "blood"
    },
    "disposition": "DIVERGES_BY_DECISION D113/D137"
  }
}
```

The sha256 is the import's `manifest.json` digest of `items.xml`, and a test recomputes it from
the import file. The values are recorded, not used: Oteryn keeps 16 entries (the accepted
loot-plan ceiling, D3 §4.2) and 60 s with no decay successor (D113, D137).

CrystalServer and TibiaWiki are not evidence for this admission (#1770 P1 4178022290):
- the repository has no pinned CrystalServer `items.xml`;
- the checked-in TibiaWiki snapshots have no record of item 5964 (the `5964` entries in
  `imports/tibiawiki/` are MediaWiki page ids of other pages).

The v2 schema therefore needs only `canary`. The shape comes from the owner decisions, not from a
source agreement, so v1's `ALL_SOURCES_AGREE_ELSE_NO_ADMISSION` policy is replaced in v2 by
`DECISION_SHAPE_WITH_PINNED_EVIDENCE`. The worker adds no other source.

### 1.4 Digests

The regenerated `content/world/definitions/reference.json`, `manifest.json`, `content.lock.json`
and `project.json` pin new digests. `world_project_v2_to_tree.py` then also rewrites
`content/content.lock.json` and every family index that embeds the reference blob SHA. All of
them are owned outputs (§2), and the diff contains no other change. The D137 baseline digests are only evaluation evidence and are
not kept. The task record names the old and the new `reference.json` sha256.

## 2. Packet

```yaml
task_id: OTV2-20261004-d3-7-corpse
decision: D3 D137 (this packet §1)
worker: oteryn-impl-worker (content lane)
review: content review; the D3 §4.8 checklist below is the acceptance list
branch: claude/d3-7-corpse-<date>
base: main after #1753 merges
migration_lease: none
depends_on: [D3 (merged), STARTER-CONTENT-1 (merged), PROF-SNOWBALL-REKEY #1753]
owned_paths:
  - docs/agents/evidence/OTV2-20261004-d3-7-corpse-item-admission.json
  - apps/game-server/src/content/item_admission.rs
  - apps/game-server/examples/materialize_content_world_project_v2.rs   # one call line and its count print
  - apps/game-server/tests/content_world_project_repository.rs           # corpse admission assertions only
  # Every output of the regeneration (#1770 P1 4177992199), written only by the materializer and
  # world_project_v2_to_tree.py, never by hand:
  - content/world/**            # reference.json, manifest.json, content.lock.json, project.json
  - content/items/**            # the i5964 record
  - content/content.lock.json
  - content/manifest.json       # only if the tool rewrites it
  - content/project.json        # only if the tool rewrites it
  # Family indexes that embed the reference.json blob SHA on main c938306d:
  - content/abilities/definitions/index.json
  - content/abilities/effects/index.json
  - content/abilities/formulas/index.json
  - content/behaviors/index.json
  - content/creatures/definitions/index.json
  - content/loot/index.json
  - content/presentations/definitions/index.json
  # content/items/index.json is covered by content/items/**. If the tool rewrites any other
  # file, stop and return a BLOCKER instead of widening the paths.
  - docs/agents/tasks/archive/OTV2-20261004-d3-7-corpse.md
validation:
  - cargo fmt --check; cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - cargo test -p oteryn-game-server item_admission
  - cargo test -p oteryn-game-server --test content_world_project_repository
  - cargo test -p oteryn-game-server --test content_reference_artifact
  - the content-tree regeneration check (the repository CI step for content/world)
  - python3 tools/agents/validate_governance.py; python3 tools/repository/validate_repository_policy.py
  - git diff --check
```

Acceptance:

- `oteryn:item.tibia.i5964` admits exactly the §1.2 shape, and every other Item record is
  byte-identical to `main` after regeneration.
- `apply_item_admission_v2` returns 1. A second run fails closed (the record is no longer
  identity-only). Tests cover a missing Item, a materializable Item, a set `temporal` group and a
  digest mismatch.
- The v1 packet, its digest and its backpack result do not change.
- `oteryn:creature.rat` still binds `corpse_item` to `oteryn:item.tibia.i5964`.
- The new `reference.json`, manifest and lock digests are pinned, and the record names them.
- Every file in the diff is either a hand-written owned path or a tool output. The record lists
  each tool output file, and a rerun of the tools on the frozen head gives no diff.
- The evidence packet holds exactly the §1.3 `canary` block and the v2 policy. A test recomputes
  the `sha256` from `imports/canary/items-xml/items.xml` and checks lines 16141-16146 and their
  four attributes. A packet with any other source is refused.

Not in scope:

- the corpse mint, loot window and decay runtime (D3-2 to D3-6);
- other creatures' corpses (D113 fixes only the rat; a general corpse rule is a later decision);
- the `death_reward.rs` comment (DEATH-2b #1747);
- any wire, protocol or client change.

## 3. Rejected options

- **Extend the v1 packet in place.** That changes the pinned v1 digest and mixes two decisions in
  one packet. A new v2 packet keeps v1 evidence stable.
- **Use the timed packet for the 60 s.** `lower_timed_item_packet.py` lowers TibiaWiki and Canary
  facts, and Canary says 10 s with decay to 3994. D113 is a decision, not a source fact, so it
  belongs in the decision-backed admission packet.
- **Hand-edit `items-*.json` and `reference.json`.** Generated content is written only by the
  tools, and digests must round-trip.
- **Keep Canary capacity 10.** D3 §4.2 fixes 16 to match the accepted loot-plan ceiling. A capacity
  of 10 would reject valid loot plans.

## 4. Decision test

The packet passes when a worker can implement D3-7 from this file alone: one Item, one new pinned
packet, one new admission function, the regenerated tree, with no question about values, sources,
order or owned paths.
