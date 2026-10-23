# GUILD-CHAT-1 — guild room on the World chat relay (consolidated task card)

```yaml
task_id: GUILD-CHAT-1
title: "GUILD-CHAT-1: guild room on the World chat relay"
mode: IMPLEMENT
status: in_progress   # implemented and locally qualified; integrator review/CI/publication pending
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
needs_registry: true   # capability 36 and snapshot type 2 registered under the integrator ruling
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

## CP decision history (resolved 2026-10-22)

Historical blocker analysis is retained below. Both questions were resolved by the binding
Integrator decision 2026-10-22 recorded at the end of this card, before any wire edit.

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

- [x] A message reaches only current members at delivery time (an excluded member receives nothing after the exclusion commit).
- [x] The sender's later guild change never retargets an accepted line; receivers in another guild get nothing.
- [x] Login shows the guild message to a member only; no presence/location/Account leak (social baseline).
- [x] Forged destination/rank/name, foreign World, expired lease, old connection generation refused before seal.
- [x] Old private/room relay byte fixtures unchanged; widest new payload within the accepted bounds.
- [x] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [x] Durable state sufficient to resume after restart / session-generation change.
- [x] The CP protocol decision (chat_v1 vs GUILD_V1) is recorded before any wire change.
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests in their own child files, not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs`/`chat_intent.rs` are wiring-only (`connection.rs` already over 8k lines).

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

## Lane checkpoint 2026-10-22

### Module layout planned before coding

- L4 social: `chat/guild_room.rs` owns the guild destination and recipient binding;
  `chat/guild_room_tests.rs` owns its tests. The existing seal/replay path is reused.
- L4 durability: a separate `durability/guild_chat.rs` owns delivery custody if needed,
  consuming the existing Guild reader without changing Guild core logic or migrations.
- L5 dispatch: `gameplay_transport/guild_chat.rs` composes send, login and final
  dequeue; its child test files exercise the real dispatch and connection path.
- Shared slot files receive wiring only. Protocol modules and registrations remain
  conditional on the CP ruling below. Existing `GuildId` and `GuildRank` need no
  duplicate pure rules or identity types.

### Prerequisite audit

- Supplied baseline HEAD is `f0c8ac0493dda9e6c60f6e20d3842b3aba40e43d`
  (GUILD-1 PR #2302). The worktree was clean at intake. This is a local baseline
  observation, not evidence that the dependency merged to protected main.
- The historical dependency row above does not describe this baseline: domain
  `guild.rs`, durability `guild.rs` and `guild/chat_membership.rs`, migration 0132,
  and the native Guild-chat PostgreSQL cases are present and registered.
- Reuse `GuildId`, `GuildRank`, `read_guild_chat_membership`,
  `read_guild_chat_recipient` and `read_current_guild_chat_data`. The transaction
  reader validates recovery, node, session, lease, World and payload GuildId,
  locking the Guild before the member. The convenience reads commit before
  returning: their returned DATA is not delivery custody.
- Reader consumer search across `apps/`, `crates/` and `tools/` found the existing
  Guild tests, but no production chat consumer. The intended consumer belongs in
  this lane; duplicating the Guild core is unnecessary.
- Existing `RelayDestination` and `ChatLine` have no guild variant. The connection
  currently dequeues, builds the chat delta, awaits the socket write and then
  acknowledges delivery. It has no Guild custody acceptance seam.
- Existing tests to reuse are
  `apps/game-server/src/lifted_tests/durability_guild_tests/chat_tests.rs`, mounted
  once by its existing owner. Their `text_run` reuses the item-transfer `Harness`,
  requires configured PostgreSQL, and creates/cleans an isolated database.
  No helper, fixture, harness, test registration or production module was copied.

### Prior checkpoint: fail closed pending CP rulings (superseded)

The continuation instruction permits implementing a missing prerequisite; it does
not select the card's explicitly open protocol or security boundary. No wire or
runtime code was changed. Two batched questions remain pending:

1. Extend `chat_v1`, with compatible negotiation and documented tag/result
   allocations authorized in this lane (recommended), or retain the hold until
   `GUILD_V1`? Existing public room values and the four-bit room set cannot carry it.
2. Accept connection-owner acceptance under fresh membership/session custody as
   the delivery boundary, releasing locks before socket I/O and permitting an
   accepted in-flight line to finish after exclusion commits (recommended), or
   specify the stronger exclusion/socket-write guarantee?

The relay payload, final delivery check and real-path tests remain one bundle.
No isolated unused public API or test-only substitute was added while these
decisions are pending. No prerequisite implementation is claimed.

### Integrator decision 2026-10-22

The owner-delegated integrator resolved both questions above before wire edits:

1. Extend `chat_v1` using additive negotiated capability `CHAT_GUILD_ROOM_V1`
   (next unallocated capability 36, requiring CHAT_V1/7). Preserve old peers'
   command, snapshot, line and room-set layouts. Allocate guild intent tag 6,
   guild line kind 5, GuildId/rank fields 9/10, guild-aware snapshot type 2,
   and disposition 10 NOT_ALLOWED (common status ACCEPTED). Register the
   allocations and bounds; do not activate GUILD_V1.
2. Delivery linearizes at the connection owner's synchronous acceptance under
   fresh durable membership/session custody. A line accepted before an exclusion
   commits may finish writing afterward. Release custody before socket I/O.

Module layout refinement before coding: protocol `chat/guild_room.rs` owns the
additive codec; social `chat/guild_room.rs` owns destination conversion and queue
class; durability `durability/guild_chat.rs` owns the bounded custody callback;
dispatch `gameplay_transport/guild_chat.rs` and its mechanic children compose
login/send/dequeue. Tests live in separate child files mounted once. Existing
shared files receive delegation, variant/field registration and wiring only.

### Integrator decision 2026-10-22: prerequisite implemented in this lane

The missing production reader consumer and custody seam now use the existing Guild owner:
`durability/guild_chat.rs` retains current canonical admission and Guild locks through the
connection owner's synchronous acceptance; `gameplay_transport/guild_chat/{admission,send,dequeue}.rs`
are its production callers. The Guild core and migration 0132 are unchanged. Admission/reconnect
clears queued lines; the existing limiter and CHAT revision stream remain. A shared 64-read
budget covers an entire connection refresh, including discarded lines. No immediate join motd
policy, history, resend, Premium gate, GUILD_V1 or migration was added.

### Final validation 2026-10-22

All Cargo calls used `CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo +1.95.0`.
The following exact test arguments passed (the parenthetical counts exclude filtered cases):

```sh
test --locked -p oteryn-gameplay-social guild_room --quiet # 5; --lib chat also passed 54
test --locked -p oteryn-gameplay-social-rules guild_chat --quiet # 0: no optional pure leaf added
test --locked -p oteryn-gameplay-dispatch --lib guild_chat --quiet -- --nocapture # 2
test --locked -p oteryn-game-server --lib guild_chat_native --quiet -- --nocapture --test-threads=1 # 6
test --locked -p oteryn-protocol-oteryn chat --quiet # 10; full package also passed 287
test --locked -p oteryn-session chat --quiet # 5; full package also passed 65
test --locked -p oteryn-synthetic-client-harness guild_chat --quiet # 1
test --locked -p oteryn-gameplay-dispatch --lib chat_intent::tests --quiet # 19
```

Also passed: dispatch `the_production` (3) and `the_bundle_world_offered_set` (1);
`fmt --all -- --check`; `clippy --locked --all-targets --quiet -- -D warnings` for
social, durability, dispatch, protocol-oteryn, session, dev-client, synthetic-client-harness
and game-server (each selected with its full `oteryn-` package name); architecture workspace
check; governance validation and its 124 unit tests; `git diff --check`.
PostgreSQL really ran: both routes reported `server_version_num=170006`; the dispatch route
also reported `GUILD-CHAT-PG real connection, native Guild writes and relay active`.
The real connection tests pin both exclusion orderings, including blocked socket I/O,
rollback/invisibility, destination stability after join/move, DISBANDING removals, failed-read
drop accounting and the shared owner-read budget. No load-contention failure was deferred.

### Reuse and file-size record

- Reused domain `GuildId`/`GuildRank`; the Guild transaction reader; canonical fresh-admission,
  name reader and bounded semantic-pass/transaction APIs. No duplicate rules or Guild owner.
- Reused ChatText, sender limiter/mute/readiness gate, ChatRelay send, quiet autocommit NOTIFY,
  existing seal/key-ring/replay/lease path, and ChatEgress room shedding and in-flight accounting.
- Reused monk-save Harness, Dependencies::new/initialize, fixture::join_two and native TCP frame
  helpers. Moved existing Guild historical seed/setup/request bodies into test-support-only
  `guild_chat_fixture.rs` plus `schema.rs`; their original support file delegates/re-exports them.
- Reused the private relayed fixture through `guild_chat/test_support.rs`; exposed only test
  visibility for Session's existing join/admission/spatial/runtime helpers. No copied harness.
- Changed families: dispatch `guild_chat*`/`guild_chat/` and chat/capability/connection/module
  wiring; durability `guild_chat*` and registration; social `chat/guild_room*`, relay payload/tests,
  egress/module wiring; protocol `chat/`, chat codecs/tests, capability table/registry-ID wiring
  and item-look allocation expectation; Session guild modules/tests and registrations; dev-client
  re-exports; synthetic live guild modules/tests, CLI/model/module wiring; Guild test support;
  this card, chat_v1.proto, protocol registry and resource registry.
- Shared-slot edits: dispatch capabilities.rs, capabilities_tests.rs, chat_intent.rs, chat_relay.rs,
  connection.rs, mod.rs and monk_save_postgres_tests.rs; durability/mod.rs; social chat/mod.rs,
  relay/mod.rs, relay/tests.rs and egress.rs; protocol lib.rs/capability_tables.rs/chat.rs;
  Session lib.rs and dev-client re-exports. Large parents contain wiring/delegation only.
- No file grew by more than 200 lines. Largest new file: guild_chat_tests.rs (181 lines).
  Largest existing Rust parent growth: connection.rs (31 lines); existing logic moved rather
  than duplicated. Whole diff re-read against CODEX-CHECKLIST.md; all added enum/field/trait
  consumers searched across apps/, crates/ and tools/ and covered by all-target clippy.

### Unfinished

Integrator security/protocol review of the frozen candidate, repository CI, dependency/base
integration and publication. This worktree remains in AUTHORING on the supplied detached HEAD;
no commit, push, ready-for-review transition, archive or merge is claimed. No owner question remains.

### FLOW port qualification 2026-10-23 (GUILD-CHAT-1b)

Current checkpoint supersedes the prior worktree/validation checkpoint: branch `port/guildchat`,
baseline `17b79f106` (merged FLOW `49cdeaf5f`), AUTHORING, local changes uncommitted.
Owned paths are the guild-chat dispatch modules/tests, protocol guild codec/allocation tests,
Session guild tests/capability expectations, shared Guild test fixture, chat proto comments and
this task record. No registry fragment or shared test mount was edited.

- Guild capability is 36; tutorial remains 31, both occur in the sorted client set, and the
  next-allocation assertion is 37. Protocol registry consistency tests passed.
- Guild dispatch and its moved PostgreSQL cases use `states.chat`. The existing nested module
  paths compiled unchanged under the automatic PostgreSQL mount.
- The shared fixture asserts `CANONICAL_POSTGRES_SERVER_VERSION_NUM` and retains the
  `GUILD-NATIVE-PG` diagnostic; both native routes reported `server_version_num=180006`.
- All Cargo commands used `+1.99.1`, `--locked` where applicable, offline mode and
  `CARGO_TARGET_DIR=/home/mole/oteryn-cache/target-codex/lanes/port-guildchat`, one build at a time.
  All-target clippy with `-D warnings` passed for protocol, Session, dispatch, durability and server.
  Formatting, repository policy, governance and its 124 unit tests passed.

Exact Rust test arguments below all exited 0 (452 tests passed, none failed or ignored):

```sh
test --locked --quiet -p oteryn-protocol-oteryn -p oteryn-session # 295 + 66
test --locked --quiet -p oteryn-gameplay-dispatch --lib guild_chat -- --nocapture --test-threads=1 # 2, live PG
test --locked --quiet -p oteryn-gameplay-dispatch --lib capabilities::tests # 61
test --locked --quiet -p oteryn-gameplay-dispatch --lib chat_intent::tests # 19
test --locked --quiet -p oteryn-gameplay-durability --lib character_bootstrap_intent::tests # 3
test --locked --quiet -p oteryn-game-server --lib guild_chat -- --nocapture --test-threads=1 # 6, live PG
```

Whole local diff re-read against the owner checklist and repository checklist. No new enum
variant, struct field, trait method or public function was introduced; no consumer updates remain.
No test was weakened, skipped, ignored or deleted. No contention failure was deferred.

Unfinished: integrator review, repository CI and publication remain with the control plane;
no local validation failure or owner question remains. Next step: integrator reviews the
uncommitted port delta and publishes through its authorized lifecycle.

### Merge conflict resolution 2026-10-23 (GUILD-CHAT-1b)

AUTHORING on `port/guildchat`, HEAD `126705380c5d81299a70f411eba0c9d6d95c956f`,
merging `origin/main` at `65e40c57f41180e48ee9b9640a8c62849b53b903`.
The initial fetch could not write the sandbox's read-only external Git metadata;
the merge was subsequently started outside this worker. No worker commit or push.

Owned repair paths: Session `lib.rs` and `domain_dispatch.rs`; protocol `lib.rs`;
Guild test `support.rs`; durability `mod.rs` and `guild_chat_fixture.rs`;
dispatch `connection.rs` and `guild_chat/wire.rs`; this record.

- Kept APP-3's appearance/domain/gated-command modules and the independent guild
  module. Guild line capability checks now run in `domain_dispatch.rs`; the
  reducer is not duplicated in `lib.rs`.
- Took the sorted capability union, including 25 and 36, in the registry and
  client expectation. Both negotiated features and their existing tests remain.
- Kept the shared historical-character helper, incorporating main's v3 Male
  intent, `requested_sex`, `sex=2` and historical-NULL explanation there.
  Searched all Rust root inserts and guild fixture consumers for sibling copies.
- Combined Guild, hunting-XP and party-kill module registrations. The connection
  still delegates to the guild adapter, forwarding main's command ID to ordinary
  chat so the party shared-XP command retains its receipt identity.

All Cargo commands use queued offline `+1.99.1 --locked` execution; the wrapper
selects `target-codex/lanes/port-guildchat`. PostgreSQL runs remain filtered to
the Guild-chat mechanic. The broader task-card `guild` filter is superseded by
the owner's mechanic-only PostgreSQL scope rule.

Passed test arguments (no failed or ignored tests):

```sh
test -p oteryn-session --lib # 70
test -p oteryn-protocol-oteryn --lib # 304
test -p oteryn-game-server --lib guild_chat -- --test-threads=1 # 6, PostgreSQL
test -p oteryn-gameplay-dispatch --lib guild_chat -- --test-threads=1 # 2, including real PostgreSQL dispatch
test -p oteryn-game-server --lib appearance -- --test-threads=1 # 3
```

Formatting, governance, repository policy and `git diff --check` passed. No source
conflict markers remain. The repair adds no enum variant, struct field, trait
method or public function. Re-read the merge resolutions and effective Guild-chat
diff against the owner checklist; searched constructors, matches, trait methods
and the changed adapter signature across `apps/`, `crates/` and `tools/`.

Unfinished: ordinary-chat regression checks and all-target clippy remain queued
without a result. The lane's exclusive build lock is held; no new build artifact
appeared for 16 minutes at the checkpoint. The worker asked the owner whether
another validation holds `port-guildchat`; the lock holder is not visible in this
sandbox's process namespace. Do not remove the lock or bypass the queued wrapper.
Pending command sessions: `35391` (requested clippy, including durability) and
`88183` (ordinary-chat regression, then social/synthetic-client all-target clippy).
Index resolution/staging remains with the owner: this worker's hard rules forbid
`git add` and its external Git metadata is read-only. Five index conflict entries
therefore remain until the resolved files are staged. CI/publication remain with
the integrator. No local test failure or load-contention deferral remains.
Next step: identify the lane holder, collect the queued validation results, then
stage the resolved files through the authorized integrator.
