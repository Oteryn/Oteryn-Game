# WRITE-0 Books, scrolls and blackboards

- Decision: `WRITE0-WRITABLE-AND-READABLE-ITEMS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  security and privacy, protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the base-mechanics close-out plan (#162 5929069698) and owner answer 6a (#162
  5929192803): moderation as in Tibia, "teksty graczy można zgłosić i usunąć". GAME-INTERACTION-01
  successor §19.4 blocks durable writable text until a named owner exists; MAIL-0 §4 became that
  owner for letters and labels only and left "books, blackboards" blocked. ITEM-USE-0 names
  "writing" as a later decision.
- Builds on: MAIL-0 §3.1, §4, §9 (the text store, the write transaction, the `ITEM_TEXT` view and
  `ITEM_TEXT_WRITE`); GAME-ITEM-01 §4.8 (typed state); the WorldProject source profile v2
  (`Document` family; player text is ItemInstance state); WORLD-INTERACTION-0 §3 (reach) and §9
  (per-channel volatile state); HOUSE-CUSTODY-0 (house items are ItemInstances); DUR-03 §16.2
  (`PRESERVE_INSTANCE`) and §39; CHAT-0 (text rules, privacy); FND-02 §20; owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| WRITE-CONTENT-1 | content lane | `readable`/`writable`, `write_policy`, `max_characters` and `write_once_to` on Item definitions from Canary `items.xml`, checked against TibiaWiki (§3); `Document` texts of map-authored readables (§4.3) | ITEM-SEM-2b; WO-1 |
| WRITE-1 | hard (persistence), persistence, security and privacy review | MAIL-0's text store and write transaction widened to every writable ItemInstance, on every reachable location (§4.1, §5); write-once (§5.3) | MAIL-TEXT-1; ITEM-MOVE-2b; BAGS-1; HOUSE-RUNTIME-1 for house items |
| WRITE-MAP-1 | impl, determinism review | writable map-authored objects as per-channel volatile text (§4.2) | LEVER-1 (overlay state); WRITE-1 |
| WRITE-MOD-1 | hard (security), security and privacy review | the text report snapshot and the moderation clear (§6) | WRITE-1; the GM tools decision for its caller |
| WRITE-WIRE-1 | impl, protocol review | capability `ITEM_TEXT_V1` (§7) | MAIL-WIRE-1 |

Tests: a write over `max_characters`, with a control character other than line feed, beyond reach,
or on an item in another character's possession is refused; a write-once item transforms once and
keeps its identity; a retry of the same write returns the first result; reported text survives the
writer's later rewrite in the report snapshot only; no text reaches a log, audit event or analytics.

Later, each with its own decision: house door lists (HOUSE-ACL owns them), the GM tools that call
the moderation clear (owner answer 6a), character and guild names (Platform), Store-bound items.

## 1. Question

Which items can players read and write, where can they write them, what is stored, and how is
player text reported and removed?

## 2. Facts

**PROVEN**

- MAIL-0 §4: `game_item_texts` (`item_instance_id` PK, text, `writer_character_id`, `written_at`,
  `text_revision`); UTF-8, at most `max_characters` scalar values, line feeds allowed; the write
  is one transaction under the composition rule 2 fence and `character_root`; a typed item state
  change outside DUR-03 value audit; text is never logged; at most `MAIL0-RL-06` (2) writes per
  character per second. "Other writable items (books, blackboards) keep GAME-INTERACTION §19.4's
  blocker."
- MAIL-0 §9: USE opens the `ITEM_TEXT` view (handle, text, `max_characters`, writable, writer's
  current name, time); `ITEM_TEXT_WRITE {handle, text}` with `OK`, `NOT_WRITABLE`, `TOO_LONG`,
  `EXHAUSTED`; both under `MAIL_V1`.
- WorldProject source profile v2: the `Document` family holds authored texts; "player-written text,
  writer identity, write timestamp and current mutable body remain ItemInstance/durability state".
- GAME-ITEM-01 §4.8: typed, bounded state only.
- Tibia manual (`CIPSOFT_OFFICIAL`): books, scrolls and letters show a generic description on Look
  and are read or written with Use (`controls.md:32`); editable items (blackboards, letters,
  scrolls) show the last editor's name (`controls.md:53`); letters record the last editor and time
  (`world.md:50-51`).
- Owner answer 6a: GM roles, audited actions, Rule Violation reports; player texts can be reported
  and removed.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- `Game::playerWriteItem` (`game.cpp:5535-5597`): the text must not exceed the definition's
  `maxTextLen`; the item must exist, not belong to another owner, not be in another player's
  possession, and be within one tile on the same floor; a changed text sets TEXT, WRITER and DATE;
  an empty text removes all three; a definition with `writeOnceItemId` then transforms.
- `items.xml`: 100 `writeable` and 2 `readable` definitions; `maxTextLen` values 64 to 3,997
  (most 1,023, 199, 99, 149, 511); 19 `writeOnceItemId`.
- The text window is opcode 0x96 (`protocolgame.cpp:8965-8998`); the map is not saved, so the text
  of a writable map object outside a house is lost at a restart.

## 3. Definitions (WRITE-CONTENT-1)

- `readable` (has text to show), `writable` with `write_policy` `rewrite` (MAIL-0 §3.1) or
  `write_once`, `max_characters` (Canary `maxTextLen`, at most `WRITE0-RL-01`), and for
  `write_once` the definition `write_once_to` (Canary `writeOnceItemId`).
- A writable definition is never stackable (content validation): text belongs to one item.
- MAIL-0's letter and label keep their definitions and the stamp rule.

## 4. Where text lives

### 4.1 ItemInstances (WRITE-1)

Every writable ItemInstance uses MAIL-0's `game_item_texts` store unchanged: the text belongs to the
item and travels with every TRANSFER, trade, mail, depot and house move, and survives
`PRESERVE_INSTANCE` transforms. MAIL-0 §4 is widened from letters and labels to every writable
definition. Amended: MAIL-0 §4.

### 4.2 Writable map-authored objects (WRITE-MAP-1)

A blackboard or sign that is a map-authored LocalObject (not an ItemInstance) keeps its text as
**per-channel volatile state** (WORLD-INTERACTION-0 §9.1): one text per object per channel, lost at
a channel restart and at the planned reset, as in Canary (the map is not saved). At most
`WRITE0-RL-02` such texts per channel; a write beyond it is `EXHAUSTED`. A blackboard inside a
house is an ItemInstance (HOUSE-CUSTODY-0) and is durable by §4.1.

### 4.3 Read-only texts

A `readable` map object or item with authored text (a library book, a notice) shows its
`Document` text from the content bundle. It is never player state and never writable.

## 5. Writing (WRITE-1, WRITE-MAP-1)

### 5.1 Who and where

The write is refused unless all hold (Canary `playerWriteItem`):

- the item is writable and not stamped (MAIL-0) (`NOT_WRITABLE`), and the **new** text is within
  `max_characters` under MAIL-0's text rules (`TOO_LONG`, `REJECTED` for a control character);
- the item is reachable: an entry of a container tree the character holds or has open (BAGS-0
  views, depot boxes while the depot view is open), an equipped slot, or a Ground item or map
  object within one tile on the same floor (WORLD-INTERACTION-0 §3.2); otherwise `NOT_REACHABLE`;
- it is not inside another character's possession or a container offered in a trade, and not an
  item with an owner other than the writer (a reward chest item); otherwise `NOT_REACHABLE`;
- on a house tile, the character is inside the house (HOUSE-RUNTIME-0); no further house right is
  checked, as in Canary.

### 5.2 Transaction

- ItemInstance: MAIL-0 §4's transaction. It always takes the writer's composition rule 2 fence and
  `character_root`, then the item row `FOR UPDATE` (serializing writes and moderation clears of one
  item), and **in addition** the fence of the item's location when it is not the writer's own: the
  §32 scope fence and the tile row `FOR SHARE` for a Ground item, HOUSE-CUSTODY-0's house fence for
  a house item. Keyed by the command's CommandRef; a retry returns the first result.
- **`text_revision`** (MAIL-0 §4) increases by one with every committed write that changes the text
  (a clear included) and never otherwise; an unchanged text writes nothing and keeps it (Canary).
- Map object: in the channel owner's turn, no transaction. Its volatile text carries a volatile
  `text_revision` with the same rule, scoped to the channel scope ownership generation.
- Rate: `MAIL0-RL-06` (2 writes per character per second) for both.

### 5.3 Write-once

A `write_once` definition transforms to `write_once_to` in the same transaction as the write: one
DUR-03 `TRANSFORM` line, `PRESERVE_INSTANCE` (§16.2; identity, location and text kept), under the
closed cause `WriteOnceCause` keyed by the CommandRef, as MAIL-0's stamp does. As in Canary, every
accepted write to a `write_once` item transforms it, even an unchanged or empty text. The versioned
write-once rules are content (WRITE-CONTENT-1). The target is not writable. No value line. Amended:
DUR-03 §39.3.

## 6. Reports and removal (WRITE-MOD-1; owner answer 6a)

- **Report.** A player who reads a text can report it (the Rule Violation flow of the GM tools
  decision). The report captures, server-side, a **snapshot**: the item or map object, its
  `text_revision`, the text, the writer CharacterId and time. The snapshot lives in a restricted
  moderation store, readable only by the moderation role, kept for `WRITE0-RL-03`, never logged.
  A later rewrite does not change it.
- **Removal.** A moderation **clear** deletes the text row if its `text_revision` still equals the
  reported one, under the item row lock of §5.2, else refuses with `STALE` (the moderator sees the
  newer text through a new report). For a map object the snapshot also records the channel scope
  ownership generation; the clear applies only while that generation and the volatile
  `text_revision` both still match, else `STALE` (after a restart the text is already gone). The
  result set beyond `OK` and `STALE` belongs to the GM tools decision. It is an audited
  moderation action (actor, item, revision, reason; never the text) under the GM tools decision's
  authority and fences; WRITE-0 fixes only its shape.
- Nothing is filtered automatically (Tibia has no text filter for books).

## 7. Wire (WRITE-WIRE-1; amends MAIL-0 §9)

- **Capability `ITEM_TEXT_V1`** carries MAIL-0's `ITEM_TEXT` view and `ITEM_TEXT_WRITE`;
  `MAIL_V1` requires it. Its number is reserved on #162 at allocation.
- USE on any `readable` or `writable` item or map object the session reaches opens the view; a map
  object is addressed by its map item handle.
- `ITEM_TEXT_WRITE` gains the result `NOT_REACHABLE` (§5.1) beside MAIL-0's results.
- A report is a GM tools command, not part of this capability.

## 8. Rows

| Row | Value | Note |
|---|---|---|
| `WRITE0-RL-01` `max_characters` of a definition | 4,000 scalar values (16,000 bytes) | content validation; Canary's largest is 3,997 |
| `WRITE0-RL-02` volatile map texts per channel | 4,096 | above it, `EXHAUSTED` |
| `WRITE0-RL-03` report snapshot retention | 180 days | then deleted |

Each with max and max+1 tests.

## 9. Rejected options

- **Text as a free attribute.** GAME-ITEM-01 §4.8; MAIL-0's typed store exists.
- **Durable text on map objects.** Canary does not save it; a durable store for every map object
  per channel would be new state without a Tibia basis.
- **An automatic word filter.** Not Tibia; owner answer 6a chose reports and removal.

## 10. Architect rulings (owner rule 5905825574)

- **R1. Map blackboards.** a) Volatile per channel, as Canary (recommended); b) durable. **Ruled
  a).**
- **R2. Reach.** a) Canary's one tile and possession (recommended); b) only items the character
  holds. **Ruled a).**

## 11. Owner questions

None. Owner answer 6a decided moderation; the rest applies Canary and the Tibia manual.

## 12. Decision test

- **Must decide now:** YES. GAME-INTERACTION-01 §19.4 blocks every book, scroll and blackboard.
- **Minimum sufficient:** MAIL-0's store and command widened, one volatile map store, one report
  snapshot and one clear.
- **Superseding evidence:** official evidence that map texts persist in Tibia.
- **Deliberately not decided:** the GM tools themselves, house door lists, names.

## 13. Before-freeze checklist

1. **Contract amendments:** MAIL-0 §4 and §9; GAME-INTERACTION-01 successor §19.4 (owner named);
   DUR-03 §39.3. Applied in this PR.
2. **Serialization:** the item's location fence and `character_root`; map texts in the owner turn.
3. **Restart:** item texts durable; map texts volatile by rule.
4. **Typed references:** ItemInstanceId, map item handle, CharacterId; text never in logs.
5. **Wire:** one capability that `MAIL_V1` requires; one new result.
6. **Split work:** none.
