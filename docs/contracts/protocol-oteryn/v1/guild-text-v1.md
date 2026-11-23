# Guild text v1

Capability 638 requires Guild v1 (40). It enables command 6411, Guild Text
Intent, on bundle Worlds backed by the native Guild owner.

## Request

The binary payload starts with a one-byte edit kind, a 16-byte Guild UUIDv7,
and a nonzero big-endian unsigned 64-bit expected Guild revision. The
remaining bytes depend on the edit kind.

| Kind | Body |
| --- | --- |
| 1, MOTD | UTF-8 text, up to 255 bytes. The native Guild writer applies its chat-text rules, including trimming, and requires nonempty text. |
| 2, member title | A 16-byte member Character UUIDv7 followed by up to 15 bytes of UTF-8 title. Empty title clears the member's title. |
| 3, rank name | A one-byte native rank level (1 through 3), followed by 3–20 names in native rank order. Each name starts with a one-byte UTF-8 length. The selected name is 1–20 bytes. Other names are 1–128 bytes and must match the names at the expected revision. |

Read the current revision and ordered rank list with Guild Query (41), replace
the selected name, and send the complete list. Supplying the proposed list
makes the request stable across retries. A request that changes rank count or
other slots at the matching revision is rejected. Rank rename requests must
carry a revision already observed for the selected guild.

The native writer resolves authority from current admission and membership.
Leaders and vice leaders may set the MOTD. Leaders may set a current member's
title or rename a rank. Character and Guild IDs in the request are selectors.

## Result

Uses [GuildIntentResult](guild_v1.proto). A native decision uses the existing
Guild codes, including OK (1), NOT_ALLOWED (2), NAME_TAKEN (9), and
STALE_REVISION (13). A malformed request, a missing capability, or a failed
rank layout precondition returns the Foundation rejection with no payload.
The committed revision, immutable receipt and audit event come from the
existing Guild transaction. Guild Query provides readback after reconnect.
