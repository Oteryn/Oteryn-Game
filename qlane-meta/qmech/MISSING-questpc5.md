# questpc5 checked missing mechanics

Checked against qmech aca848557. Paths below refer to this package, not a frozen lane snapshot. Definition shards are compact JSON: record pointers and stage keys identify the exact quest step. These are missing contracts observed in code, not whole-quest qualification claims.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Nomads Land Quest | Server: exact mutable Ground stack/order prerequisite | Same definition, s4 requires arranging crates/amphora at the window. `crates/gameplay/quests/src/quest/world_callbacks.rs:151` WorldRequirement only contains position/appearance for an immovable map entry, with no mutable Ground item identity/count/order predicate. | Partly: physical window puzzle. |
| Targuna Quest | Client/wire delivery and Sterling binding remain; server persistence implemented (M16 slice) | `content/quests/definitions/quests-00200-00299.json`, `/records/79/definition/oteryn_recipe/payload/recipe`, s2-s3. `quest/reward_callbacks.rs` now admits a bounded `map_mark`; `durability/quest_state/map_marks.rs` records it with the native source/receipt and exposes a live-fenced read; migration 2070 enforces same-transaction coupling. No client projection or production Sterling callback is enabled by this slice. | Partly: Sterling marker delivery remains in progress. |
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
