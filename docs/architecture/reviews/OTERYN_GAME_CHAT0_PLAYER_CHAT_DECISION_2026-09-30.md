# CHAT-0 Player chat

- Decision: `CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  security and privacy) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner's direction to finish the playable game (2026-09-30); players cannot talk
  today
- Builds on: the scope matrix rows "World chat", "Private messages" (World communication owner,
  World, ordered per room or conversation, reaching all channels); FND-02 §10 (identities on the
  wire), §16 (snapshot barrier), §19 (bounds), §20 (private chat payloads never logged); SPELL-D1
  (casts by typed index, command 3; unchanged); NPC-0 (command 7, greetings from NPC content,
  capability 3); CHAR-NAME (`0022`, one global name namespace); migrations `0001`-`0003` (session
  state, runtime scope assignment); PREMIUM-DELIVERY-0 (PR #1369, `premium_current`); owner rule
  5905825574 (Global parity)
- Amends: nothing. Amended by GUILD-0 §8 (§5 payload, pending on acceptance of GUILD-0). SPELL-D1 stays as it is: a muted cast answers its existing
  `SPELL_CAST_DISPOSITION_REJECTED` ("ineligible actor"), so no new disposition is needed (§6).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| CHAT-1 | impl, protocol review | capability `CHAT_V1`, the chat command and domain, local speech with its ranges, the NPC greeting, the spam limiter and in-memory mute (§3, §4, §6, §7) | this decision |
| CHAT-2 | hard, security and privacy review | the World relay with its key, readiness and log preconditions (§5), private messages, the World rooms, the durable mute row | CHAT-1 |

Later, each with its own decision: party and guild chat, private chat channels (Premium), the VIP
list, rule-violation reports (with Platform), chat history.

## 1. Question

How do players talk to each other: nearby, privately, and to the whole World?

## 2. Facts

**PROVEN**

- The scope matrix: World chat and private messages belong to a World communication owner, reach
  every channel of the World, ordered per room or conversation. No chat owner or wire exists.
- FND-02 §20: private chat payloads are never logged at ordinary levels; §10 limits which
  identities go to clients; §16 bounds per-session egress under the snapshot barrier.
- SPELL-D1: a cast is command 3 with a typed spell index; `SpellBook::spoken` exists but no
  production path calls it.
- NPC-0: NPC talk is command 7 for clients with capability 3; greetings come from NPC content.
- `0022`: character names are one global namespace keyed by `name_key`.
- `0001`-`0003`: a session row holds the World, the runtime scope and `session_state` (2 active,
  1 disconnected in grace); `game_runtime_scope_assignments` names the node holding a scope.

**CIPSOFT_OFFICIAL** (the Tibia manual, `controls_communication.md` §4.2)

- Local chat: say, yell (long range, 30 s cooldown) and whisper (very short range).
- Spam control on local chat, public channels and private messages: 1 line per 2.5 s, a burst of 4;
  exceeding it mutes for 5 s × (recent offences)², counted across relogs; the buffer refills to
  full after the mute; more than 20 distinct private-message recipients in 10 minutes mutes the same
  way; **a muted character cannot send private messages or cast spells**.
- Free accounts cannot yell or use World Chat, English Chat or Advertising below level 20.
- Rooms: World Chat, English Chat, Advertising (a vocation needed, 1 message per 2 minutes), Help.
- Private messages need the sender at level 3; "a player with this name is not online" otherwise.
  Ignore and white lists filter on the client.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- Say and whisper reach the same floor within ±8 × ±6 tiles (`map_const.hpp:12-13`); whisper text
  reaches adjacent tiles only, others see "pspsps" (`game.cpp:7497-7506`); yell reaches ±18 × ±14,
  upper-cased, not at level 1, with the multi-floor rule of `spectators.cpp:125-137` (above ground
  floors 0-7, or to 8 from z=6 and to 9 from z=7; underground ±2) (`game.cpp:7517-7534`,
  `:7632-7636`). Text is at most 255 characters (`protocolgame.cpp:2734`).
- NPC greetings match as a word anywhere in the line, within the NPC's talk range (4 by default);
  some NPCs use other greetings ("hail king"). A ghost-mode recipient counts as not online.

## 3. Local speech (CHAT-1)

- `say`, `whisper` and `yell`: Channel-local and never durable; the runtime sends the line to the
  players in range on the speaker's channel, with the ranges and floor rules of §2 (Canary).
- Yell is upper-cased, has a 30 s cooldown and is refused at level 1.
- **Spells are not parsed from chat.** SPELL-D1 stays: the client recognises spell words the player
  types and sends command 3 instead of a chat line, so the player experience matches Tibia.
  Showing a successful cast's words to spectators is left to a later spell presentation decision;
  until then spectators see the cast's effect only (`SpellBook::spoken` has no production caller).
- **NPC greeting.** When the speaker's client has capability 3, a `say` whose text contains, as a
  word, a greeting of an NPC within that NPC's talk range (`CHAT0-RL-08`, 4 tiles until NPC-0 fixes
  its own) also starts that NPC's conversation, as command 7 would. With several NPCs in range, the
  nearest wins, then the lowest actor id. The line is still shown to every spectator.

## 4. Text and bounds (CHAT-1)

- Text is UTF-8, 1 to 255 Unicode scalar values, at most 1,020 bytes (`CHAT0-RL-01`), without
  control characters (`REJECTED` otherwise), trimmed at both ends.
- Clients see a speaker as its actor (local speech) or its name (rooms and private messages), never
  its CharacterId (FND-02 §10).

## 5. World relay: private messages and World rooms (CHAT-2)

- **Relay.** One PostgreSQL `LISTEN`/`NOTIFY` channel per World (`oteryn_chat_<world_id>`),
  listened to by every node serving that World. It needs no new process. `NOTIFY` is sent outside
  any durable transaction (autocommit), so chat never serializes game commits.
- **Security.** Every payload is sealed with AES-256-GCM:
  - each World has its own relay key, held by the game nodes as an environment secret; no key
    material is in the repository;
  - the nonce is 96 random bits per line, because many nodes share one key;
  - the payload names its `key_id` in clear; the associated data is the channel name and the
    `key_id`;
  - rotation: a new key is added, nodes send with the newest and open with any key of the last
    `CHAT0-RL-12` (24 hours), then the old key is removed;
  - other database roles that can `LISTEN` see only ciphertext; a line that fails to open is dropped
    and counted. `log_parameter_max_length = 0` on the chat connection is a CHAT-2 precondition.
- **Replay.** The sealed plaintext carries a random 128-bit `message_id` and the sender's
  `sent_at` (database time). Each node drops a line older than `CHAT0-RL-13` (30 s) or whose
  `message_id` it saw in that window (a bounded de-duplication set per World), so a captured line
  cannot be sent again.
- **Trust boundary.** Every node of the World opens every private message, recipient CharacterId
  included, and delivers only its own sessions' lines: the game nodes of one World are one trust
  boundary.
- **Delivery is at most once.** A node whose listener is not connected is not ready for chat: it
  refuses room and private messages (`CHAT_UNAVAILABLE`) until it listens again. Lines sent while a
  listener was down are lost, as in-memory Tibia chat is on a crash; the sender's `OK` means "sent".
  The cluster `NOTIFY` queue use is watched (`CHAT0-RL-09`); above it chat is refused, not games.
- **Private message** `{recipient_name, text}`: the name resolves by `name_key` (`0022`). The
  recipient is online only when it has a session row of this World in state 2 (active); a
  recipient disconnected in grace (state 1) is `NOT_ONLINE`, so no line is lost silently. Oteryn has
  no ghost mode. Otherwise, or when the name belongs to another World, the answer is `NOT_ONLINE`, so the
  reply never reveals where a name exists. A private message to oneself is `REJECTED`. The line
  goes to the World channel keyed by the recipient CharacterId; the node holding that session
  delivers it; no row is written. The sender sees "Message sent" parity through `OK`.
- **World rooms:** World Chat, English Chat, Help and Advertising (a vocation needed, one message per
  2 minutes, `CHAT0-RL-10`). A room line goes to the World channel; each node delivers it to its
  sessions that opened the room.
- **Payload:** a fixed binary layout, not JSON, so nothing is escaped. Plaintext: kind (1 byte),
  `message_id` (16), `sent_at` (8), room (1) or recipient CharacterId (16), sender name
  (length-prefixed, at most 120 bytes), text (length-prefixed, at most 1,020 bytes): at most 1,187
  bytes. Sealed: `key_id` (1) + nonce (12) + ciphertext + tag (16): at most 1,216 bytes. The `NOTIFY`
  text is its standard base64: at most 1,624 bytes, under `CHAT0-RL-06` (2,048) and `NOTIFY`'s
  8,000. A line that would exceed it is `REJECTED` before sending, never truncated or split.
  Amendment (pending on acceptance of GUILD-0; `OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md` §8): a
  guild-room line is its own kind whose destination is the sender's `GuildId` (16 bytes) and the
  sender's rank level (1 byte), taken from its committed membership at send time and sealed with
  the line: plaintext at most 1,188 bytes, sealed at most 1,217, base64 at most 1,624, still under
  `CHAT0-RL-06`. A receiving node delivers it only to sessions whose character is a member of that
  `GuildId` by a committed read at delivery, and drops it when that read fails; it never resolves
  the sender's current guild.
- **Privacy:** text is never written to a table or to ordinary logs (FND-02 §20); the per-session
  egress queue (§7) holds lines in memory only, and nothing is replayed from storage.

**Amendment (pending on acceptance of MAIL-0; `OTERYN_GAME_MAIL0_PARCELS_AND_LETTERS_DECISION_2026-09-30.md` §9).** The relay
gains one payload kind, the mail notice: the recipient CharacterId, with no sender name and no
text, sealed like every line. It is sent after a posting commits, at most once and best effort;
the node holding the recipient's session shows "New mail has arrived." next to a depot locker. It
does not use the sender's spam bucket, which MAIL-0's posting rate replaces.

## 6. Spam control and gates (CHAT-1; durable row CHAT-2)

- Per character: a bucket of 4 lines, refilled one per 2.5 s, over local speech, rooms and private
  messages (spell casts do not use it, as in Canary). Beyond it the line is dropped and the
  character is muted for 5 s × n², n being its offences in the last 30 minutes (`CHAT0-RL-04`, an
  Oteryn choice: the sources say "recently"). More than 20 distinct private recipients in 10 minutes
  mutes the same way. After a mute the bucket refills to full.
- **A muted character can neither chat nor cast any spell** (the manual): the spell path (command 3)
  reads the same mute.
- **Durable mute.** `game_character_chat_mutes` (CharacterId, the last 16 offence times, muted
  until) is written through one SECURITY DEFINER function that checks the session fence (the
  composition decision rule 2 checks, no revision advance). It is read at admission, reconnect and
  channel transfer, so it survives relog and channel changes. While a mute's write has not
  committed, the character is refused all chat and casts (fail closed) and the write is retried; the
  session's teardown writes it before the session closes.
- A muted cast answers SPELL-D1's existing `SPELL_CAST_DISPOSITION_REJECTED`; chat answers
  `MUTED {seconds}`.
- **Gates:** yell and the World, English and Advertising rooms need level 20 until Premium is
  delivered, then level 20 or `premium_current`; private messages need level 3.

## 7. Wire (CHAT-1)

- **Capability `CHAT_V1`**; its number, one command type and one state domain are reserved on #162
  at allocation.
- **`CHAT_INTENT`** (a oneof; an empty oneof is `REJECTED`): `say {mode: SAY | WHISPER | YELL,
  text}`, `private {recipient_name, text}`, `room {room, text}`, `open_room {room}`,
  `close_room {room}`. It answers after local validation and after the `NOTIFY` is sent, never
  after delivery. Results: `OK`, `MUTED {seconds}`, `EXHAUSTED {seconds}` (yell, Advertising),
  `LEVEL_TOO_LOW`, `NOT_ONLINE`, `NO_VOCATION`, `ROOM_NOT_OPEN`, `CHAT_UNAVAILABLE`, plus the common
  results.
- **Domain `CHAT`:** its snapshot is the set of open rooms; delta type 1 is one line (kind, speaker
  actor or name, mode or room, text, position for local speech); delta type 2 changes the open-room
  set. The revision stays above any the session has seen across a reconnect (the NPC-0 pattern). A
  replacement snapshot drops lines; lines are not replayed from storage.
- **Egress bound:** at most `CHAT0-RL-11` (64) undelivered lines per session; beyond it the oldest
  room lines are dropped and one "lines dropped" marker is sent, so a room flood can never trip
  slow-client termination (FND-02 §16). Private and local lines are dropped last.
- Ignore and white lists stay on the client, as in Tibia.

## 8. Rows

| Row | Value |
|---|---|
| `CHAT0-RL-01` text | 255 characters, 1,020 bytes |
| `CHAT0-RL-02` spam bucket | 4 lines, one per 2.5 s |
| `CHAT0-RL-03` yell cooldown | 30 s |
| `CHAT0-RL-04` offence window | 30 minutes, the last 16 offences |
| `CHAT0-RL-05` distinct private recipients | 20 per 10 minutes |
| `CHAT0-RL-06` sealed relay payload | 2,048 bytes |
| `CHAT0-RL-07` open rooms per session | 4 (all rooms) |
| `CHAT0-RL-08` NPC greeting range | 4 tiles, until NPC-0 sets one |
| `CHAT0-RL-09` cluster `NOTIFY` queue use | refuse chat above 50% |
| `CHAT0-RL-10` Advertising interval | 2 minutes |
| `CHAT0-RL-11` undelivered lines per session | 64 |
| `CHAT0-RL-12` relay key overlap | 24 hours |
| `CHAT0-RL-13` relay line freshness and de-duplication window | 30 s |

## 9. Rejected options

- **A new chat service or broker.** A sealed `NOTIFY` relay reaches every node of a World in commit
  order with no new process.
- **Plain-text `NOTIFY`.** Any database role could read or forge lines.
- **Parsing spells from chat on the server.** It would amend SPELL-D1; the client already knows the
  spell words.
- **Storing private messages.** Tibia keeps none, and FND-02 §20 treats them as sensitive.
- **Routing by node id.** Nodes change at failover; the World channel keyed by CharacterId does not.

## 10. Decision test

- **Must decide now:** YES. A playable game needs chat, and NPC greetings ride on it.
- **Minimum sufficient:** one command, one domain, one sealed relay channel per World, one mute row.
- **Superseding evidence:** relay load beyond `NOTIFY`'s capacity; official ranges that differ.
- **Deliberately not decided:** party, guild and private chat channels, VIP list, reports, history.

## 11. Before-freeze checklist

1. **Contract amendments:** none.
2. **Serialization:** the relay orders by commit; the mute row is written under the session fence.
3. **Restart:** chat is transient and at most once; the mute row is durable.
4. **Typed references:** CharacterId (server only), actor ids and names (clients), WorldId, rooms.
5. **Wire:** §7, capability `CHAT_V1`.
