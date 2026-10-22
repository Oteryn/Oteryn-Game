# GUILD-CHAT-1 — guild room on the World chat relay (consolidated task card)

```yaml
task_id: GUILD-CHAT-1
title: "GUILD-CHAT-1: guild room on the World chat relay"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
class: "impl (security review, protocol review)"
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILD-CHAT-1 (rows 113, 116); §7 'GUILD-*' row GUILD-CHAT-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md   # §3.1-3.2, §3.5, §4.1-4.2, §8
base_branch: main
branch: null
pr: null
needs_migration: false
needs_registry: true   # only if CP resolves the open protocol decision below by extending chat_v1 (new ids)
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2142, branch skeleton/guild-chat-1
(the draft carries a task record only; it has no code)

## Goal

One auto-joined guild room per guild on the CHAT World relay: the payload carries the sender's
`GuildId` and send-time rank, the receiver is checked against the payload's `GuildId` at delivery
time, and the guild message of the day shows at login (GUILD0 §8). A relay `OK` means sent, never
delivered. Relay payload, delivery-time membership check and tests are one inseparable bundle.

## Dependencies (re-verified against origin/main 864beba6d, 2026-10-22)

| Dependency | State |
|---|---|
| CHAT-2A (sealed relay payload, key ring, replay window) | MERGED (#2076); `docs/agents/tasks/archive/CHAT-2A.md` |
| CHAT-2B (World relay over LISTEN/NOTIFY, rooms, private messages) | MERGED (#2227, restart of #2128) |
| CHAT-0 accepted (§5 guild-room relay payload amendment) | ACCEPTED (ACCEPT-SOCIAL-MAP-0 §1 makes the GUILD0 amendment effective) |
| GUILD-1 (membership truth, `GuildId`, rank, motd, revision) | MERGED (#2136) only as an unregistered checkpoint whose sources were removed by #2280 as dead code; **BLOCKING** until the Guild core is re-landed (`docs/agents/tasks/GUILD-1.md`, status NOT_FULLY_QUALIFIED). No `GuildId` type or membership reader exists on main |
| GUILD-WIRE-1 (`GUILD_V1` wire) | NOT_STARTED re-implementation (draft #2159 closing); held, **not** a prerequisite and not authorized by this card (`docs/agents/tasks/GUILD-WIRE-1.md`) |
| CHAT-MUTE-CAST-1 / CHAT-WIRE-1 | MERGED (archive); the existing `Say`/room send path is the only consumer today |

Rows marked BLOCKING must merge (or be accepted) before the slot commit; owned code may be
drafted earlier but must not assume unmerged schemas. HOUSE-ACL-1, HOUSE-WIRE-1, GUILD-BANK-1,
GUILDHALL-1 and every wire child stay held (SOCIAL-MAP-PACKETS §0.1); ARCH-BATCH-PREMIUM-SOCIAL-PACKETS §3
lists GUILD-CHAT-1 after CHAT-2 and GUILD-1.

## Open CP decision (not decided here)

**Where the guild chat protocol lives.** Two candidates, to be ruled by CP before any wire code:

1. Extend `chat_v1` (capability 7, `docs/contracts/protocol-oteryn/v1/chat_v1.proto`,
   `crates/protocol-oteryn/src/chat.rs`) with: send-to-own-guild text, auto-joined guild-room state,
   guild line with send-time rank styling, `NOT_ALLOWED` result for a non-member, and the login
   message observation.
2. Put the same messages in `GUILD_V1` (GUILD-WIRE-1), which would make GUILD-CHAT-1 depend on that
   capability and its hold.

Neither is chosen; numeric tags, enum values, compatibility negotiation, payload maxima and the
common-result mapping of `NOT_ALLOWED` are unallocated. No fake text-prefix command, no
server-invented public room id, no GuildId smuggled into the existing public `ChatRoom` enum or the
four-bit `ChatRoomSet`. Until decided, work can cover only the relay codec and server-side reader
seams; player-visible guild chat and the login-message criterion cannot be activated.

## Accepted decisions

- **GUILD0 §8:** one auto-joined room per guild; login guild message; leader/vice rendered
  distinctly; sender `GuildId` and rank are taken from committed membership when the command is
  accepted; the receiving node checks membership in the **payload's** `GuildId` at delivery and
  **never resolves the sender's current guild**. A membership read failure drops the line, counted.
- **GUILD0 §3.1/§3.2:** membership is World-scoped strong durable truth; leave/exclude/rank changes
  apply immediately across Channels, not at logout. **§3.5:** no CharacterRevision advance follows a
  guild change. **§4.2:** no Channel, position or Account/other-character linkage in a guild line.
- Not a member is accepted `NOT_ALLOWED` (exact common-result mapping pending protocol review).
- No new signed hint, relay service, broker, durable chat receipt/outbox/history or auto-resend after
  an ambiguous send; no cache or event/hint authorizes a delivery (cache is candidate selection only).
- Accepted bounds: plaintext ≤1,188, sealed ≤1,217, base64 ≤1,624, NOTIFY ≤2,048 bytes; guild
  membership bound 2,000 (`PARITY_PENDING`) is a membership bound, not a fanout buffer.
- No chat-only FORMING/ACTIVE or Premium gate and no early close at DISBANDING without an owning
  ruling; each committed member removal must prevent later delivery. Whether a newly joined online
  member gets the motd immediately is unspecified: do not invent it. Motd uses CHAT-0's 255-character
  text constraint, no author id, no synthetic sender.

## Reference behaviour from the draft (re-implement, do not copy)

The draft is a design record (REPORT_ONLY, 2026-10-15); its requirements:

1. **Send.** Authentic admitted command; validate text with native `ChatText`; apply the existing
   sender limiter/mute and relay readiness/queue-pressure gate before Channel work. Resolve
   `CharacterId`/`WorldId` from the current canonical session (connection generation, Character
   lease, scope ownership), then read committed `GuildId` + rank in the Guild owner pass. A copied
   session id, an earlier `current_session_at` result, or client room/guild/rank/name is not
   authority. Only the owner-derived destination reaches `OutgoingLine`/`seal`.
2. **Relay payload.** New guild destination in the existing seal/replay path (no second envelope,
   key or auth scheme): kind (allocated by the shared codec owner), random message id, DB send time,
   `GuildId` (16 bytes, UUIDv7), rank (1 byte, 1..20), length-prefixed sender name and text. Malformed,
   truncated, extra or out-of-range fields are refused, never coerced to a public room. Old
   private/room byte layouts unchanged. One sealed NOTIFY in autocommit outside the membership transaction.
3. **Listener/fanout.** Existing `ReplayWindow::receive` (readiness, key/channel authentication,
   stored replay lease, 30 s freshness, once-only id) then guild dispatch. AEAD proves a trusted node
   sealed it, not receiver membership. Fanout only to locally admitted, CHAT-negotiated sessions
   owned for this World, bounded by existing egress totals and bounded owner-read batches; no
   2,000-future fanout, no global roster to clients.
4. **Delivery linearization.** The line keeps payload `GuildId` and recipient binding through the
   64-line egress queue (guild lines join the room shedding class); a **fresh committed membership
   read at final authorized dequeue**, serialized with leave/exclude/move/disband writes (guild rows
   before member rows, GuildId then CharacterId), keeps custody protection until the connection owner
   accepts the line and releases it before blocked socket I/O. A check followed by an arbitrary await
   is a race; PostgreSQL locks are never held across an unbounded write. Whether an already in-flight
   line is inside the delivery boundary needs explicit CP acceptance; the boundary is not weakened to
   "excluded before relay reception".
5. **Login/reconnect.** Read member/motd from one qualified guild view at admission; absent means no
   room, read failure means unavailable (never stale success). Reconnect/transfer rebuilds from the
   current session and membership, clears old queued lines and keeps CHAT revision continuity; a
   reconnect is not a fresh limiter allowance; no replay from storage. Lease expiry, generation
   replacement, scope handoff or terminal logout invalidates the delivery binding immediately.
6. **Revision.** Guild `revision` may correlate a login snapshot or invalidate local presentation; it
   is not a capability. No per-member revision is invented.

## Owned paths (ADR-0022 layout)

Old `apps/game-server/src/chat/guild_room.rs` (the draft's only owned path) no longer exists; chat
moved into the gameplay crates. Mapping used:

| Old | Current owner |
|---|---|
| `apps/game-server/src/chat/{relay,egress,line}` | `crates/gameplay/social/src/chat/` (L4 runtime) and `crates/gameplay/social-rules/src/relay.rs` (L3 pure channel naming and refusals) |
| `apps/game-server/src/gameplay_transport/chat_intent.rs` | `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs` (1635 lines; wiring only) |
| guild membership reads | `crates/gameplay/durability/src/durability/` (guild reader owned by GUILD-1; `chat_text.rs` exists) |

- `crates/gameplay/social/src/chat/guild_room.rs` (new, destination binding and delivery eligibility composition)
- `crates/gameplay/social/src/chat/guild_room_tests.rs` (new)
- `crates/gameplay/social-rules/src/guild_chat.rs` (new, pure rank presentation and destination validation, if CP wants a pure leaf)
- `crates/gameplay/social-rules/src/guild_chat_tests.rs` (new)
- `crates/gameplay/dispatch/src/gameplay_transport/guild_chat.rs` (new, L5 send/login/dequeue adapter)
- `crates/gameplay/dispatch/src/gameplay_transport/guild_chat_tests.rs` (new)
- `docs/agents/tasks/GUILD-CHAT-1.md`

Slot-only (final commit at the queue turn, after merging main): the relay codec extension in
`crates/gameplay/social/src/chat/relay/mod.rs` (+ `relay/tests.rs`) and `chat/egress.rs`/`line.rs`
(shared with the CHAT-2 owner; needs a CP lease); `gameplay_transport/chat_intent.rs`, `connection.rs`,
`mod.rs`, `capabilities.rs` wiring; `crates/gameplay/durability/src/durability/` guild reader hook
(GUILD-1 owner interface); `docs/contracts/protocol-oteryn/v1/chat_v1.proto`,
`crates/protocol-oteryn/src/chat.rs`/`lib.rs`, `crates/session/src/lib.rs`, and the registries
`docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`, `RESOURCE_LIMITS_REGISTRY.json`,
`OTERYN_GAME_ERROR_CODE_REGISTRY.json` only after the CP protocol decision. No migration.

## Wiring points (verified on origin/main 864beba6d)

- `crates/gameplay/social/src/chat/relay/mod.rs` (583 lines): `RelayDestination` has only `Private` and `Room`; `seal`, `ReplayWindow::receive`
- `crates/gameplay/dispatch/src/gameplay_transport/chat_relay.rs` (528 lines): listener and replay lease
- `crates/gameplay/dispatch/src/gameplay_transport/chat_intent.rs`: `pre_gate` rejects non-`Say` intents, `chat_open` returns an empty `ChatRoomSet`
- `connection.rs` `chat_next -> ChatLine -> write_frame().await -> chat_delivered` (no custody proof today)
- `crates/gameplay/durability/src/durability/fresh_admission.rs` (`current_session_at` is an observation, not a delivery grant)
- `crates/protocol-oteryn/src/chat.rs` (536 lines), `chat_v1.proto`: four public `ChatRoom` values; capability 7, command 13, domain 12 already belong to CHAT

## Acceptance criteria

- [ ] A message reaches only current members at delivery time (an excluded member receives nothing after the exclusion commit).
- [ ] The sender's later guild change never retargets an accepted line; receivers in another guild get nothing.
- [ ] Login shows the guild message to a member only; no presence/location/Account leak (social baseline).
- [ ] Forged destination/rank/name, foreign World, expired lease, old connection generation refused before seal.
- [ ] Old private/room relay byte fixtures unchanged; widest new payload within the accepted bounds.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] The CP protocol decision (chat_v1 vs GUILD_V1) is recorded before any wire change.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs`/`chat_intent.rs` are wiring-only (`connection.rs` already over 8k lines).

Planned qualification (no product tests exist): PostgreSQL 17.6 guild storage reads (uncommitted changes
invisible, rollback restores), authenticated-send negatives, relay codec roundtrip rank 1/2/20 and
tamper cases, relay lifecycle (freshness, replay, key overlap, clock rollback), cross-node destination
stability, deterministic-barrier delivery race on the real connection path, DISBANDING step removals,
owner handoff, slow-client/queue shedding and privacy.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-social guild_room
cargo test -p oteryn-gameplay-social-rules guild_chat
cargo test -p oteryn-gameplay-dispatch guild_chat
cargo test -p oteryn-protocol-oteryn chat   # only if the chat_v1 option is chosen
cargo +1.95.0 run -p oteryn-architecture-check -- workspace .
```

## Reference evidence

- `docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md` §8
- CHAT-0 §5 (guild-room relay payload amendment); `docs/agents/tasks/archive/CHAT-2A.md`, `CHAT-2B.md`
- `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` §4, §5 (Wave 4), §7
