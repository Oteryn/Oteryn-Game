# Quest batch one: reproducible source documents

Run from the Oteryn repository root:

```sh
python tools/content-schema/quest-authoring/quests_batch_one --crystal-root <crystal-repository> --output .
python tools/content-schema/quest-authoring/quests_batch_one --crystal-root <crystal-repository> --output . --check
```

`--quest dark_trails`, `--quest spike_tasks`, `--quest what_a_foolish_quest` and `--quest greetings` select individual output groups; repeat the option to select several. With no selection, all six documents are generated. `--check` compares exact UTF-8 bytes and never writes.

The importer reads only Git objects at Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. It checks that the commit exists and uses `git show <pin>:<path>`, so a dirty worktree or another checked-out branch cannot silently change the source. A pinned `git grep` rechecks the complete NPC storage-reference cohort for each of the three quests. No Lua executes, no network access is needed, and no files from an author's scratch directory are inputs. Literal decoding uses the repository's existing `lua_tables.py` reader.

| Selection | Generated files | Scope |
| --- | --- | --- |
| `dark_trails` | `content/quests/dark-trails.json`, `content/dialogues/quests/dark-trails.json` | Reviewed storage-only bindings and full literal messages from all14 matching NPCs. Unselected source-only replies do not create runtime rules. |
| `spike_tasks` | `content/quests/spike-tasks.json`, `content/dialogues/quests/spike-tasks.json` | Twelve reviewed task definitions and full messages from all13 matching NPCs. Only the separately qualified native officer subset can execute. |
| `what_a_foolish_quest` | `content/dialogues/quests/what-a-foolish.json` | Full source archive only: Bozo table literals and the other16 matching NPCs. No runtime model, transition, greeting or QuestState extension is generated. |
| `greetings` | `content/dialogues/quests/batch-one-greetings.json` | Exact static GREET, FAREWELL and WALKAWAY messages for Ezebeth, Gnomilly, Gnombold, Gnomargery and Mazarius. The donor NPC handler supplies explicit default farewell/walkaway literals where necessary. |

Each reply carries a pinned donor file and line span. The Spike branch table binds explicit reviewed source spans; it does not infer semantics from the nearest keyword. Unsupported dynamic expressions fail extraction. The two bounded exceptions are documented in `source_dialogues.py`: Bozo's dynamic table references are already represented by the complete literal tables; Simon's cookie refusal selects source topic8. Hjaern's unrelated ice-shard gold interpolation remains outside this quest archive.

Preservation does not imply runtime admission. In particular, the source archive retains Barazbaz's original tab character in an unselected notebook response. The active native replies fit the existing protocol's paragraph and line limits. The native caller separately refuses unqualified messages before a state change.

After regenerating these documents, run the repository's ordinary content regeneration/pin and affected-package checks. The generator's byte comparison proves reproducibility from the named donor objects; it does not establish map admission, actor lifetime, durable writer authority or full quest playability.
