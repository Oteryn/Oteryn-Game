# questpc3 contract notes

Owner cadence: publish each finished tested mechanic slice immediately; maximum30 minutes between pushes. Investigations and missing facts are bounded to10 minutes. If a source gap remains at that limit, record the selected most likely behavior here and implement it; do not wait on another lane.

## Beginning: initial Santiago greeting

Use retained Crystal source at9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d, data-global/npc/santiago.lua:63-72. Corpus source blob c55210f3b5ccaa5437f8f22148c91b5134f52880, SHA256 e83d14b26fb9e6bca12cd3ec50abe34daaeb56f2719b5a1dee3d8efc4402a1bd, in tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz. Classification OTS_HYPOTHESIS_ONLY. Facts were found within the investigation budget; no guessed completion rule is needed.

The first successful greeting checks SantiagoNpcGreetStorage below1, then sets it and SantiagoQuestLog to1 in the same quest transaction. Reopening the conversation cannot restart that write. Use the existing NPC greeting vocabulary and fenced common quest dialogue executor. This state-only greeting has no inventory, currency or XP authority.

Santiago's later greeting at source santiago.lua:88-99 consumes three physical item7882 legs and grants100 experience. A death counter is insufficient proof for that hand-in. The item requirement, source progress and XP obligation must compose atomically when that slice is authored.
