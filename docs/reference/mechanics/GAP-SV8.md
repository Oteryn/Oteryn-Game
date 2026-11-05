# GAP-SV8 — movement spells, planting, Soulpit and Animus

Base: `origin/main` / `c8b3890e601e6073713dcac2ea9ee271f4a257e8`.
W17 was removed by the owner's follow-up; this package contains no W17 changes.
No migration, capability, wire command or protocol message number is allocated.

## Reverification, in package order

| Mechanic | Evidence on the base |
| --- | --- |
| M11a | `crates/gameplay/world/src/map/spell_tiles.rs:111-113` sets all immovable flags to true; `crates/gameplay/spell/src/spell/world_execution.rs:528` refuses the Levitate destination. |
| M11b | The same bundle flags prevent rope destinations; `crates/gameplay/spell/src/spell/world_execution.rs:516-518` consumes them. Also, ordinary speech dispatch in `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/spells.rs:270` only selects the parameter owner for `takes_parameter` spells, whereas donor Magic Rope has no parameter. |
| W19b | `content/quests/world-callbacks.json:334` and `:360` already define the Dryad Garden entrance/exit, executed by `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:145`. The named garden donor is a teleporter, not a planting action. Seed planting has different USE and item-drop callbacks. |
| W07 | `crates/gameplay/dispatch/src/gameplay_transport/soulpit/handoff.rs:38` defines an adapter, but there is no production implementation. `soulpit/entry.rs:232-234` refuses entry without it. `native_activity.rs:768-777` commits generic/boss entry independently of the Soulpit transaction writer. |
| W08 | `crates/gameplay/durability/src/durability/soulpit/progress.rs:201` already grants survivor mastery transactionally; production acquisition still depends on W07. Existing native mastery presentation is documented in `docs/contracts/protocol-oteryn/v1/animus-mastery-v1.md:31`. |

## Movement source parity

Reference: Canary `main` lineage, exact cut
`47dfd51f45280a59a1d3e50ba7edd573d7234446` (confirmed as the merge base
of that cut and `main` through the GitHub compare API). Classification:
`OTS_HYPOTHESIS_ONLY`. These are donor loader facts, not independent official proof.

| Rule | Donor file:line at that cut | Consumer |
| --- | --- | --- |
| Solid, pathfinding and movable appearance properties | `src/items/items.cpp:236-247` | Generated catalogue, indexed by the palette's source appearance identity. |
| XML movability/solidity overrides and magic-field classification | `src/items/functions/item/item_parse.cpp:110-129`, `:213-214`, `:691-711`, `:809-813` | Existing `build_source_world.py` loader plus the `blocking` override in `build_bundle_immovable.py`. |
| Immovable solid/path/nonfield-path properties | `src/items/item.cpp:1616-1621` | `spell_tiles/immovable.rs::flags`. |
| Unique ID or action 100 makes an item immovable | `src/items/item.cpp:1631-1641`, `src/utils/const.hpp:73` | Applied separately for each top-level bundle placement. |
| Tile flags aggregate item properties | `src/items/tile.cpp:1844-1889` | Bundle boot ORs the three properties before interning each tile shape. |
| Levitate words and up/down parameter | `data/scripts/spells/support/levitate.lua:1-18`, `:35`, `:44` | Native movement controls submit ordinary local SAY. |
| Magic Rope words and no parameter | `data/scripts/spells/support/magic_rope.lua:25-36` | Native SAY and parameterless spell casts share the existing vertical relocation owner. |

The production bundle path additionally exposed a frame mismatch: bundle runtime
floors are legacy `+z` (`apps/game-server/src/node/serve.rs:2374-2386`), whereas
captured spell rooms use native `-z`. Relocation now interprets depth and upstairs
in that room's frame. Donor floor rules remain `levitate.lua:4-12` and
`magic_rope.lua:13`; no movement formula is changed.
Read-only durable item probes also admit proven absent cells in the current
bundle's bounded plane, preserving all generation checks. Item placement still
requires an existing tile through `SpellGroundTarget::from_native_owner`.
Coordinate bounds follow Canary `src/game/movement/position.hpp:69-71` and the
accepted bundle floor contract, `docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md:68`.

Unknown catalogue identities remain blocked. Existing bundle unknown-tile,
pickupable-item, door, collision, current-item, session and cost guards remain
in force. No new gameplay difference is accepted by this package.

The generated catalogue contains 42,107 source identities. Reproduce/check it with:

```sh
python3 tools/content-schema/native-gameplay/build_bundle_immovable.py --source-root <reference-root> --check
```

The generator verifies SHA-256 before parsing either input:

| Input at the pinned cut | SHA-256 |
| --- | --- |
| `data/items/appearances.dat` | `aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50` |
| `data/items/items.xml` | `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2` |

The original test fixture calls ID 100 grass, but the donor marks it solid.
The new multi-floor fixture uses walkable donor ground 103. Existing fixture
assertions now distinguish source flags from their synthetic collision inputs.
The explicit artifact test now uses the production manifest instead of an empty
Item-profile fixture. It checks every known tile against the independent movement
floor-change owner: rope ground itself has no floor change, but another stacked
entry can supply one (Canary `src/items/tile.cpp:1837-1841`).

## Stopped mechanics

W19b: STOPPED at the shared USE/drop planting transaction design.
Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/scripts/actions/other/dryad_garden.lua:1-26` only teleports; those callbacks already exist.
Actual planting is `data/scripts/actions/items/ice_flower_seeds.lua:3-15` and `data/scripts/movements/seeds.lua:3-17`: consume one seed, transform, decay, and conditionally award Preservationist.
The existing terrain writer (`crates/gameplay/durability/src/durability/terrain_tool.rs:86-89`) requires a carried tool; it is not a dropped-item owner.
Required decision: extend the placement transaction or define a shared planting transaction for both ingress paths; no USE-only substitute is introduced.

W07: STOPPED at shared instance/template and transaction ownership design.
`soulpit/handoff.rs:33-36` requires allocation, session transfer, core consumption and encounter creation to share one transaction.
`native_activity.rs:768-777` currently calls internally committing generic/boss entry; `durability/soulpit.rs:149` instead requires the caller's open Type2 transaction.
Qualified activity templates (`content/bosses/activity_template.rs:77-95`) currently describe a single boss, not a Soulpit wave instance.
Required decision: extend that shared template/transaction contract; a separate teleport or early core burn would violate its stated boundary.

W08: STOPPED on W07's production-entry dependency, not on a missing multiplier/query UI.
The existing lifecycle writer already calls `animus_mastery::complete_in_transaction` for current survivors.
The donor completion source is Crystal `data-global/scripts/quests/soulpit/soulpit_fight.lua:100-106` and `src/creatures/players/animus_mastery/animus_mastery.cpp:33-61`.
Without a qualified production Soulpit entry/return owner, the acquisition path cannot be declared complete.
No alternative client-controlled mastery grant is added.

## Validation

`cargo fmt --all --check` and explicit feature-fragment formatting checks pass.
Clippy covers all six changed crates, all targets/features, with `-D warnings`.
Library tests use `--all-features --lib -- --test-threads=4` with the PostgreSQL
admin URL unset: durability 671, spell 314, world 218, dispatch 2307, client-ui 84,
client 117 passed; 19 pre-existing tests are ignored in that library run.
The live PostgreSQL `bundle_vertical_move_postgres_dispatch_levitate_up_down_and_magic_rope`
test executes three committed/replayed scenarios: up 85.03 s, down 86.88 s,
rope 130.47 s; total 302.38 s. No full PostgreSQL suite or nextest run was used.
`pin-check` reproduces bundle digest `22a8d5c75d8ec58b77b6a35b8edf9b0883c486c0ae31bfe69c21a35655ffc6e8`.
The explicit `pinned_bundle_tiles_answer_every_base_tile --ignored --nocapture`
run passes in 242.61 s: 19,373,519 tiles, 19,355,477 known, 18,042 unknown,
1,225 rope spots. The unknown tiles retain the existing refusal behavior.
The catalogue generator's `--check` and `oteryn-architecture-check workspace .`
also pass. Native Windows window execution is unavailable on this Linux host;
the client ingress/session/refusal/logout flows are covered by the passing tests.
