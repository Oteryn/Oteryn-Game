# Party chat (CHATCH / S04c)

Optional capability 240 `CHAT_PARTY_ROOM_V1` requires 7 `CHAT_V1` and 42
`PARTY_V1`. It is offered only by `BUNDLE_WORLD_OFFERED_CAPABILITIES`.
The bundle offer count becomes 77, below `SELECTED_CAPACITY` 112. The coordinator
reserved 240 for this lane. No new command, domain, migration or ordinal is used.
Older peers selecting 7 and 42 receive the same bytes as before.

Command 13 uses `ChatIntentV1.party` (tag 14), containing one nonempty UTF-8
text field, bounded by the existing 1020-byte chat limit. The client supplies
neither destination nor speaker identity. Domain 12, delta type 1 uses kind 7:
name (field 4) and text (7). Both are required; other or repeated fields fail
closed. PartyId and the send timestamp remain server-only queue metadata; no
durable party identity is exposed to the client. This extends `chat_v1.proto`; public-room bits and snapshots stay unchanged.

Every selected session automatically participates while its character is a party
member. The existing Party owner resolves the current party under its World lock
and the existing chat custody transaction proves recovery, node, active session,
connection generation, character lease and runtime-scope fences. Every member can
speak, including the leader. Chat text, staff mute and spam limits are unchanged.
An absent party yields `NOT_ALLOWED`; unavailable authority/relay yields
`CHAT_UNAVAILABLE`; malformed wire or missing selection is rejected. The existing
ordered command ingress retains the reply, so replay never sends a second line.

The existing authenticated World relay carries the Party destination, name and
text between nodes. It stores no chat history. `OK` means sent, with at-most-once
best-effort delivery, as for Guild chat. Each recipient rechecks its own durable
membership under the Party World lock before taking egress custody. Its current
`joined_at` must precede the authenticated relay send timestamp. Leave, kick,
disband, a different party, stale fences or an unselected extension refuse the
line. An already accepted socket write may finish after membership changes, as
with the existing Guild custody boundary. Native tabs are a separate client task;
`oteryn-session` and the synthetic client's `party TEXT` command support this wire.

| Rule | Evidence |
|---|---|
| Automatic participation; only current members; loss on leaving/disbanding | `docs/reference/tibia-manual/controls_communication.md:36` (Global repository reference; wins over donor manual-open behavior) |
| Party-specific destination, absent for nonmembers | Crystal 201189f `src/creatures/interactions/chat.cpp:447`, `:626`, `:686` (`OTS_HYPOTHESIS_ONLY`) |
| Speaker must belong to channel; delivery only to its users, including speaker | Crystal `src/creatures/interactions/chat.cpp:216` |
| No leader, level, vocation or Premium restriction; ordinary channel speech | Crystal `src/creatures/interactions/chat.cpp:588`, `:601`, `:231` |
| Close on leave/disband | Crystal `src/creatures/players/grouping/party.cpp:112`, `:127`, `:200` |
| World relay, fencing, bounded text, replay and egress semantics | Existing Oteryn CHAT_V1 / Guild custody / Party owner; these are Oteryn transport rules, not donor wire compatibility |
