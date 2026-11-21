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
