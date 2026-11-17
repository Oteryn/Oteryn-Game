# Owner-corrected filtered Wound context

Read2026-11-13 at main `3e168ce6afe374485da20ee0ac13d3749558e2f8`.
The owner correctly identified that the earlier Discord pass was shallow and
first-window-only. It was not an exhaustive search of these communities.
This follow-up preserves the actual prepared query/filter state and opens two
selected conversations rather than inferring mechanics from search snippets.

Discord displayed nine results with five channel filters: tasks-and-bestiary,
content-updates-discussions, polish-chat, monk-channel and english-chat. The
combobox DOM contains `wound`, these five channel tokens, and trailing `charms`.
The JSON retains a readable normalized transcription. This must not be
described as a separately submitted server-wide `wound`-only query.

The decisive contextual distinction is:

- [Verilo / Antarya,1Dec2025,08:20](https://discord.com/channels/836592213327609957/1336272311661236286/1444951371735371909)
  denies an effect, including Wound. His reply explicitly references
  [Grooszka's08:16 question](https://discord.com/channels/836592213327609957/1336272311661236286/1444950271888330753)
  about **Crypt Strike armor penetration** and Carnage. It is a community
  answer without supplied controlled measurements in this window, and cannot
  automatically answer the separate weapon physical-pierce question.
- [Lewah,18Sep2026,08:46](https://discord.com/channels/836592213327609957/1336272311661236286/1550397624773775381)
  asks directly about **weapon physical pierce** affecting Wound. The opened
  adjacent window contains the question, then unrelated FH-gem discussion;
  no answer to that question was identified in that window. This does not
  establish absence elsewhere. Monkee Shizzaren's17Aug2026 question is retained
  as a matching snippet, with context explicitly unreviewed.

In the1December conversation, [Siconix at08:18](https://discord.com/channels/836592213327609957/1336272311661236286/1444950895019425865)
also asserts Carnage ignores armor/resistance. This conflicts with current
source flags: Carnage has `reduced_by_armor=true`, `ignores_resistances=false`;
Wound has both flags false. No experiment accompanies the assertion in the
inspected window, so this is a specific unresolved source comparison.

Current `attack/charm_combat.rs::damage_amount`, lines228–265, accepts runtime,
target, damage kind, magnitude and armor draw. It applies armor according to
the flag, then the target's elemental response unless resistance is ignored.
There is no attacker equipment-pierce parameter in this function. The inspected
seam is compatible with no direct pierce amplification; it is not a complete
Crypt Strike/weapon-pierce callgraph or Global runtime test. The catalogue
flags and three source blobs were re-derived/hashed at the pinned main.

`discord-wound-context.json` retains four dated source records, three
context-qualified records across two opened windows, source receipts and
explicit limitations. No raw chat/media, game action, runtime patch or mechanic
status promotion is included. The remaining mechanic-specific synonym/filter
searches remain future work; this small follow-up does not claim completion.
