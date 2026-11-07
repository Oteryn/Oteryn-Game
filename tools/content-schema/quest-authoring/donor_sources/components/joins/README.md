# Component associations: source evidence, not completed quests

Offline builder uses the assignment, canonical definitions, normalized Source bundle,
existing graph crosswalk and verified pinned corpus bytes. No network, Lua execution,
Native admission or canonical Quest mutation occurs.

```
python component_crosswalk.py --repo-root REPO --assignment ASSIGNMENT.json --corpus-manifest CORPUS.json --out crosswalk.json
```

The corpus manifest supports absolute cache paths or paths relative to its parent.
All 248 input components are accounted for exactly once and all 352 canonical quests
remain in the report. Baseline donor variants remain separate source/revision/path IDs.

`ACCEPTED_SOURCE_ASSOCIATION_PARTIAL_SEMANTICS` means a pinned lexical accessor target
matches an existing declared Quest track, or an exact source path/Git blob already has
an existing Source graph→Quest association. It does not establish actual accessor
receiver dispatch, callback coverage, activation, exclusive ownership or completed quests.
The existing progress occurrence is an anchor, not proof that the new file executes it.
Directory candidates are explicitly unproven and cannot promote accepted ownership.
Generic callbacks, BossLever factories and helper files are not counted as independent
quests. Classification as globally shared nonquest logic would need separate proof.

Results: 248 components: 2 accepted partial Source associations (Secret Library Brokul
lever variants), 201 directory candidates only, 45 unlinked. 32 quests have directory
candidate components; 1 has accepted association. 45 unlinked components include
Soulpit/other folders with no canonical candidate. Zero absent pinned source blobs.
Every file still has an explicit whole-file semantic coverage hold. Existing quest
missing_data is preserved verbatim; this report does not assert quests without affected
components are complete. Runtime owner assignments are not inferred from Source gaps.

Tests (9): masked comments/strings, exact lexical storage arguments, Unicode span,
custom accessor lexical-only, all248/all352, candidate promotion rejection,
completion guard, pinned proof fence, closed-schema negatives. Set
QUEST_COMPONENT_REPO_ROOT / QUEST_COMPONENT_ASSIGNMENT / QUEST_COMPONENT_CORPUS_MANIFEST
to replay real-packet tests with different local cache paths.
