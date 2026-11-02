# Animus Mastery v1

Capability 168 exposes command 232 to an admitted session. It reads the admitted Character's own Animus Mastery completions. Completions are server-owned and immutable.

## Request

Exactly 4 bytes: tag `0`, a little-endian `u16` offset, and a `u8` limit from 1 to 50. Any other payload is refused.

## Reply

A little-endian `u16` total completion count, a `u8` count of keys in this page, then that many species keys. Each key is a `u8` length followed by that many bytes of lowercase ASCII (`a-z`, `0-9`, `:`, `_`, `.`, `-`). Keys are in ascending order. The reply is at most 8192 bytes.

Keys are nonempty and at most160 bytes. A page contains at most50 distinct
keys and never more keys than its total count. A native client rejects
truncated or trailing bytes, invalid keys, duplicate keys and unordered pages.

## Availability and authority

The composed authority offers capability168 in production, Item-view and
bundle-World sets when its durable query owner is available. Unsupported
authorities remove it. Every query proves the current character, session,
connection generation, lease and scope in its read transaction. The existing
command receipt supplies an identical result when the same command is replayed.

Historical mastery reads are independent of Soulpit acquisition availability.
Only the existing Soulpit completion owner can grant that activity's mastery;
the client has no grant command. Crystal201189f
`data-global/scripts/quests/soulpit/soulpit_fight.lua:100-106` and
`src/creatures/players/animus_mastery/animus_mastery.cpp:33-61` are the reference.

## Native presentation

Ctrl+M opens the Animus Mastery window. It shows the server's total completion
count and a bounded species page, with Refresh, Previous and Next actions.
Refused or malformed replies preserve the last confirmed page. Session teardown
clears it, so another character cannot inherit the previous character's list.
