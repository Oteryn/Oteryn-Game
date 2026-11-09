# npcfix2 checked mechanics — PARTIAL

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
| --- | --- | --- | --- |
| The Tainted Souls Quest | Server: persist the captured target debit supplied by qm22's quantity-aware resolver. Owned by qm22; npcfix2 supplies source-qualified quest content. | `tools/content-schema/quest-authoring/samples/completion242/recipes.json`, `oteryn:quest.the_tainted_soul`, s2. `crates/gameplay/items/src/item_craft.rs` provides `carried_cooking_conversion_with_quantity` and `CarriedCookingConversion.target_debit`; the checked durable caller in `crates/gameplay/durability/src/durability/item_craft.rs` still resolves through `carried_cooking_conversion_with_sub_type` and decrements ordinary ingredients by one. qm22 now refuses capture-enabled recipes in that quantity-blind API, so the durable owner still needs to consume the quantity witness and persist its debit. Before that refusal guard, real PG red cases observed 36 antlers remaining from 37 and `NoRoom` for a full backpack. | Partly: the selected physical transformation cannot retain its captured stack semantics. |

M21's nested source opening/leaf pickup and key provenance are implemented on this
work branch. The World authoring tool restores all three Outlaw key source
bindings. Small Ruby already has a physical source and a real production-reset
fixture in `postgres_tests/l6_world_pickups.rs`. Skeleton Decoration's Tyrsung
barrel is the existing personal claim UID 6216 in the container-source packet;
it must not also mint a second shared reward. This report does not infer complete
quest readiness from those engine facts.
# npcfix2 checked mechanics — PARTIAL

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
| --- | --- | --- | --- |
| The Tainted Souls Quest | Server: persist the captured target debit supplied by qm22's quantity-aware resolver. Owned by qm22; npcfix2 supplies source-qualified quest content. | `tools/content-schema/quest-authoring/samples/completion242/recipes.json`, `oteryn:quest.the_tainted_soul`, s2. `crates/gameplay/items/src/item_craft.rs` provides `carried_cooking_conversion_with_quantity` and `CarriedCookingConversion.target_debit`; the checked durable caller in `crates/gameplay/durability/src/durability/item_craft.rs` still resolves through `carried_cooking_conversion_with_sub_type` and decrements ordinary ingredients by one. qm22 now refuses capture-enabled recipes in that quantity-blind API, so the durable owner still needs to consume the quantity witness and persist its debit. Before that refusal guard, real PG red cases observed 36 antlers remaining from 37 and `NoRoom` for a full backpack. | Partly: the selected physical transformation cannot retain its captured stack semantics. |

M21's nested source opening/leaf pickup and key provenance are implemented on this
work branch. Outlaw source bindings are being restored with the World authoring
tool. Small Ruby already has a physical source and a real production-reset
fixture in `postgres_tests/l6_world_pickups.rs`. Skeleton Decoration's Tyrsung
barrel is the existing personal claim UID 6216 in the container-source packet;
it must not also mint a second shared reward. This report does not infer complete
quest readiness from those engine facts.
