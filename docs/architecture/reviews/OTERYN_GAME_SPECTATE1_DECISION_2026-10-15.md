# SPECTATE-1 Observer contract

- Decision: `SPECTATE1-OBSERVER-CONTRACT-V1`
- Status: **CANDIDATE** (architect-authored). Acceptance needs exact-head validation, independent review (security, privacy, protocol and concurrency) and protected integration. Owner acceptance is the owner's to give; none is recorded here.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the mechanics completion plan row 138 (`OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md`, batch SPECTATE-1: "spectating"); the task record `docs/agents/tasks/SPECTATE-1.md` (goal "read-only view of a player session"; criteria "spectator cannot issue any gameplay command" and "consent of the spectated player"; the record notes no SPECTATE child exists in App. 7); the gap register §20 ("spectators, replay/event streams and privacy", `GAME-INSTANCES-01`); VSL-MOVE-01 §15 (spectator policies deferred).
- Builds on: the social presence baseline (`SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md`, owner-accepted: consent, no unilateral tracking, fail toward less disclosure); the instance baseline (`INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`, owner-accepted: placement privacy, the Gateway is not a gameplay proxy, single-use admission material); ADR-0003 (the client never selects a server); FND-04 and FND-04B (GameSession, lease, generation, §22 locator); VSL-MOVE-01 §13 (interest derived post-commit from committed state); CHAT-0; NET-03; GAME-CHANNEL-01; ARENA-1 A11 (CANDIDATE, the Tibiadrome roof); owner rule 5905825574 (Global parity).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Runtime, migration, wire and production authority: NONE until accepted and allocated. Each child needs its own #162 allocation.

## 1. Scope

This brief decides what an observer is, how it differs from an actor that sees things, how consent, privacy and visibility apply, the observer protocol and its bounds, and the child graph. It does not decide whether a player-facing observer feature ships at all under Global parity (§5, owner question), staff observation (GM-1, §6) or replays (§6).

## 2. Facts

- **Reference.** The only spectating the repository reference describes is the Tibiadrome: "Free accounts may spectate from the roof only" (`docs/reference/tibia-manual/combat.md` §5.3.14), which is a character standing on ordinary map. No manual chapter, TibiaWiki snapshot or Crystalserver path (`imports/crystalserver/summer-update/source-tree.json`) describes a player broadcast, cast or observer session; Crystalserver's `src/map/spectators.cpp` and `data/libs/functions/spectators.lua` are the internal "who is in range" query, not an observer feature.
- **Ordinary visibility today.** VSL-MOVE-01 §13 derives each observing character's interest set from committed spatial state (rows `MOVE-VIEW-WIDTH` 36, `MOVE-VIEW-HEIGHT` 28, `MOVE-VIEW-FLOORS` 8, `MOVE-RL-08`, `MOVE-RL-11`). An actor's interest set is a consequence of where its character stands; it carries no consent and no permission.
- **Privacy.** Instance placement, channel, GameNode and exact position are non-public (instance baseline, Privacy consequences); presence fails toward less disclosure; no unilateral tracking (social presence baseline, Rejected interpretations).
- **Sessions.** A GameSession is bound to one character lease and generation (FND-04); routing comes only from the World Registry and Gateway (ADR-0003); admission material is bounded by `FND02-ADMISSION-MATERIAL-BYTES`; egress per admitted session by `NET03-OUTBOUND-QUEUE-ENTRIES` and `NET03-OUTBOUND-QUEUE-BYTES`.

## 3. Rules

**O1. Two different things.** *Actor visibility* is what a character perceives because of where it is: its interest set, ordinary chat in range, the Tibiadrome roof. *Observation* is a session receiving another character's view without being there. Actor visibility is not an observer permission, in both directions:
- no relation (being in view, party, guild, house access, VIP or contact, same account, same instance, killer or victim) grants observation;
- an observer is not an actor: it has no position, is in no interest set, is never returned by a "who is in range" query (spells, boss gates, encounter `near` areas, quest portals), is not counted in presence or online lists, and cannot be targeted, traded with or spoken to by the world.

**O2. The observer session.** An observer connects through the Gateway like any client (ADR-0003); the account is authenticated; the Gateway issues single-use, short-lived observer admission material (within `FND02-ADMISSION-MATERIAL-BYTES`) for the node that owns the target's current scope, resolved through the FND-04B §22 locator, never through a client-chosen endpoint. The admitted session is of kind `OBSERVER`: its own GameSessionId and generation, no character lease, no CharacterId actor, no fence, no command sequence for gameplay. It is bound to exactly one observation grant (O3) and one target GameSession generation.

**O3. Consent is a grant by the target, scoped to its current session.**
- The target character's own session creates an *observation grant* by an explicit command: off by default, never inherited from settings, another character of the account, or a previous session. The grant names its audience: `CODE` (whoever presents a single grant code the target shares out of band; rotated on demand) or `CONTACTS` (named characters with accepted contact consent under the social presence baseline; never account-wide promotion, never alternate characters implied).
- The grant lives in the target's runtime owner, bound to the target's GameSession generation. It is not durable. Revocation by the target, logout, a session generation change, a lease change, death and a scope change (O7) end it.
- Ending a grant ends every observer stream bound to it within `SPECTATE1-RL-05`; a stale grant fails toward less disclosure (no frame after the end is sent). Codes are compared in constant time and refused uniformly (`NOT_AVAILABLE`), so an absent target, a revoked grant, a wrong code and an offline character are indistinguishable.
- Granting is per target; nothing grants observation of a scope, an instance or a channel as such.

**O4. The projection: world tier only.** An observer receives the target's *world projection*: the map, creatures, items on the map, effects, ordinary public speech and presentation events the target's own client receives from the target's interest set, plus the target's own appearance, health and mana bars as other characters in view would see them. Never projected: the target's inventory, containers, equipment detail, skills, stats, imbuements, conditions beyond what bars show, private messages, party, guild and private chat rooms, VIP or contact lists, NPC trade, market, store, house lists, typed command outcomes and any account or Platform data. Additional disclosure tiers are not v1 (§5).

**O5. Read-only by construction.** The observer session's command table holds only observer commands: `OBSERVE_STOP` and the transport keepalive. Every gameplay command type (movement, use, attack, spell, chat, NPC, trade, container, logout and any later one) is refused at the transport dispatch with `NOT_ALLOWED` before any gameplay handler runs; the observer path has no entry into the gameplay command dispatch chain. Acceptance evidence: a test that sends every registered command type on an observer session and asserts refusal and no state change.

**O6. Fan-out never affects the target.** Frames are derived after commit from the same post-commit observation facts the target receives (VSL-MOVE-01 §13); observation adds no read of authoritative state and no lock. Each observer has its own egress, bounded by the NET-03 rows; on overflow the observer is dropped (`OBSERVE_END{OVERLOAD}`), never the target, and the target's tick never waits on an observer. A target has at most `SPECTATE1-RL-01` observers; further admissions are refused `FULL`.

**O7. Scope changes and PvP.** When the target changes scope (channel change, instance admission or exit, cross-node handoff, house entry), every observer stream ends with `OBSERVE_END{TARGET_MOVED}` and the grant ends; the target grants again in the new scope if it wants. Observation is refused, and an existing grant ends, while the target is in a scope whose content or ruleset marks it `observable: false`. In v1 that default is false for PvP arena matches (ARENA-1 A9) and any scope with PvP enabled for the target, which prevents live ghosting; boss rooms and other instances default to observable.

**O8. The target knows.** The target's client shows that a grant is active and the current observer count. Observer identities are not shown to the target and the target's identity is not revealed beyond what the projection shows; an observer's account never becomes visible to anyone through observation (no alternate-character inference).

**O9. Audit and abuse.** Grant creation, revocation and each observer admission and end produce Game events (no position, no code). Repeated wrong codes from one account are rate-limited (`SPECTATE1-RL-04`) and count as abuse signals for GM-1. Observation never extends a session's lifetime or keeps a scope warm.

**O10. Protocol.** One capability `SPECTATE_V1`, gated and not offered until the owner decision of §5 and CP enablement. The family in `crates/protocol-oteryn/src/spectate.rs`: target-side commands `OBSERVE_GRANT_SET{audience}`, `OBSERVE_GRANT_REVOKE`; target-side state (grant active, observer count); observer-side `OBSERVE_STOP`; observer frames that reuse the existing world map, spatial, presentation and public chat payloads unchanged inside an observer envelope (no second encoding of world state); `OBSERVE_END{reason}` with reasons `REVOKED`, `TARGET_GONE`, `TARGET_MOVED`, `OVERLOAD`, `NOT_OBSERVABLE`. Registration in `PROTOCOL_OTERYN_V1_REGISTRY.json` and the error code registry happens at the child's slot turn, not here.

**O11. Multiplicity and durability.** Observation writes nothing durable except audit events, mints nothing and grants nothing, so channel count is irrelevant (GAME-CHANNEL-01 §13). A node restart ends all observer sessions; there is no recovery and no replay.

**O12. Rows (proposed; registered by the children before implementation).**

| Row | Value | Source |
|---|---|---|
| `SPECTATE1-RL-01` observers per target GameSession | 50 | engineering ceiling |
| `SPECTATE1-RL-02` observer sessions per account | 2 | engineering ceiling |
| `SPECTATE1-RL-03` observer sessions per node | set from measurement before activation, as `HOUSERT0-RL-04` | measurement |
| `SPECTATE1-RL-04` grant code attempts per account | 5 per minute | engineering ceiling |
| `SPECTATE1-RL-05` grant end to last frame | one Channel owner work cycle | engineering |
| `SPECTATE1-RL-06` grant code length | 16 bytes of CSPRNG output | engineering |
| Observer egress | `NET03-OUTBOUND-QUEUE-ENTRIES`, `NET03-OUTBOUND-QUEUE-BYTES` (cited, unchanged) | NET-03 |
| Observer admission material | `FND02-ADMISSION-MATERIAL-BYTES` (cited, unchanged) | FND-02 |
| Projection volume | the target's own rows (`MOVE-RL-08`, `MOVE-RL-11`, `SPELLPRES0-RL-01`), unchanged | VSL-MOVE-01 and spell presentation |

## 4. Packets (proposed)

Allocated one by one by the control plane after acceptance and the §5 owner answer; each its own PR.
1. **SPECTATE-GRANT-1** (hard, security and privacy review): O3, O7 and O8 in the target's runtime owner: grant commands, audience checks against contact consent, code generation and constant-time compare, grant end on every trigger, audit events. Needs CHAT-1 (contact consent surface), SCOPE-HANDOFF-1.
2. **SPECTATE-SESSION-1** (hard, security and concurrency review): O2, O5, O6 and O9: Gateway observer admission and the `OBSERVER` session kind, the read-only command table and its all-commands refusal test, per-observer egress and drop, the per-target cap. Needs SPECTATE-GRANT-1 and the FND-04B §22 locator.
3. **SPECTATE-WIRE-1** (protocol, Reg): O4 and O10: capability, family, the projection filter and its privacy tests (no inventory, private chat or command outcome ever reaches an observer). Needs SPECTATE-SESSION-1, MAP-WIRE-2, CHAT-1b-2b; its capability is reserved on #162 at allocation.

Order: SPECTATE-GRANT-1, SPECTATE-SESSION-1, SPECTATE-WIRE-1. The plan row's other listed prerequisites gate only what the projection carries, not the contract.

## 5. Open points (not decided here)

- **Owner question (Global parity):** no repository reference shows a player observer feature in Global Tibia (§2). Options: (a) build the contract and keep `SPECTATE_V1` off until a reference or an owner deviation exists; (b) treat it as an accepted Oteryn deviation and enable it after the children; (c) drop player observation and keep only staff observation (GM-1). Recommendation: (a), so the privacy contract is fixed before any producer exists.
  - **Owner answer (2026-10-15): (b).** Player observation is an accepted Oteryn deviation from Global parity. `SPECTATE_V1` may be offered once SPECTATE-GRANT-1, SPECTATE-SESSION-1 and SPECTATE-WIRE-1 have merged and CP enables it; the O1 to O12 rules are unchanged.
- Additional disclosure tiers (equipment, skills, the target's own chat) per grant.
- An observer chat room, and whether observers may see each other.
- Whether a broadcast delay could replace the O7 PvP refusal.

## 6. Split out, not decided here

- **Staff observation** (GM invisibility, following a reported character): GM-1, with moderation authority and audit, never the player grant.
- **Replays and recorded streams** (gap register §20): no producer in this scope; a replay would be a durable disclosure and needs its own decision.
- **Instance spectator galleries and tournaments**: GAME-INSTANCES-01 and ARENA-1 §6.
- **The Tibiadrome roof**: ordinary map and visibility (ARENA-1 A11), outside this contract.

## 7. Handback

```yaml
result: CANDIDATE
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_SPECTATE1_DECISION_2026-10-15.md
resource_values_changed: false
production_authority_changed: false
implementation_may_resume: false
required_fresh_allocation: true
required_independent_review: "security, privacy, protocol, concurrency"
owner_answer_s5: "b (accepted Oteryn deviation, 2026-10-15)"
blocked_on: "contract acceptance (exact-head validation, independent review) and the three children before SPECTATE_V1 is offered"
```
