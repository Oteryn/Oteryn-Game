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

## Mastered species XP bonus

With capabilities 168 and 613 selected, command 6388 accepts the original
four-byte request. It returns a little-endian `u16` XP bonus in basis points
(100 = 1%) followed by the unchanged command 232 reply, up to 8194 bytes.
The prefix is 0 when the character has no masteries. With at least one mastery,
it is 200 plus 10 per ten masteries, capped at 400. It applies to each species
in that page. The total and bonus cover the whole character even on an empty
page beyond the end. Command 232 retains its original reply without the prefix.

## Bestiary XP bonus

With capabilities 168 and 613 selected, command 6389 accepts the seven-byte
`ff 01` browse or detail request from the Bestiary catalogue. The reply starts
with a `u8` count, then one little-endian `u16` basis-point value per card in
catalogue order, followed by the existing catalogue page. The count must equal
the number of cards (0–20). Each value is 0 if unmastered or the mastered species
bonus defined above. Total reply size remains capped at 22080 bytes. This is a
read of the current character and active native catalogue. Command 84 keeps
its normal reply and Echo Warden opt-in.

The server uses the same formula for these reads and for raw XP settlement.
Crystal `201189f8` supplies the per-creature Bestiary projection
(`src/server/network/protocol/protocolgame.cpp:2669–2671,3382–3389`) and
multiplicative raw XP rule (`src/creatures/players/player.cpp:3618–3623`,
`src/creatures/players/animus_mastery/animus_mastery.cpp:54–57`). The exact
rate is corroborated by the Tibia Fandom archive
[Animus Mastery](https://tibia.fandom.com/wiki/Animus_Mastery).
