# Exiva settings v1

Capability 147 owns command 170. The command edits only the admitted character's durable
Find Person privacy state.

## Request

`[0, modes]` replaces the mode mask (`1` all, `2` player allow list, `4` guild, `8` party,
`16` VIP). `[1, character_uuid]` adds a player, `[2, character_uuid]` removes one,
`[3, guild_uuid]` adds a guild, and `[4, guild_uuid]` removes one.
UUIDs are the canonical 16 network bytes. Added players and non-disbanded guilds must be in
the admitted character's World. Each list is deduplicated and capped at 100 (Canary
`config.lua.dist:99` at `47dfd51f`). Removing an absent entry is harmless.
Unknown bits, lengths and operations are refused without changing durable state.

Each successful edit commits an immutable receipt under the admitted session and command
ID. Retrying a committed command returns success without applying another edit or revision.
The session-generation fence is checked before reading a retained receipt.

The five requested modes use the existing Find Person privacy policy. A caster is admitted
by the guild allow list when their current durable guild is listed. This command does not
add a new World-type configuration policy.

## Reply

One byte: `1` committed, `0` refused. The ordinary command status carries the same result.
