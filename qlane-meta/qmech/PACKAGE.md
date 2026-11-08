# QMECH — quest mechanics package

One branch and one PR (`claude/happy-davinci-ijz3wy-qmech`) collect every
missing server mechanic that blocks ready quests. The package is taken to the
Molehill PC, merged with main, validated there, and only then marked ready.

## Rules for lanes writing here

- Push only to `claude/happy-davinci-ijz3wy-qmech`. Before every push:
  `git fetch origin claude/happy-davinci-ijz3wy-qmech` and merge it; on a
  non-fast-forward rejection fetch, merge, re-run your focused checks, push again.
- Do not merge origin/main; the coordinator does that on the PC.
- Use only your pre-assigned migration range. One commit per mechanic, each
  with its own red-then-green test and its own PG cases.
- Do not edit another lane's files. Shared registries: additive edits only.
- After every push paste the `git ls-remote` line.

## Members

| mechanic | lane | migrations | quests unblocked | state |
|---|---|---|---|---|
| outfit / addon grants | engoutfit | 1820-1829 | 47 | open |
| boss / encounter | engenc, engenc2 | (lane's range) | 27 | ENGENC2: shared native phase foundation; Cults production closure claimed, not DONE; ENGENC: tested private boss loot imported (Soul War/Rotten Blood); native arena reused; MISSING-ENGENC.md PARTIAL |
| reward / achievement claims | engclaim | 1830-1839 | 14 | open |
| world change | engworldchg | 1840-1859 | 10 | open |
| NPC quest dialogue | engnpc2 | (lane's range) | - | open |
| quest items | engitem | 1640-1659 | clock waits; Nomads Land text | imported tested clock/text; MISSING-ENGITEM.md PARTIAL; cycle MINT WIP |
| other blockers | (to assign) | - | 44 | open |
