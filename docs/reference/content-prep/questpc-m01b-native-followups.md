# Three later native EXPLORE entries

Packet audit found three non-World follow-up stages still unwired despite the
older continuation note saying all such stages were covered. Each is canonical
s3, after a real s1 survey and a separate s2 action/kill condition. These bindings
must not credit s3 for the first survey, manufacture prior evidence or complete
any reward set.

The exact source claim declarations are retained in
`tools/content-schema/quest-authoring/samples/chests/claims.json`; the existing
resolver verifies one placement and ownership by the canonical quest's
`source_refs.quest` and `source_refs.claims`, and an adjacent clear approach on
the source floor. Source namespace revision is
`canary-04b83b51+crystalserver-9f5a72c6`. Physical tile stacks were independently
inspected in the retained native B3 regions; no source objects are changed.

| Quest | Exact source placement | Chosen clear s3 approach | Separate s2 condition |
|---|---|---|---|
| Dragon Corpse | `canary:reward-claim/kv/dragoncorpse`:32179,32224,z9;[355,4025] |32178,32224,-9;355|USE evidence1 (shovel/scythe objective)|
| Dragon Tower | `canary:reward-claim/quest/u7_1/dragon_tower/rewards/backpack1`:33072,32169,z2;[408,2469]|33073,32169,-2;408|10 Elf Scout/Elf Arcanist kills|
| Edron Goblin | `canary:reward-claim/kv/edrongoblin1`:33095,31800,z10;[353,2469]|33096,31800,-10;353|10 Goblin kills;Current Premium retained|

The Dragon Corpse success path starts at clear32177,32223,-9, steps South onto
retained [355,2125], then East to the corpse approach. Crystal's item-name and
field-type reference identifies2125 as a non-replaceable fire magicfield:
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml
This OTS_HYPOTHESIS_ONLY cross-check supports the native field identity; no field
damage formula or player survivability qualification is asserted by these cases.

Own PG cases use the real dispatcher. They first refuse s3 without s1, earn s1
through native STEP, refuse again without s2, and arrange only the separate USE
or KILL owner's completed condition. Neither EXPLORE progress, visit receipts nor
live root revision is seeded. Disconnected source floors are positioned by the
existing server fixture teleport helper; each visit credit still needs native STEP.
The tests retain objects and assert refused replay, fresh once, revisit safety and
zero whole-quest completion. They do not qualify s2 execution, transport between
all source floors, equipment-bag delivery or either two-chest completion set.

All three real native dispatch PG cases passed, zero failed/ignored:
Dragon Corpse67.987155645s,Dragon Tower68.044933223s,Edron Goblin68.476461392s;
combined204.51s. The first surveys were earned through STEP; only the separate
non-EXPLORE s2 condition was arranged. Whole-quest completion remained zero.
The initial full authoring run exposed one-callback total/order assumptions:
later entries now follow existing s1 entries, and the Dragon Corpse activation
check requires exactly both intended keys while retaining its original s1 check.
The full123-module/1231-test retry passed without weakening assertions.
