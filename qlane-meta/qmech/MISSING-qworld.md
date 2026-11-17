# QWORLD — checked missing mechanics (PARTIAL)

Checked on qmech `3786efb97`; scope is the three retained QWORLD entries. This is a partial blocker inventory, not a claim that their complete quest programmes are implemented.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Fury Gates Mini World Change; Grimvale Mini World Change; Their Master's Voice World Change | Server adapter: authored native-death objectives carrying the required world-change condition | `content/quests/definitions/quests-00100-00199.json`, records 48 and 54, selected recipe s3 (10 Fury) and s2 (10 Werewolf); `quests-00200-00299.json`, record 49, s3 (Mad Mage). `crates/gameplay/quests/src/quest/native_kills.rs::COUNTERS` contains four outfit trial bindings only; `Counter::transition` always sets `world_change: None`. `dispatch/.../quest_kills.rs` and `durability/.../quest_state/native_kills.rs` admit only that closed registry. The quest writer already supports a world-change condition; the missing bridge is its native-death registration/admission, not a second world-state store. | partly: the selected kill stages and subsequent completion remain unbound |

## Existing work and transfer boundary

The durable world-state store, reset catalogue, counters, snapshots, quest/world-callback/travel gates and the Fury/Grimvale/Voice physical PG cases are already present in this package through merged QWORLD PR #3026 and collector #3030. Do not replay the old pre-squash QWORLD implementation over newer shared code. The QWORLD lane retains migrations **1320–1329**; this report takes none.

No client blocker was established in this check. No source quantity or missing client command is inferred from a title. The existing 19 world-change unit cases and two PG cases are engine evidence, not qualification of all selected recipe stages. The audit-report branch is not quest/mechanic payload and is not transferred here.

Reuse inspected: QENGINE2's `authored_bindings::Trigger` supports NPC dialogue, carried USE, map USE and step-in but no death trigger; QUESTPC2 has a native kill-objective implementation for Elvenbane that must be considered before extending this adapter. That branch's source is evidence for reuse, not copied or claimed as new QWORLD work here.

Correction after checking Crystal201189f `data-global/lib/quests/their_masters_voice.lua:111–149`: Gobbler_onUse checks slime_ids, preserves the tool and transforms the target to12065 at line127. The existing `ItemOnTarget.target_replacement` and `reusable` fields already express that consequence; remove-only support is NOT a missing mechanic for this quest. `world/src/map/daily.rs:233–254` already maps native fungus appearances12059–12063 in the source rectangle. Personal cleaning-stage/tool/NPC registrations remain quest binding work, not justification for another map-removal engine.

## M32 addressed

Fury/Werewolf native death bindings now load from `content/quests/native-death-objectives.json` through `quest/native_kills/qworld.rs`; `Counter::transition` carries the configured world condition into the existing writer. Real PG cases prove closed/open occurrences, pre-hunt refusal and duplicate settlement (Fury150.93681868s; Werewolf150.9516698s). M32 covers Fury Gates and Grimvale only; the historical Voice row is outside the coordinator's M32 allocation. See `docs/reference/content-prep/QWORLD-M32.md` for the M31 personal-stage gate contract.
