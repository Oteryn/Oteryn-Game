# SERVER_INFO_V1

Capability 500 gates command 6241, independent of existing strict quest and spell codecs.
No mutation or casting authority is conveyed. Unselected, unfenced, malformed and unavailable
reads are rejected with an empty result. Existing wires remain unchanged.

## Request

UTF-8 JSON (at most 256 bytes): `"Quests"`, `{"Spells":[1,2]}`.
Spell indices are 1..2^28-1, strictly increasing, at most 16, nonempty.
Quest titles are restricted to the admitted session's visible quests.
Spell descriptions and durations come from the exact executable cast book, excluding inactive aliases.

## Reply

UTF-8 JSON, at most 65536 bytes, closed fields, as `server_info::Reply` defines.
Quest rows pair canonical quest indices with accepted display names (128 bytes).
Spell rows echo the exact requested indices; library text is optional only where accepted
content omits it (8192 bytes maximum). Durations are milliseconds; group ordinals use S9.
Decoding rejects invalid ordering, excessive counts and replies inconsistent with the request.
