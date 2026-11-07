# Close 89 NPC progress Source target holds

Bounded fix to existing all-NPC builder, no product writes, new fetches, wiki,
interpreter rewrite or runtime promotion. Reused existing corpus and loose AST.

Before: 4290/4379 exact Source write joins, 89 held target-equality occurrences.
After: 4379/4379 exact Source write joins, 0 held occurrences (4359 unique writes;
20 are canonically shared occurrences). 80 Quest keys,633 files,752 function contexts.
All89 were aliases declared directly inside creatureSayCallback, rather than file
root: storage, S, questStorage, TheNewFrontier and UnnaturalSelection. This was a
bounded Source matcher limitation, not missing donor files or canonical divergence.

Exact new closures by Quest key:
- oteryn:quest.outfit_and_addon_quests:8
- oteryn:quest.rottin_wood_and_the_married_men_quest:16
- oteryn:quest.the_inquisition_quest:3
- oteryn:quest.the_isle_of_evil_quest:24
- oteryn:quest.the_new_frontier_quest:4
- oteryn:quest.the_paradox_tower_quest:6
- oteryn:quest.unnatural_selection_quest:28

Function aliases accept a sole direct-body LocalAssign with literal Storage path,
exact Source declaration/function/event node witnesses, before event use. Competing
assignments, branch/nested declarations, parameter/method/loop-variable shadows
reject alias promotion. Top-level alias matching also rejects numeric/generic loop
shadowing and lexical Storage root redefinition or Storage-root/property mutation. Source table property mutations do
not establish runtime values; this remains a syntactic Source name association.
New89 joins additionally check Source AST literal to-value against canonical write.to.
File/Git hashes, canonical Source path/pin/line hash and AST identity checks remain.

34 meaningful controls PASS (20 scoped-shadow controls plus14 real-packet controls).
Includes wrong targets/line families, loops at both scope levels, nested method/local
shadow, early use, conditional competing writes, dynamic RHS, root Storage shadow,
literal-to checking, callback context bounds, no native promotion and no whole AST copy.
Full633-source generation PASS with existing loose AST cache. No duplicate AST storage.

Product integration copies builder/schema/two tests and adds scoped test class to
its existing top-level unittest wrapper. The packet was generated with original
manifest provenance, so Root regenerates against its portable manifest as usual.
Tests support adjacent scratch fixture and existing product refinement fixture path.
Builder API/CLI unchanged: build(...,all_source_quests=True), --all-source-quests.
Inputs continue to hash source_data semantics only, excluding full canonical shard
and recipe hashes, avoiding circular thin recipe refs.

These counts certify Source attribution/context joins only. They do not close all
Quest data, NPC dialogue reachability, helper execution, runtime binding or complete
quests. native_admission=false, quests_complete=false, runtime_readiness=NOT_ASSESSED.
