# questpc3 contract notes

Owner cadence: publish each finished tested mechanic slice immediately; maximum30 minutes between pushes. Investigations and missing facts are bounded to10 minutes. If a source gap remains at that limit, record the selected most likely behavior here and implement it; do not wait on another lane.

## Beginning: initial Santiago greeting

Use retained Crystal source at9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d, data-global/npc/santiago.lua:63-72. Corpus source blob c55210f3b5ccaa5437f8f22148c91b5134f52880, SHA256 e83d14b26fb9e6bca12cd3ec50abe34daaeb56f2719b5a1dee3d8efc4402a1bd, in tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz. Classification OTS_HYPOTHESIS_ONLY. Facts were found within the investigation budget; no guessed completion rule is needed.

The first successful greeting checks SantiagoNpcGreetStorage below1, then sets it and SantiagoQuestLog to1 in the same quest transaction. Reopening the conversation cannot restart that write. Use the existing NPC greeting vocabulary and fenced common quest dialogue executor. This state-only greeting has no inventory, currency or XP authority.

Santiago's later greeting at source santiago.lua:88-99 consumes three physical item7882 legs and grants100 experience. A death counter is insufficient proof for that hand-in. The item requirement, source progress and XP obligation must compose atomically when that slice is authored.

## Pilgrimage of Ashes: chosen recipe and free grants

The canonical chosen recipe is content/quests/definitions/quests-00100-00199.json:1#/records/85/definition/oteryn_recipe/payload/recipe at5e72a3f0b. Preserve its fixed Guide → Norf → Humphrey → Edala → Kawill → Pydar → Eremo → Guide route, Premium requirement, once-only completion and20 Platinum Coins. Current owner instructions require actual blessings despite the historical recipe's dialogue-only adaptation.

The generic Guide NPC target means the existing named town guides (guide_alexena through the other canonical guide identities in content/npcs/definitions/npcs-00500-00999.json). No Dawnport NPC is included. These are an interchangeable guide archetype: start/completion may use any member. This resolves the unspecified guide identity within the10-minute investigation budget.

Grant Spiritual Shielding at Norf, Embrace at Humphrey, Fire at Edala, Spark after Pydar (Kawill is the preceding visit), and Wisdom at Eremo. Match the existing rulesets/character/blessings/catalogue.json Service/NPC identities exactly. Each new blessing and quest stage share a fenced Quest receipt, with a separate audited quest-grant provenance accepted alongside existing paid purchase provenance. Paid purchase pricing remains unchanged. An already-owned blessing is retained with its original provenance; the required visit may advance without a duplicate grant or fee. Require current Premium through the existing transaction-aware Premium owner. Missing/expired Premium or a mismatched stage/keyword writes nothing.

Selected migration number1096 was checked against origin/main91afbe0ba1 and open PR claims with the repository number tool. Recheck before publication; never modify an existing migration to add this mechanism.

## Tutorial native occurrence bridge

Pinned Crystal201189f Santiago.lua:88 consumes three physical cockroach legs at Greet6; no donor hook writing5to6 was recovered in the bounded source investigation. Authored conservative rule: three eligible native Cockroach kill-credit occurrences after the club lesson (Greet5) advance Greet6. Deaths never mint legs, claim hand-in log5 or pay100XP. The existing native lethal queue captures membership and mission revision; the same fenced quest writer deduplicates occurrences. A generic bounded SetWhenAtLeast effect references another original locked effect value, preserving its own value until threshold. Other native counters retain existing hashes and stage1 gates. The physical hand-in remains a separate exact3legs transaction.

Pilgrimage presentation and placements: the canonical definition at records/85 is script_only with shown_in_quest_log=false, so completion retains that hidden journal presentation. All18 participating NPCs already have placement candidates in tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json:1 @b7f139977, sha256=1f9b6ebbd79d92fbcb4d6e522c6dc30c95ad83ca6f4b0f5030ee81fb2f47635a. Admit those reviewed candidates through convert_placements.py for the complete route; do not invent coordinates or use Dawnport.

## Zirella instruction path

Crystal201189f data-global/npc/zirella.lua:104-143 provides six paired progress writes after the keyword introduction. Shared NPC callbacks now preserve that sequence, resume the right topic from each stage, and repeat the stage6 instructions without a receipt or XP. The delivery and50XP reward remain separate operations. The canonical Oteryn recipe selects five Wood; the eventual authored ground-branch/cart binding must retain that selected count while following the source ground custody instructions. No tree/cart source IDs or placement were recovered, so author them explicitly through the existing World/Item owners.
