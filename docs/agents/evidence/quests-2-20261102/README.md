# Quest batch 2 — per-quest review snapshots

Each `<quest>.json` is a self-contained authoring review packet for one quest's
commit. `source_row` preserves the complete inventory row and
`source_disposition` preserves its author's delivery decision. Normalization does
not rewrite a source fact, remove a limitation, or certify a complete playable
quest. This directory is evidence; it is not loaded as game content.

All 21 quest packets are present: **1390 source rows**, comprising **172 authored
subsets**, **4 explicitly partial callback cores**, **20 preserved existing
consumers/data rows** and **1194 Left out**. The source labels and limitations in
each packet govern what those counts mean.

The four normalized statuses have narrow meanings:

| Status | Meaning |
| --- | --- |
| `IMPLEMENTED` | The explicitly named code or data subset was authored. The original validation claim and remaining restrictions still apply. |
| `PARTIAL` | The source disposition explicitly calls its authored callback core partial; presentation or physical admission remains excluded. It is not promoted to complete implementation. |
| `EXISTING` | The inventory identifies an already existing consumer or qualified data which this batch preserves. This is not counted as new implementation. |
| `LEFT_OUT` | The element remains deferred for the author's recorded reason and linked QUESTION/ANSWER. |

`delivery_kind` distinguishes NPC code, callback declarations, callback cores and
existing data/consumers. Source status strings are retained. A callback with a
missing physical tile can refuse; authored callback data does not establish its
WorldBundle admission. Likewise, a source dialogue capture does not itself admit
an NPC or its durable rewards.

Every row has witnesses for its exact pinned donor file and line range: the whole
blob SHA-256, line count and excerpt SHA-256. Input document digests identify the
frozen lane packets used at authoring time. All original row values needed for
later checks are embedded; no check depends on an author's scratch directory.

## Check the committed snapshots

The default demands all 21 quest files and rejects duplicates, missing source
rows, changed normalized claims, invented pattern IDs and unlinked Left out:

An intentionally unassigned pattern is preserved only when the original row has
an explicit `NOWE WZORCE` note. The summary counts such rows without inventing a
new Pxx identifier.

```sh
python tools/content-migration/quests_2_review_snapshots.py check
```

That command checks internal evidence and coverage, and explicitly reports that
donor bytes were not reread. To also verify every witness against pinned Git
objects, supply the canonical donor checkouts:

```sh
python tools/content-migration/quests_2_review_snapshots.py check \
  --crystal /path/to/crystalserver --canary /path/to/canary
```

The checker reads `git show <pinned-commit>:<path>`, not a mutable working-tree
file. It never executes donor Lua or fetches a network resource. An explicit
smaller `--expected-quests` remains available for intermediate assembly, but is
not a successful 21-quest coverage gate.

## Authoring provenance

The `capture` operation accepts `--lane-root` as an explicit temporary input and
joins each inventory to its owner's frozen disposition. It checks every joined
row and source before writing any packet. Classic, emperor and soul adapters
retain their different original schemas. Root-authored packets are normalized
from their embedded `inventory` and disposition; both remain in the result.
This import command is not needed to check committed evidence.

```sh
python tools/content-migration/quests_2_review_snapshots.py capture \
  --lane-root /path/to/frozen-lane-packets \
  --crystal /path/to/crystalserver --canary /path/to/canary
python tools/content-migration/quests_2_review_snapshots.py capture-root \
  --root-packet /path/to/threatened-final-status.json \
  --root-inventory /path/to/threatened-inventory.json \
  --crystal /path/to/crystalserver --canary /path/to/canary
python -m unittest discover -s tools/content-migration \
  -p test_quests_2_review_snapshots.py -v
```

The binding answers are preserved as per-row links. Global comparisons remain
`NIEZNANE/API403`; these packets introduce no Global-only mechanics or values.

The final Threatened Dreams packet preserves **116 rows: 26 implemented NPC
rows, 4 partial callback cores and 86 Left out**, with **AUTO 4 / RĘCZNIE 112**.
Its final inventory is checked against every embedded source row before writing;
`capture-root` changes only that quest's packet.

The final source-only closure adds five Threatened Stone/rake dependency rows and
one Wrath clay/rake dependency row from the already pinned `rake.lua`. All six
remain `LEFT_OUT`; no runtime rule is added by this evidence delta. Wrath has
**151 rows: 32 authored NPC rows and 119 Left out**. The two Threatened chest rows
are explicitly `RĘCZNIE`, because their combined item/QuestState owner remains
deferred. Root-declared AUTO/manual and disposition counts must now equal the
embedded rows; the checker rejects a contradictory header.
