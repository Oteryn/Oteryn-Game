# GAP-SV8 — movement spells, planting, Soulpit and Animus

Current continuation base: `d3b15da91c7633516566ae8bd8729c99d126034b`.
The production implementation section below supersedes historical STOPPED notes.
M11a/M11b are already integrated; W17 is excluded. Only migration 0975 is added;
no capability, command or message number is allocated by SV8.
The base has duplicate state domain102; its existing Item Effects domain moves to
next-free104 while cast view keeps102, per the base head commit. Capability350
and all commands retain their assigned numbers; clients use the shared constant.

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

## Production implementation on srvpkg4

- **oteryn-choice — W19b materialized AddItem:**
  `durability/item_loop_move/materialized_planting.rs` selects the single live
  empty pot from current Ground custody and invokes the ordinary native move,
  then `consume_transform_seed_drop.rs` consumes the newly dropped seed and
  transforms the pot inside that same fenced Type2 transaction. Exact loop-command
  replay precedes selection, because a committed planting has retired the pot.
  The immutable receipt binds the Ground seed to the same-XID move line; migration
  0975 keeps the existing predecessor, source/target and audit checks. Failure
  rolls back the move, seed, pot and timers together. Existing materialized USE
  remains in `consume_transform_planting.rs`. Donor Crystal
  `201189f871d23597b82caf88fd611454a2ddefb8`,
  `data/scripts/movements/seeds.lua:3-17` and
  `data/scripts/actions/items/ice_flower_seeds.lua:3-17` distinguish the two
  callbacks. Ordinary seeds use the donor 324→330→306 absolute decay stages;
  ice seeds retain 14031→14032→306. No second planting transaction is created.
  **oteryn-choice — native planting presentation:** reuse srvpkg4's existing
  capability350 ITEM_EFFECTS_V1, its item-only codec and native feature0350 route
  (`apps/client/src/features/0350-item-effects/protocol.rs:2-10`), which delegates
  to the existing native MagicEffect renderer. No generic capability23 activation
  is introduced. A bounded journal hold is reserved before SQL; exact committed
  receipts publish POFF3 once, rollback releases the hold, and uncertain outcomes
  retain it for existing pending-effect reconciliation. Ordinary item moves cannot
  emit a planting cue. The existing ambient101 and item104 publication domains
  keep their independent bounds; the base's MAX+1 mixed observation test remains
  unchanged. The actual production refresh router separates batches before encoding
  (`connection/serve/refresh.rs:620-653,717-765`). Both the focused CPU regression
  and real planting PG fixture use the existing item-only codec.
  **oteryn-choice — Ground context qualification:** native root admission keeps
  the complete same-transaction locked census. A differing context is compatible
  only for current Channel roots with exact map/content/position/generation and
  the existing immutable map-mint or Ground-action source proof. Admission DTOs
  are normalized after qualification; original custody and returned rows retain
  their original contexts. The direct spell census remains strict. This follows
  the materialized USE owner's existing source-provenance pattern and admits the
  donor AddItem callback without bypassing ordinary move/capacity ownership.
- **oteryn-choice — W07 source scene:** the active Canary public bundle has the
  staging/return objects but air at the donor combat destination. Use a dedicated
  immutable source room, booted through the existing bundle writer, reader and
  collision owner, rather than treating absent public tiles as walkable.
  `build_soulpit_room.py` reproduces 579 Crystal tiles/118 loader identities from
  `data-global/world/world.otbm` at the Crystal cut above, git blob
  `e61658ec0ab7b70ec35025ae969426d6fc57ae75`, SHA-256
  `c0549a4c94e1706861252a6cd7db7a3545cdd2f96259f405c5481e45d02a3f85`.
  Source bounds/entry/obelisk/exit follow Crystal
  `data-global/lib/others/soulpit.lua:31-58,138-140,178-179`.
  **oteryn-choice — room-local appearance identities:** 71 existing Item entries
  keep their admitted canonical references. The other 47 are non-pickupable,
  non-door terrain/decor appearances; they use bundle-local Terrain references,
  whose map-protocol meaning is local to the scene, never inventory Item IDs.
  Their separately typed capture physics preserves ground, solid and projectile
  independently (eight solid decorations do not block projectiles). Ordinary
  bundle boot keeps its existing behavior. This follows the existing terrain
  appearance route and source loader facts (`src/items/items.cpp:236-247`,
  `src/items/functions/item/item_parse.cpp:110-129,213-214`). It grants no inventory
  identity or materialization permission. No canonical Item, WorldProject, shard,
  global provenance or pin is rewritten. The source extractor emits sorted,
  column-described records, one placement per line; no donor map asset is shipped.
- **oteryn-choice — W07 entry/return:** the production BossActivityHandoff uses
  the existing shared ActivityHandoffOwner and physical Instance reservation.
  `activity_handoff/soulpit.rs` joins the core debit and encounter creation to
  the prepared handoff transaction before its group commits or sources terminate.
  Pending native attempts retain the exact selection for uncertain-outcome retry.
  Crystal `data-global/scripts/quests/soulpit/soulpit_fight.lua:37-77` validates
  the complete staging roster before core consumption. The existing shared return
  and recovery owner retains the source Channel and its exit placement; clients
  cannot supply a return Channel or position. The native driver applies committed
  wave steps to actual Instance actors and retains per-generation cadence ownership.
  The dynamic activity turn retains one owned return intent per original session
  and scope generation; the production cadence/USE caller invokes the existing
  generic native return pipeline directly, as the existing boss-activity owner does.
  A dedicated drain mutex serializes publication, and pending intents retain the
  exact original admission and prevent Instance retirement until shared publication
  or an already-published successor lineage is proved. No runtime lock crosses SQL.
- **oteryn-choice — W08 acquisition:** complete through the existing transactional
  `soulpit/progress.rs` survivor credit, with no separate mastery grant command.
  Crystal `data-global/scripts/quests/soulpit/soulpit_fight.lua:100-106` and
  `src/creatures/players/animus_mastery/animus_mastery.cpp:33-61` are the acquisition
  donor. The linked completion fixture enters through real USE and the shared
  physical Instance. Native lethal reward snapshots feed existing fenced party-XP
  and proficiency settlement without ordinary loot, Bestiary, task or Forge grants.
  Donor XP scaling: Crystal `src/creatures/monsters/monster.cpp:747-750,2341-2348`;
  weapon proficiency 1500: `src/creatures/players/player.cpp:6584-6607,12932-12936`.

**oteryn-choice — lethal reward ordering:** freeze the full native Soulpit damage
and XP snapshot in the lethal combat turn. Both the ordinary reward drain and
actual player-death settlement drain that same fenced reward owner before
experience loss. This preserves the donor immediate kill reward ordering even
when the final boss kill and player death occur before the next activity tick;
there is no second XP writer. Crystal `src/creatures/monsters/monster.cpp:747-750,2341-2348`
and `src/creatures/players/player.cpp:6584-6607,12932-12936` supply the reward rules.
The production PostgreSQL regression uses a positive count of actual physical
boss kills and the real death receipt, including exact replay and no mastery for
a participant who dies before completion.

**oteryn-choice — current Instance read authority:** production entry exposed a
Channel-only equipment reader. The additive `character_equipment/activity_read.rs`
reader proves the actual current Instance gameplay fence, node, committed shared
handoff participant, destination session/generation and saved destination content
pin, with no published return. It derives the retained origin Channel only as
read metadata and reuses the sealed custody reader. The same private proof serves
read-only lifecycle/premium facts; no item-move or other mutation authority is
extended. This follows the existing House read adapter and the shared room/arena
handoff ownership pattern; donor fight admission remains Crystal
`data-global/scripts/quests/soulpit/soulpit_fight.lua:37-77`. The real Instance
fixture rejects a wrong content pin and the retired source session, and initializes
its level requirement using native progression and the fenced XP writer.

**oteryn-choice — proficiency at shared transfer:** real physical Boss settlement
exposed two existing transfer gaps. Training proficiency saves rejected every
Instance despite the downstream current gameplay fence supporting that scope.
Only Training now accepts a genuine current Instance; other causes retain their
restrictions. Shared entry/return force the existing source proficiency checkpoint
before taking the transfer lane, then refuse any dirty or retained checkpoint
under that lane. No second proficiency writer or SQL grant is added. This retains
the Crystal `src/creatures/players/player.cpp:6584-6607,12932-12936` 1500-point rule
across the actual session handoff, rather than waiting for a normal save interval.

**oteryn-choice — exact retained exit retry:** a failed SQL return can retain its
physical transfer reservation. The original actor/session/fence/command/target
intent resumes that existing attempt before fresh `spell_idle` admission; changed
bindings refuse. Fresh exits still pass the ordinary current actor and idle checks.
This follows the existing shared return retry owner and Crystal's exit callback,
without introducing a caller-supplied destination or another return transaction.

**oteryn-choice — source-qualified wave positions:** a real PG wave exposed the
old rectangle sampler: seven invalid positions could acknowledge an empty wave.
Crystal `data/libs/systems/zones.lua:12-36` first filters the Zone and uniformly
samples its valid tile list; `data/libs/functions/tile.lua:39-83` requires ground,
no solid/projectile/immovable-solid property and deliberately ignores creatures.
The immutable scene has 638 rectangle cells, 573 materialized tiles, 509 ground
tiles and 163 qualified spawn tiles. Production now samples only that qualified
list, with replacement, through the current native movement/projectile owners.
`src/items/item.cpp:1224-1225` defines immovable-solid as a subset of solid.
The fixed Boss position follows `soulpit_fight.lua:117`; normal positions follow
that same line's Zone call. The retained carrier wave request is consulted before
new drawing, so a lost Spawned acknowledgement replays its exact actors/positions
even when the scene changes. No additional actor is invented on failed creation.

Crystal `src/lua/functions/core/game/game_functions.cpp:306-308` defaults Soulpit
creation to extended=false/force=false, and `src/items/tile.cpp:624-662` retains
physical creation refusals. **Accepted existing native physical-owner difference:**
that donor's canPushCreatures exception for occupied pushable monsters
(`tile.cpp:637-658`) is not granted by Oteryn's existing creation owner; this
package retains its stricter vacancy requirement rather than synthesizing a push
or forcing an actor onto another actor. The admitted Rat fixture has
canPushCreatures=false, so its occupied-cell behavior matches the donor.

**oteryn-choice — deterministic real PG encounter:** a test-only constructor
supplies a valid fixed v7 handoff nonce, chosen through the exact shared production
selector against the real qualified scene. All four planned waves avoid fixture
occupancy; the fourth protects the donor's fixed Boss position without adding a
production exclusion. Production Default still uses SecureIdentifiers. Control
allocation, native actors, USE dispatch, core consumption and SQL transactions are
unchanged. This guarantees the death-order fixture actually kills a physical Boss,
while the separate CPU tests retain partial/empty physical attempt semantics.

W08 reuses the live client flow: capability 168 / command 232
(`crates/protocol-oteryn/src/animus_mastery.rs:3-4`). The selected-capability and
current-session query at `gameplay_transport/animus_mastery.rs:42-71` reads the
same `game_character_animus_masteries` completion set through
`durability/animus_mastery.rs:73-94,127-128`. Client
`apps/client/src/play/session_loop.rs:177-180` routes the result to
`apps/client/src/features/0168-animus-mastery/protocol.rs:8-14`; the native panel
uses the existing decoder in `crates/client-ui/src/features/0168-animus-mastery/state.rs:27-44`
and the Window/Refresh flow at `apps/client/src/features/0168-animus-mastery/native.rs:39,99,133-150`.
No alternate mastery grant, panel or transport allocation is introduced.

Validation provenance: W19b has an executed pre-fix rollback regression (ordinary
native move committed before planting existed). W07/W08 implementation and fixture
authoring overlapped; no genuine pre-implementation live-production RED run was
captured. The old rejected destination fixture is not counted as PG execution.
The source selector repair did have a genuine physical-wave PG failure first.

### Fresh srvpkg4 validation

The first clippy attempt detected the base duplicate domain102 at protocol build.
The minimal integration repair above is included before the fresh validation rerun.

The srvpkg4 presentation integration preserves its independent ambient and item
flows. The older SV8 combined-cap23 selection was removed before final validation;
the existing ambient overflow assertion is unchanged, and the new regression
checks the actual item-only batch after the production split.

Final frozen source checks: `cargo fmt --all --check` and explicit rustfmt checks
of **86** changed Rust files passed. All **nine** changed crates (including session)
passed `cargo clippy --locked ... --all-targets -- -D warnings` in **4m 01s**.
The capture generator's `--check` reproduced 579 tiles and SHA-256
`e497528182cd0673699edc86adc7450af21f8f92fe8a4df44e309492cb64de55`.
No global content, import, ruleset or deployment pin differs from srvpkg4.

The existing client item-effects negotiation test first failed with
`UnsupportedPushedDomain { domain_id: 104 }`: the base had a literal102 route.
The session dispatcher now uses the shared item-effects constant, and its
independent route expectation records104. No assertion was removed or loosened.
The full session library then passed **171/171 in 21.10 s**. The full protocol
library passed **713/713 in 1.42 s**; the world library passed **222**, with **one
existing ignored** test, in **75.90 s**, including all47 local Terrain identities,
all579 captured placements and the163 qualified spawn cells. The focused native
item-batch CPU regression passed1/1 in0.00s.

Mandatory PostgreSQL18.6 cases executed with `OTERYN_REQUIRE_PG=1`:

| W19b native wire case | Duration |
| --- | ---: |
| Nested carried pot USE | 53.754963072 s |
| Preserve carried plant USE | 64.536657658 s |
| Preserve Ground plant USE | 57.606214757 s |
| Ground empty pot USE | 53.419866764 s |
| Ordinary materialized pot seed drop | 52.392153887 s |
| Ground seed remainder | 52.720425048 s |
| Split-stack materialized pot seed drop | 52.113627555 s |

W19b passed **7/7 in386.55s**. Each drop case checks real commit, injected rollback,
exact replay and one committed POFF; USE preserves the existing identity/decay
rules. All seven emitted `W19 POSTGRES_CASE_COMPLETED` and `PG_EXECUTED` markers.
The fixture negotiates existing capability350 and round-trips its item-only codec.

W07 production cases passed **3/3 in168.17s**. Actual Boss-kill/player-death ordering
observed XP4462 before loss and emitted the real physical Boss witness; its death
boundary took **495.224772ms**. Four actual waves, persistent1500 weapon points,
one mastery, replay and shared Channel return passed in **53.286462926s**.
Atomic entry, first-wave progression and authenticated exit passed in
**52.932314518s**, including two real SQL aborts, altered-target/stale-generation
refusals, and exact unchanged-command return replay. All three emitted their
explicit `PG_EXECUTED` marker.

Linked W07/W08 atomic completion/mastery commit passed **56.54s**, SQL boundary
**422.656662ms**; post-Animus-insert rollback passed **66.16s**, boundary
**319.260944ms**. Both emitted `POSTGRES_CASE_COMPLETED` and retained their actual
post-insert witness, current-fence and complete rollback assertions. Total **2/2,
122.70s**. No selected PG case was skipped or ignored. The server code tested by
these12 cases is unchanged by the later client-only routing repair; its client
behavior is covered by the full171-test session run above.

**Final status: W19b DONE; W07 DONE; W08 DONE.** M11a/M11b are already in the base;
W17 is excluded. Migration0975 is the only added migration. No new capability or
command ID is allocated; existing Item Effects uses domain104 to repair the base
collision while cast view keeps102. No full nextest or full PG suite was run.
No protected path or merged migration changed, and no commit or push was made.
