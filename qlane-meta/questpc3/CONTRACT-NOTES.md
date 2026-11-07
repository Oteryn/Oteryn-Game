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

## Threatened Dreams finite inventory exchanges

Retained Crystal tree9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d in tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz is the source. aurita.lua:81-88 (blobebaf7b39b5d59d1642d60a9dbe763219dda00724; sha256d8a1fe20f4ec11245498191b0269ca76ca5949c92db529e640faacc87a57ea35) gives one25731 atcouple4/EmptyVial<1 without a fee. taegen.lua:174-188 (blobbf435e06539adfba12820ba042310d76da4c630e; sha256cd1bd27585798ae5a732a30c5a6f96f4ec6e8400ad4e2854d1dda945b040e74f) takes exactlytwo each675/676/677/678 and gives25733 attopic3/DarkSunCatcher<1. valindara.lua:75-90 (blob572697b6e855387cefc8e6336ea55dd6ee304678; sha2568cd64f9b2e7d678e91a07485789f4b08c168b7f52d1c741ecb77cfca3fbeb4fa) offers cloak atcouple4, then takes100of26181 and gives25779 attopic1. Preserve the existing canonical couple track mission03_1, historical initialzero and the source's funded cloak re-offering. These optional exchanges do not globally complete the quest.

Canonical portable evidence is content/items/definitions/items-14500-14999.json:1 records19/21/69 at07eb09f31 (sha256436459e353077c46e6fd14bb074bde396f8865e8c3264fd15655e73702fccb59). Vessels25731/25733 weigh1200 and cloak25779 weighs400, with explicit movable/pickupable and nonstacking facts. Admission changes only materializable and stack_class; no other Item fact is invented or overwritten. questpc3_item_admission.py content is the targeted canonical refresh; the same strict hook remains in full tree generation and roundtrip validation. No family count or shard reference changes, so no unrelated legacy World rebuild is needed. The three native NPC positions reuse existing promotion candidates; Valindara's absent source facing uses the converter's existing north default.

Shared greeting documents are sorted by canonical NPC key in each producer. This makes the Beginning, Pilgrimage and Threatened generators independent of invocation order while preserving all foreign records.

Threatened inventory native policy and topic composition: canonical materializable Item facts alone do not supply the runtime native mint policy. Reuse quest_callback_profiles to include every NPC output and container child, while adding admitted take/present evidence without overriding existing native input owners. Legacy Threatened prompts and shared physical rewards now share settled context-bound numeric topics; a completed shared reward invalidates stale legacy affirmatives. The donor NPC file:line/blob/SHA citations remain on each authored callback.
