# Source reference traversal, batch 2

Offline bounded extension of the existing product builder at
`donor_sources/refinements/joins/builder.py`. Integrate this extension at a distinct
path (e.g. `refinements/joins_next`); do not replace its imported baseline builder.

```
python builder.py --repo-root REPO --assignment ASSIGNMENT.json --corpus-manifest CORPUS.json --out exact-source-joins.json
```

Before:17 associated component files,231 unmatched,7 canonical Quest keys.
After:37 associated component files,211 unmatched,9 canonical Quest keys.
**20 additional components** have exact named-reference chains from existing Source
Quest graphs or canonical progress source occurrences. All previous17 associations
are retained. The corpus is reused; no network, wiki, Lua execution or product write.

The bounded traversal follows literal `Game.createMonster("Name",...)` into a unique
same-donor/pin/datapack registered MonsterType, its literal events table, and a unique
registered CreatureEvent file. Named event requests can also propagate transitively.
Each chain node has exact Source hashes and Unicode character/line witnesses. Monster
name ASCII case normalization has separate evidence from the exact pinned C++ registry.
Single top-level local factory/table bindings, an empty initial descriptor table, and ordered events-before-register assignment are required. Competing assignments, parameter/local shadows, event-table aliases/mutation and unknown table escapes are rejected. All literal MonsterType calls, including unsupported bindings, plus XML descriptors still count when rejecting
ambiguous names; all literal CreatureEvent constructors are counted for uniqueness.
Comments, string bodies, computed names/tables and ambiguous registrations cannot create
new named-reference edges. Cycles cannot create owners: only pre-existing Source roots
seed a finite shortest-reference traversal.

New associations include Cults of Tibia Evaporate/Leiden/Splash, Forgotten Knowledge
HealthForgotten/Soulcatcher, Grave Danger Scarlett and Heart of Destruction transformer
mechanics. Some routes have depth2, which is recorded. Literal boss/monster names alone,
folder/title proximity and source directory membership never establish ownership.

`association_scope` distinguishes:
-36 files with nonexclusive Quest Source references;
-1 shared file referencing multiple canonical Quests;
-32 world-event registrations without proven Quest owner;
-81 shared factory instances without proven Quest owner;
-98 other files without proven Quest owner.

World/shared mechanism labels describe captured registration shape, not a declaration
that the component cannot belong to a Quest. Registered events can be optional, disabled
or overridden at runtime. Captured uniqueness does not prove live registry uniqueness.
Source factory/receiver dispatch, player/world storage equivalence and runtime activation
remain unproven. Every file retains semantic-body and activation holds; no Quest or Native
readiness is promoted. Zero Source byte gaps were encountered in this pass.

Input digests retain the baseline stable Source semantic slices and add a canonical
reference-root semantic slice, not whole canonical shards or supplement references.
No cyclic artifact input is added. All248 assignment records remain accounted for.

18testsPASS: literal versus computed names; comment/string exclusion; registered tables;
missing/dynamic event fields; table/factory shadowing, rebinds, mutation, escape, scope and source order; before/after membership; prior associations preserved;
all chain hash/span/pin/pack checks; pinned C++ normalization; depth2 qualification;
world/shared unowned classification; closed schema and Native/complete-Quest negatives.
Test paths can use QUEST_COMPONENT_REPO_ROOT, QUEST_COMPONENT_ASSIGNMENT and
QUEST_COMPONENT_CORPUS_MANIFEST environment variables.
