# VIP-2 — VIP presence on the World relay (skeleton task record)

```yaml
task_id: VIP-2
title: "VIP-2: VIP presence on the World relay (skeleton)"
mode: IMPLEMENT
status: skeleton
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/vip-2
base_sha: 5742f30f3
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 5 row VIP-2 (row 114); App. 7 VIP-*; §5 Wave 5"
class: "hard (security, privacy)"
needs_migration: maybe
needs_registry: maybe
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "apps/game-server/src/vip/presence.rs (new) + tests"
  - "docs/agents/tasks/VIP-2.md"
  # Widened by CP D1435 (2026-10-18): full completion and wiring in this PR.
  - "apps/game-server/src/durability/mod.rs (vip_presence registration only)"
  - "apps/game-server/src/chat/relay/mod.rs + tests.rs (relay kind 3, presence)"
  - "apps/game-server/src/gameplay_transport/chat_relay.rs + chat_relay/tests.rs (presence sender, listener, RL-10/RL-11 meter)"
  - "apps/game-server/src/gameplay_transport/mod.rs (login/logout presence feed, owner install)"
  - "apps/game-server/src/gameplay_transport/logout_postgres_tests.rs (offline hint on the real logout path)"
  # Widened by CP D1444 (2026-10-18): VIP is one mechanic; VIP-1b and the VIP_V1 wire land here.
  - "apps/game-server/migrations/0124_vip_lists.sql (VIP-1b production schema and grants)"
  - "apps/game-server/src/domain/vip.rs + domain/mod.rs (registration only)"
  - "apps/game-server/src/durability/vip.rs + durability/vip_tests.rs"
  - "apps/game-server/src/vip/presence_tests.rs (end-to-end watcher client case)"
  - "apps/game-server/src/gameplay_transport/vip_intent.rs (new: command 31, domain 21 deltas)"
  - "apps/game-server/src/gameplay_transport/chat_intent.rs (VipRuntime field only)"
  - "apps/game-server/src/gameplay_transport/connection.rs (domain 21 snapshot, deltas, command 31)"
  - "apps/game-server/src/gameplay_transport/capabilities.rs + capabilities_tests.rs (capability 31)"
  - "crates/protocol-oteryn/src/vip.rs + vip_tests.rs (new), lib.rs (pub mod), item_look_tests.rs (capability count)"
  - "crates/session/src/lib.rs (capability 31 gated route and client test)"
  - "docs/contracts/protocol-oteryn/v1/vip_v1.proto (new)"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (capability 31, command 31, domain 21)"
  - "docs/contracts/RESOURCE_LIMITS_REGISTRY.json (VIP0-RL rows)"
slot_only_paths:
  - "apps/game-server/src/chat/ relay wiring"
  - "apps/game-server/src/gameplay_transport/connection.rs (presence push)"
  - "migration if a consent column is needed"
  - "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (if Reg)"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in apps/game-server/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

## Goal

VIP online presence over the World chat relay, gated by consent and `vip_status` (hide my status) per VIP0 §5.1.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| VIP0 acceptance (CANDIDATE) | NOT ACCEPTED | **BLOCKING** |
| VIP-1 | NOT MERGED (Wave 4) | **BLOCKING** |
| CHAT-2 (relay; CHAT-2B) | MERGED (#2227) | met (D1435) |
| HOUSE-RUNTIME-1b (named-House session fence) | OPEN (#2165) | remaining dependency: House watchers fail closed |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- apps/game-server/src/vip/presence.rs (new) + tests

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- apps/game-server/src/chat/ relay wiring
- apps/game-server/src/gameplay_transport/connection.rs (presence push)
- migration if a consent column is needed
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (if Reg)
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/chat/ (World relay from CHAT-2)
- apps/game-server/src/gameplay_transport/chat_intent.rs
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- presence + consent gate + tests in one PR

## Acceptance criteria

- [ ] no presence leaks without mutual consent
- [ ] hidden status never broadcast
- [ ] relay rate-limited
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.
- [ ] Every BLOCKING dependency above merged (or acceptance landed) before the PR leaves draft.

## Validation commands

```sh
cargo test -p oteryn-game-server vip
cargo clippy -p oteryn-game-server -- -D warnings
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md §5.1
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## Local integration packet — card-only, no runtime claim

This allocation is **REPORT_ONLY**, draft #2164, `skeleton/vip-2` at
`588faecb01c70239ae023a3611564d43e0daeb6c`. Only this card may change. The governing VIP0
decision is ACCEPTED on `main@bda15eba7a84f65fcb963a86e8b2bf18d69c800a`; its §3/§5.1/§8/§11
and the accepted social-presence baseline control this packet. The historical skeleton's
CANDIDATE/dependency descriptions above are retained as history, not a current acceptance claim.
VIP-1 and CHAT-2 runtime integration remain actual prerequisites. The broader historical source
lease does not authorize implementation, shared wiring, protocol reservations or lifecycle action
in this card-only round. All original criteria/plan/CP-builder rules remain open and unchanged.

### Exact privacy semantics to preserve

- The list belongs to Account+World, but consent belongs to the specific inviter Character and
  target Character. A session sees `UNCONFIRMED` unless its **own Character** has the matching
  ACCEPTED invitation. Another Character of the same Account inherits the list, not this consent.
  "Mutual consent" above means the named invitation was accepted; it does not require two
  reciprocal VIP-list entries or authorize account-wide friendship/alternate-character discovery.
- With consent, HIDDEN gives OFFLINE without any login/logout notice. SHOWN gives coarse ONLINE
  exactly when the target has a session on the same World in state1 or2; absence gives OFFLINE.
  Grace disconnect, channel transfer and death before terminal session release do not become a
  logout. Private messaging remains CHAT-0's state2-only rule, a different question.
- No ChannelId, instance, node, position, target Account, vocation, guild or ownership linkage
  enters VIP client state, logs or an indirect error. Party `channel_visibility` is not `vip_status`.
  No Store/Premium/account-wide relationship or exact-contact permission is created here.
- At every evaluation (snapshot, hint re-read, reconcile), read consent, target visibility and
  authoritative target session state together. A failed/stale/unavailable read never sends ONLINE.
  An ONLINE already shown is valid only while these facts were confirmed within the last10s
  (`VIP0-RL-16`); the60s full reconcile is not a substitute. Listener outage demotes every shown
  ONLINE to UNCONFIRMED, sends no other status change, suppresses notices and refuses VIP writes
  with VIP_UNAVAILABLE. Reconnect reconciles immediately, without notices.

### Actual source graph and missing owner seams

| Actual source | What exists and what cannot be assumed |
|---|---|
| `migrations/0001_admission_reconnect_journal.sql`; `durability/fresh_admission.rs` | Canonical session rows and unique nonterminal Character index exist. `FreshAdmissionStore::current_session_at/current_sessions_at` use native admission relation locks and reconciliation in a root pass. These are committed observations, not ongoing delivery grants, and do not join VIP consent/privacy. The batch helper takes known GameSessionIds; it is not the required target-Character status batch query. |
| `durability/character_progression::assert_session_fence`, `character_authority::assert_recovery_fence`, `runtime_scope_assignment::prove_current_incarnation` | Real native recovery, Account/Character/World/session generation/Character lease/scope assignment/node possession/guard checks exist. They are owned/private seams, not callable boolean policies. Existing numeric ownership/generation fields or a saved current_session snapshot cannot reconstruct independently current authority. |
| `migrations/0046_world_parties.sql`; `durability/world_party.rs` | Actual settings have party_invites, channel_visibility and source_policy; blocks already exist. No installed vip_status, vip_consent_revision or VIP table exists on this inspected source. `initialize_party_social_baseline_in_transaction` is a genuine PARTY default producer, not permission to mint VIP consent or silently reinterpret party visibility. |
| Retained `deliverables/vip-1` (base c09e640f7fce59f7b373c7eb5636a35b98ba34e4) | Actual prepared `domain/vip.rs` and `durability/vip.rs` implement only Withdraw/Reject/Revoke, persisted rate/receipts and native fenced erasure. Private receipt/relationship SQL is not a generic presence reader. Its actual tests extend settings/create VIP tables only in an isolated fixture. No invitation/acceptance producer, hide-status writer, list/presence snapshot API, production SECURITY DEFINER/grants, handles or wire is delivered or assumed merged. Its RESULT and source hashes are bound as evidence, not VIP-2 runtime proof. |
| `chat/relay/mod.rs`, `relay/tests.rs`, archived `CHAT-2A.md` | Real AES-256-GCM World relay seal/key rotation/replay rules exist. RelayDestination only has Private and Room; RelayLine/OutgoingLine carry chat name/text. `seal` creates message ID/nonce; `ReplayWindow::receive` verifies World AAD/key/freshness/replay. No accepted VIP presence/list/consent hint codec or listener/fanout implementation exists. The durable replay-lease API is not a Character lease, scope owner or consent proof. |
| `gameplay_transport/chat_intent.rs` | ChatRuntime owns local session limiter/egress. Non-Say paths are CHAT_UNAVAILABLE; chat_open returns an empty ChatRoomSet. admit/refresh clear undelivered chat. No VIP watcher index, privacy timer, revision reconcile or presence sender/receiver exists. |
| `gameplay_transport/mod.rs`, `connection.rs`, `chat/egress.rs` | chat_next returns a bare ChatLine; Next::Chat builds a delta, awaits write_frame and later calls chat_delivered. Queue+inflight count is64; an in-flight line is documented as unretractable. No VIP deadline/current recipient/consent binding survives through this path. Reusing chat text or this queue as a hidden presence protocol would neither provide VIP domain semantics nor close final-delivery custody. |
| Protocol/chat registry and capabilities | No native VIP_V1 codec/capability/domain exists in the inspected source. Existing CHAT IDs/layouts do not become VIP IDs. All numeric assignments, wire compatibility, local handles and production registration remain CP/protocol-owner work; held guild/other wire scopes are not released. |

The accepted missing-row SHOWN default (§5.1) applies after the VIP owner installs/admit its actual
settings schema; a missing column, unknown schema or failed query is unavailable, not SHOWN or
consent. An absent ACCEPTED row never defaults to acceptance. Accepted consent does not expire
with its former invitation deadline; only a PENDING invitation expires at that deadline. Never
turn the ending-consent receipt into proof that a relationship is currently accepted.

### Proposed narrow owner/files and interfaces — not existing APIs

| Proposed file/owner change | Concrete responsibility |
|---|---|
| Future `apps/game-server/src/vip/presence.rs`, `presence_tests.rs` | Node-local watcher index, last-sent status and bounded confirmation/reconcile scheduling over genuine local session bindings. Consume owner reads/hints as observations; do not issue consent, authority, signed hints or a new persistent presence table. |
| Future `apps/game-server/src/durability/vip_presence.rs`, tests, with VIP-1 owner | A native owner read composes current watcher context with list/entry/accepted inviter-specific consent, target vip_status and same-World target nonterminal session in one coherent indexed observation. No raw caller Account/World/Character selector can bypass list ownership. Batch the accepted at-most100 entries/session and shared target reads without sharing per-inviter consent results. Expose no target Account or exact placement. |
| Existing VIP-1 settings/list/consent owner | Finish invitation/acceptance/hide-status/source schema/definers and revisions; after actual commits emit the already accepted semantic hints. Preserve root-before-list and ordered two-list locking. Read APIs must distinguish known absent consent/hidden/offline from failure, without revealing rejection/block/cap reasons. |
| Existing CHAT-2 `chat/relay/mod.rs`, tests and runtime listener owner | Add the accepted presence/list/consent semantic branches inside the existing sealed World relay and strict codec, preserving old private/room decoding. Reuse existing random ID/nonce, keyring/AAD/freshness/replay and at-most-once behavior. No guessed kind bytes, new envelope/key/trust root/process/table/NOTIFY channel, text tunneling or extra signature. |
| Existing native admission/logout/teardown/recovery writers | Hook only genuine session-row transitions into/out of states1–2 after their transaction commits. FreshAdmissionStore::commit_revalidated/release are actual admission/terminal seams; loss2→1 and replacement1→2 stay online. The actual recovery owner that ends a crashed node's rows must publish the same post-commit wakeup; do not infer offline from socket closure or a node-cache timeout. |
| Existing connection/authority owner with VIP-1 wire owner | Build snapshot/delta4 status+announce and maintain monotonic VIP revision above every value seen across reconnect. Keep per-session handles/local entry numbers and capability gating. At final authorized handoff, preserve current recipient binding and confirmation deadline; do not cast a status observation into a reusable authorization token. |

Proposed names for owner negotiation, **not implemented/callable signatures**:
`read_vip_presence_for_current_watcher(current native execution context)` derives the Account,
World and inviter Character from native authority and returns only qualified observations for
that list. `reconcile_local_vip_views(current local session set)` batches list and target-view
revision reads and rebuilds invalidated index entries before status evaluation.
`handle_vip_world_hint(verified existing-relay semantic hint)` uses the target/revision only as a
wakeup, never consumes an online/consent bool as truth. `handoff_current_vip_delta(current
connection execution context)` owns final deadline/recipient checks and actual bounded transport
custody. Actual native context/custody types must be supplied by their current owners; no fake
privacy callback, caller current flag, hand-built signed hint or invented sealed grant is proposed.

### Sender, receiver, index and revision lifecycle

1. **Sender after commit.** The real admission/release/recovery writer emits the accepted
   `{presence, character_id, online, at}` hint only after its actual durable transition. A
   failed/rolled-back write emits nothing. Hide-status emits its accepted presence wakeup after
   commit, with no notice. VIP-1 list/consent writers emit the accepted list/target revision hints.
   A matching historical receipt replay changes nothing and must not manufacture a new transition,
   revision or login notice. Missing hints are repaired by authoritative reconcile, not replayed
   as old actions. Hook placement/order and failure accounting belong to those writer owners.
2. **Receiver verification first.** Receive only through the genuine World's leased ReplayWindow
   and existing keyring/AAD/freshness/replay path. Unknown/malformed/oversized/foreign-World/replayed
   lines drop without identifiers in ordinary logs. Verified sealing establishes a permitted node
   transport source, not target online state, current consent or delivery permission. An unwatched
   target is dropped. Ignore hint.online for status; perform the authoritative combined read.
3. **Watcher index.** Key target Character to local watcher sessions, each bound internally to
   its current Account+World+inviter Character, native GameSession/connection generation and list
   entry_no. Build from current rows on admission/reconnect/channel transfer; update on writes,
   list hints and revision reconcile. Shared Account entries must not share invitation authority.
   Remove stale session/index/queued work on logout, teardown, replacement or transfer and rebuild
   at the destination from current rows. Never transfer an ONLINE confirmation as current proof.
4. **Coherent evaluation.** Native read derives the watcher Account/World/Character, checks its
   current session/lease/scope/node/guards under real owner rules, resolves the exact entry and
   accepted consent for that inviter, then target settings and same-World session. Use actual
   settings/roots/consent writers' compatible locks or owner-approved coherent read semantics;
   do not combine unrelated cached revisions. An invitation's never-reused ID, local entry_no,
   list/consent revisions and session binding invalidate superseded observations, not authorize
   them. A block/reject/expiry with no accepted consent returns the same UNCONFIRMED.
5. **Timers and failure.** Every10s re-read visibility/session/consent for targets shown ONLINE,
   batching target facts while preserving per-inviter eligibility. At the confirmation deadline,
   a missing/slow/failed read cannot extend ONLINE; invalidate its eligibility and fail toward
   less disclosure, using the existing UNCONFIRMED state rather than inventing a wire enum.
   Known hidden becomes OFFLINE; known removed consent becomes UNCONFIRMED. Every60s and at
   listener reconnect re-read all watched target statuses plus hosted Account+World list revisions
   and Character consent-view revisions. Re-read changed views, rebuild index, emit1–3/5 deltas,
   then status differences without announce. A future/rollback/stale confirmation time is not
   freshness; use the existing DB/owner clock source and checked conversion, no second clock.
6. **Notice discipline.** A presence hint may announce only when the new authoritative read
   confirms the real login/logout transition it names, status changed and that entry has notify.
   Reconcile/snapshot/reconnect/consent changes and HIDDEN changes never announce. Preserve the
   last confirmed underlying session-presence observation separately from visible OFFLINE caused
   by HIDDEN, so SHOWN while a session stayed online cannot become a false login. That internal
   observation is data, not authority; if transition correlation is uncertain, announce=false.
   Delayed login after logout reads offline and sends no stale login; out-of-order/duplicate hints
   cannot replay sounds or advance status from hint contents alone.
7. **Outage/restart/load.** Listener down invalidates confirmations and pending ONLINE deliveries,
   demotes shown ONLINE to UNCONFIRMED and refuses writes. Resume the genuine relay lease/floor
   from durable source (not cached process time), reconcile current rows and rebuild local index
   before restoring ONLINE; no historical notice replay. Relay30s freshness/lease is not VIP10s
   visibility freshness. Fanout is bounded by actual local session/list/accepted-watcher limits,
   existing egress/work budgets and CHAT queue watch (50% NOTIFY queue refusal), not an unbounded
   task per watcher. VIP0-RL-10 hint rate andRL-11 p99 latency are unmeasured until real runtime;
   no arbitrary measured result, new cap, registry row or private parallel queue is invented.

### Final delivery and the coordinated GUILD-CHAT custody gap

Read-only coordination with GUILD-CHAT-1 confirmed the same real transport seam: a bare ChatLine
escapes `chat_next`, then Next::Chat performs an awaited socket write, with no membership/current
recipient binding retained. VIP must not reuse that seam as proof that a privacy read and final
delivery are one operation. Queue wait can exhaust a10s confirmation; reconnect can replace the
recipient connection; a downgrade must not be overtaken by an old queued ONLINE. At minimum,
future bounded VIP egress must retain server-only entry/recipient/session binding, source view
correlation and confirmation deadline until the actual current connection accepts custody, then
drop/re-evaluate expired or superseded ONLINE and coalesce/drop stale updates so revision/order
cannot resurrect it. No target Account, placement or internal binding becomes client payload.

The exact irreversible transport handoff/cancellation/inflight boundary is a **cross-domain CP
prerequisite** on the actual connection owner, not solved by this packet. A second SQL check
followed by arbitrary await, a boolean/callback, a committed read snapshot or PostgreSQL locks
held across unbounded socket I/O is not that boundary. Native current recipient connection,
session generation, Character lease, scope owner/current node possession and readiness must be
checked/composed there using their real APIs. A verified relay message supplies none of these.

VIP's accepted10s confirmation window remains distinct from GUILD-0's membership-at-delivery
rule: this packet does not silently require zero-delay revocation or weaken VIP's10s upper bound
to60s/full-reconcile or30s/relay freshness. CP must resolve and qualify how the actual transport
and client state prevent stale queued/inflight ONLINE beyond that bound under stalled I/O and
listener loss; already delivered bytes are not retractable. No invented timeout field or client
expiry mechanism is asserted to exist. Shared delivery work can coordinate with GUILD-CHAT, but
that does not release GUILD-WIRE/GUILDHALL/GUILD-BANK or any held wire surface.

### Required future qualification before implementing/activating

| Boundary | Single-invariant negative or lifecycle evidence required |
|---|---|
| Invitation/Account privacy | Same Account's other Character; no consent; expired PENDING; deleted/recreated invitation with old handle/ID; removed/re-added entry; rejection/block/cap indistinguishable. Only exact accepted inviter may see ONLINE. |
| Current context | Independently stale/missing recovery, wrong Account/Character/World, ended/replaced watcher session/connection, stale Character lease/scope generation, wrong node possession/guard; no delivery or authority reconstructed from old rows/hints. |
| Target presence | State1 and2 ONLINE with consent/SHOWN; state3 or other-World target OFFLINE; grace/death/transfer no logout; failed read no ONLINE; private-message state2 semantics remain separate. |
| Visibility bound | HIDDEN with accepted consent never ONLINE/notice; lost hide/revoke hint corrected within10s; slow/failing query cannot keep ONLINE past deadline; future/backward/expired confirmation not refreshed. Include queued and inflight boundary, not only timer arithmetic. |
| Relay | Real seal/open/key rotation/AAD/replay/freshness/durable lease; wrong World, malformed kind, unknown field/oversize, duplicate/out-of-order login/logout and stale hints. No new trust root or arbitrary signed fixture authority. |
| Announcement | SHOWN after HIDDEN while same session stays online, acceptance/new entry/reconcile/reconnect, duplicate and delayed hints never false-login/sound; notify=false never announces. |
| Revision/index lifecycle | Lost list/consent hint repaired within60s and immediately on reconnect; two inviter Characters remain isolated; list/remove/withdraw/revoke/rename later hooks invalidate only proper bindings; stale work cannot rejoin a new connection. |
| Delivery/custody | Revoke/hide/expiry or recipient replacement while queued and between read/custody/write; downgrade cannot be overtaken by old ONLINE; dropped/slow connection, cancellation and bounded queue overflow fail toward less disclosure. Exact accepted server handoff is tested, not faked as a callback. |
| Concurrency/restart | Reciprocal consent/list writes preserve ordered native locks; hide/show and session termination race with evaluation; actual native PostgreSQL reload and relay restart/clock rollback do not resurrect confirmations, revisions, notices or watchers. |
| Bounds/privacy | Actual100-entry/query bound,100 accepted watchers/target, canonical relay2048-byte bound, existing queue budget and queue-use refusal; measure hint rate/p99 only in real runtime. No target Account/Channel/instance/node/position or relationship IDs in client state/log/error. |

These are proposed future tests, **not run results**. This round runs only governance/card-scope
and source/hash/patch checks. Evidence is in `/workspace/oteryn-prep/vip-2-evidence`; no Rust,
database, relay, client, runtime or performance qualification is claimed. Runtime implementation
remains blocked on the concrete VIP-1 schema/producers/reader/wire, CHAT-2 codec/listener/readiness,
native writer hooks and agreed final delivery boundary, not on a speculative consent boolean.

### Current-main source binding qualification

The skeleton predates merged LOGOUT-WIRE. Final binding therefore records both the exact
skeleton and `main@bda15eba`, rather than claiming whole transport files are identical. Bounded
function comparison confirms current `chat_next`, `chat_delivered`, `current_session_at`,
`current_sessions_at`, `release`, `commit_revalidated` and the `Next::Chat` delivery arm retain
the same inspected behavior. Main additionally has the genuine
`FreshAdmissionStore::release_logout_session` and `gameplay_transport/logout.rs::logout_session`.
The former permits only the exact current ACTIVE controller transport, delegates to the native
terminal-release chain and returns Terminal on an already committed replay, never a second
commit. Its actual post-commit transition is another required presence source; neither a replayed
Terminal observation nor an unacknowledged/unknown logout result manufactures a new offline edge.
Hook the actual durable transition owner, not the later client acknowledgement or socket close.
The new logout paths do not implement VIP hints, readers, privacy or final-delivery custody.

### Bounded native implementation phase (2026-10-15)

This later phase supersedes the earlier **card-only work-scope statement**, while preserving
its source/drift record and the original unchecked acceptance criteria, dependency/plan rows,
and CP builder rules. The owning VIP0 acceptance is recorded on main@bda15eba. The original
VIP-1/CHAT-2 integration gates remain open; no acceptance, readiness or complete runtime claim
is made. Root expanded the lease to exactly `vip/presence.rs`, `vip/presence_tests.rs` and this
card. The independent local review is by `house_runtime_1a`, not an architect acceptance.

**Actual private native port.** `DurabilityRoot::read_current_vip_presence` and `PresenceOwner`
are implemented together in the owned `vip/presence.rs`. They consume the unchanged genuine
Root semantic pass/transaction, root-bound reconciled recovery, admission relation lock order,
`assert_session_fence`, node possession, session/connection/Character lease/scope generations
and Account guards. Account and World are derived from that native context, never request
fields. The reader returns a private-field observation, not a continuing current-authority or
delivery capability. Read errors/authority rejection never manufacture current status.

The exact intended composition is a **durability child** using the shared owner's future
literal `#[path = "../vip/presence.rs"] pub(crate) mod vip_presence;`. That registration exists
only in disposable validation snapshots. No shared source visibility/registration was changed.
Registering the file directly under an unrelated root `vip` namespace would not resolve its
actual native durability-private imports. Production composition remains a CP seam.

**Private storage dependency, not an installed schema.** The combined reader uses the exact
VIP-1 proposed `game_vip_lists`, `game_vip_entries`, `game_vip_consents`, and settings
`vip_status`/`vip_consent_revision` column subset. Tests install those dependencies explicitly
in isolated fixture databases; no migration number, production DDL/grants or write producer is
introduced. The already-prepared VIP-1 ending-consent writer is not treated as merged API.
Future invitation/acceptance/hide/list producers must participate in the same genuine native
admission relation lock order before row locks. Accepted consent is for the active inviter
Character only; another Character of the same Account cannot share it. Accepted invitations
remain accepted after their old pending deadline. Missing admitted settings rows mean SHOWN;
missing schema is unavailable. The SQL default is explicitly `1::smallint` to retain the native
SMALLINT decoder, with a real PostgreSQL type/default regression.

The watcher reader is currently **Channel-only**, matching unchanged native
`assert_session_fence` plus its explicit `RuntimeScopeRefV1::Channel` match. The target's coarse
same-World state1/2 EXISTS can include genuine House sessions without revealing placement.
Named-House watcher support must consume the actual shared House-session fence at CP; this
component neither copies HOUSE1c guard logic nor claims full session-scope coverage.

**Bounded owner.** The actual native-root-bound owner holds at most its configured local
session capacity,100 entries and coalesced pending candidates per session, plus one in-flight
candidate. Reverse target→local-session indexing, reserved work tickets, listener epochs,
actual native fence bindings and checked monotonic revisions reject stale work/reconnect
candidates. Caller revision floors discard or reissue pending/in-flight candidates above the
floor. Fresh privacy/consent/binding changes cancel obsolete in-flight ONLINE immediately;
notify revocation clears an already pending notice even when its status stays OFFLINE.

ONLINE confirmation expires at10s measured conservatively from before the native read.
`due_sessions` provides bounded work selection for10s visibility reads and60s full reconcile;
listener reconnect dirties every cached watcher and restores no ONLINE by itself. The public
hint refresh marks the target dirty and reads actual current rows, including previously OFFLINE
targets. Hint.online is correlation data only. Hide/unhide, consent/reconcile/reconnect and
uncertain/delayed/duplicate hints never invent a login/logout notice. Actual DB failure or slow
read cannot renew an ONLINE confirmation.

**Three different boundaries.** `next_current_update` performs the genuine combined native
read before returning status DATA. If only the VIP query fails or the listener is down, its
withdrawal path still requires a genuine current native recipient-only read and can return
only UNCONFIRMED/no-notice candidates. `next_withdrawal_candidate` separately exposes bounded,
existing-session-only UNCONFIRMED/no-notice **DATA without any DB or current-recipient claim**;
it creates no watcher and grants no connection control. This allows local withdrawal work to
be observed even when the native recipient read is unavailable. `delivered` acknowledges only
local candidate completion; it is not proof of socket delivery, current recipient custody or
client expiry. Final transport custody must bind the genuine current connection/session before
any socket handoff. No supplied current/consent bool, privacy callback or invented sealed grant
exists in these APIs.

This is a combined **per-watcher-session** reader; it does not implement VIP0's node-wide
indexed target/revision batch. `rebuild_index` currently scans bounded local sessions/entries.
Node-wide batching/coalescing, scheduler integration, RL-10 hint rate, RL-11 p99 latency and
full-node10s/60s scale bounds are unmeasured. Full DB/network outage and stalled/in-flight client
privacy are not qualified by a relation-unavailable fixture. CP still owns the exact final
socket/client ten-second withdrawal/expiry/disconnect guarantee.

**Qualification and repairs.** Immutable RED2 evidence records7pass/6fail: five actual pure
state regressions (stale in-flight privacy/binding, retained notice, revision floor, partial
malformed/overflow mutation) and a distinct native PG expression-type failure. The independent
real PG17.6 probe shows `COALESCE(NULL::smallint,1)` is integer while the corrected typed default
is smallint. Grouped repairs include bounded copy-before-replace for read completion, stale
candidate cancellation, notice suppression, above-floor reissue, withdrawal-only public APIs,
reconnect epoch invalidation and current-source hint coverage. Final exact-source results and
snapshot/delta/hash bindings are in
`/workspace/oteryn-prep/vip-2-evidence/implementation/validation.json`; the earlier design-phase
evidence remains preserved separately and is not relabeled as runtime validation.

Tests reuse the unchanged actual native Root/recovery/node/session fixture already registered
by tracked `src/lib.rs`, through `crate::bestiary_postgres_harness`. No fixture registration or
duplicate local fixture mount is needed. Pure synthetic observations are labeled data, never
admission proof. Native PostgreSQL fixtures explicitly install private candidate dependencies and target
presence facts; they do not claim full invitation/session production flows. Disposable native
snapshots add only the disclosed durability path registration. The runner mechanically removes
only the owned test attributes/adds parent visibility and forwarding dispatchers, preserving
actual test bodies and all native private APIs; original untransformed native unit context is
metadata-checked and linted separately. No full server binary/global unit ELF or complete
production writer/wire qualification is claimed.

**Still required at CP.** Install/merge complete VIP-1 schema/definers/producers and approved
production namespace; integrate genuine House watcher fencing; add the genuine sealed VIP
relay codec/listener/readiness and native admission/logout post-commit sources; qualify
node-wide batches/revision hints and final current connection/socket/client custody. Preserve
held guild/wire scope gates. No wire IDs, active source pins, content, registries, migration
numbers, client changes or new authority/trust source are part of this owned preparation.

The final lifecycle sweep also covers cancellation of reserved async reads: the10s bounded
reservation deadline immediately invalidates its work ticket and re-enables reconciliation,
so a dropped future cannot suppress60s work forever. Delayed completion rejects both before
and after a fresh reservation. Never-qualified empty reservations are removed on failure or
timeout; a bad session cannot consume the only slot and deny the real admitted session.
Qualified watchers retain their withdrawal data. The native capacity1 regression exercises
wrong session→actual session through the public owner, not a supplied current predicate.

## Family continuation: indexed native batch — 2026-10-16

Local AUTHORING starts at `273564ad575be249126eb2c90e9caf7423f558f9`, existing
`skeleton/vip-2` / draft PR #2164. The single allocated author changes only
`vip/presence.rs`, its existing `presence_tests.rs`, and this task's notes. Shared
registration/placement remains the intentional durability-child CP seam. Accepted
VIP0 is effective; historical CANDIDATE wording does not block owned preparation.
No commit/push, public IDs, codec, shared wiring, migration or production change.

The native owner now offers `refresh_batch_current`, bounded by the actual configured
local-session capacity and accepted 100-entry/list limit. Every watcher supplies its
root-bound recovery capability, node possession and complete current native fence
for this call. The reader validates each through unchanged native functions in one
semantic transaction/admission-lock domain; a bad watcher refuses the whole batch.
Stored watcher fences and prior observations never replace those current facts.

After native validation, exactly two bulk VIP statements read all selected list and
Character consent-view revisions, then all bounded entries/relationships/settings/
coarse presence. The target CTE materializes distinct target+World pairs before
reading session/settings facts; the consent join still names each exact inviter
Character. Sharing a target or Account list confers no invitation authority. Each
list has an independent LIMIT101/cap100 check. Missing settings use the actual
SMALLINT SHOWN default; missing schema fails unavailable. No target Account or
placement leaves the reader. The native current-authority checks remain per watcher;
this is not a claim that those independent checks cost a constant number of queries.

The reverse target index now updates only the replaced/forgotten watcher's bounded
target-set differences. It retains other watchers of a shared target and scans no
unrelated watcher's entries. Batch completion stages only selected watcher copies
and applies their projections/index changes after every result passes validation.
A cancellation guard invalidates all reserved tickets and frees never-qualified
capacity immediately; failed qualified batches cancel positive candidates and
withdraw toward UNCONFIRMED. Listener epochs and the existing ten-second deadline
reject late results. Full batch refreshes never announce login/logout notices.

Six appended tests (two pure, four real PostgreSQL) cover shared-target index
replacement/removal; atomic malformed completion and cancelled-ticket rejection;
actual distinct watcher sessions/Accounts with exact-inviter isolation, list/consent
revision changes, HIDDEN and no-notice fanout, missing-settings native type default,
and disappeared entries; independently wrong second-watcher connection/lease/scope/
Character/session/node plus direct-core stale recovery; actual ended watcher session;
a public async batch cancelled while pg_stat_activity proves its bulk query waits
on the fixture's relation lock; listener epoch replacement; configured capacity,
101 genuine native-history target entries and missing-column failure. Historical
fixture DATA is not a gameplay admission or consent producer. All inherited test
bytes remain literally preserved as the file prefix. These are authored tests;
execution/Clippy evidence is recorded only after the lead's exact-source native run.

**Still partial / precise unblock:** native House watcher support needs the real
shared House-session/node/scope fence, which the current Channel-only native checker
does not provide. VIP-1 production schema/definers/producers and wire remain pending.
CHAT-2B is closed unmerged, not an open implementation dependency; its World relay
successor/architect disposition must supply accepted sealed VIP kinds, listener and
readiness, and actual post-commit sources. Real CHAT0-RL-09 budget ownership must
precede hint rate limiting; RL-10/RL-11 need integrated runtime measurement. Shared
connection/client custody must enforce the ten-second queued/inflight ONLINE bound.
No callback, local delivered acknowledgement, synthetic codec or arbitrary measured
number resolves those gates. CP slot owns registration, scheduler/listener/socket/
wire/registry/migration work. Recheck on lead native validation results, shared House
fence, approved relay/custody contract and exact CP slot. Full VIP-2 remains pending.

### Native cancellation fixture RED and bounded repair — 2026-10-16

The frozen local candidate `3848fce156410ac271f501fb8af9869f1ab6e862` compiled in
the lead's real Rust 1.94 native library-test context. Its actual PostgreSQL 170006
run passed 21/22 cases; the public cancellation case returned
`Unavailable(RootUnavailable)` after its cancellation assertions. The original
source/binary and RED log `evidence/family/vip-presence-runtime.log` remain retained;
no full-family PASS follows from that run. The production presence source is unchanged.

The test immediately reused the native max-one Root after dropping a blocked semantic
pass. `try_issue_semantic_pass` admits only an already-idle holder; cancelled SQLx
transaction/holder return is asynchronous. Immediate new work can correctly refuse
RootUnavailable. Cancellation is a genuine event permitting the existing bounded
maintenance API, not authority to open a holder in an active read. The fixture now
releases its lock, records request_ready, runs exactly one maintain_ready_once, opens
a fresh root-bound reconciled capability and rebuilds batch inputs before continuing.
There is no retry loop, sleep, clock/deadline change, new authority, relaxed guard or
production-source edit. Phase-specific errors distinguish maintenance, reconciliation
and the subsequent batch. Fresh native focused execution and independent review are
pending; source formatting and whitespace checks pass. All original 16 inherited tests
remain unchanged. This record does not upgrade the original failed run to PASS.

## CP takeover checkpoint — 2026-10-17

Bounded checkpoint on draft #2164 under CP takeover (D1379). Merges protected
`main@f094e0320289975a220852ff60ca7baf6d234ff4` (via `1d632319`) with merge commits; no
conflict and no production/test source change in `vip/presence.rs` or `presence_tests.rs`.
Dependency state on that main: VIP0 ACCEPTED; CHAT-2 accepted by CP and CHAT-2B merged
(#2227); VIP-1 merged as #2143
(`durability/vip.rs`, `domain/vip.rs`), still unregistered and without an installed VIP
migration, as this component is. The historical BLOCKING rows above are history.

**Validation on the merged tree** (PC, shared PostgreSQL 17.6 through the unchanged
`in-pg-netns.sh`; code tree of the `main@f094e032` merge):

Every cargo run touched this worktree's `apps/game-server/src/lib.rs` inside the shared
build slot (`slot-run.sh`, D1420/D1429); an earlier run without that touch tested a foreign lib (0 VIP cases
selected) and is discarded.

- `cargo fmt --all --check`: pass (committed tree and overlay).
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings` (committed tree,
  module unregistered as on main): pass.
- Overlay `cargo clippy --locked -p oteryn-game-server --lib -- -D warnings`: pass.
- Overlay `cargo clippy --locked -p oteryn-game-server --lib --profile test -- -D warnings`: pass.
- Overlay `in-pg-netns.sh cargo test --locked -p oteryn-game-server --lib -- vip_presence`:
  22 passed, 0 failed, 0 ignored; all 7 native cases reached PostgreSQL (no PRE-ROUTING line).
- Overlay `cargo test --locked -p oteryn-game-server --lib` without a PostgreSQL URL: 3,387
  passed, 29 ignored, 7 failed. The 7 are this module's native cases, which refuse by design
  ("no silent skip") when `OTERYN_TEST_POSTGRES_ADMIN_URL` is unset; they pass in the PG run
  above. Sibling durability tests return early instead. Registration must put them in a lane
  with PostgreSQL or adopt the sibling convention (CP decision).
- `python tools/agents/validate_governance.py`: pass;
  `python -m unittest discover -s tools/agents/tests`: OK (124 tests).

The registration overlay is local only (`#[path = "../vip/presence.rs"] mod vip_presence;`
after `mod world_party` in `durability/mod.rs`) and is not committed.

**Dispositions of the 2026-10-16 CP gap review.**

- P2 durability-child placement: deferred to the CP slot. The file imports durability-private
  seams (`character_authority`, `assert_session_fence`, `lock_admission_relations`), so it can
  only compile as a durability child; moving it changes paths outside this record.
- P2 unregistered module: deferred to the CP slot. A committed registration in
  `durability/mod.rs` would also enter all 30 `tests/*.rs` wrapper crates that mount
  `src/durability/mod.rs`; none of them mounts `world_clock.rs` (and 17 lack the
  `bestiary_postgres_harness` the tests use), so `--all-targets` would break. The CP slot must register it
  lib-only (or give those wrappers the needed modules), same as VIP-1's `durability/vip.rs`.
- Node-wide indexed batch/revision path: delivered (`refresh_batch_current`, 2026-10-16).
- House-session watchers: blocked outside owned paths. The genuine named House fence on main is
  the private, request-bound `named_house_current` in `durability/house_interior.rs`
  (HOUSE-RUNTIME-1a, #2163). Copying its SQL here is forbidden; the House owner must expose a
  scope-only `pub(super)` session fence (session, node, house key) for presence to consume.
- Relay hint rate limit, RL-10/RL-11: blocked. With CHAT-2B (#2227) merged, `chat/relay`
  still has only `Private` and `Room` kinds; the sealed presence kind, listener and post-commit admission/logout senders are slot
  or CHAT owner work, and RL-10/RL-11 are runtime measurements of that integrated path.
- Sealed relay codec/listener and final socket custody: CP slot (`chat/`, `connection.rs`).

**Remaining for full VIP-2:** lib-only registration; House scope fence seam; presence relay kind,
listener and senders; hint budget inside `CHAT0-RL-09` with RL-10/RL-11 measured; connection
custody of the ten-second ONLINE bound; VIP-1 migration/definers/wire. Acceptance checkboxes
above stay open.

## CP D1434 checkpoint — 2026-10-18

CP D1434 (#2164) answers the four open questions: registration is lib-only; the 7 native cases
run in the PostgreSQL lane through `in-pg-netns.sh`; the House owner exposes a scope-only
named-House fence; the CHAT owner adds the presence relay kind (this PR consumes it later).
All four stay outside owned paths.

Codex P2 (discussion_r4305168482, dequeue by revision): fixed at the root. `State::next` now
takes the pending update with the lowest revision instead of the lowest entry id, keeping
per-entry coalescing (pending is bounded by `LIST_MAX`). A refresh that re-coalesces a
low-numbered entry can no longer overtake an older pending revision of a higher-numbered entry.
Regression test: `pending_updates_dequeue_in_revision_order_not_entry_order`. Merges
`main@1f51f5df` with a merge commit; no conflicts.

## CP D1435 completion — 2026-10-18

CP D1435 supersedes D1434's split: this PR owns full completion and wiring. Merges
`main@64d5e4da` with a merge commit.

Wired into the running game:

- Registration: `durability/mod.rs` mounts `#[path = "../vip/presence.rs"] mod vip_presence`.
  Every root that path-loads durability compiles it; its 8 native PostgreSQL cases run only in
  the `durability_postgres` crate (`CARGO_CRATE_NAME` gate), the CI PG lane, and refuse without
  `OTERYN_TEST_POSTGRES_ADMIN_URL` there.
- Relay kind 3 (`chat/relay`): sealed `PresenceHint {message_id, sent_at, character, online}`,
  strict decode, same AEAD, freshness and replay window as chat lines.
- `gameplay_transport/chat_relay.rs`: when the relay is configured, `serve_gameplay` installs a
  `PresenceOwner` as the presence sink. A fresh admission queues ONLINE after commit; an accepted
  retire (logout) queues OFFLINE and forgets the session's watcher state. Reconnect and death
  send nothing (VIP0: death is not a logout). A bounded, per-character coalescing outbox feeds a
  sender that publishes only while the relay reads `db_now()` (inside `CHAT0-RL-09`); received
  hints go to `PresenceOwner::target_hint`, listener readiness to
  `listener_reconnected`/`listener_down` (RL-09 reconcile on reconnect).
- RL-10/RL-11: the relay reports `vip_presence_relay sent refused received peak_per_second
  hint_p99_micros` with the drop report. The limit is the CHAT0-RL-09 gate plus the bounded
  outbox (no new numeric cap); RL-11 is measured to the receiving node (hint latency p99).
- Node-wide indexed batch reader: `refresh_batch_current` (2026-10-16).
- House seam: `PresenceOwner` resolves only Channel scopes; an Instance (House) scope is
  `AuthorityRejected` (fail closed) until HOUSE-RUNTIME-1b #2165 exposes the scope-only
  named-House fence. This is the one remaining listed dependency.

Tests: relay kind-3 codec, malformed and replay cases; outbox bounds/coalescing; meter p99;
listener edges; a two-node PostgreSQL relay case (login and logout reach the other node's owner);
the real `logout_session` path queues OFFLINE once (Busy repeat queues nothing; in-fight refusal
keeps ONLINE); native owner case: A watches B, B logs in, A gets ONLINE with announce, B logs out,
A gets OFFLINE with announce, an unwatched hint changes nothing.

Codex P2 (discussion_r4305168482): fixed under D1434; thread answered and resolved with this head.

Outside this PR at D1435 (both closed by D1444 below): VIP-1 production migration/definers/
producers; `VIP_V1` capability/delta/wire on #162. A crash-recovery path that ends another
node's rows sends no hint; the 60 s RL-09 reconcile corrects it.

## CP D1444 completion — 2026-10-18

CP D1444 (one PR per mechanic): VIP is one mechanic, so VIP-1b and the `VIP_V1` wire land in
this PR. The monk_save timing test is left to DEATH-ML-LOSS-1.

- VIP-1b: production migration `0124_vip_lists.sql` (main ends at 0122; 0123 is claimed by the
  open GM-1 and PVP-1b branches, so this takes 0124; lists, entries, consents, groups, group
  members, command receipts, invitation sequence; least-privilege runtime column grants, no
  PUBLIC or control access, immutable receipts). The runtime role writes through these grants,
  so no SECURITY DEFINER function is needed. Producers: `durability/vip.rs` (add, list edits,
  accept/reject/revoke/withdraw, `admit_vip` views) behind `domain/vip.rs` rules. The presence
  test fixture now uses the migration instead of its own schema.
- Capability 31 `VIP_V1` (no requirement), command type 31 `VIP_INTENT`, state domain 21 `VIP`
  are reserved on `PROTOCOL_OTERYN_V1_REGISTRY.json` (#162 rules: append-only, next free IDs
  after ADMIT-CAP-1's 29/30) with `vip_v1.proto` and the `protocol-oteryn::vip` codecs, every
  payload bound tested (intent 536, result 4, entry 566, consent 5502, snapshot 62935 bytes).
- Server wiring: the capability is offered and gated (command 31, domain 21);
  `gameplay_transport/vip_intent.rs` sends the domain 21 snapshot at admission (revision above
  any seen across reconnect), answers command 31 with `VipIntentResultV1`, and pushes entry,
  groups, consent and status deltas. Presence statuses come from the installed
  `PresenceOwner` (RL-09 reconcile 60 s, one native read per due session); a node without the
  relay listener keeps every status `UNCONFIRMED` and answers writes `VIP_UNAVAILABLE`. The
  batched reader, the hint-announce refresh and the DB-outage withdrawal path are tested but not
  yet scheduled by the node, so `presence.rs` keeps its reasoned `dead_code` allow.
- Client: `crates/session` routes capability 31 through the gated table (snapshot, command 31,
  domain 21 deltas) and re-exports the VIP codecs; it is not in `CLIENT_SUPPORTED_CAPABILITIES`
  until the client keeps a typed VIP store, because status deltas are unsolicited pushes.
- End to end: `presence_tests.rs` adds a VIP by name over the wire codec, accepts the
  invitation as the target, and measures RL-11 from the target's logout/login to the watcher's
  decoded `VipStatusV1` (`VIP0-RL-11 end to end` line). The session crate case adds a VIP and
  reads ONLINE then OFFLINE with announce.

- Codex P2 on 8b368b35 (`listener_reconnected`): a reconnect now also clears the failed-read
  backoff (`retry_at = now`), so the cleared withdrawals are replaced by a fresh read within the
  confirmation bound; unit test `reconnect_reconciles_at_once_after_a_failed_read_backoff`.

Remaining listed dependency: HOUSE-RUNTIME-1b #2165 (Instance/House scope stays
`AuthorityRejected` until the named-House fence exists).
