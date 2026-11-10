# Guardcatcher native custody

The real quest MINT caller requires a native Item profile for `oteryn:item.tibia.i17669`. The canonical identity already exists; no replacement identity or generated SHA is invented.

Verified source: https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml
The already pinned XML describes item 17669 as `guardcatcher`, primary type `quest items`, weight 800. It has no charges or decay attributes. The existing source importer and pinned client appearances qualify it as non-stackable with CharacterInventory and Ground custody. Canonical facts and the existing Crystal bindings remain authoritative.

Task reference: https://www.tibiawiki.com.br/wiki/Task:Rafzan#Guardian_Angel_-_Kind_Of
The checked task text says Rafzan supplies the bow, three successful Adventurer uses complete the task, and the bow disappears at completion. It also describes an Angry Adventurer failure branch without specifying its probability. This custody admission does not infer a combat weapon, capture chance, failure rate or scripted USE permission. Those require their own dispatch/content qualification.

The PostgreSQL case uses the existing fenced quest reward MINT caller: no backpack refuses without progress, a real equipped backpack permits one native tool, and historical replay does not mint a second tool. Its private grant track is a boundary fixture, not a claim that Rafzan dialogue is wired.
