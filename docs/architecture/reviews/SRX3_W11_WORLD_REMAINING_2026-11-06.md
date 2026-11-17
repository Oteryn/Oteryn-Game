# SRX-3 W11 World-chain implementation map

Pins: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`; Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`. All source paths below are Crystal. This is a work-routing audit, not a DONE claim. Known rules remain implementation work; only the eighth Phantom coordinate is a source-fact STOP. Wiki checks: wiki-pending.

## Exact unresolved_semantics World entries

Indices match `soul-war-crystal-reconstruction/*/manifest.json` and the45-row source audit.

| Entry | Donor file:line | Exact remaining production obligation | Current caller |
|---|---|---|---|
| claustrophobic_raid_1 /3 | `data-global/scripts/quests/soul_war/moveevent-claustrophobic-inferno-raid.lua:43–51` | On reset remove current zone monsters; after5s refresh zone and remove its then-current players; record end wall second. | Mounted `soul_war_world_raid.rs` +existing Raid outbox; runtime GREEN pending. |
| claustrophobic_raid_1 /4 | same file:58–81 | Sand-timer committed STEP with source direction predicate; reject restart for10s after last reset. | Mounted `paced_movement` →`soul_war_world::committed_step` →existing Raid scheduler; runtime GREEN pending. |
| claustrophobic_raid_2 /3 | same file:43–51 | Same reset/delayed-current-player lookup, separate raid2 state. | Mounted fixed source STEP/cue pipeline described below; runtime GREEN pending. |
| claustrophobic_raid_2 /4 | same file:58–81 | Same STEP/direction/cooldown for raid2. | Mounted fixed source STEP/cue pipeline described below; runtime GREEN pending. |
| claustrophobic_raid_2 /5 | `data-global/lib/quests/soul_war.lua:208` | Bare `Pos` in the eighth Phantom slot has no unique source coordinate. | STOP for this one spawn only; other seven and all other raid rules continue. |
| claustrophobic_raid_3 /3 | raid script:43–51 | Same reset/delayed-current-player lookup, separate raid3 state. | Mounted fixed source STEP/cue pipeline described below; runtime GREEN pending. |
| claustrophobic_raid_3 /4 | raid script:58–81 | Same STEP/direction/cooldown for raid3. | Mounted fixed source STEP/cue pipeline described below; runtime GREEN pending. |

The source raid stage producer is the same raid script:13–40: three actual monster species, exact per-raid lists from lib, initial `timeToSpawnMonsters="3s"`, source spawn period and survivor duration, automatic stage progression. The nominal declarative Encounter recipes do not consume actual World STEP or kick players. Donor Soul War OTBMs exist, including the three Ebb/Flow maps; no broad map-absence claim is made. Reviewed XML Phantom placements do not uniquely identify the missing eighth coordinate.

## Other required World-chain producers and consumers

These are independently required by the complete W11 chain even where the45-row Encounter manifest did not enumerate them.

| Mechanic | Exact donor evidence | Current implementation / remaining caller |
|---|---|---|
| Five hunt entrances | `data-global/scripts/quests/soul_war/moveevent-soul_war_entrances.lua:1–50` | Private boss catalogue admission is a different caller. Need actual committed World STEP: level250, current Flickering Soul teleport-access flag, exact source destinations; failed access returns to captured from-position. |
| Final approach | same file:52 onward | Current private final-room roster Quest checks prove level/access/five mini-boss flags; verify the distinct World approach STEP/teleport and rejection presentation. |
| Mirrored Nightmare mirror item | `data-global/scripts/quests/soul_war/soul_war_mechanics.lua:120–140` | Native direct Mirror HP/birth and paid-batch reactions exist; actual static-item33782 USE, shared30s cooldown, default-placement spawn and33783 transform remain a World interaction obligation. |
| Apparition kill access | same file:142–196 | Source iterates every actual damage-map player ID without the getKillers PZ-age filter, increments that named apparition counter, then checks25 kills for each required apparition at the entrance (lib:481–503). Do not reuse only XP-party recipients or the recent-damage boss-taint boolean. Core SoulWarState currently stores boss flags, not these access counters; no production World kill/entrance adapter found. |
| Ebb/Flow global map cycle | `data-global/scripts/quests/soul_war/globalevent-ebb_and_flow_change_maps.lua:3–94,119–201`; `data-global/lib/quests/soul_war.lua:505–585` | Need current authenticated World map owner to apply exact alternate captured OTBM geometry, source phase clocks, pool40s return, pending-event cleanup and player/summon relocation. Existing source-room loading is not this live World cycle. |
| Ebb/Flow boats and door | mechanics:482–528; lib:641 onward | Actual committed STEP/USE consumers must read the same map phase and apply exact source transport/door behavior. No native caller found. |
| Rotten Wasteland shrines/access | mechanics:530–575 | Four-count state, exact source spelling `rotten-wasterland-*`, repeated-last-shrine check and access interaction need the canonical Quest/Item writer. No World production caller found. |
| Pulsating Energy drop | mechanics:697–721; lib `pulsatingEnergyMonsters` | Eligible actual corpse owner +canReceiveLoot, exact Vibrant Phantom/Cloak of Terror/Courage Leech profile, then create34005 at captured monster death position. Existing loot/planner output alone does not perform this callback. |
| Pulsating Energy pickup/floors | mechanics:723–805 | New `quests::quest::soul_war::source_items` pure state plan exists, not a production writer. Need actual34005 STEP, persistent counter/three flags, three source floor transport gates. Source typo `furius-crater/pulsating-energy` is distinct from Maw's `furious-crater`. |
| Cloak health-event pool creation and STEP | mechanics:807–858 and lib:20–24 | Before HP clamping, health callback creates33854 when no33854 is already on the current tile. Literal precedence is `(attackerPlayer && primary>0) || secondary>0`, with an earlier nil-attacker guard; raw inputs are signed, so normal negative damage does not qualify, while a positive healing input can. Its allowed ordinary gameplay producer is not yet proved. Player STEP loses20/15/10%maxHP through typed ENERGY Lua addHealth; Cloak STEP heals1500–2000; every creature STEP consumes the pool. Item producer is staged; health-event creation, actual step/HP and decay callers remain separate. |

The GM-only `/changeflowmap` TalkAction at mechanics:322–339 is not a substitute for the global cycle and is excluded from player-facing gameplay under the owner's no-OT-only-command scope.

## Branch overlap and actual Rust search

Read-only object audit resolved srvpkg4 to exact `5e60c19701fe3acdda0a3d1331da1239440a9d1e`; questpc is `6c09e0365110db6683f938dd08d5d584744cbf37`. Against base `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`, both have zero diff in `entities/carrier/runtime_actor_periodic.rs`, dispatch `creature_field.rs` and `actor_spell.rs`. Srvpkg4 retains periodic lethal `origin:None` at475 and strips finisher provenance at292–293. Its neighboring condition/potion changes do not add a World Soul War caller. Questpc's Soul War reconstruction tools/sample are unchanged against base, and its reward-profile adjustment only supplies an added fluid field.

A current Rust-wide search for claustrophobic / mirrored-nightmare / rotten-wasteland / ebb-and-flow / pulsating-energy finds the taint rectangle helper and the new pure SourceItemState keys. It finds no actual World raid, Ebb/Flow map-cycle, mirror-use, apparition access, shrine or floor-gate production adapter. Descriptive reconstruction JSON, validators, source crosswalks and wiki metadata are source evidence, not DUPLICATE-ON-BRANCH implementations. Recheck the owner's later quest branch head before porting any newly completed World leaves; keep shared Quest writer changes minimal and additive.

## Existing W11 seams to reuse

- `soul_war_callbacks::drive`, real reserved ThinkOccurrence batches, exact source52/56 witness, and source-bound private room/whole-published-World geometry.
- `CurrentMonsterMap::current_soul_war_taints` / canonical visible Character reader for DATA; no fake command or offline Character lease.
- `quests::quest::soul_war::source_items::{SourceItemState,Kind,Plan}` for persistent pickup/action rules; the existing Quest writer must atomically join real movement/USE and exact Item consumption.
- Actual per-scope collision fact overlay and canonical source placement/teleport primitives. A capture loader is not permission to mutate unrelated map geometry.
- Existing native source death/loot receipts and GMF MINT owner for deterministic extra ground output, never a second loot/corpse writer.

The Claustrophobic spawn chain also inherits the actual Encounter framework, not only its quest-local script: Crystal `data/libs/systems/encounters.lua:28-48,147-176,181-239,286-328`. There are six20s stages; each stage schedules its literal positions3s after entering that stage, with1/2/3s teleport effects. Thus nominal spawn deadlines are3/23/43/63/83/103s and the last stage resets at120s. `enterStage` cancels that stage's outstanding events before advancing. `start` refuses a second start while already active. Reset removes the current zone's monsters immediately, then the separate5s callback refreshes and removes the players present at that later moment; it is not a captured initial roster. Every successful framework spawn calls `monster:setRewardBoss()` at217, including these three ordinary species. Its complete closure is Crystal `data/libs/functions/monster.lua:171-175`: it calls `setReward(true)` only when the actual MonsterType already has `isRewardBoss`; C++ `src/creatures/monsters/monster.cpp:254-256` reads that immutable type flag. Therefore this helper does not turn these three ordinary species into reward bosses; their ordinary native reward route remains correct.

A real whole-World STEP regression is now mounted as `srx3_claustrophobic_actual_world_step_starts_source_raid_postgres`. It boots the exact published default World bundle, proves the complete published-world context, creates its Player through the existing Character/admission writers and calls ordinary paced movement from34013,31048,9 to the literal raid1 sand timer34013,31049,9. Its expected effect is a single actual durable raid firing, not a descriptive Encounter package. Its fresh Native11 execution reached the intended firing assertion and produced the genuine218.47s RED recorded below.


## Whole-World/native Item reference integration failure

The real `srx3_claustrophobic_actual_world_step_starts_source_raid_postgres` case reached the current published World bundle (`a25ff2dde36d61bf1a68fae74647616be38b6987bb5f3140de7feeb45c12441d`) and failed during the production-style map boot with `ItemReference`, in90.28s. This is a prerequisite failure, not a Claustrophobic STEP behavior RED. The exact test filter is `srx3_claustrophobic_actual_world_step_starts_source_raid_postgres --test-threads=1 --nocapture`; scratch output is `/tmp/srx3-claustrophobic-world21-red.log`. The earlier optional `default-world.otbundle` cache was stale (`3af394…`) and correctly rejected by the hash guard; the second run used the officially built a25 cache.

Read-only standard-library JSON/header analysis found34117 raw Item definition keys,34111 native Item index keys and20030 Item-routed World palette entries. Every palette key exists in the native index, but4962 compact references differ. All World palette references match the sorted raw Item definitions exactly (zero mismatches). The native index omits six real definitions: `i42851`, `i42853`, `i43589`, `i43590`, `i43625`, `i43860` (each under `oteryn:item.tibia.`). Source files are `content/items/definitions/items-*.json`, `tools/content-schema/native-gameplay/item-keys.json` and the actual a25 bundle manifest. `crates/gameplay/content/src/content/item_ref.rs:80` defines the sorted index, and `crates/gameplay/world/src/map/facts.rs:878-880` correctly requires exact Item reference equality. No key alias, synthetic reference or map-hash fallback is warranted; the registered native key producer must retain the complete current admitted key universe and regenerate it.

The retained structured proof is [srx3-world-native-item-reference-audit.json](../../agents/evidence/srx3-world-native-item-reference-audit.json).

First16 mismatches in actual World palette order:

| Item | World reference | Native reference |
|---|---:|---:|
| i429 | 25237 | 25235 |
| i430 | 25256 | 25254 |
| i431 | 25267 | 25265 |
| i432 | 25284 | 25282 |
| i433 | 25298 | 25296 |
| i434 | 25311 | 25309 |
| i435 | 25322 | 25320 |
| i436 | 25374 | 25370 |
| i437 | 25400 | 25395 |
| i438 | 25462 | 25457 |
| i439 | 25555 | 25549 |
| i440 | 25590 | 25584 |
| i441 | 25637 | 25631 |
| i442 | 25669 | 25663 |
| i443 | 25679 | 25673 |
| i444 | 25690 | 25684 |

## Reusing the actual raid and physical owners

The production extension must retain `RaidRun`/`RaidSpawnIdentity` receipts and the current Map/actor owners. A generic ledger-wide cleanup is not source-equivalent. Crystal `src/game/zones/zone.cpp:140-144` removes all currently cached monsters of the actual zone, including ambient monsters and summons, while literal raid2 positions include actors outside its declared area (for example Phantom34020,31072,10 lies beyond maxY31068). Such a firing's outside actors survive reset when they remain outside. Use current zone membership for source removal, then close the durable raid metadata while retaining outside actors; never clip the source spawn lists or remove only the firing's registered actors. Ordinary source removal creates neither death nor loot/XP.

Zone destination initialization is `data-global/lib/quests/soul_war.lua:903-907`; exact exits are raid1→34009,31083,9; raid2→34011,31028,10; raid3→34014,31085,11. The delayed callback refreshes actual membership before `Zone::removePlayers` (`src/game/zones/zone.cpp:130-137`), which performs source NOLIMIT teleports and removes the `goshnars-hatred-damage` presentation icon. It does not reset the Torment counter in that function. No frozen starting roster, movement-blocker substitution or count-derived icon redisplay should replace this behavior.

## Actual reset scheduling and cancellation

There is no separate120s reset scheduled at start. Quest raid script:21 divides `suriviveTime=120` by `spawnTime=20` only to build six stages at:24-40. Each stage's actual start calls `EncounterStage:autoAdvance` (`data/libs/systems/encounters.lua:37-48`), which schedules its own next-stage callback20s later. `Encounter:nextStage`:312-316 invokes reset only when the sixth stage advances. `Encounter:reset`:286-294 calls the quest's onReset, then `enterStage(unstarted)`; `enterStage`:156 invokes `cancelEvents`:129-133, stopping the remaining registered stage handles. Constructor/resetConfig:81-116 adds no reset deadline. The global `data/scripts/globalevents/encounters.lua:1-15` only calls a stage's tick; this quest has no such tick. The separate `startOnEnter` helper's leave/reset hook is not registered by this quest.

The fixed native source controller was tested standalone with `rustc --edition2024 --test crates/gameplay/encounters/src/bosses/claustrophobic_inferno.rs`, then filter `srx3_claustrophobic_ --test-threads=1 --nocapture`. The initial source-ingress/cadence/reset implementation produced three genuine RED failures; the source implementation then passed all four original checks. Two added edge checks also passed (six total; compile+run0.118s): a delayed first stage advance at100s makes the nominal120s point enter stage3 rather than fabricate a reset; an actually executed late sixth-stage reset invalidates every remaining stage handle, leaves only its independent5s kick, then drains to no pending events. Scratch outputs are `/tmp/srx3-claustrophobic-pure-{red,green}.log`. These rule proofs do not replace the still-pending real World STEP/raid production proof.

## Internal raid source-kind allocation

Internal persisted `source_kind=4` is reserved for the three fixed keys `raid:soul-war.claustrophobic-inferno-{1,2,3}`. Existing1XML/2adaptive-Lua/3calendar meanings stay unchanged. This is an internal Raid owner classification, not a new protocol message, capability or command. Forward migration0985 extends the existing CHECK for only these three keys, leaving the existing table, grants and owner chain in place.

SQL-only verification cloned the actual current migrated table's defaults/CHECK constraints into a transaction-local probe. The old constraint admitted the legacy2 control (1.558ms), then rejected the first fixed4 row with23514 (0.469ms). Applying the exact forward constraint to that temporary clone admitted the legacy control and all three literal4 keys (four rows,0.408ms), while unrelated4 and unknown5 still raised23514 (negative-check block4.452ms). The transaction rolled back; no live table/row/role was changed. Retained [RED SQL](../../agents/evidence/srx3-claustrophobic-source-kind-red.sql), [RED output](../../agents/evidence/srx3-claustrophobic-source-kind-red.log), [GREEN SQL](../../agents/evidence/srx3-claustrophobic-source-kind-green.sql) and [GREEN output](../../agents/evidence/srx3-claustrophobic-source-kind-green.log) show the actual executed constraints and timings. The subsequent native World STEP/firing behavioral RED is recorded below; its production GREEN remains required.

The cross-controller event receipt was additionally exercised: raids1/3 each have19 points and could otherwise issue the same ordinal/deadline/effect. A genuine RED showed a prepared receipt from raid1 advancing raid3. Private raid identity is now part of the receipt; all seven standalone checks pass (`srx3_claustrophobic_`, compile+run0.249s). This is an owner-data identity fence, not a gameplay change.

Native10's official Item-reference correction allowed the fresh25 test to complete the actual full World boot and published World proof. The run then exposed a fixture-only attempt to rebase an already bundled room; the fixture now rebases from its original entry room using the same authenticated World index. That precondition failure is not counted as a gameplay RED; the later fresh Native11 intended-assertion failure is recorded below.

### Actual Native11 World trigger RED and fixed-source continuation

The fresh Native11/World18fb test completed the real full World boot, authenticated published-frame conversion, and ordinary committed South STEP from `(34013,31048,9)` to the literal first sand timer `(34013,31049,9)`. It then failed the intended production assertion: zero durable source firings instead of one. Exact filter `srx3_claustrophobic_actual_world_step_starts_source_raid_postgres`; executed duration218.47s, scratch log `/tmp/srx3-claustrophobic-world-native11-red.log`. Earlier World fixture failures are not counted as this behavioral RED.

The ensuing production composition uses the existing raid schedule/firing/target/run/effect owners. Source kind4 remains restricted to the three literal raid keys. Internal effect/receipt action kind5 denotes a fixed source cue; it is not a protocol or command number. Existing effect kinds1–4 and source kinds1–3 retain their meanings. Bounded source cursor/cue JSON columns extend the existing tables, with the existing owner/generation/pin checks and an additional closed source-key trigger. No fabricated textual announcement, dynamic area anchor, or separate actor/map/raid store authorizes the callbacks.

The actual old effects CHECK admitted legacy kind3 (0.689ms), then rejected kind5 with23514 (0.433ms). A transaction-local clone with the forward bounded cue shape admitted the source cue (0.280ms); missing cue/unknown kind negatives passed in1.210ms, then rollback0.231ms. See checked-in `docs/agents/evidence/srx3-claustrophobic-action-kind-{red,green}.{sql,log}`. Full migration and runtime GREEN remain required separately.

The shared source kernel now resides at `crates/gameplay/encounter-rules/src/raid/source_claustrophobic.rs`; Encounters reexports it. The same literal source implementation supplies both durable cursor validation and dispatch effects, avoiding divergent interpreters. The existing physical STEP caller supplies exact committed actor/session/revision plus authenticated World frame; SQL independently rechecks Character recovery, Node registration, current Channel generation/reset/map pin, and serialized World/key state. Cues remain pending until physical owner acknowledgement; stage-relative delays are scheduled from the actual callback execution. The physical consumer uses native raid realization identities, current Item/Map facts and the shared default N8 placement rule. Reset removes current zone monsters; the independent five-second kick reads current players and removes only the Torment icon. Runtime/codec verification is still pending at this writing.

The Soul Sphere health source conflict is separate and remains explicit: canonical parsed15, both pinned donors10000, frozen Wiki text15.000 (15000). No source override or complete Greed chain is claimed while that owner rule fact is unresolved. The only undefined Claustrophobic coordinate remains raid2's eighth Infernal Phantom; its other seven literal Phantom spawns and every settled callback continue independently.

The final native raid admission's free-cell requirement is equivalent for this closed roster: pinned Crystal `data-global/monster/quests/soul_war/normal_monsters/{brachiodemon,infernal_demon,infernal_phantom}.lua:61` declares `canPushCreatures=false` for all three (`pushable=false`:57), and all three actual native Behavior records retain that value. The shared N8 chooser therefore rejects occupied ordinary cells before the existing final admission. No generic raw occupancy/force override is introduced. Placement shuffles reuse the existing unbiased `RaidRandom`, bound to the immutable firing/source-event/draw ordinal.

### Fresh33k World probes and Native12 map capture

The real published World hunt probe ran219.97s and reached the actual current Player/source-frame setup, but the ordinary South step refused before the source access callback. This is a movement/map predecessor investigation, not an access-rule RED. Pinned Crystal has ground28457 at33615,31421,10 and ground28456 plus teleport33004 with destination0,0,0 at33615,31422,10; both grounds are nonsolid and speed150, and33004 is nonsolid/pathfinding-blocking. Existing World teleport landing already specifies remaining on a tile when its destination is absent, so the blank destination alone does not justify a new movement grant. The test now prints the actual published collision predecessor for both cells; its assertions remain intact.

The separate Claustrophobic actual STEP probe ran221.70s, committed its source sand-timer movement and reached the production durable-start caller, which returned a retry and left zero firings. Review identified an invented rejection of canonical initial reset epoch0 and a fixture that reused the private activity server-only reset target. The epoch0 rejection was removed while retaining the independent current reset proof. The World fixture now supplies its actual published bundle target separately from its native server digest; the production bundle equality guard was preserved. The19-spawn and source-cursor assertions have not yet executed and fresh verification remains required.

The repository tool `tools/content-schema/native-gameplay/build_srx3_ebb_maps.py` now captures all12355/34392/18449 source tiles from the three pinned Ebb/Flow maps, including the real outlying coordinates, ordered Item depth, flags, zones and attributes. Three focused parser/source-integrity cases failed before implementation and pass afterward. The approved Native12 publication and `--check` both succeeded at `content/encounters/soul-war-ebb-source-maps/`. These captures remain inert reference data until the actual lazy-map replacement owner is qualified and connected; their presence alone is not a production mechanic. The33276 WorldObject identity must be a new unique source row: a repository-wide canonical census found no previous row to replace.
