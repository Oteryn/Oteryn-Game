# Rule violation reports

Capability 585 requires selected PLAYER_REPORT_V1 (76). Legacy command 93 and
all bytes in [player_report_v1.proto](player_report_v1.proto) continue to work
unchanged. No unsolicited messages or state domain are introduced.

## Named report request

Command 6333 starts with one advisory choice byte (0 none, 1 account ban,
2 namelock), followed by the strict legacy request with kind 2. Duplicate,
unknown, malformed or oversized data is refused. The existing named character,
reason, report type and comment rules apply. Target and comment cannot contain
NUL. The suggestion is bound to the immutable report receipt and stored with
the queued report in migration 3054.
A suggestion grants no authority and applies no sanction. GM ban and namelock
choices reference Crystal 201189f8's `data/scripts/talkactions/gm/ban.lua` and
`namelock.lua`. Server-side staff and Platform sanction authorities still
decide any action.

## Named report result

The unmodified PlayerReportReplyV1, through the usual session generation,
rate budget, duplicate suppression and receipt rules.

## Conversation request

Command 6334 takes byte 1 followed by a 1–1024-byte UTF-8 message without NUL
to continue the active conversation, or byte 2 alone to close it. Other
payloads are rejected without a result payload.

## Conversation result

Both well-formed operations return Rejected with the exact UTF-8 payload
`Sorry, not possible.` Crystal 201189f8's `parseRuleViolationReport` in
`src/server/network/protocol/protocolgame.cpp` and its Game callback expose
report intake only. The Fandom archive's Report Rule Violation Channel entry
records the older conversation as deprecated. There is no private channel,
conversation state, or report mutation on this path.
