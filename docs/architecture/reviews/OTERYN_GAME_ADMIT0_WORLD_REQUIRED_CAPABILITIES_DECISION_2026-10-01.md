# ADMIT-0 World-required gameplay capabilities

- Decision: `ADMIT0-WORLD-REQUIRED-CAPABILITIES-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol and
  security, with the FND-04 owner) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: owner answer 4a (#162 5929192803): "the owner consents to the FND-04 amendment
  (WORLDINT-ADMIT-1)"; RUNE-USE-0 `RUNEUSE0-C7` and R5; WORLD-INTERACTION-0 §8.6. This is the
  decision child WORLDINT-ADMIT-1 that WORLD-INTERACTION-0's brief names.
- Builds on: FND-02 §9 and §11 (capability negotiation, bootstrap and resume), FND-04A §6, §7 and
  §7.1 (fresh-admission applicability and final revalidation), FND-04B §12, §13 and §18
  (reconnect and recovery revalidation), FND-04C §4 (the error catalogue), ADR-0018 (the browser
  client keeps FND-02 and FND-04 unchanged), MAP-WIRE-1, RUNE-USE-0 §11, WORLD-INTERACTION-0 §8
  and §10.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ADMIT-CAP-1 | hard (authority, protocol), protocol and security review | the World policy field `required_gameplay_capabilities` and its closure (§3); the check in fresh admission, same-session reconnect and recovery (§4); the three error codes (§5); forced selection of required capabilities (§3.3) | the capability numbers of `MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS`, `WORLD_INTERACTION_V1` reserved on #162 (MAP-WIRE-2, FIELD-WIRE-1, WORLDINT-WIRE-1) |
| ADMIT-CAP-2 | impl, determinism review | the channel activation guard (§6): FIELD-2 player effects and blocking walls run only on a channel whose World policy carries the field set | ADMIT-CAP-1; FIELD-2 |

Tests: a bootstrap without a required capability is refused with `ADMISSION_CAPABILITY_REQUIRED`
and mutates nothing; with it, the capability is selected; a reconnect and a recovery without it are
refused with their codes; a World policy change that adds a requirement outside a planned world
reset is refused; a channel without the set never applies a player field effect.

## 1. Question

How does a World refuse a client that cannot see the fields, walls and world objects that can hurt
or block its character, without making those capabilities core protocol for every World?

## 2. Facts

**PROVEN**

- FND-02 §9: optional capabilities are negotiated by intersection; "a capability becomes active only
  if selected by the authoritative negotiation path"; "an unknown selected/required capability
  fails closed". No rule today lets a World require a capability.
- FND-02 §11: `ClientBootstrap` carries the supported optional capabilities; `ClientResume` carries
  "protocol/transport/schema/capability support evidence"; FND-04 owns admission and decides resume
  eligibility.
- FND-04A §6 and §7 step 11: the grant binds and admission revalidates, independently,
  `world_policy_revision` among the gameplay revisions; §7.1 repeats them at the atomic boundary.
- FND-04B §13 (COMMIT) and §18: reconnect and recovery revalidate their revisions, including
  `world_policy_revision`.
- FND-04C §4: "the complete FND-04 integration catalogue"; it has no capability refusal.
- RUNE-USE-0 §11 (`RUNEUSE0-C7`) and R5: player-affecting fields and blocking walls wait for "an
  accepted gameplay-admission rule, owned by FND-04, that refuses a session without
  `WORLD_SPATIAL_FIELDS` on any channel where such a field can exist".
- WORLD-INTERACTION-0 §8.6: the rule must refuse a session that did not negotiate
  `MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS` and `WORLD_INTERACTION_V1` on a World where any field can
  hurt or block a player ("every public World under this decision"); §10.1: `WORLD_INTERACTION_V1`
  requires `MAP_STATE_V1` and `ITEM_USE_V1`.
- `crates/protocol-oteryn/src/lib.rs`: `REGISTERED_CAPABILITY_IDS_V1 = [1, 6, 7]`,
  `MAX_CAPABILITY_COUNT = 128`; the three capabilities above are not yet numbered.
- ADR-0018: the browser client keeps FND-02 and FND-04 without exemptions.

## 3. The World requirement (ADMIT-CAP-1)

### 3.1 The field

- The World policy (the record `world_policy_revision` versions) gains
  `required_gameplay_capabilities`: a sorted, unique list of registered capability ids, at most 16
  (`ADMIT0-RL-01`).
- Its **effective set** is its closure under each capability's own declared requirements (for
  example `WORLD_INTERACTION_V1` brings `MAP_STATE_V1` and `ITEM_USE_V1`). The closure is computed
  when the policy revision is published and stored with it; an unregistered id or a cycle refuses
  the publication.
- Every public World under WORLD-INTERACTION-0 carries `MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS` and
  `WORLD_INTERACTION_V1` (owner answer 4a). A World with an empty list behaves exactly as today.

### 3.2 Changing it

- The list changes only with a new `world_policy_revision` published for a **planned world reset**,
  when admission is closed and every player is logged out (ADR-0021 §4.7). A publication that adds a requirement
  while the World has a live GameSession is refused. Removing one is allowed at any revision.
- Rationale: FND-04A §6 already invalidates grants bound to an older revision; the reset rule makes
  sure no admitted session ever lacks a capability its World requires.

### 3.3 Selection

A required capability is always selected for an admitted session (FND-02 §9 "selected by the
authoritative negotiation path"). The server never offers a degraded session in its place.

## 4. The check

- **Fresh admission** (FND-04A §7 step 11 and §7.1): the bootstrap's supported list must contain the
  effective set of the target World's current policy revision. Checked with the other independent
  revisions, before GrantNonce eligibility (step 12): a refusal consumes no nonce.
- **Same-session reconnect** (FND-04B §12 and §13 item 10): the candidate's resume evidence must
  contain every capability the session selected at admission. A client cannot drop one by
  reconnecting.
- **Recovery** (FND-04B §18): the resume evidence must contain the effective set of the World's
  current policy revision.
- The check reads only the client's declared support. Support is a claim, not trust: the server
  still sends and enforces everything authoritatively; the rule only guarantees the client was
  told it can render what can hurt or block it.

## 5. Errors (FND-04C)

| Code | Category | Progression | Retry authority | Mutation | Public class |
|---|---|---|---|---|---|
| `ADMISSION_CAPABILITY_REQUIRED` | `UNSUPPORTED_REVISION` | `TERMINAL` | a client build that supports the set | `NO_AUTHORITY_MUTATION` | `CLIENT_UPDATE_REQUIRED` |
| `RECONNECT_CAPABILITY_REQUIRED` | `UNSUPPORTED_REVISION` | `TERMINAL` for the candidate | the original client build | `CURRENT_AUTHORITY_PRESERVED` | `CLIENT_UPDATE_REQUIRED` |
| `RECOVERY_CAPABILITY_REQUIRED` | `UNSUPPORTED_REVISION` | `TERMINAL` | a client build that supports the set | `NO_AUTHORITY_MUTATION` | `CLIENT_UPDATE_REQUIRED` |

Safe correlation: the missing capability ids (registered public numbers, not secrets). Amended:
FND-04C §4.1, §4.2 and §4.3 (this PR).

## 6. Activation (ADMIT-CAP-2)

- A channel applies player effects of fields and blocking walls (FIELD-2) only when its World's
  current policy revision carries `WORLD_SPATIAL_FIELDS` and `WORLD_INTERACTION_V1` in its
  effective set. Otherwise fields stay creature-only (`RUNEUSE0-C7` unchanged for that World).
- On acceptance of this decision and ADMIT-CAP-1 landing, `RUNEUSE0-C7` is satisfied for such
  Worlds; FIELD-2 may then implement Magic Wall, Wild Growth and player-affecting fields
  (WORLD-INTERACTION-0 §8). Amended: RUNE-USE-0 §11 and WORLD-INTERACTION-0 §8.6 point here.

## 7. Rejected options

- **Make the capabilities core protocol v1.** FND-02 §9 keeps core semantics fixed for protocol
  major 1; Worlds without fields (tests, future profiles) would pay for them.
- **Hide fields from clients that lack the capability.** A player would take damage from, or be
  blocked by, tiles it cannot see (RUNE-USE-0 R5).
- **Check at runtime per tile.** The check belongs where authority is granted; a mid-session refusal
  would strand an admitted character.
- **Add requirements to a live World.** Admitted sessions would lack them; the planned reset is the
  only safe point.

## 8. Architect rulings (owner answer 4a; owner rule 5905825574)

- **R1. Where the list lives.** a) The World policy, versioned by `world_policy_revision`
  (recommended: already bound and revalidated by FND-04A and FND-04B, no new revision dimension);
  b) a new revision dimension. **Ruled a).**
- **R2. Reconnect.** a) The session's selected set, unchanged (recommended: a reconnect is the same
  session); b) the World's current set. **Ruled a)**; the World's set cannot grow while the session
  lives (§3.2), so the two agree.
- **R3. Bound.** a) 16 required ids (recommended: far above the three needed, small for revalidation);
  b) up to `MAX_CAPABILITY_COUNT`. **Ruled a).**

## 9. Owner questions

None. Owner answer 4a gave consent; every other choice is a bound or an application of FND-02 and
FND-04.

## 10. Decision test

- **Must decide now:** YES. Without it Magic Wall, Wild Growth and every field that hurts players
  stay creature-only (`RUNEUSE0-C7`).
- **Minimum sufficient:** one World policy field, one check at three existing revalidation points,
  three error codes, one channel guard.
- **Superseding evidence:** a protocol major 2 that makes the capabilities core.
- **Deliberately not decided:** the capability numbers (reserved by their wire children), the client
  integrity attestation (SEC-CLIENT-01, owner answer 5b), PvP field rules (PARTY-PVP-0).

## 11. Before-freeze checklist

1. **Contract amendments:** FND-02 §9, FND-04A §6 and §7, FND-04B §13 and §18, FND-04C §4.1-§4.3,
   RUNE-USE-0 §11, WORLD-INTERACTION-0 §8.6. All applied in this PR.
2. **Serialization:** the check runs inside the existing atomic revalidation of each path; the
   policy publication serializes with the World's session set (§3.2), and the loser (a publication
   during a live session) is refused.
3. **Restart:** the effective set is stored with the policy revision; a GameSession's selected set
   is part of its session state (FND-02 §11).
4. **Typed references:** capability ids are registered FND-02 ids; the policy is referenced by
   (`WorldId`, `world_policy_revision`).
5. **Wire:** no new message; older clients get `CLIENT_UPDATE_REQUIRED` on a World that requires
   what they lack.
6. **Split work:** none; one check per existing atomic boundary.
