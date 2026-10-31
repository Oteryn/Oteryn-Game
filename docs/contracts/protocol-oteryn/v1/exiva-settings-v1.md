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

P17C extends this existing command without allocating another capability or command:
`[5]` reads the complete player list and current modes. `[6, name_utf8...]` adds a
player by name; removal uses the existing UUID operation. Names follow Oteryn's
existing 2..29 ASCII character naming contract. Resolution is server-owned and
scoped to the admitted character's World. Existing UUID operations remain valid.
The native client opens with Ctrl+E, edits names and removes the selected UUID.
It refreshes after each accepted edit, and on each open (including after login).

Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
`src/server/network/protocol/protocolgame.cpp:11871-11904` (modes, name resolution
and edits), `:11926-11941` (authoritative player list),
`src/creatures/players/player.cpp:13804-13838` (target-owned permissions).
Client structure: OTClient/Crystal `modules/game_console/exiva_option.lua:20-29` at client revision
`908d5e2fd98dfc170c6ce803d516984aa01d589b`
(read modes and player names into the editor). The existing cap of 100 comes from
Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` `config.lua.dist:99`.
Wire tags, Ctrl+E and UUIDs are Oteryn adapter choices, not donor values.

Each successful edit commits an immutable receipt under the admitted session and command
ID. Retrying a committed command returns success without applying another edit or revision.
The session-generation fence is checked before reading a retained receipt.

The five requested modes use the existing Find Person privacy policy. A caster is admitted
by the guild allow list when their current durable guild is listed. This command does not
add a new World-type configuration policy.

## Reply

Edits retain the one-byte result: `1` committed, `0` refused. A successful read
returns `[2, modes, count]` followed by `count` entries of `[uuid_16, name_len_u8,
name_utf8...]`. At most 100 entries, each name at most 29 bytes: maximum response
4603 bytes. Reject duplicate UUIDs, unknown modes, malformed names and trailing
bytes. A read creates no privacy revision or edit receipt; authority is checked
under the same session-generation fence as edits. The ordinary command status
carries success/refusal. A rejected read returns `[0]`.
