# Exiva settings v1

Capability 147 owns command 170. The command edits only the admitted character's durable
Find Person privacy state.

## Request

`[0, modes]` replaces the mode mask (`1` all, `2` player allow list, `4` guild, `8` party,
`16` VIP). `[1, character_uuid]` adds a player and `[2, character_uuid]` removes one.
UUIDs are the canonical 16 network bytes. Added players must be live characters in the
admitted character's World. Entries are deduplicated and capped at 100 (Canary
`config.lua.dist:99` at `47dfd51f`). Removing an absent entry is harmless.
Unknown bits, lengths and operations are refused without changing durable state.

Each successful edit commits an immutable receipt under the admitted session and command
ID. Retrying a committed command returns success without applying another edit or revision.
The session-generation fence is checked before reading a retained receipt.

The five requested modes use the existing Find Person privacy policy. This command does
not add the donor's guild allow-list editor or a new World-type configuration policy.

## Reply

One byte: `1` committed, `0` refused. The ordinary command status carries the same result.
