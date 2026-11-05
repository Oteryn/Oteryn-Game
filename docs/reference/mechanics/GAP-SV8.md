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

## Original investigation (superseded by continuation below)

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


## Continuation on integrated SV8

Base: `a0fbfbc5fb07b9685d3f43c9ec342195be4cb15d`, fetched from
`claude/happy-davinci-ijz3wy-srvpkg3`. The original uncommitted tree was discarded.
The preceding investigation/validation describes the earlier movement-spell package;
it is not validation evidence for this continuation. Continuation donor references
are classified `OTS_HYPOTHESIS_ONLY`.

- **oteryn-choice — W19b:** retain the production fenced writers already integrated
  by the planting work. USE composes in consume/transform; AddItem composes in the
  native item-transfer transaction, with the same terrain CAS, source-timed decay
  and type-2 audit ownership. Do not create a second planting transaction owner.
  Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
  `data/scripts/actions/items/ice_flower_seeds.lua:3-17` and
  `data/scripts/movements/seeds.lua:3-17` distinguish USE from AddItem.
  Concrete production paths: `item_craft/planting.rs::use_planting`,
  `durability/consume_transform_planting.rs::prepare`, and
  `durability/item_loop_move/seed_drop.rs::commit_item_loop_seed_drop`.
  The static-map paths already consume one seed, transform, retain decay and
  grant Preservationist through their existing owners. The remaining AddItem
  consequence for a materialized pot should join its existing native transfer
  transaction, rather than perform a second commit.
  **Implementation remains incomplete:** `loop_move/seed_drop.rs:34-35` accepts
  only `MapEntryKey::Base`; a materialized pot does not select that consequence.
  The existing map-pot tests do not qualify materialized-pot AddItem execution.
- **oteryn-choice — W07:** use a dedicated multi-wave Soulpit template under the
  existing `ActivityHandoffOwner`, with the core debit and encounter creation
  inserted before its prepared group becomes committed and before any source
  session ends. Reuse its physical reservation and entry/return recovery owner.
  Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
  `data-global/scripts/quests/soulpit/soulpit_fight.lua:37-77` validates the
  whole staging roster before consuming the core; `:100-106` owns completion.
  This is a selected technical design, not an owner decision request.
  **Implementation remains incomplete:** the production handoff implementation,
  qualified multi-wave template and native wave/return bridge are still absent.
  An incomplete transaction-only draft was discarded; it is not in this patch.
- **oteryn-choice — W08:** keep acquisition in the existing Soulpit completion
  transaction, with current survivor fences and exact occurrence replay; do not
  add a separate grant command or client-controlled acquisition path.
  Crystal `data-global/scripts/quests/soulpit/soulpit_fight.lua:100-106` and
  `src/creatures/players/animus_mastery/animus_mastery.cpp:33-61`;
  current writer `durability/soulpit/progress.rs::credit_survivors`.
  **Implementation remains incomplete:** the production acquisition path still
  depends on W07. No W07/W08 completion or PostgreSQL qualification is claimed.

The integrated base failed Rust compilation with duplicate `TitleFacts` fields
(`E0062` for `drome_rota_top_five` and `next_rotation_unix_ms`). This continuation
removes only their obsolete default assignments, retaining the existing computed
values. No Tibiadrome gameplay rule is changed. No migration or protocol number
is allocated, and no commit or push is made.

A second inherited validation failure was the audit source-site inventory:
`item_text/write_once.rs:182,227-228` already binds the selected fenced tuple,
but `item_mint_audit.rs::every_type2_insert_takes_its_tuple_from_a_type2_transaction`
omitted that production site from its exact expected list. The list now includes
it; both per-insert tuple assertions remain unchanged. The red focused case is
retained in `/tmp/sv8-type2-red.log` (0 passed, 1 failed, 0.05 s).

### Continuation PostgreSQL evidence

Six existing `map_seed_planting` wire cases executed against the retained local
PostgreSQL cluster. The first invocation passed ice-seed drop and Preservationist
USE, then four cases failed before database creation because the workspace disk
was full (2 passed, 4 infrastructure failures, 94.21 s). After recovering space,
only those four blocked cases were retried: all passed in 188.76 s (wall 188.792 s).
The logs explicitly record `MAP_SEED_PLANTING_PG_EXECUTED` for all six cases,
including rollback/replay and pickup/absolute-deadline checks.
The retry used the same freshly compiled dispatch test executable, relocated to
`/tmp/sv8-dispatch-test-current`; its source tree was unchanged.
This qualifies the existing static-pot slice only. No new W07/W08 PostgreSQL case
was implemented or counted, and no full PostgreSQL suite or nextest run was used.

Disk recovery preserved task test executables and build archives under `/tmp`;
relocated archives retain symlinks at their original build-cache paths.
No PostgreSQL data, tool installation, dependency source or durability setting
was deleted or changed.

### Final focused checks

`cargo fmt --all --check` passes. `cargo check --locked -p oteryn-gameplay-durability
--all-features` passes. `cargo clippy --locked -p oteryn-gameplay-durability
--all-targets --all-features -- -D warnings` passes (11.39 s).
With the PostgreSQL admin URL unset, `cargo test --locked -p
oteryn-gameplay-durability --all-features --lib --quiet -- --test-threads=4`
passes: 688 passed, 0 failed, 0 ignored, 54.48 s. The first complete library run
had 687 passes and the one stale source-list failure (55.06 s); that assertion was
repaired rather than skipped or weakened. Disk-full attempts did not count as
successful validation. `git diff --check` passes.

**Package status: W19b STOPPED, W07 STOPPED, W08 STOPPED.** Their selected designs
are recorded above, but the remaining mechanic implementations were not completed
in this continuation. The delivered code delta repairs only the two inherited
validation failures; it does not claim new playable Soulpit or mastery acquisition.
