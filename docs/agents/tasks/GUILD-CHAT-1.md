# GUILD-CHAT-1 — guild room on the World chat relay (skeleton task record)

```yaml
task_id: GUILD-CHAT-1
title: "GUILD-CHAT-1: guild room on the World chat relay"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 5 row GUILD-CHAT-1 (rows 113, 116); §7 'GUILD-*' row GUILD-CHAT-1"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/guild-chat-1
needs_migration: false
needs_registry: false
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

One guild room per guild on the CHAT-0 World relay with `GuildId` payload and delivery-time membership check, auto-joined; the guild message at login (GUILD0 §8).

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

single child GUILD-CHAT-1 (impl, security review).

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| GUILD-1 | BLOCKING |
| CHAT-2 (World relay) | BLOCKING (CHAT-2B #2128 open) |
| CHAT-0 re-review (CHAT-2 gate) | BLOCKING |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## owned_paths (builder edits freely)

- `apps/game-server/src/chat/guild_room.rs`
- `apps/game-server/src/chat/guild_room_tests.rs`
- `docs/agents/tasks/GUILD-CHAT-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/chat/ (relay from CHAT-2B) room set; `ChatRoomSet` import in connection.rs l.87
- chat/mod.rs registration
- membership read from GUILD-1 tables at delivery time

## Inseparable bundles (never split across PRs)

- relay payload + delivery-time membership check + tests

## Acceptance criteria

- [ ] A message reaches only current members at delivery time (excluded member receives nothing after exclusion commit).
- [ ] Login shows the guild message; no presence/location leak (social baseline).
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server guild
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_GUILD0_GUILDS_AND_GUILDHALLS_DECISION_2026-09-30.md §8
- CHAT0 §5 (guild-room relay payload amendment)
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local authoring packet — native Guild/CHAT integration (2026-10-15)

This is **REPORT_ONLY_NATIVE_INTEGRATION_PACKET** for existing draft #2142 on
`skeleton/guild-chat-1` at `0f0a2855170874d760f2c25f4be33eab3517f92c`.
The exclusive allocation permits this card only; the original plan row, dependency snapshot,
open acceptance criteria and CP builder/slot rules above remain historical and unchanged.
No Rust, schema, protocol, registry, wire or runtime was installed or qualified. META 3.1.0
was resolved by the lead; this is one-writer AUTHORING. The worker does not commit or
push; root may batch-publish the existing draft under the user authorization. CP alone
controls ready, merge and queue actions.
Read-only protected main was `bda15eba7a84f65fcb963a86e8b2bf18d69c800a`.

### Effective source and holds

ACCEPT-SOCIAL-MAP-0 §1 makes the GUILD-0 amendment to CHAT-0 §5 effective when its
source/target decisions are accepted; GUILD-0 §8 and accepted CHAT-0 §5 contain the same
GuildId/rank and delivery rule. SOCIAL-MAP-PACKETS §0.1 explicitly says:
“`HOUSE-ACL-1, HOUSE-WIRE-1, GUILD-BANK-1, GUILDHALL-1 and every wire child stay held`”.
ARCH-BATCH-PREMIUM-SOCIAL-PACKETS §3 separately lists GUILD-CHAT-1 after CHAT-2 and
GUILD-1, and holds GUILD-BANK-1/GUILDHALL-1/GUILD-WIRE-1 on their own chain. The
allocation authorizes this non-held chat design, **not** any held guild query/intent,
balance, house ownership/ACL, capability or client management surface. No new signed
hint, relay service, broker or private-invitation mechanism is proposed.

Accepted GUILD-0 §8 requires one auto-joined room per guild, login guild message, distinct
leader/vice rendering, sender GuildId and rank from committed membership when accepting
the command, and receiving membership in the **payload's** GuildId at delivery. Its exact
negative requirement is: a membership read failure drops the line, counted; a receiving node
“`never resolves the sender's current guild`”. §3.1 makes guild membership World-scoped
strong durable truth; §3.2 applies leave/exclude/rank changes immediately across Channels,
not at logout. §4.2 forbids revealing Channel/position or linking other characters of an
Account. No CharacterRevision advance follows from a guild change (§3.5).

### What exists, and what the prepared GUILD-1 does not supply

| Actual inspected owner | Native behavior and precise missing seam |
|---|---|
| `chat/relay/mod.rs`, CHAT-2A archived card | `RelayDestination` has only `Private {recipient: CharacterId}` and `Room {room: ChatRoom}`; `RelayLine`/`OutgoingLine` carry name/text. `seal` generates random message id and nonce; AAD binds World channel and key id. `ReplayWindow::receive` is the sole external open/admit path. No guild variant or membership check. |
| Same relay owner | Existing key rotation overlap 24 h; freshness 30 s; bounded replay set 8,192; durable relay lease API (`resume`, `lease_due`, `lease_stored`) protects restart/clock rollback. This is a relay replay lease, **not** a Character lease or scope-owner permission. CHAT-2A explicitly defers actual LISTEN/NOTIFY, lease storage, readiness/queue watch and fanout. |
| `gameplay_transport/chat_intent.rs` | `ChatRuntime` owns per-session limiter and bounded egress, not Guild truth. `pre_gate` rejects all non-`Say` intents as `ChatUnavailable` before Channel locks. `chat_open` resolves a name via the current durable session, admits/refreshes chat and returns an empty `ChatRoomSet`. No auto-join, motd read, room send or relay caller exists. |
| `durability/fresh_admission.rs` | `FreshAdmissionStore::current_session_at` obtains the canonical session and DB time in a fenced root pass; `current_sessions_at` batches session observations. A returned snapshot is an observation, not an ongoing delivery grant: neither method atomically combines guild membership with the later connection write. |
| `gameplay_transport/mod.rs`, `connection.rs` | `chat_admit` ignores its actor argument today; `chat_next` returns a bare `ChatLine`, then `Next::Chat` creates a domain delta and awaits `write_frame`, then calls `chat_delivered`. Guild/current-owner checks cannot be assumed to exist in these seams. |
| `chat/egress.rs` | 64 queued plus in-flight lines, room-first shedding and one dropped marker. The queue loses any server-only membership binding when reduced to current `ChatLine`; an in-flight line cannot be retracted. `admit`/`refresh` clear old queued lines. |
| Protocol `chat.rs`, `chat_v1.proto`; `crates/session/src/lib.rs` | Four public `ChatRoom` values and a four-bit `ChatRoomSet`; strict oneof/enum/field decoding. No guild send selector, auto-joined guild state, guild/rank line or motd observation. Current IDs (capability 7, command 13, domain 12) already belong to CHAT; no new numeric assignment is made here. `ChatLog` stores 64 observed lines and replacement snapshots drop history. |
| Read-only `deliverables/guild-1/RESULT.json` and actual delivered `domain/guild.rs` | PREPARED_LOCAL_SUBSET, base `ac7f709b9596f4660aabf0523d0196595cf1d29e`: pure name alias, checked rank levels, state and deadline predicates, 10 harness tests. No schema, GuildId type, membership reader/writer, revision/event publisher, lease binding or login integration. Its functions explicitly do not establish current authority. Neither that file nor its proof is assumed merged. |

Searches on both task source and protected main found no native `game_guild_members` /
`game_guilds` persistence or GuildId API. Accepted table names describe the future GUILD-1
owner contract, not callable APIs. SOCIAL-MAP §1.2/§2.2 corrects GUILD-1 prerequisites to
BANK-1 and PREM-WIRE-1; it does not require PARTY-1, INBOX-1 or the complete NPC-ECON
family. This does not declare the missing GUILD-1 or CHAT-2 runtime merged. The historical
CHAT acceptance/re-review/dependency table above is not silently rewritten from source
text or this local preparation; CP must establish then-current exact-head admission.

### Proposed files and owner interfaces — unallocated design, not implemented APIs

| Proposed change | Owner and bounded responsibility |
|---|---|
| `apps/game-server/src/chat/guild_room.rs`, `guild_room_tests.rs` | Historical future lane: compose authoritative sender resolution, validated guild relay destination and receiver eligibility with existing CHAT owners. No copied guild membership store, name normalizer, spam bucket, authority boolean or guild mutation. Pure tests cover rank presentation and destination isolation only; they do not prove authorization. |
| Future GUILD-1 `durability/guild.rs` | Supply owner-internal committed sender/login membership reads and transaction-scoped delivery membership validation against native Character/World/session evidence. Return actual typed GuildId, checked rank, guild revision, current motd and membership identity from one admitted read; `None`/typed failure must remain distinguishable. Names/signatures here are requirements to negotiate with that owner, not existing public types. No new member-revision field is inferred: accepted members have character/guild/rank/joined_at, while the guild has `revision`. |
| Existing `chat/relay/mod.rs`, `relay/tests.rs` | Add the §8 guild destination variant and strict codec inside existing seal/replay implementation, preserving old private/room layouts. No separate envelope/key/auth scheme, public GuildId field or new relay protocol. Must be leased with CHAT-2 owner; current card-only allocation grants no edit. |
| Existing `gameplay_transport/chat_intent.rs`, `mod.rs`, `connection.rs` | Bind authentic admitted sender/recipient, current connection/Character lease/scope owner, cheap limiter/readiness gate, login refresh, qualified final delivery and teardown. CP allocates these shared changes together with CHAT-2; no fabricated `is_member`/`is_current` arguments. |
| Existing `chat/egress.rs`, `line.rs` | Preserve server-only guild destination and recipient/current-session binding through bounded queueing; recheck at final authorized dequeue. Guild lines belong to the existing room shedding class, with the same total bound/marker/in-flight accounting. Do not grow an unbounded parallel guild queue. |
| Existing `docs/contracts/protocol-oteryn/v1/chat_v1.proto`, protocol `chat.rs`/tests and Session consumer | A narrow future CHAT semantic extension needs an explicit CP/protocol allocation: send-to-own-guild text, auto-joined guild-room state, guild line with send-time rank styling and login message. No client-selected destination GuildId/rank. Numeric tags/enum values, compatibility negotiation, maximum payloads and result mapping are **unallocated**. Do not smuggle GuildId into an existing public room enum/bit or ship GUILD_QUERY/GUILD_INTENT/emblems/rosters from held GUILD-WIRE-1. |

Proposed owner entrypoints (names for CP review, **not callable native APIs**):
`accept_guild_chat_send(current admitted execution context, parsed ChatText)` resolves and
validates the sender inside its owner pass, returning the server-only guild/rank/name values
for one relay send or a typed refusal; it accepts no caller GuildId/rank or membership flag.
`read_guild_chat_login(current admitted execution context)` returns a qualified current
membership/motd observation plus guild revision, with absent/unavailable/stale distinctions.
`dequeue_current_guild_chat(current connection execution context)` composes retained
queued destination, fresh receiver membership and the bounded transport custody operation
below; it must not return a reusable authorization token or plain approval boolean. The
actual context types must come from native admission/connection owners, not this new file.

The client CHAT extension is a concrete cross-domain prerequisite, not permission to lift
“every wire child” holds. Until its accepted CHAT-only amendment/allocation and compatibility
plan exist, preparation can cover the private relay/reader seams but cannot activate player
Guild chat or claim the login-message acceptance criterion. No fake text-prefix command or
server-invented public room id bridges this gap.

### Send, relay and fanout dataflow

1. An authentic command reaches the existing admitted CHAT path. The future CHAT gate must
   serve Guild chat explicitly: validate text with native `ChatText`, apply the existing sender
   limiter/mute and relay readiness/queue-pressure checks before expensive Channel work. Do
   not inherit Advertising/vocation/yell/Premium gates for Guild chat without accepted rules.
   `ChatUnavailable` covers unavailable service, not an authority grant.
2. From canonical session/transport, resolve CharacterId and WorldId with current connection
   generation, Character lease and runtime-scope ownership/readiness. A copied session id or
   previous `current_session_at` result does not qualify this pass. Under the GUILD owner
   read/serialization seam capture committed membership GuildId and send-time rank together.
   A client's room/guild/rank/name, old login auto-join, Account relation or cache is not authority.
   Not a member is accepted `NOT_ALLOWED`; the current CHAT result enum has no such value,
   so its exact common-result/payload mapping needs protocol review rather than a guessed code.
3. Feed only that owner-derived destination and rank to existing `OutgoingLine`/`seal`.
   The accepted fixed plaintext layout is kind, random message id, DB send time, GuildId
   (16 bytes), rank (1 byte), native length-prefixed sender name and native text. Choose no kind
   byte here; the shared codec owner allocates it against current main, including other proposed
   mail/party variants. Strictly validate UUIDv7 GuildId and rank 1..20 using the eventual native
   Guild owner types; reject malformed/extra/truncated fields, never coerce to a public room.
   Preserve accepted conservative bounds: plaintext ≤1,188, sealed ≤1,217, base64 ≤1,624,
   NOTIFY ≤2,048 bytes. Native old max constants describe the old two kinds; all boundary tests
   must cover the new widest case without reinterpreting an old kind's encoding.
4. Send one sealed NOTIFY on the existing World channel in autocommit, outside the durable
   membership transaction. CHAT `OK` means sent, never delivered. A leave after accepted
   capture does not retarget the already accepted line. No durable chat receipt/outbox/history,
   no auto-resend after an ambiguous send, no serializing a game value transaction on network I/O.
5. The CHAT-2 listener checks readiness, key/channel authentication, stored replay lease,
   freshness and once-only message id using `ReplayWindow::receive`, then dispatches Guild
   destination. AEAD proves a trusted node sealed a destination; it does not prove the current
   receiver membership. Do not skip delivery checks after successful decryption or re-seal with
   an ad hoc signed membership hint. All World nodes remain the existing trust boundary.
6. Consider only locally admitted, CHAT-negotiated sessions currently owned for this World,
   across its Channels; never fetch/send a full global roster to clients. For each eventual
   delivery, use the receiving Character's committed membership in payload GuildId. Sender
   leave/move must never cause lookup of the sender's new guild. Render send-time rank 1/2
   distinctly, not the sender's later rank; no server-side spatial identity, Channel, position,
   Account id or other-character linkage goes into a Guild line.
7. Fanout must be bounded by actual local admitted sessions and existing egress total, with
   bounded owner read batches/work per pass negotiated against then-current CHAT-2 budgets.
   Accepted 2,000 members (`PARITY_PENDING`) is a membership bound, not a new fanout buffer
   or permission to run 2,000 unbounded futures. No new batch cap is invented in this packet.
   Failure of a committed read drops the affected delivery, counted without logging text or
   identifiers; never fall back to a stale cache. Any optional index/cache is candidate selection
   only, and revision changes may invalidate it but cannot authorize delivery.

### Delivery linearization, current revision and membership transitions

A committed read at relay receipt followed by enqueue is insufficient: exclusion may commit
before a queued line is written. Required future design retains payload GuildId and recipient
binding through egress, obtains a **fresh committed membership read at final authorized
handoff**, and serializes that decision with leave/exclude/move/disband membership writes.
GUILD-0 §4.1 has guild rows before member rows, ordered by GuildId then CharacterId; a
proposed reader can take compatible shared guild/member locks in that order, with native
admission/session authority checked in the owner-approved transaction order. The concrete
read API and lock order with admission guards must be reviewed by both owners. No
independent lock registry or home-grown cross-node mutex is proposed.

The existing `chat_next -> ChatLine -> write_frame().await` seam **cannot prove** that a
membership check, current-session check and irreversible transport handoff share one
linearization point. Proposed owner delta is a bounded authorized-dequeue operation that
keeps the guild/member read protection until the current connection owner accepts custody
of the line, then releases it before potentially blocked network I/O. A second check followed
by an arbitrary await is still a race; a plain snapshot or boolean is not this operation.
Exact custody/commit ordering and cancellation must be designed on the actual then-current
connection API, not asserted via a fake closure. Holding PostgreSQL locks across an
unbounded socket write is not an acceptable substitute. A recipient excluded before that
server delivery point gets nothing; bytes irrevocably handed off earlier cannot be recalled.
CP/architect must explicitly accept and test that delivery boundary against §8's requirement,
including whether an already in-flight line is inside it. This packet does not weaken “excluded
before delivery” to “excluded before relay reception” or claim this unresolved seam solved.

The guild `revision` can correlate a login snapshot or invalidate stale local room presentation;
it is not a membership capability and CHAT domain revision remains independent. An equal
old revision, cached joined flag or event/hint cannot authorize. Do not invent per-member
revisions or advance CharacterRevision to make chat work. A receiver leaving G then joining
H is tested against G on each queued G line; H receives none by re-resolving the sender.
Rejoining G is governed by **current** membership at delivery as written, not an invented
membership-epoch rule. Leave/exclude/move writers remain GUILD-1, not this task.

The §3.2 ordinary guild-command state gate names guild management/economy commands;
§8 delivery names membership. Do not silently apply a new chat-only FORMING/ACTIVE or
Premium eligibility gate. In DISBANDING, members are removed in bounded durable steps;
at minimum each committed removal prevents later delivery, and DISBANDED has no
members by its guard. If CP wants chat closed at the initial DISBANDING transition while
member rows remain, that stricter state policy needs an explicit owning ruling, not inference
from pure `GuildState::ensure_command_allowed`. The qualification matrix must pin the
chosen state behavior before activation.

### Admission, message, reconnect and owner change

At successful login, read the current member/guild message from one qualified committed
GUILD owner view, then establish the server-derived auto-joined room for that admitted
Character; absence means no Guild room/message, read failure means unavailable (not stale
success). Motd uses accepted 255-character CHAT-0 text constraints; no guessed empty-message
rule, author id, synthetic sender or motd-as-player-chat encoding. Its dedicated observation
shape and ordering with the CHAT snapshot must be allocated. Only current members see it.
An old cached motd must not leak after removal, character switch or a late login read.

A live join/leave/exclusion or membership move refreshes local room presentation from strong
truth; absence of a push does not retain authority to send/receive. Whether a newly joined
online member gets motd immediately is not specified by the login rule; do not invent it.
No roster, presence/channel location or guild bank data is needed for auto-join.

Reconnect/transfer rebuilds from current canonical session and membership, clears old queued
lines using native refresh/replacement behavior and keeps CHAT revision continuity above
what the session already saw. Keep the existing mute semantics; a reconnect is not a fresh
limiter allowance. Never replay chat from storage or resubmit an unknown send automatically.
On Character lease expiry, connection-generation replacement, scope-owner handoff/readiness
loss or terminal logout, invalidate the old delivery binding immediately. A new owner starts
its relay replay state through the native stored lease rules; a replay lease alone never proves
it owns a Character. Test the admission teardown and connection write custody together: a
new current session cannot receive an old generation's prepared Guild line by id reuse.

### Planned qualification matrix — no product tests executed here

| Future layer | Required meaningful positive and negative evidence |
|---|---|
| Native Guild storage, PostgreSQL 17.6 | Actual accepted GUILD-1 migrations/writers/readers: committed sender/login view; uncommitted changes invisible; leave/exclude/move committed before delivery blocks it; rollback restores prior membership. No substitute schema or caller boolean. |
| Authenticated send | Real admitted Character/World mapping; forged destination/rank/name ignored or rejected; foreign World, missing member, expired Character lease, old connection generation, stale scope owner and unavailable CHAT refused before seal. |
| Shared relay codec | Old private/room byte fixtures unchanged; Guild roundtrip rank1/2/20; rank0/21, malformed UUID, truncated/extra bytes, unknown kind, oversized name/text fail. Flip GuildId/rank ciphertext, channel or key id: authentication fails. Exact widest bounds and no plaintext logs/Debug. |
| Native relay lifecycle | New kind still obeys 30s freshness, 24h retired-key overlap, duplicate id, full replay set, unleased listener, restart/DB clock rollback and ready/queue-pressure gates. No separate Guild replay window or recovery path. |
| Cross-node destination | Sender leaves G or joins H before delivery: line stays G; receivers in H get nothing; still-current receivers in G receive once. Sender's changed rank does not restyle an accepted line. |
| Final delivery race | Deterministic barriers at membership read, queued line, authorized dequeue and transport custody; exclusion on each side of accepted delivery point; failed read drops/counts; already in-flight limitation explicitly reviewed. Test actual connection path, not a closure that returns true. |
| Guild lifecycle/revisions | Join/leave/kick/rejoin, rank edits, FORMING/ACTIVE and DISBANDING step removals; eventual DISBANDED has no delivery; cached stale revision/room flag never authorizes; optional early DISBANDING close only after ruling. |
| Session and owner handoff | Grace-disconnected/terminal/replaced transport, expired lease, old/new owner overlap and Channel transfer; old generation never gets a late line or motd, restart drops old queue, new current membership determines auto-join. |
| Login/client compatibility | Guild message only after qualified login to a member; late read after exclusion discarded; no-member/empty-policy/read-failure handled distinctly; negotiated CHAT extension accepted by both codecs/Session; old strict client never receives unknown enum/fields. No GUILD-WIRE activation. |
| Fanout/slow client/privacy | Multiple Channels/nodes; bounded per-pass work and total64 queued+in-flight including Guild class; room-first shedding/marker and disconnect during write; no text retention, CharacterId/Account/Channel/location leakage or whole-roster side channel. |

CP must allocate the missing GUILD owner reads, CHAT-2 delivery/runtime, shared relay codec
and narrow CHAT wire/client contract together, resolve delivery custody and any disband state
question, then run actual native persistence/security/privacy/protocol qualification. Keep
relay payload + delivery-time membership checks + tests inseparable. No table, migration,
registry number, shared wiring or held GUILD-WIRE/BANK/HALL work is authorized by this card.
