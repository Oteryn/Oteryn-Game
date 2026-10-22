# SPECTATE-1 — spectating (consolidated task card)

```yaml
task_id: SPECTATE-1
title: "SPECTATE-1: spectating (player observation of a consenting target session)"
mode: IMPLEMENT
status: NOT_STARTED   # re-implementation pending under ADR-0022
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row SPECTATE-1 (row 138); §5 Wave 5; App. 7 LOOP-WORLD-1, SPECTATE-1"
brief: "docs/architecture/reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md (not on main; its rules are carried below)"
class: "hard (child graph; security, privacy, protocol, concurrency review)"
base_branch: main
branch: null
pr: null
needs_migration: false   # observation is not durable; a future SPECTATE-AUDIT-1 outbox migration is a separate PR
needs_registry: true
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
```

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
- Wire: `crates/protocol-oteryn/src/spectate.rs` (new), `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (slot).

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
- Account caps in the core are node-local; cross-node caps belong to O2. Node
  capacity is a measured composition input with no activation default.

## Acceptance criteria

- [ ] Spectator cannot issue any gameplay command: every registered command type is refused before a gameplay handler runs, with no state change.
- [ ] Privacy: explicit consent of the spectated player; uniform `NOT_AVAILABLE` refusals; constant-time code compare.
- [ ] Projection privacy tests exclude inventory, private chat and command outcomes.
- [ ] No durable observer state; restart ends observer sessions (O11); target generation change ends grant and streams (O3).
- [ ] Every concurrent transition serialized (named lock or deterministic order) and the losing writer typed.
- [ ] Audit uses the existing event foundation; activation waits for all three children and CP enablement.
- [ ] Every BLOCKING dependency merged before the PR leaves draft.
- [ ] Structure: one mechanic per module file in its ADR-0022 layer.
- [ ] Structure: new tests live in their own child files, not appended to existing files.
- [ ] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [ ] Structure: no file over ~1000 lines grows by more than ~150 lines.

## Validation

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p oteryn-gameplay-social spectate
cargo test -p oteryn-protocol-oteryn spectate   # WIRE-1
python3 tools/agents/validate_governance.py
```
