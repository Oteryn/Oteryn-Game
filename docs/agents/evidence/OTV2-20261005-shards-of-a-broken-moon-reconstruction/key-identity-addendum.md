# Asura Citadel key identity addendum

Status: **EXACT IDENTITY; ITEM ADMISSION / DOOR RUNTIME PENDING**

This current disposition supersedes the earlier DERIVED identity hold. The stronger witness is retained in `item-successor-admission-candidate.json` and `asura-citadel-door-amendment.md`.

On main@3bacc59e6adbf138655e6f0c1cf0b4fdf9a36096, `imports/crystalserver/bindings/items.json` contains the exact Summer 54262 -> `oteryn:item.tibia.i54262@definition-r1` binding. The existing canonical Item remains `materializable=false`. Identity resolution does not imply item delivery or door activation.

Pinned Crystal Summer has no numeric `keyNumber` or door-table entry binding the key to the Asura Citadel door. The source door is 53380 at `33899,32666,8`. Do not derive a numeric key from the item id.

The Door/Key owner must compose identity-based i54262 USE-WITH during the quest and a read-only Shards completion bypass. `ITEM-USE-1` is also required because current transport rejects USE-WITH. Do not add an invented QuestGate OR or a private dispatcher.

Lost-key regrant stays with the qualified Javala branch in `binding-plan.json`; Item/Durability performs the grant, and NPC producers use the existing `QuestTransitionRequest` writer. No direct QuestState write is permitted.
