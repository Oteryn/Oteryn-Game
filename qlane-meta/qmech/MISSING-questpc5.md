# questpc5 checked missing mechanics

Checked against qmech aca848557. Paths below refer to this package, not a frozen lane snapshot. Definition shards are compact JSON: record pointers and stage keys identify the exact quest step. These are missing contracts observed in code, not whole-quest qualification claims.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Nomads Land Quest | Server: exact mutable Ground stack/order prerequisite | Same definition, s4 requires arranging crates/amphora at the window. `crates/gameplay/quests/src/quest/world_callbacks.rs:151` WorldRequirement only contains position/appearance for an immovable map entry, with no mutable Ground item identity/count/order predicate. | Partly: physical window puzzle. |
| Targuna Quest | M16 implementation and canonical Sterling binding present; transport qualification pending | Same recipe s2-s3. Capability 449 / command 733 and the private native minimap layer are implemented. Reused `content/quests/npc-callbacks/m16-targuna-marker.json`, mounted its NPC conversation, and source-bound its offers/refusal/greeting. The canonical callback Root PG case passed 57.993081347s on PostgreSQL 18.6. | Partly: dialogue/query PG execution and complete boot qualification remain; no missing engine hook is inferred. |
| Targuna Quest | Server: quest-guarded object taming with atomic quest progress | Same definition, s8-s9 earn Bunch of Turnips then tame Reliable Ram. `crates/gameplay/content/src/content/native_gameplay/mount_taming.rs:17` TamingRule has target item/mount/chance/equipment but no quest guard/progress plan; `crates/gameplay/dispatch/src/gameplay_transport/mount_taming.rs:292` builds MountTamingRequest without a quest transition. | Partly: final ram step. |
| Last Creep Standing | Server: quest callback for temporary chosen disguise and matching portal, restoring prior look on exit | `content/quests/definitions/quests-00300-00351.json`, `/records/12/definition/recipe`, s2-s9 require eight named disguises/portals. `crates/gameplay/quests/src/quest/reward_callbacks.rs:106` ActorConditionRequirement admits only Fire/Drunk; RewardCallback has no disguise/restore effect. `world_callbacks.rs` Callback has no chosen-disguise predicate. This is the quest callback boundary, not a claim that the general appearance owner is missing. | Partly: eight trial steps; unresolved reward identities also remain quest data work. |

## Finished-work transfer audit

This report adds no quest activation, content or migration. The continuation's thirteen DONE labels refer to frozen questpc5/a2564ddc historical proofs; they are not implementations on qmech. Do not count them as transferred or use those historical durations as fresh qmech PG evidence. The four excluded PR heads were fetched for overlap inspection. No mixed unfinished lane snapshot or duplicate migration is imported by this report. Finished slices require fresh package qualification before transfer.

## Resolved M15

ENGITEM commit `68d538460` exposes bounded ItemReward text and carries it into the existing MINT/Text owner with SQL plan binding (migration1641). Reused that implementation; did not add a duplicate engine or migration. Fresh owner PG case PASS51.497609265s and two independent callback regressions PASS0.02s. This qualifies the text mechanic, not the Nomads Land journey. The selected recipe collects the book at the cabinet; no invented NPC book reward is enabled. See `M15-questpc5.md` for validation and the separate package hash-baseline failure.

## M16 server slice (current qmech integration)

Focused case `questpc5_personal_map_mark_is_fenced_atomic_and_replayed` executed against PostgreSQL 18.6: `M16_PG_COMPLETED duration=52.950609832s success=true`. It covers unmet progress, invalid text, stale lease/read revision, atomic marker/source/receipt World coupling, replay, repeat refusal and immutable label. Parser case `personal_map_mark_admits_only_bounded_native_cross` was RED before the field existed and is GREEN.

Migration 2250 from qmech supplies the sibling SQL alias repair; the unpublished local 2071 variant was removed. Only lane migration 2070 remains. The build script watches the embedded migrator directory so newly added SQL cannot be silently omitted by a cached owner crate. This is a tested server slice, not full M16 or whole-quest qualification.

M15 was also rerun with the freshly compiled current-qmech migrator and migration 2250: `ENGITEM_PG_EXECUTED literal_reward_text duration=61.626140577s`, PASS.

## M16 wire/client slice

Reused qmech's newly merged `local-minimap` exploration, pan/zoom and local-marker editor. Server facts use a separate bounded in-memory collection, are cleared on new admission/World bind, and are excluded from the World-only local save. Scope-change coalescing discards older facts. Malformed accepted replies terminate the usable session; refusals retain confirmed facts. Queries do not count as gameplay input.

Focused GREEN: two protocol codec cases (including the exact 100866-byte maximum), two callback/binding cases, the floor/anchor/zoom UI projection case, and three native client delivery/privacy cases. `cargo check --workspace --all-targets` PASS 5m01s. The production adapter PG case compiles, but its executable link failed with `collect2: fatal error: ld terminated with signal 7 [Bus error]` at low disk space; stale lane artifacts were removed. This new PG case has not executed yet and must not be reported as green. The unchanged marker-storage SQL retains the prior real-PG 52.950609832s proof.

Whole node-boot attempts: `NODE_BOOT_WORLD=0 bash tools/qualification/node_boot/run.sh`; `NODE_BOOT_WORLD=1 bash tools/qualification/node_boot/run.sh`; `NODE_BOOT_SPELLS=1 bash tools/qualification/node_boot/run.sh`. All exit 2 with `NODE_BOOT_RESULT=BLOCKED reason=exact_platform_checkout_missing`; no partial stage is counted as qualification. M16 remains PARTIAL pending the new PG run, Sterling data binding and complete boot qualification. No CLAIMS.md edit.

The exact Platform checkout was subsequently fetched at the runner pin; the three initial missing-checkout results above are historical prerequisites, not a permanent blocker. Complete runner retries remain pending. The Docker daemon is reachable.

## M16 canonical Sterling consumer (2026-11-08)

Reused QN6's existing `m16-targuna-marker.json` callback and offer files rather than registering a second reward. `questpc5_targuna_map.py` verifies the retained Crystal Sterling SHA and emits the source replies, mission topic, Morla refusal and greeting. The NPC conversation now mounts the canonical callback. The Root test loads that exact registered reward, not a synthetic stand-in: `M16_POSTGRES_EXECUTED server=18.6`, `M16_PG_COMPLETED duration=57.993081347s success=true`. Three focused authoring cases passed. Workspace all-target check passed in 4m23s; fmt and native-provider regeneration passed.

The latest requested qmech merge (`0326dc2e5`) was committed before the owner's no-more-merges decision. Its additive duplicate diagonal arms and duplicated M20 test bodies were removed while retaining every unique assertion, including the newer durable-recipe-plan test. No further main/qmech merge is planned. The dialogue/query PG case previously hit compiler OOM and remains pending; this is a tested consumer slice, not a complete Targuna journey claim.

Fresh consumer-slice checks: the quest-authoring checks runner and all 115 unit modules passed; content-tree validation passed (34111 Items, 252 Mounts). Focused personal-map native-client, protocol, projection and binding cases passed, together with all three retained M20 recipe regressions. Native-provider regeneration used the repository tool and changed no derived bytes.
