# SRX-3 Spite, Hatred and shared Torment source execution record

Pinned Crystal revision: `201189f871d23597b82caf88fd611454a2ddefb8`.
Pinned Canary revision: `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
Wiki checks remain wiki-pending; the executable donor scripts settle these rules.

## Settled source rules

- Spite corpse step: Crystal `data-global/scripts/quests/soul_war/soul_war_mechanics.lua:364-398` and `data-global/lib/quests/soul_war.lua:602-617`: any existing outfit condition rejects the entire operation; otherwise consume corpse33876, outfit286 for14000ms, and uniform1..100 <=10 heals floor(maximumHP/10) if the boss exists.
- Spite fire: mechanics.lua400-480, library619-643: independent GlobalEvent interval14000ms, one of four literal positions, Item33877, delayed5000ms lookup; only a remaining fire empowers an existing Spite by additive10defense. Successful player stomp persists `goshnar-spite-fire` wall-second; the56second limit includes absentKV default0. Refused cooldown leaves the fire intact.
- Two Weeping Soul XML sources: `data-global/world/world-monster.xml:177815-177817,177835-177837`; positions33738,31632,14 and33748,31634,14, radius1, offsets0, respawn10s. Verified Git blob `7949251d25d8db8f0b2ca65b67df76ac577a5caf`; SHA256 `f90e82203a112c9068d4ed123e3e5bb4c1b9743f2c2d3bcb12039ac8ebd9ad4e`. Generated capture `tools/content-schema/native-gameplay/srx3-spite-spawns.json` is evidence for the existing activation/respawn owner, not spawn authority by itself.
- Hatred: mechanics.lua579-666, library489 and416-419. Lever stores180 and resets the participant passed by the lever condition callback. `data/libs/functions/boss_lever.lua:175-214` and `lever.lua:143-155` prove that callback evaluates every actual pad participant in order; successful entry resets them all. Earlier accepted participants are already reset when a later participant fails a condition. Each actual Burning Think checks the OLD counter (180/135/90/45), stores old-1, and changes to Ashes/Spark/Flame/Blaze; zero instead resets180 and returns. Sorrow33793 adds10, allowing reversal. Returning to Ashes empowers the current boss by10percent. Hatred disappearance removes the first current creature of each of the four names even when it is not a death.
- Source `Monster:setType` restoreHealth defaults true: `src/lua/functions/creatures/monster/monster_functions.cpp:144-190`. Reuse the canonical current-incarnation profile transformation owner; do not remove and respawn the actor.
- Hatred/Mego incoming health callback adds current empowerment to defense once per combined event. Outgoing multiplier changes PRIMARY only. Literal mechanics.lua655 tests attackerHatred OR TARGET namedMegalomania; preserve that defect for ordinary player names. Do not silently repair Mego's outgoing branch.
- Shared Torment: library1398-1450 and1545-1579. One shared last-execution clock, integer wall seconds. Symbol requests3000ms, Mego10000ms. Damage uses the PRE-increment counter;24..36 uses the explicit table. Counter clamp is30/36 per caller; warnings execute outside the clamp branch. Reset on login. Remorse pickup count persists separately in CharacterKV.
- Torment's helper takes four parameters; Symbol's fifth Hatred name is ignored. The body resolves global `bossName` or default Megalomania; no global assignment exists in the pinned source. An isolated Hatred instance therefore resets rather than damages players through this helper. This literal donor behavior is retained.

## Validation and integration state

Dependency-free Rust tests were executed directly with the installed `rustc --edition2024 --test` against each leaf, before and after implementing its pure rules: Torment4 failures then4passes; Hatred4 failures then4passes; Spite3 failures then3passes, plus its independent fire clock1failure then1pass. Shared persistent Character source-Item rules4failures then4passes (including independent Energy thresholds, third-flag-alone retention, and Sanity wrong-target cooldown). These are fast pure tests, not PostgreSQL or production-caller proof. XML capture validation2passes using the repository Python test. Focused whole-crate Cargo checks, actual carrier mutation tests, dispatch and native play remain required before calling either encounter DONE.

PlayerSpellState counter staging was released after its actual owning-state RED (expected24/Some24, observed0/None); green rerun remains required. Carrier mutations were released after the actual source-established owning-state RED described below. The production template gate remains closed until actual callers, durable step operations and source spawn activation are complete. No synthetic command reference may authorize automatic movement steps.


Two additional Hatred XML Symbol sources are pinned at33738,31599,14 and33748,31599,14 (`world-monster.xml:177649-177654`), respawn10s. Spite additionally has Spitter33744,31636,14 (`:177857-177859`), respawn10s. BossLever.lua227 clears current zone monsters before its boss/configured-monster creation. These five XML points must use the existing spawn owner with the initial clear and normal respawn/warning chain; they are not immediate permanently injected actors. SourceSpite capture contains no initially placed33876 corpse.

The Hatred/Spite actual carrier regressions executed against Entities19 and returned the neutral mutations' `InvalidCreatureTarget` after real Instance/policy construction. Source inspection rules out the fixture errors (missing native health policy would be `InvalidSpawnDefinition`; invalid slot/context has distinct errors). Three bodies were released with explicit diagnostic context: exact source boss percent, current-Spite additive defense/max-health heal, and source-four-form canonical setType restoration. Effective defense now uses the shared per-actor delta owner, retaining immutable policy equality. Green whole-crate reruns remain pending.

Item producer split: dynamic noncorpse source profiles33793/33877/34005/33950/33854/34006/34007 are prepared offline. Immutable Maw33890 stays a captured real static Item target without a new custody grant. True corpse33792 (also Condensed Remorse),33876,33891,33949,33984 remain with the existing corpse producer; no duplicate aliases or corpse writer are introduced.

### Corrected Hatred owner proof

The first entity runs did **not** establish Hatred mutation RED: their fixture
cloned a nonzero Outfit look into the Burning object-look policies, so native
policy admission refused before either mutation. The corrected fixture clears
Outfit look for object-look forms and admits the actual15000 maximum before
setting current health100 through the existing test-only health helper. No
assertion was removed or weakened.

With only the two mutation methods temporarily restored to their prior neutral
refusal, the corrected filter `srx3_hatred` reached all three intended failures:
`REACHED exact Hatred mutation`, `REACHED exact Burning setType`, and
`REACHED exact Hatred literal child spawn`. See
`/tmp/srx3-hatred-corrected-neutral-red3.log` (three failures; compile2.10s).
Restoring the snapshot-fenced counter and canonical full-health transformation,
then adding only the source Hatred→Ashes child/forced-center family, made the
same three cases pass; `/tmp/srx3-hatred-corrected-green.log` (compile2.67s).
Spark remains forbidden as a newly created Hatred child. Spite's independent
current-defense/heal fixture passed in `/tmp/srx3-spite-entities20.log`.
These are canonical physical-owner proofs, not a complete admitted room journey.

The retained pinned XML witness now includes all five known ambient rows:
Weeping Soul two, Spiteful Spitter one, and Symbol of Hatred two. The generator
and retained evidence tests showed two-versus-five RED, then three tests GREEN
in0.002s. This witness is not yet consumed by the native spawner owner; source
coordinates are known, but a data file alone does not establish that caller.

### 2026-11-07 caller checkpoint

The offline XML witness now covers nine exact rows: the prior five plus one
Poor Soul, two Necromantic Focus and one Lesser Splinter. Existing assertions
for the five-row subset remain. The five-versus-nine regression failed before
the official offline generator update; four retained-evidence tests then passed
in0.001s. This remains evidence rather than spawner admission: the Content
ambient qualification API is deliberately neutral until its real native test
runs, and no independent respawn clock has been introduced.

The real native Hatred think test reached an unchanged Ashes form where Spark
was required (`/tmp/srx3-hatred-native-think-20b-red.log`,50.68s). Its source clock
and canonical transformation body is now mounted. The same fixture now includes
all four source forms at boss disappearance; cleanup remains neutral until the
new cleanup assertion actually fails. No full Hatred journey is claimed.

Crystal `src/game/zones/zone.cpp:130`–137 removes the named torment icon without
resetting its counter. A pure PlayerSpellState regression now requires that
icon-only removal survive entity projection and that a later clamped source
pulse restore the icon. Existing Spell binaries do not contain this new test;
the icon-removal method is still neutral, pending behavioral RED.

### Registered Spite GlobalEvent proof (Native11 data window)

The source creator is a registered **GlobalEvent**, not a CreatureEvent or a
boss spell: Crystal `soul_war_mechanics.lua:415` and 431–432 register the independent
14-second loop; 417–429 selects one of the four positions from
`lib/quests/soul_war.lua:620`–626 and creates 33877. Its five-second callback at
400–413 adds 10 defense only when the fire still exists and the current Spite is
found, then removes that fire. No live Creature parent or successful lever is
required at registration or creation.

`build_srx3_spite_global_event.py` retains both complete pinned file digests and
Git blob hashes plus the bounded creation/expiry bodies. The initially empty
proof failed the one-event assertion; two source-proof tests then passed in
0.001s. A separate actual-tree regression failed because the real Encounter
lacked the loop, then all three tests passed in 0.009 s after
`spite_global_event_rule.py` added the timer, four literal anchors and four equal
map-item choices. The rule deliberately adds no automatic Item decay: expiry is
an owned defense callback. The full World-to-tree converter invokes that same
helper, preserving the source-authoring regeneration path.

The Spite source-world generator retains this proof in its existing pinned
`source_world` role. Its 238,037 bytes hash to
`793d54aea707dc7e1923148d6bf5d2f3ffc9a40bd0240e2622882558e6893efe`;
`--check` passed. Removing only the new metadata reproduces the exact 235,692 prior
map bytes embedded in the old Native10 binary, hash
`fda2d5ac329dd832d3a9fcf3522a03a28575bf81ddcf231a18629b40756a5cff`.
Tiles, item policies, attributes, frame and stack order did not change.

The native qualification API remains neutral until its real source-world test
runs. The durable adapter will use allocated internal source cause 4 with an
actual scoped GlobalEvent identity and NULL actor columns, through the existing
physical Item writer. It must not manufacture a Creature, Ability or ActorRef.
Data publication alone does not establish a completed Spite encounter.

### 2026-11-07 current-owner integration checkpoint

The actual source USE adapter now reads the fenced backpack tree and resolves the original native USE target through the current composed map (or current visible creature). Static Maw selection does not require a materializable Item policy or live boss. Crystal `src/lua/creature/actions.hpp:137` and `actions.cpp:269-280,688-694` establish default same-floor, one-cell reach for these Actions; neither source registration enables far use. House target selection remains refused until its own effective physical map owner supplies the proof; an origin Channel map is never substituted. The dispatch route retains the original request IDs, source unit, exact wire binding and selected-target proof across uncertain writes, then calls the sole revision-sequenced Quest/Item owner. That paired writer remains neutral pending its actual PostgreSQL failure test; no successful transaction is claimed here.

The Cloak event producer now stages historical source receipts from the original health callback, before full-health clamp or Fourth Taint cancellation. It uses the existing carrier and actual owner-cycle stamps. The paid spell reservation retains the exact event delta across unknown outcomes and preallocates bounded queue capacity before SQL; known publication appends without allocation. The original two native failures were observed in `/tmp/srx3-entities-native-health31c.log`; fresh green execution is pending. The non-Player positive-secondary-only branch still requires its original two-component canonical metadata path and is not qualified by a positive scalar primary or a net HP delta. The MINT consumer remains separately owned by the existing scoped Item writer.

Actual Primal server transcript replay reached native Session admission in the fresh client test but subsequently ended with `SessionUnavailable` (one test, 2.23 s, `/tmp/srx3-primal-native-client-32c.log`). The fixture now propagates the peer failure rather than hiding it behind that local outcome. This is not a successful native play test and not a production decoder regression claim.

### Accepted Q6a Cloak override (current contract)

The signed-health observations below remain source evidence, but their earlier normal-hit/no-pool completion inference is superseded by the explicitly accepted **Q6a** rule in `OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md` §13.7: a Player HIT on Cloak leaves a pool. This is a narrow accepted Cloak rule, not permission to normalize signed components globally or change Malice/Cage behavior. OriginNONE reflection still skips HealthChange, including when reward attribution names a Player; NULL absorption healing likewise does not acquire a Player callback. The new actual direct/swing/staged and consumed-condition tests require one retained intent per callback and no duplicate on replay. Their producer changes remain held until fresh behavioral RED, and the existing Item MINT caller needs its own actual proof. No positive-heal-only or inert completion claim applies.

### Historical signed health-event correction (source observation; Q6a inference superseded)

Both pinned engines pass **raw signed** `CombatDamage` components to Lua: Crystal `src/lua/functions/lua_functions_loader.cpp:598-607`, Canary `:595-604`. Crystal `src/lua/creature/creatureevent.cpp:468-483` calls Lua before taking absolute values from its return; it then restores negative damage sign. The ordinary path is Crystal `src/creatures/combat/combat.cpp:905,944` → `src/game/game.cpp:7934-7948`; `combatBlockHit` restores negative loss at `game.cpp:7285`. Consequently Cloak's literal `primaryDamage > 0` predicate (`soul_war_mechanics.lua:809-829`, Canary `:787-807`) does **not** mean an unsigned damage magnitude. Normal Player loss must produce no blood; positive Player healing can qualify even at full HP because the callback precedes the clamp. The earlier positive-hit test expectation was an incorrect migration assumption, not an accepted source repair. New direct/swing/staged negative-loss tests expose it; valid positive-heal, replay, Unknown reconciliation, source removal and owner retirement coverage remains intact. The signed producer correction awaits the fresh negative-rule failure before changing its caller mapping.

The literal secondary branch is retained without fabricating a producer. `CombatDamage` defaults both components to zero (`creatures_definitions.hpp:1732-1736`); native Monster/formula paths assign primary only (`combat.cpp:197-239`; Canary `:154-209`). Weapon splitting/imbuement is gated by an actual Player (`combat.cpp:790-794,2028-2108`), and `doTargetCombatHealth` initializes primary only (`global_functions.cpp:356-397`). Reflection's secondary block requires a preexisting non-NONE secondary type (`game.cpp:7327-7371`). The two Crystal global `creatureOnCombat` hooks scale existing components and alter Monster→Player only; they do not create a Monster→Cloak secondary. Cloak registers only its blood health callback and Fourth prepareDeath (`cloak_of_terror.lua:29-32`); its health callback returns unchanged components. Damage-healing response calls `combatChangeHealth(nullptr,...)` (`game.cpp:7400-7401`), so it fails the explicit attacker guard.

No ordinary positive-Player-heal→Cloak production caller is claimed. The pinned IH/UH scripts reject Monster targets (except their separate Leiden special case); Mass Healing excludes ordinary monsters, and Heal Friend targets player names. An artificial healing spell, target grant or unsigned normalization is not introduced to make a MINT play test pass. Malice/Cage's positive-only reflection guards require the same signed audit; Mirror, Cruelty and Hatred's sign-independent scaling/transformation rules are unaffected.

### Fresh signed-health and source-boundary checkpoint (2026-11-07)

The native signed-loss regression `srx3_cloak_signed_normal_damage_does_not_satisfy_positive_lua_guard` executed against unit snapshot34a and failed after the actual canonical Player hit reduced Cloak HP: the unsigned bridge incorrectly queued one event instead of zero (55.21s, `/tmp/srx3-cloak-signed-red34a.log`). The bridge now preserves negative loss for Lua, positive healing separately, and the existing full-health healing reservation/replay path. This supersedes the earlier unsigned-loss interpretation; fresh green is required. The two native Hatred incoming health controls passed together in55.98s (`/tmp/srx3-hatred-incoming-green34a.log`), covering combined damage and full-health healing before clamp.

The actual Spite source-world GlobalEvent boundary reached its neutral qualifier and failed in48.28s (`/tmp/srx3-spite-global-target-red33k.log`). The released qualifier now requires the exact Spite source view, full native closure and artifact pin, exact retained registered-event DATA, admitted fire Item identity and all four actual immutable source tiles. The separate XML-spawner boundary reached `SourceHookUnavailable` in43.33s (`/tmp/srx3-spite-ambient-target-red33k.log`); its native artifact admission and existing-spawner production caller remain unfinished. A Rust-only capture include does not substitute for artifact-bound XML evidence.

The server-only `source_step` UUIDv8 stored decoder failed its focused roundtrip before implementation (`/tmp/srx3-source-step-codec-red34a.log`). Its decoder now distinguishes the data namespace from UUIDv7 sessions. The physical callback's pending conversion retains a full immutable SHA-256 binding behind this opaque occurrence; it must not allocate a transport CommandRef. SQL acceptance remains closed until its own PostgreSQL regression executes.

### Native12 bounded XML admission and event Item closure

The official XML capture now retains the nine Soul War rows plus Ragiaz's Bone Capsule row (`world-monster.xml:182951`,33485/32333/14,radius1,60s); the Soul War rows retain their literal10s. Capture10 rows/3344B SHA256 `7362ef992588921b6c2f7c16f0e3565f4c5a59a937390ec40429433f9cc14c66`. The official activity-source generator embeds only the matching rows on each of the five existing room captures. The strict parser rejects changed clocks, incomplete lists and another room's rows. Its production qualifier requires the current native artifact, exact retained rows, real admitted native profile health and the actual source tile. All five official writes/checks passed; XML tests5/5 passed. The existing physical Spawner registration and source-specific startup/respawn placement consumers remain required; this DATA admission alone does not qualify a complete encounter.

The existing boss-movement Item producer additionally admits the four exact Gaz/CorruptedSoul ground Items20121/23726/23727/23728 and literal fog portal33004, reusing existing canonical identities and protected bindings. It creates no new Item ordinals. The four vortex Items have no invented Teleport type or automatic decay; their source callbacks own their timers. Crystal XML20121 has only `primarytype=fields` (`items.xml:40660-40662`);23726..23728 have only the ranged vortex name (`:47257`). Fog portal33004..33007 explicitly has `type=teleport` and `effect=teleport` (`:62782-62785`). The source producer derives Ground-only, nonmovable, nonpickupable policies from the actual DAT/XML/C++ owner. Its13-record capture SHA256 is `750d588a999460145627b9cecca3975b056dd341e61ffdb9872f9732793c3e06`; official publish/check and unchanged ordinal checks passed. Actual native-input missing-policy regressions failed before the new overlay; their final green requires the root's registered Native12 refresh.

### Snapshot36b corrections and remaining boundaries

The Native12 movement capture contains thirteen exact Item bindings. Eight are
protected imports (20121,23726,23727,23728,33004,37000,43294,43295); five are new
bindings (42851,42853,43589,43590,43625). The loader originally protected only the
three older imports, so snapshot34g's factories refused before any target
behavior. Those failures are prerequisite failures, not source-rule REDs. The
loader now checks full equality for exactly the eight imported bindings, inserts
only the five new ones, and refuses changed imports, duplicates and other IDs.
The two actual-record fast controls passed in7.02s in snapshot36b
(`/tmp/srx3-movement-binding-closure-direct36b.log`). No input or ordinal rewrite
was required. The Item producer's five tests also passed in1.620s after the
registered Native12 refresh (`/tmp/srx3-native-event-items-green12.log`).

The canonical absorption control reached its HP-net-heal assertion and then
failed because the positive absorbed heal inherited the Player attacker
(56.95s, `/tmp/srx3-cloak-absorption-red36b.log`). Crystal
`src/game/game.cpp:7400-7401` invokes that heal with a NULL attacker, so it has no
health-change callback. The released bridge now emits no Cloak event for this
branch; its residual Player loss remains negative. Fresh green is pending.
This does not establish an allowed positive Player-heal-to-Cloak gameplay path.

The source movement test proved both actual translated-item bindings first,
then failed because a real facing-only commit could issue another MoveEvent
proof (43.80s, `/tmp/srx3-source-step-turn-red36b.log`). Its factory now requires
that the current movement revision equals its last translation revision. All
session, scope, physical Item and pin checks remain; fresh green and the actual
paired SourceStep transaction are still required.

The ten ordinary World XML sources already exist in the base and srvpkg4
Spawn.Source shards: Poor Soul in `spawns-10000-11999.json`, eight Soul War rows
in `spawns-48000-49999.json`, Bone Capsule in `spawns-50000-51999.json`. This
generic World registration slice is DUPLICATE-ON-BRANCH/base. Private boss
Instances still need empty-point registration after the actual committed lever
activation, source-qualified forced respawn, three warning effects and North
facing through the same SpawnRealization/RespawnSchedule owner. Content now
issues an opaque exact-row/four-pin policy and the Entity registration boundary
is deliberately neutral pending its actual native RED. No parallel World
spawner, synthetic startup creatures or new respawn clock is introduced.

Hatred's volatile reset must preserve source evaluation order. Crystal
`data/libs/functions/boss_lever.lua:149-174` first rejects a nonparticipant,
disabled boss or occupied zone; `:219-225` checks minimum participants before
conditions. `data/libs/functions/lever.lua:143-155` evaluates pad conditions and
stops on the first false result. For each evaluated player, the level and
cooldown predicates (`boss_lever.lua:181-212`) precede `onUseExtra` at214;
`data-global/lib/quests/soul_war.lua:416-419` resets that player's Torment.
Consequently an earlier qualified pad retains its reset if a later pad fails,
but no pad resets for the earlier minimum-count or occupied-zone refusal. The
production admission prelude remains required; resetting every pad only after
a successful handoff would not implement the literal source.

The opaque source-step UUIDv8 cause roundtrip is green in snapshot36b
(1test/0.01s, `/tmp/srx3-source-step-codec-green36b.log`). The direct Lua Torment
shield/ring control also passed (1test/0.01s,
`/tmp/srx3-torment-shield-ring-green36b.log`): PRE-counter24 gives1400 Death loss,
a1000-point spell shield or2000 mana at ring2:1 absorbs1000, leaving400HP loss.
These are pure canonical-owner controls, not PostgreSQL or native Think play
proof. The verified negative `Creature:addHealth` path supplies Death type11,
originNONE and NULL caster; the element argument is not a boolean effect flag.

### Native movement cursor evidence and readonly Item ordering

Crystal `src/lua/creature/movement.cpp:375-405` first invokes one position
callback, then iterates the live tile at increasing index with an initially
captured last index. A callback removal shifts later entries; returning0 stops
the Item loop. `Tile:getItemById` delegates to the first matching live Thing
(`tile_functions.cpp:233-260`, `game.cpp:2814-2837`). Thing order is ground,
always-on-top Items, creatures, then down Items (`items/tile.cpp:1536-1567`).
Spite's position-only callback must select one current33877; it must not be
reinterpreted as one MoveEvent for every matching physical Item.

The existing official appearance decoder proves33792 has top order1, while
33876/33877/34005/33854/34006/34007 have order0. Their current private room
captures do not retain these dynamic policies. All34117 existing native Item
keys, including arbitrary dropped Items that affect the live cursor, have exact
IDs in the pinned DAT; none requires a guessed default. The source DAT Git blob
is `37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0`, SHA256
`17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1`.
The readonly `build_native_item_thing_orders.py` verifies the complete pinned
DAT/C++/proto sources and preserves the existing Item-reference order. Its
actual-index test failed before implementation (0 versus34117), and a missing
appearance initially failed to refuse. The three source tests then passed
in0.544s. Only a scratch packet was generated and checked (34117 rows,
1151145bytes, SHA256
`ffe45ffe8fc861fdb8bb85dd57b1e0ebf79dec4cb059fc5069503e082b07d698`);
artifact registration/publication remains pending the next coordinated window.
No Item definition, materialization flag or protocol reference is allocated.

The canonical position owner now supplies opaque committed OLD/NEW movement
DATA. A source cursor must retain the original occurrence before later handlers,
and accept an old-tile continuation only through the known preceding callback's
actual teleport. The generic SQL standing-read pose guard remains strict. The
existing source-floor/teleport owner and synchronous onMove Item acknowledgement
must compose before an internal relocation; a second movement queue or invented
transport command is not a substitute. These are remaining engineering callers,
not missing donor facts or owner design questions.

### Fresh 37d source boundaries (2026-11-07)

The exact public XML policy test reached the empty Instance registration boundary
and refused with `PlanConflict`. The existing spawn owner now stages the admitted
XML definitions, validates its usual ceilings/continuity/headroom, and installs
empty points and their existing respawn schedule from the committed activation
clock. World/activation/four-pin and Instance guards remain mandatory. Startup,
forced respawn placement, all three source warning effects, and the actual entry
caller remain separate unfinished work; this registration does not claim those.

The same six-case Entity process finished in98.46s: XML and the two zero-residual
absorption cases failed; positive-residual Mirror and Hatred controls and the
Cloak NULL-heal control passed. Zero-residual absorption now bypasses Mirror's
Player health event and Hatred's defense increment. A positive residual retains
its original Player event. This follows `combatChangeHealth(nullptr, ..., heal)`
in pinned Crystal `game.cpp:7400-7401`, rather than treating the absorbed heal as
an attacker's separate health callback. Fresh green validation remains pending.

The actual movement/Item SourceStep test passed its translated Remorse/Energy
prerequisites, then failed the arbitrary33877-tile assertion in44.88s. The factory
now limits Spite fire to the four literal position registrations in
`soul_war_mechanics.lua:476-478`; its genuine turn-only revision refusal remains
in place. This is a qualifier fix, not a completed live Item cursor or paired
Quest/Item step writer. Logs are `/tmp/srx3-xml-absorption-entity37d.log` and
`/tmp/srx3-source-step-nonanchor-red37d.log`.

### Drome supplementary library route (2026-11-07)

The skill-approved third-party TibiaData v4 route was attempted once for each
actual species, with ordinary HTTPS and no changed User-Agent, retry or bypass.
All five requests returned HTTP502; no creature response or admission fact was
obtained. The dated response manifest is
`/tmp/srx3-tibiadata-drome/fetch-record.json`. These supplementary requests do not
replace the pinned donor/A4 evidence or settle appearance, movement speed and
attack scheduling. Wiki remains **wiki-pending**.

NEED-FETCH (actual HTTP502):

- https://api.tibiadata.com/v4/creature/domestikion
- https://api.tibiadata.com/v4/creature/hoodinion
- https://api.tibiadata.com/v4/creature/mearidion
- https://api.tibiadata.com/v4/creature/scissorion
- https://api.tibiadata.com/v4/creature/murmillion

Fresh Entity37j ran the seven focused cases in140.17s: initial XML registration
and all five absorption controls passed; the new second XML warning assertion
failed (0 versus3) after the actual qualified registration and first warning.
The released fix retains the opaque XML policy on its source definitions and
adds two warning phases to the same existing pending occurrence/deadline index,
with1.4-second intervals. Each drive rechecks the retained policy's current
World, activation and four pins. Ordinary D115 constructors retain `None` and
their prior single-warning4.2-second path. No second timer, population, item
writer, source input or SQL change was added. Phase green validation, scoped
actual effect publication and forced-center/native-facing targets are pending.

### Rotten regular-minion native boundary

The existing full27-profile/34-callback witness already covers the 15 native
regular minions and `monster_onthink_area_icons.lua`; no new source input is
needed. A neutral adapter now receives the same opaque reserved native think
batch before attacking, verifies current published-World/frame/map ownership,
full native policy identity, source spectator range and current player sessions,
then stops at a distinct boundary marker. The source helper has no summon
exclusion, so this adapter deliberately does not reuse the boss-only helper
that rejects masters. It creates no icon, timestamp, counter or second clock.

The unrun real World PG regression uses the pinned Bloated Man-Maggot XML point
33887,31662,14 (world-monster.xml178092–178094), actual native AI admission,
and the actual canonical named-icon owner. It requires ArrowUp1 plus durable
hazard/upper-left KV timestamps. Neither native-boundary RED nor icon/KV green
is claimed before that case executes. Private source rooms and persistent
source-think fusion remain unfinished callers rather than missing source facts.

### Native cue385 fragment coverage convention

The root accepted preserving the existing client registration architecture on
2026-11-07: transport-only385 inherits Session presentation/Chat and the existing
native presentation views; it does not gain a fake independent panel or identity
reducer merely to satisfy structural coverage. Current23/30/286 and the explicit
275 decoder/renderer-only registration use the same unsupported-fragment
convention. By contrast371 owns an actual entity-label helper and panel fragment.
The checker requires local reducer/test symbols and a supported panel for every
commandless supported handler. Thus385's baseline must remain unless that actual
structural ownership changes. After genuine native Scene play, its reason can
truthfully distinguish supported inherited transport from absent independent UI;
codec/Session unit green alone does not justify a rendered-cue claim. The exact
pending analysis and target filters are in `/tmp/srx3-client384-coverage-analysis.md`.
No baseline, checker or registration source was changed for this analysis.

Current allocation is385 after the controlled main4508fd2f1d31ba316792ec05a968f9101daa67f9 collision correction. The named temporary384 analysis file and earlier actual384 captures/logs retain their historical identifiers.

### Actual Area15 and registered GlobalEvent boundaries (snapshot38/39)

The Area15 World test subsequently reached the real reserved native think eight
times, retaining sequence1, and failed at the canonical ArrowUp assertion
(`None` versus `Some(("rotten-area",0,17,1))`). Its actual PostgreSQL execution
took238.090s; `/tmp/srx3-global4-area15-pg38b.log` retains the complete result.
This releases the source-owned icon/three-KV integration, but that integration
is still unfinished. It must commit/drop the Map read transaction before the
sole Quest writer: the current-visible reader locks reconnect-session rows,
which a second concurrent Quest transaction would otherwise try to re-lock.
The Character revision slot likewise must be acquired after releasing runtime
and spell-state locks. The existing `SourceActorReservation` snapshots and
reserves canonical fixed slots through an unknown result; it is the intended
continuation owner, rather than a second mutable icon/counter store.

The snapshot38 GlobalEvent result failed its retained-DATA prerequisite, not
the writer. Source-room conversion deliberately clears `source_world()` on the
Bundle representation while retaining an opaque exact source-role recipe.
The narrowly corrected qualifier now reads that immutable recipe DATA and
checks the actual current Bundle geometry and full pins. Generic Bundle views
still clear the role and refuse qualification. The fresh snapshot39 test then
reached `genuine registered Spite GlobalEvent physical writer boundary` before
the neutral writer rejection, after57.057s of real PostgreSQL execution
(`/tmp/srx3-global4-real-writer-red39f.log`). The existing scoped-ground writer
body has consequently been connected through its actor-free Global variant.
The class4 SQL constraint remains unchanged pending its own actual CHECK
failure. Neither the14s production loop,5s current-field removal, nor the
Spite-fire StepIn caller is claimed complete by this writer boundary.


### Canonical Hatred periodic and Spite expiry component proof (42/42c)

Actual native Entity42 tests exposed the missing Player-owned condition callback:
the periodic tick removed HP but left Hatred defense160 instead of170. Three
controls passed, including NULL periodic damage and the existing direct/full-health
callbacks. After staging the same canonical defense delta with the consumed tick,
all four passed (75.41s); exact tick replay retains the successor once.

Two native Spite expiry tests then reached the old scope/health guard after proving
World identity or canonical lethal HP0 plus queued source registration. Both failed
there (70.30s), then passed (75.31s) after allowing only the registered Global expiry
mutation in World and before completed death. The tests retain HP0 Weeping-heal
rejection and completed-source refusal. This validates the lower mutation, not the
unwired14s Global creation/5s expiry production controller or a completed boss row.
Exact result counts, filters and SHA256 hashes of the original logs are recorded
in `docs/agents/evidence/srx3-hatred-periodic-spite-expiry-red-green.json`.
