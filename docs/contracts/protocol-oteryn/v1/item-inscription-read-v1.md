# ITEM_INSCRIPTION_READ_V1

Capability 511 (requires ITEM_VIEW_MOVE_V1 and MAIL_V1) gates command 6246. It lets the
writing window open with an item's current text, as Tibia Global sends the text and the maximum
length with the window and shows a read-only item's text without accepting a rewrite. Without
it a client cannot know a carried item's text and must not send a MAIL `WriteText` replacement
for an item it cannot prove blank.

The command is read only: no text, revision, receipt, rate token or session generation changes.
Unselected, unfenced, malformed, foreign, not carried and not readable requests are rejected
with an empty result. Existing wires remain unchanged.

## Request

UTF-8 JSON, at most 64 bytes, closed fields: `{"item_handle":7}`. The handle is a nonzero
ITEM_VIEW handle of the session; it must resolve to an item instance. Custody is the MAIL text
writer's: an equipped item, a main-backpack entry or an item nested below one, read in one
snapshot under the session's current item fence.

## Reply

UTF-8 JSON, at most 32768 bytes, closed fields, as `item_inscription::Reply` defines:
`{"text":"...","max_length":1999,"writable":true}`.

- `text` is the stored text, empty when the item has none; at most `max_length` characters,
  no control character other than newline.
- `max_length` is the item's content maximum text length, 1..=4000.
- `writable` is true exactly when the MAIL text writer's content policy admits a rewrite of the
  item's definition; a readable item that is not writable reports false.

## Attribution reply

Bundle-world capability 596 requires 511 and gates command 6356. Use the
same request, custody fence and size limits as command 6246. The reply has
the original three fields and may also carry these fields.

- `writer_name` is the stored writer Character's current display name
  (1–64 UTF-8 bytes, no control characters).
- `written_at_unix_ms` is the stored write time as a signed integer in Unix
  milliseconds, rounded down.

An item without stored text omits both fields. A system-authored inscription
can have a timestamp without a writer. A non-finite historic timestamp omits
the attribution fields. The writer and timestamp come from the same fenced
snapshot as the inscription. Command 6246 always uses its original closed reply
schema, including when capability 596 is selected.

[Crystal 201189f8, ProtocolGame::sendTextWindow](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/server/network/protocol/protocolgame.cpp#L8481-L8515)
