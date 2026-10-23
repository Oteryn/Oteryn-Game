# SPECTATE-1 — spectating (consolidated task card)

```yaml
task_id: SPECTATE-1
title: "SPECTATE-1: spectating (player observation of a consenting target session)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row SPECTATE-1 (row 138); §5 Wave 5; App. 7 LOOP-WORLD-1, SPECTATE-1"
brief: "docs/architecture/reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md"
class: "hard (child graph; security, privacy, protocol, concurrency review)"
base_branch: main
branch: w4/spectate-1
pr: null
needs_migration: false   # observation is not durable; a future SPECTATE-AUDIT-1 outbox migration is a separate PR
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2317](https://github.com/Oteryn/Oteryn-Game/pull/2317). Dormant observer hooks, consent/session handling, projection filtering and command isolation are merged. Evidence: `crates/gameplay/dispatch/src/gameplay_transport/spectate_session.rs`.

Remaining: Durable audit, accepted Platform O2 admission boundary and activation remain; capability 33 is not offered by production. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2183, branch skeleton/spectate-1

## Goal

A read-only view of another player's session. The owner accepted player
observation as an Oteryn deviation from Global parity (answer (b), 2026-10-15).
`SPECTATE_V1` is offered only after SPECTATE-GRANT-1, SPECTATE-SESSION-1 and
SPECTATE-WIRE-1 have merged and CP enables it.

## Accepted contract (SPECTATE1-OBSERVER-CONTRACT-V1)

Accepted with amendments by CP/architect D1035–D1039
([ruling](https://github.com/Oteryn/Oteryn-Game/pull/2183#issuecomment-6216592075)),
clarified by D1095 ([#1622 comment](https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6219643346)).
The brief file was only authored on the draft; the first child PR re-lands it at
`docs/architecture/reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md` (new),
together with the SPECTATE plan row only. Builds on
`docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md`,
`docs/architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`, ADR-0003, FND-04/FND-04B
(§22 locator), VSL-MOVE-01 §13, CHAT-0, NET-03, GAME-CHANNEL-01, ARENA-1 A9/A11.

Facts: the only reference spectating is the Tibiadrome roof
(`docs/reference/tibia-manual/combat.md` §5.3.14), an ordinary character on the
map; Crystalserver `spectators.cpp` is the in-range query, not an observer feature.

- **O1 Two things.** Actor visibility (interest set, chat in range, roof) is not an
  observer permission. No relation (view, party, guild, house, VIP/contact, same
  account/instance, killer/victim) grants observation. An observer has no position,
  is in no interest set, never appears in "who is in range" queries (spells, boss
  gates, encounter `near`, quest portals), presence or online lists, and cannot be
  targeted, traded with or spoken to.
- **O2 Observer session.** Connects through the Gateway (ADR-0003); authenticated
  account; single-use short-lived observer admission material
  (`FND02-ADMISSION-MATERIAL-BYTES`) for the node owning the target's current scope via
  the FND-04B §22 locator. Session kind `OBSERVER`: own GameSessionId/generation, no
  character lease, no actor, no fence, no gameplay command sequence; bound to one grant
  and one target GameSession generation. Gateway part is a separate Platform child.
- **O3 Consent grant.** Created by the target's own session by explicit command; off
  by default, never inherited. Audience `CODE` (single shared code, rotatable) or
  `CONTACTS` (accepted contact consent; never account-wide or alternates). Lives in
  the target's runtime owner, bound to its session generation, not durable. Ends on
  revoke, logout, generation change, lease change, death, scope change. Ending ends
  every stream within `SPECTATE1-RL-05`; no frame after the end. Codes compared in
  constant time; every refusal is uniform `NOT_AVAILABLE`. Per target only.
- **O4 Projection: world tier only.** Map, creatures, map items, effects, ordinary
  public speech and presentation events from the target's interest set plus own
  appearance/health/mana bars as others see them. Never: inventory, containers,
  equipment detail, skills, stats, imbuements, conditions, private messages,
  party/guild/private rooms, VIP/contacts, NPC trade, market, store, house lists,
  typed command outcomes, account/Platform data.
- **O5 Read-only by construction.** Observer command table: `OBSERVE_STOP` and
  keepalive only. Every gameplay command type is refused `NOT_ALLOWED` at transport
  dispatch before any handler; no entry into the gameplay dispatch chain.
- **O6 Fan-out never affects the target.** Frames derived post-commit from the same
  facts the target receives; no extra authoritative read or lock. Own egress per
  observer (NET-03 rows); overflow drops the observer (`OBSERVE_END{OVERLOAD}`), never
  the target; target tick never waits. Max `SPECTATE1-RL-01` observers, else `FULL`.
- **O7 Scope change / PvP.** Scope change (channel, instance, cross-node handoff,
  house entry) ends streams `TARGET_MOVED` and the grant. Refused/ended while the
  target is in an `observable: false` scope; v1 default false for PvP arena matches
  and any scope with PvP enabled for the target; boss rooms observable.
- **O8 Target knows.** Client shows grant active + observer count; identities never
  shown either way; no alternate-character inference.
- **O9 Audit/abuse.** Grant create/revoke, observer admit/end are Game events (no
  position, no code). Wrong codes rate-limited (`SPECTATE1-RL-04`) and are GM-1 abuse
  signals. Observation never extends a session or keeps a scope warm.
- **O10 Protocol.** Capability `SPECTATE_V1` (gated). Family
  `crates/protocol-oteryn/src/spectate.rs` (new): `OBSERVE_GRANT_SET{audience}`,
  `OBSERVE_GRANT_REVOKE`, target state (active, count), `OBSERVE_STOP`, observer
  envelope reusing existing world map/spatial/presentation/public chat payloads
  unchanged, `OBSERVE_END{REVOKED|TARGET_GONE|TARGET_MOVED|OVERLOAD|NOT_OBSERVABLE}`.
- **O11** Nothing durable except audit; restart ends all observer sessions, no replay.
- **O12 Rows (register before use):** RL-01 observers/target 50; RL-02 observer
  sessions/account 2; RL-03 per node from measurement before activation; RL-04 code
  attempts 5/min/account; RL-05 grant end→last frame one Channel owner work cycle;
  RL-06 code 16 bytes CSPRNG; NET03 egress rows and FND02 admission bytes cited unchanged.

Open (not decided): extra disclosure tiers, observer chat room, broadcast delay
instead of PvP refusal. Split out: staff observation (GM-1), replays, instance
galleries/tournaments (GAME-INSTANCES-01, ARENA-1 §6), Tibiadrome roof.

## Children and delivery (D1036–D1039, D1095)

1. **SPECTATE-GRANT-1** (O3, O7, O8) and 2. **SPECTATE-SESSION-1** (O5, O6, O9, Game
   part of O2) — separate commits of one mechanic PR, with the brief re-land first.
3. **SPECTATE-WIRE-1** (O4, O10) — separate PR with protocol review; owns
   `spectate.rs`, codec, registry, capability, projection privacy filter + tests;
   allocated only after green GRANT/SESSION.
4. **O2 Gateway admission** — separate Platform child allocated by CP.
5. **SPECTATE-AUDIT-1** — durable outbox writer/migration after green GRANT/SESSION.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| Accepted brief D1035–D1039/D1095 | ACCEPTED (brief file must be re-landed) |
| FND-04B, MAP-WIRE-2, SCOPE-HANDOFF-1 | on main (D1095) |
| CHAT-1b-2b | MERGED (#2093) |
| HOUSE-RUNTIME-1a (registry slot predecessor for type 4) | MERGED (#2163) |
| ADMIT-CAP-1 (A2 hooks TARGET_GONE/TARGET_MOVED come after it) | MERGED (#2187) |
| CHAT-2B relay (local public chat source) | MERGED (#2227) |
| Contact consent native consumer (CONTACTS audience) | BLOCKING for CONTACTS; CODE may ship first, CONTACTS stays uniformly unavailable |
| Event-ID uniqueness across runtime recreation | OPEN question (#2131 comment 6225521255) |
| WIRE-1 / O2 Gateway | after green GRANT/SESSION |

LOOP-WORLD-1 is not a prerequisite.

## Prerequisite audit (2026-10-22)

Baseline: `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`, clean worktree at entry.
The CODE grant/session owner, observer transport, projection filter and spectate
wire family are absent from `apps/`, `crates/` and `tools/`. CONTACTS remains
uniformly unavailable; it is not a prerequisite for the CODE audience.

Existing owners to reuse are `GameSessionAuthoritySnapshot`,
`FreshAdmissionStore::current_session_at`, native `ChannelRuntimeV1`, the
connection's capability negotiation and bounded frame codec, world-map/spatial
and public-chat payloads, the TLS provider's secure random source, and the ANL
event envelope. Observer state must never acquire a Character lease or actor.

The current FND-04 fresh/recovery profiles and verifier authenticate
Character-bound player admission. They define no observer trust purpose,
single-use observer material, or observer bootstrap. O2 deliberately assigns
Gateway admission to a separate Platform child. Reusing player admission as
observer authentication or inventing an unsigned account/session assertion would
cross that authority boundary. The owner question is whether this delivery ends
at dormant Game admission hooks pending that child, or consumes an accepted
observer admission profile/bootstrap supplied by the owner.

Evidence: `FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md` requires `character_id`
and fixes verifier trust purpose before token authentication;
`FND-04_REAUTHENTICATED_RECOVERY_GRANT_PROFILE_V1.md` also requires
`character_id`. The native verifier is
`crates/gameplay/foundation/src/foundation/fnd04_verifier.rs`. The transport's
`FreshAdmissionAttempt` requires a CharacterId and its `admit_frame` accepts
only player bootstrap/recovery. No observer admission consumer was found.

### Integrator decision 2026-10-22: prerequisite implemented in this lane

Option A resolves the audit question: dormant Game-side hooks, with every observer
bootstrap refused today. No Platform profile, trust purpose or production observer
admission was added. CODE machinery is implemented; CONTACTS remains unavailable.
The prior checkpoint's pending owner question is closed by this decision.

Planned-before-coding module layout, now implemented under ADR-0022:

- `social/src/spectate/grant.rs`: consent, CSPRNG rotation, revision fencing, lifecycle termination.
- `social/src/spectate/session.rs`: actor-free identity, target/account/node caps, bounded egress.
- `social/src/spectate/rate_window.rs`: bounded rolling account attempt windows.
- `social/src/spectate/projection.rs`: typed native world-tier inputs and public-chat filter.
- `social/src/spectate/audit.rs`: immutable registered audit packets and terminal reservations.
- `social/src/spectate/runtime.rs`: exclusive owner transition interface, empty on restart.
- `dispatch/src/gameplay_transport/spectate_session.rs`: named lock, read-only dispatch,
  immediate egress, failed/partial-write closure and the real admission refusal hook.
- `protocol-oteryn/src/spectate.rs` and `spectate/{codec,envelope,audit}.rs`: dormant codecs.

Tests are child files mounted once. Shared files receive wiring only. The existing
private native admission fixture stays in place; eight-line `admission_test_support.rs`
adds a `test-support` wrapper (`shared_test_facts`) without copying the fixture body.
Event-ID choice: fresh CSPRNG incarnation namespace per runtime recreation and
non-reused local ordinals; these are producer keys, not ANL EventIds. The future
SPECTATE-AUDIT-1 writer must allocate/freeze globally unique UUIDv7 EventIds before
commit and prefix acknowledgement. No durable state or migration changed; 0139 is unused.

## Target layout (ADR-0022)

- Grant/session owner (process-local, no IO): `crates/gameplay/social/src/spectate/` (new;
  L4 runtime, beside `social/src/chat/`), one file per mechanic: grant, session,
  rate window, projection filter, audit facts.
- Observer transport refusal and admission hook: `crates/gameplay/dispatch/src/gameplay_transport/`
  (new `spectate_session.rs`; wiring-only lines in `connection.rs`, `mod.rs`, `capabilities.rs`).
- View sources (read, unchanged): `crates/gameplay/dispatch/src/gameplay_transport/world_map.rs`,
  `world_spatial.rs`; `crates/protocol-oteryn/src/world_map.rs`, `world_spatial.rs`.
- Audit payload: `docs/contracts/game-events/v1/spectate.proto` (new), registry entry in
  `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` (slot).
- Wire: `crates/protocol-oteryn/src/spectate.rs` (new), registry fragments under
  `docs/contracts/protocol-oteryn/v1/registry/` (REGISTRY-SPLIT-1: one file per entry).

## Reference behaviour from the draft (re-implement, do not copy)

- Process-local GRANT/SESSION owner: CSPRNG code grants with rotation and
  compare-and-set revisions; grants bound to independently refreshed native
  GameSession snapshots (generation, lease, scope, session id); synchronous
  termination on lifecycle change; caps per target/account/node; rolling window of at
  most five timestamps per account (fixed-window rollover was a review P2, fixed);
  early refusal of every gameplay command; bounded observer queues with immediate
  non-blocking writes; failed/partial write closes the observer.
- Only native world-spatial and local public chat payloads enter the queue;
  private and room chat filtered; inventory, account data and command outcomes have
  no forwarding API.
- Audit leaf (D1118): `SpectateObservationV1 { Kind kind=1 (GRANT_CREATED,
  GRANT_ENDED, OBSERVER_ADMITTED, OBSERVER_ENDED); optional bytes
  observer_session_id=2 (UUID, exactly for observer events); EndReason end_reason=3
  (REVOKED, TARGET_GONE, TARGET_MOVED, OVERLOAD, NOT_OBSERVABLE, OBSERVER_STOPPED;
  unspecified for non-terminal) }`. Target session only in `EventEnvelope.game_session_id`;
  no code, account or position. Golden bytes for all kinds/reasons/presence/malformed.
  Sink success means durable commit before acknowledging the copied ordinal; storage
  I/O must not borrow the mutable runtime owner. The producer reserves capacity for
  every active terminal event.
- Reserved registry values: event type 4 `SPECTATE_OBSERVATION`, `DURABLE_AUDIT`,
  `SECURITY_SENSITIVE`, retention `SPECTATE_AUDIT_RETENTION_V1` P30D, staff/security
  readers only (O9/GM-1), `atomic_mutation_evidence=false`. Reconfirm free numbers at
  the slot turn.
- Port onto the DISPATCH-WIRING-1 head: capability 31, command types 31/32 and state
  domain 23 were taken on main (TUTORIAL-1, BED-1, DAILY-1), so the dormant reservation
  moved to capability 33, command types 34/35 and state domain 25 (CP to confirm).
  Message type 15 and event type 4 are unchanged.
- Account caps in the core are node-local; cross-node caps belong to O2. Node
  capacity is a measured composition input with no activation default.

## Acceptance criteria

- [x] Spectator cannot issue any gameplay command: every registered command type is refused before a gameplay handler runs, with no state change.
- [x] Privacy: explicit consent of the spectated player; uniform `NOT_AVAILABLE` refusals; constant-time code compare.
- [x] Projection privacy tests exclude inventory, private chat and command outcomes.
- [x] No durable observer state; restart ends observer sessions (O11); target generation change ends grant and streams (O3).
- [x] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Audit uses the existing event foundation; activation waits for all three children and CP enablement.
- [ ] Every BLOCKING dependency merged before the PR leaves draft.
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests live in their own child files, not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation and unfinished work (2026-10-22)

PROVEN locally against baseline `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`
plus the uncommitted lane delta. No commit/push/reset/checkout/stash was run.

```sh
cargo check --locked -p oteryn-gameplay-dispatch -p oteryn-gameplay-social -p oteryn-protocol-oteryn -p oteryn-gameplay-foundation --quiet
cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-gameplay-social -p oteryn-protocol-oteryn -p oteryn-gameplay-foundation --all-targets --quiet -- -D warnings
cargo test --locked -p oteryn-gameplay-social --lib spectate --quiet
cargo test --locked -p oteryn-gameplay-dispatch --lib spectate --quiet
cargo test --locked -p oteryn-protocol-oteryn spectate --quiet
cargo test --locked -p oteryn-protocol-oteryn --quiet
cargo test --locked -p oteryn-gameplay-foundation --lib foundation::admission::tests --quiet
cargo fmt --all -- --check
rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/spectate_session_ingress_tests.rs
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
git diff --check
```

Results: check/clippy/fmt passed; social spectate 8, dispatch spectate 6,
protocol spectate 4 (full protocol suite 289), native admission 41,
governance validator and 124 governance tests passed. Cargo reports an existing
vendored Tokio documentation warning; no owned-code clippy warning remains.
No PG target ran: no persistence rule/database object/migration changed. No
CI-only contention failure occurred. Workspace cargo was not run, per D1466.

Full diff, including new files, re-read against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`;
the discovered map-snapshot bound was corrected to reuse native codec limits.
All new type constructor/match/impl references were searched with `rg` across
`apps/`, `crates/`, `tools/`; no existing public enum acquired a variant.
New public production methods `SpectateRuntime::apply`, `SpectateSession::dispatch`,
`ObserverEnvelope::is_ingress` have the real admission/dispatch call chain and
`spectate_all_observer_bootstraps_fail_before_player_authority` coverage; dispatch
suites cover every registered gameplay command, egress and concurrent writers.
Fixture-only methods are gated by test support. No tests weakened, removed or duplicated.

Reused: native admission fixture/commit/current-snapshot constructors and typed IDs;
connection tests' `Authority`, `Identifiers`, `drive`, `run` through one child include;
native map/spatial codecs (including negotiated lights/handles), public-chat and
presentation codecs/bounds; `charm_wire` protobuf helpers; TLS-provider CSPRNG,
AWS-LC constant-time comparison; bounded `write_frame` and NET03 egress rows;
ANL schema/registry/retention conventions (durable writer deferred). Shared spectate
test adapters wrap these owners and serve the dispatch suites; no fixture body copied.

Unfinished activation gates: Platform O2 authenticated single-use bootstrap/cross-node caps;
SPECTATE-AUDIT-1 durable envelope/outbox writer; production lifecycle/publication source
binding and target-client presentation when authorized; measured node/rate-table/audit
capacities; GM-1 abuse-signal binding; independent security/privacy/protocol/concurrency review, CI, CP integration
and enablement. CONTACTS waits for its native consent consumer. No capability/admission
is offered; the Game-only owner request port is not authentication or a client grant route.

## Final changed-file sizes

No file grew by more than 200 lines. Existing large Rust files received wiring and
one test-support gate; the private fixture body is unchanged. Runtime is L4 social;
transport/IO composition is L5 dispatch; bounded serialization is in the protocol crate.

| Changed file | Final lines | Growth from HEAD |
|---|---:|---:|
| `crates/gameplay/dispatch/src/gameplay_transport/connection.rs` | 8263 | +2 |
| `crates/gameplay/dispatch/src/gameplay_transport/mod.rs` | 8168 | +1 |
| `crates/gameplay/dispatch/src/gameplay_transport/spectate_concurrency_tests.rs` | 40 | +40 |
| `crates/gameplay/dispatch/src/gameplay_transport/spectate_egress_tests.rs` | 82 | +82 |
| `crates/gameplay/dispatch/src/gameplay_transport/spectate_session.rs` | 141 | +141 |
| `crates/gameplay/dispatch/src/gameplay_transport/spectate_session_ingress_tests.rs` | 63 | +63 |
| `crates/gameplay/dispatch/src/gameplay_transport/spectate_session_tests.rs` | 136 | +136 |
| `crates/gameplay/foundation/src/foundation/admission.rs` | 2875 | +4 |
| `crates/gameplay/foundation/src/foundation/admission_test_support.rs` | 8 | +8 |
| `crates/gameplay/social/src/lib.rs` | 55 | +2 |
| `crates/gameplay/social/src/spectate/audit.rs` | 115 | +115 |
| `crates/gameplay/social/src/spectate/audit_tests.rs` | 84 | +84 |
| `crates/gameplay/social/src/spectate/grant.rs` | 143 | +143 |
| `crates/gameplay/social/src/spectate/grant_tests.rs` | 89 | +89 |
| `crates/gameplay/social/src/spectate/lifecycle_tests.rs` | 107 | +107 |
| `crates/gameplay/social/src/spectate/mod.rs` | 30 | +30 |
| `crates/gameplay/social/src/spectate/projection.rs` | 112 | +112 |
| `crates/gameplay/social/src/spectate/projection_tests.rs` | 68 | +68 |
| `crates/gameplay/social/src/spectate/rate_window.rs` | 30 | +30 |
| `crates/gameplay/social/src/spectate/rate_window_tests.rs` | 14 | +14 |
| `crates/gameplay/social/src/spectate/runtime.rs` | 162 | +162 |
| `crates/gameplay/social/src/spectate/session.rs` | 156 | +156 |
| `crates/gameplay/social/src/spectate/session_tests.rs` | 133 | +133 |
| `crates/gameplay/social/src/spectate/spectate_test_support.rs` | 90 | +90 |
| `crates/gameplay/social/src/spectate/world_projection_tests.rs` | 98 | +98 |
| `crates/protocol-oteryn/src/capability_tables.rs` | 193 | +8 |
| `crates/protocol-oteryn/src/lib.rs` | 4796 | +2 |
| `crates/protocol-oteryn/src/spectate.rs` | 80 | +80 |
| `crates/protocol-oteryn/src/spectate/audit.rs` | 85 | +85 |
| `crates/protocol-oteryn/src/spectate/codec.rs` | 157 | +157 |
| `crates/protocol-oteryn/src/spectate/envelope.rs` | 72 | +72 |
| `crates/protocol-oteryn/src/spectate/frame_tests.rs` | 36 | +36 |
| `crates/protocol-oteryn/src/spectate/spectate_tests.rs` | 161 | +161 |
| `docs/agents/tasks/SPECTATE-1.md` | 343 | +149 |
| `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` | 912 | +0 |
| `docs/architecture/reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md` | 100 | +100 |
| `docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json` | 200 | +2 |
| `docs/contracts/protocol-oteryn/v1/registry/registry.json` (message type 15) | - | +1 |
| `docs/contracts/protocol-oteryn/v1/registry/{capabilities/0033,command_types/0034,0035,state_domains/0025}-*.json` | - | new |
| `docs/contracts/resource-limits/entries/<ordinal>-SPECTATE1-RL-0{1..6}.json` | - | new |
| `docs/contracts/game-events/v1/spectate.proto` | 26 | +26 |
| `docs/contracts/protocol-oteryn/v1/spectate_v1.proto` | 45 | +45 |
