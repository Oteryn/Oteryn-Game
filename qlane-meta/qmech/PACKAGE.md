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
| reward / achievement claims | engclaim | 1830-1839 (unused) | 0 full; 4 reward-binding prerequisites | PARTIAL: native placement / once leaf+container / achievement slice; independent outputs and quest bindings pending |
| world change | engworldchg | 1840-1859 | 10 | open |
| NPC quest dialogue | engnpc2 | 2290-2299; none used | Twenty Miles equipped-head prerequisite; M11 The Dream Courts / Stricken Soul name branch | 88d39c9d9 merged; M11 eb9aae7e3828e112982be054de300827085ae039; source two-confirmation/name conversation, locked current-name predicate, PG 6.72541493s; full Dream Courts quest remains outside this mechanic |
| quest items | engitem | 2250-2259 (new);1641 legacy | Oramond M12; Goblin/Seal/Targuna M13 | M12 engine slice01b524d4d PG GREEN; content wiring continuing; M13 next |
| other blockers | (to assign) | - | 44 | open |
