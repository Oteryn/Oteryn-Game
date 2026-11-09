# M07 Lion trial slice

M07 remains PARTIAL: Dreamer's Challenge's carrot, Fire Bug, Death Ring entrance
and shared tree restoration still require their native callers and durable item
participation. Only the Cobra Tongue offering is registered and PostgreSQL-qualified in this slice.
The flower and water bindings are captured but await native Item admission.
This slice provides the shared pagoda reset owner;
it does not qualify either full quest or the Lion treasure cache.

Reference: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/scripts/quests/lions_rock/actions_lions_rock.lua:13-79`.
The native target ordinals and pagoda positions are recorded in
[the retained native trial evidence](questpc4-M07-native-trials.md).

| Source rule | Implementation |
| --- | --- |
| Cobra Tongue 9634, Lion's Mane Flower 21389, Holy Water 21466 at their literal targets | Closed `lions_rock` callbacks, including bare B3 counterparts, through the existing world-object USE dispatcher |
| Personal trial flag below one | Locked native Quest rule; a completed flag rejects a new offering |
| Questline starts at two, then advances by one | `max(questline, 1) + 1`, within the existing Tibia Tales track owner |
| Consume exactly one carried input with progress | Existing fenced Quest callback transaction, physical item consumption proof and audit |
| Only unlit pagoda 2074 accepts an offering | Current Channel facts checked before the durable call; remote revision checked again at atomic publication |
| Pagoda 2075 restores to 2074 after 15 seconds | Existing Channel decay owner, with the committed remote revision and deadline anchored after durable success |
| Shared readiness does not reset personal completion | Separate map decay and durable character tracks |
| Replay does not consume again or extend the timer | Existing command replay plus revision-fenced projection |

No new migration, wire command, capability or hand-edited generated pin is needed.
The existing session, content, map, reset and Item fences remain in the callback
owner. The new auxiliary trial flags retain `oteryn:quest.tibia_tales` ownership.

The mounted dispatch PostgreSQL cases use synthetic source geometry to exercise
real USE, not SQL-seeded quest progress. They qualify transaction and projection
behavior, not production-map collision or a complete quest route. World unit
coverage also checks two competing sources and a newer intervening map revision.

Validation limitation: the unchanged `published_ungated_quest_hash_baseline`
test fails for Ice Islands (`ca68476c…` observed, `ce91fb47…` expected), including
with the base loader and callback registry restored. Its assertion is unchanged.
The provisional flower and water PostgreSQL cases fail with `DefinitionNotFound`;
those cases and their activation remain unfinished working-tree changes.
