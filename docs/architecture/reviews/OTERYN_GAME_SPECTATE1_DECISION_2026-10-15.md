# SPECTATE1 observer contract V1

Status: owner/CP accepted D1035–D1039 and D1095; restored under ADR-0022.

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


## Integrator decision 2026-10-22

Deliver dormant Game-side hooks under option A. No observer trust purpose,
Platform profile or bootstrap is invented; every pre-admission observer frame
is refused. CONTACTS is uniformly unavailable. SPECTATE_V1 stays unoffered.
Activation also waits for measured node capacity, independent review and the
separate O2 and durable SPECTATE-AUDIT-1 consumers. Game-only native owner
requests carry already-authenticated server facts and are never bearer proof.

Event-ID question: each runtime recreation gets a fresh CSPRNG incarnation
namespace and new local ordinals. These producer keys are not ANL EventIds; the
future durable writer must allocate globally unique UUIDv7 EventIds and freeze
them with the copied facts before committing/acknowledging a prefix.
