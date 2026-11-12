# WORLD-SRC continuation: discovery speed source investigation

Scope: read-only donor/Wiki research; immutable locators are retained here; raw inspected files remain under `/tmp/world-src-cont/map-speed/`.
Donor evidence class: **OTS_HYPOTHESIS_ONLY**. Source text is data, not instructions.

## Independently fetched evidence

- Canary: `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`.
- Crystal: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`.
- Both recursive Git trees were re-fetched directly; neither response was truncated.
- `map-speed-fetches.json` records every raw-file URL, exact revision, tree Git blob,
  SHA-256 and line count. Each fetched file's computed Git blob SHA-1 matched its
  entry in the independently fetched tree.
- The inspected chain includes discovery Lua/geometry/debug tool, player/creature
  speed, login/event Lua, Lua bindings, protocol map/speed messages, titles,
  badges, achievements, outfits, Charos, conditions, Game speed/respawn and monster spawns.

## Source-established semantics

All locators below use the pinned revision above; no current-branch source is substituted.

| Finding | Exact donor locator |
| --- | --- |
| Crystal completion stores `done-<subId>`, clears POIs, sends discovery state, then calls milestones. It does not invoke a speed mutation. | Crystal `data/scripts/discovery_system.lua:605-615` |
| Milestones count fully completed main areas, persist `discovery/map-percent`, grant the Discoverer outfit/achievements and addons, and invoke title checks. No speed term/formula appears in this complete function. | Crystal `data/scripts/discovery_system.lua:528-563` |
| Discovery runs through a one-second global tick; login invalidates unfinished POIs on geometry change and sends discovery/donation state. Neither route applies a speed effect. | Crystal `data/scripts/discovery_system.lua:666-681,743-768` |
| `checkNewTitles()` forwards to the title owner. MAP eligibility reads only `discovery/map-percent`; title management changes title KV/unlocked lists, not movement speed. | Crystal `src/lua/functions/creatures/player/player_functions.cpp:1623-1634`; `src/creatures/players/cyclopedia/player_title.cpp:46-71,154-156,279-282` |
| Canary's MAP title check is explicitly unfinished and returns false; its complete recursive tree contains no discovery-named system/geometry Lua files. This filename census alone does not exclude differently named scripts. | Canary `src/creatures/players/components/player_title.cpp:271-274`; `canary-tree.json` / `canary-candidate-paths.txt` |
| Achievement grants update points, unlocked KV and notification only; outfit grants update ownership/addons and cosmetic notification only. These are not hidden speed rewards. | Crystal `src/creatures/players/achievement/player_achievement.cpp:27-46,93-96`; `src/creatures/players/player.cpp:6840-6855` |
| Player base speed is vocation base plus level minus one, with the donor's respective maximum/staff handling. Neither inspected function consumes discovery state. | Crystal `src/creatures/players/player.cpp:7222-7231`; Canary `src/creatures/players/player.cpp:7331-7339` |
| Effective creature speed is base plus `varSpeed`; player step-speed returns the clamped result. No discovery-specific term is present in this speed chain. | Crystal `src/creatures/creature.cpp:1528-1539`; `src/creatures/players/player.hpp:2057-2059`; Canary `src/creatures/creature.cpp:1643-1654`; `src/creatures/players/player.hpp:1909-1912` |
| Generic speed changes add a delta to the existing variable-speed term and notify speed; haste/paralysis derive their delta from base speed and remove that delta on expiry. This establishes the donor's existing generic mechanism, not how an absent discovery bonus should stack. | Crystal `src/game/game.cpp:7085-7095,7106-7117`; `src/creatures/combat/condition.cpp:2418-2444` |
| The speed packet carries base and actual speed. DiscoveryData carries main-area status/progress plus discovered/discoverable subarea IDs, with no speed value/formula. | Crystal `src/server/network/protocol/protocolgame.cpp:4093-4118,7740-7747` |
| Canary's map-action handler only consumes request fields; it implements no map action or discovery-speed mutation. | Canary `src/server/network/protocol/protocolgame.cpp:3332-3349` |
| Crystal map actions handle donations or resend state; no speed mutation appears in that handler. | Crystal `src/server/network/protocol/protocolgame.cpp:4167-4279` |
| Donation boost is explicitly monster respawn, not player movement: area funds buy a day of improved respawn; the Lua publishes zone rectangles, the Game tests x/y rectangles without a z check, and monster spawn intervals are halved there. | Crystal `data/scripts/discovery_system.lua:186-195,226-241,271-280`; `src/game/game.cpp:9592-9619`; `src/creatures/monsters/spawns/spawn_monster.cpp:185-208,364-374` |
| Canary Charos's map/discovering dialogue promises a Discoverer outfit, not a numeric speed bonus. Crystal Charos contains city attunement dialogue and no discovery reward implementation. | Canary `data-otservbr-global/npc/charos.lua:124-132`; Crystal `data-global/npc/charos.lua:68-122` |

Negative conclusion is bounded: no discovery-speed implementation or numeric formula was
found in this inspected end-to-end chain. This does **not** prove that the current Tibia
bonus is zero, nor does it authorize silently choosing a formula from remembered values.

## Permitted Wiki API attempt

The exact request below returned **HTTP402 Payment Required**, Content-Type
`application/json`, without a page or revision ID:

`https://tibia.fandom.com/api.php?action=query&format=json&prop=revisions&rvprop=ids%7Ctimestamp%7Ccontent&rvslots=main&titles=Measuring%20Tibia`

Method GET; normal User-Agent `Oteryn-Game source research (read-only)`;
Accept `application/json`. Receipt: `map-speed-api.json`. No HTML page, alternate host,
CipSoft server, retry, captcha/protection bypass or unofficial numeric overlay was used.

STOP (five lines):
1. Neither pinned donor establishes a discovery-speed formula or invokes speed from map completion in the inspected chain.
2. Numeric bonus/cap and raw units remain missing; displayed item speed is not evidence for a server delta.
3. Area/floor/Premium eligibility and haste/paralysis/equipment composition remain missing.
4. Login/death/teleport/revision refresh rules remain missing; the exact permitted Wiki URL above returned HTTP402 without a revision.
5. Stop the speed contribution pending these facts or an explicit owner compatibility rule; preserve canonical discovery progress without inventing a zero bonus.

## Facts still missing

1. Exact numeric bonus as a function of completed main-area count, including any cap and
   whether newly added main areas contribute to that count.
2. Exact eligibility/scope: completed main area versus subarea, floors, exclusions,
   Premium/vocation conditions, and when crossing a boundary changes the active contribution.
3. Units: native/raw movement-speed units versus displayed values. The item-tooltip
   formatter shifts item speed right by one (Crystal `protocolgame.cpp:7342-7347`), so an
   unsourced published/displayed number cannot safely become a raw server delta.
4. Composition with haste/paralysis, equipment, mounts and other speed sources; whether the
   discovery term changes the base used by percentage effects or is added afterwards.
5. Refresh/recovery rules after completion, login, death/respawn, teleport and map revision.

## Production integration candidates (engineering observations, not game facts)

Once those facts are established or an explicit compatibility rule is approved:

- Keep canonical progress in the current Character Cyclopedia owner. Derive completed-main
  counts from its per-subarea canonical envelope and source parent mapping; preserve the
  existing legacy grandfathered completion. Current implementation lives in
  `crates/gameplay/durability/src/durability/character_cyclopedia/storage.rs`.
- Resolve area membership from the admitted native bundle table at the server actor's
  position. Do not use the retired authored-square ID or client-supplied membership.
- `paced_movement.rs:114-151` is the real STEP entry; it reaches
  `actor_spell.rs:2475-2500` and `actor_movement.rs:64-95` for initialized player speed.
  `spell/actor_conditions.rs:1361-1374` composes current condition/runtime/equipment terms,
  then `characters/src/speed.rs:100-116` clamps and feeds pacing. A discovery contribution
  belongs in this actual pipeline, not solely a UI/stat display or an unused helper.
- `movement_equipment.rs:16-52,60-89` provides an existing pattern for a current,
  session/actor/position/content-bound read revalidated under the mutation owner's lock.
  A separate discovery read/projection should preserve its own provenance; relabeling it
  as equipment would hide the unresolved stacking decision.
- Completion/login/respawn projections can refresh a cached monotonic completion view,
  with area membership evaluated for movement. The appropriate refresh and formula stage
  cannot be selected from these donor sources alone.
- Relevant source-driven tests would cover actual paced STEP duration within/outside
  completed regions, crossings/teleports, condition/equipment combinations, reconnect,
  stale actor/session/geometry rejection and legacy-progress preservation.

No repository files, builds, commits, pushes, installations or external writes were made.
