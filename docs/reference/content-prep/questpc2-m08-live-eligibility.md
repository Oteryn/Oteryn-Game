# M08 live NPC eligibility

The NPC receipt transaction must reload level from the locked Character progression row, vocation from Character build state, and Premium for the locked Character account through the existing Premium authority. A preview is advisory and cannot authorize a durable reward. Migration 0030 defines an absent build row as vocation `none`.

Checked reference data already captured in `tools/content-schema/quest-authoring/wiki_quest_facts.json`:

| Quest | Requirement | Reference |
| --- | --- | --- |
| Elemental Spheres Quest | Level 80, Premium | https://tibia.fandom.com/index.php?curid=25542&oldid=1178813 |
| Hot Cuisine Quest | Level 35, Premium | https://tibia.fandom.com/index.php?curid=36952&oldid=843854 |
| The Hidden City of Beregar Quest | Premium; level 100 only for Pythius's Firewalker Boots part | https://tibia.fandom.com/index.php?curid=30690&oldid=843688 |

Pinned donor `zimbadev/crystalserver@9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`: `data-global/npc/arkulius.lua:98-169` checks level 80 and the four base vocation families; `data-global/npc/pythius_the_rotten.lua:53` checks level >99. Source blobs are retained in the repository donor corpus, not copied here. Arkulius's four vocation predicates include promoted members of each family. The donor does not provide a Monk branch; this change must not invent one.

Source URLs:
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/arkulius.lua#L98-L169
- https://github.com/zimbadev/crystalserver/blob/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/pythius_the_rotten.lua#L53

Decision: preserve quest-specific level requirements, with no blanket level-100 gate on Beregar. Eligibility support does not qualify encounter or cooking mechanics owned by other assignments.
