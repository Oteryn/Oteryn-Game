# ADMIT-0 Channel-required gameplay capabilities

- Decision: `ADMIT0-CHANNEL-REQUIRED-CAPABILITIES-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol and
  security, with the FND-04 owner) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: owner answer 4a (#162 5929192803): "the owner consents to the FND-04 amendment
  (WORLDINT-ADMIT-1)"; RUNE-USE-0 `RUNEUSE0-C7` and R5; WORLD-INTERACTION-0 §8.6. This is the
  decision child WORLDINT-ADMIT-1 that WORLD-INTERACTION-0's brief names.
- Builds on: FND-02 §9 and §11, FND-04A §6, §7, §7.1, §11 and §12, FND-04B §12, §13, §18, §19, §24-§26,
  FND-04C §2.1, §4 and §9, the node-boot composition decision D1 and D5 (declared readiness
  revisions), `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (capabilities and offer gates),
  ADR-0018 (the browser client keeps FND-02 and FND-04), RUNE-USE-0 §11, WORLD-INTERACTION-0 §8 and
  §10.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ADMIT-CAP-1 | hard (authority, protocol), protocol and security review | the `requires` field in the protocol registry; the declared `required_gameplay_capabilities` beside `world_policy_revision` in the D1 configuration, its closure and boot validation (§3); the check at fresh admission, reconnect and recovery (§4); forced selection; the three error codes in every FND-04 table (§5); `ADMIT0-RL-01` registered | the numbers of `MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS`, `WORLD_INTERACTION_V1` reserved on #162 by their wire children |
| ADMIT-CAP-2 | impl, determinism review | the channel activation guard (§6) | ADMIT-CAP-1; FIELD-2 |

Tests (FND-04A §12, FND-04B §26, FND-04C §9 fixtures): a bootstrap without a required capability is
refused with `ADMISSION_CAPABILITY_REQUIRED` and consumes no nonce; a refusal never precedes
authentication; with the capability it is selected; a reconnect and a recovery without it are
refused with their codes; a boot whose closure holds an unregistered, cyclic or unoffered id fails;
a channel without the field set never applies a player field effect.

## 1. Question

How does a channel refuse a client that cannot see the fields, walls and world objects that can
hurt or block its character, without making those capabilities core protocol for every World?

## 2. Facts

**PROVEN**

- FND-02 §9: optional capabilities are negotiated by intersection; added "only when an older
  same-major peer can safely continue without the feature"; "a capability becomes active only if
  selected by the authoritative negotiation path". No rule lets a channel require one.
- FND-02 §11: `ClientBootstrap` carries supported optional capabilities; `ClientResume` carries
  "protocol/transport/schema/capability support evidence"; FND-04 owns admission and resume
  eligibility.
- Node-boot D5: `world_policy_revision` and the other readiness revisions are declared per channel
  scope in the D1 configuration, read at boot, and must match what the Platform puts in grants;
  each dimension is compared independently. FND-04A §6 and §7 step 11 bind and revalidate them;
  FND-04B §13 item 10 and §18 revalidate them for reconnect and recovery.
- FND-04A §11 owns its error rows ("FND-04C may integrate but not silently alter accepted rows");
  FND-04B §25 freezes its transition semantics; FND-04C §4 integrates the catalogue; FND-04C §2.1
  lists the safe correlation fields.
- FND-04B §24: before successful recovery authentication, diagnostics reveal no semantic match
  state.
- `PROTOCOL_OTERYN_V1_REGISTRY.json`: capabilities 1, 6 and 7, each with an `offered` flag and an
  offer gate, no `requires` field; `crates/protocol-oteryn/src/lib.rs`: `MAX_CAPABILITY_COUNT = 128`.
- RUNE-USE-0 §11 (`RUNEUSE0-C7`) and R5: player-affecting fields and walls wait for "an accepted
  gameplay-admission rule, owned by FND-04, that refuses a session without
  `WORLD_SPATIAL_FIELDS` on any channel where such a field can exist".
- WORLD-INTERACTION-0 §8.6 names `MAP_STATE_V1`, `WORLD_SPATIAL_FIELDS` and `WORLD_INTERACTION_V1`;
  §10.1: `WORLD_INTERACTION_V1` requires `MAP_STATE_V1` and `ITEM_USE_V1`.
- ADR-0018: the browser client has no FND-02 or FND-04 exemption.

## 3. The channel requirement (ADMIT-CAP-1)

### 3.1 Declared with the policy revision

- The D1 configuration declares, beside each channel scope's `world_policy_revision`,
  `required_gameplay_capabilities`: a sorted, unique list of registered capability ids, at most 16
  declared ids (`ADMIT0-RL-01`); the closure (§3.2) is bounded by `MAX_CAPABILITY_COUNT`. Amended:
  node-boot D1 and D5 (this PR).
- **Operator rule:** a changed list is deployed with a new `world_policy_revision` token. Safety does
  not depend on it: every check runs against the current scope generation's set, and the Platform
  needs only the token (D5).
- Every public World's channels under WORLD-INTERACTION-0 declare `MAP_STATE_V1`,
  `WORLD_SPATIAL_FIELDS` and `WORLD_INTERACTION_V1` (owner answer 4a). An empty list behaves as
  today.

### 3.2 Closure and boot validation

- The protocol registry gains a `requires: [id]` field per capability (ADMIT-CAP-1), taking the
  dependencies now written in prose (MAP-WIRE-1, WORLD-INTERACTION-0 §10.1).
- At boot the node computes the **effective set**: the closure of the declared list under
  `requires`. Boot fails closed (the scope does not publish readiness) if the closure holds an
  unregistered id, a cycle, or an id this build does not offer (its offer gate is closed): a
  required capability must be one the server can serve.
- The effective set is fixed for the scope's ownership generation. A changed list takes effect only
  with a new boot, that is a new ownership generation. **Premise:** node-boot D3's stop-then-start
  replacement, under which a session reaches a new channel generation only through fresh admission,
  reconnect or recovery, each of which checks it (§4). Overlapping replacement and healthy-session
  migration (FND-04B §10) are superseding triggers.
- **Every transfer into a channel scope** (Channel to Channel, Instance to Channel, a handoff, a
  migration, any later transfer contract) must, before its commit: check the destination scope's
  effective set against the session's declared support (refusing with `RECONNECT_CAPABILITY_REQUIRED`
  semantics, current authority preserved), select every capability of the set, and reconcile by a
  replacement snapshot (§4). Until the contract of such a transfer adds this check, that transfer
  into a channel with a non-empty set is refused (fail closed). This holds whatever the source
  scope's set was, so a session recovered in a scope with an empty set cannot enter a public channel
  unchecked (WORLD-INTERACTION-0 §9.2).
- **Instances:** an InstanceRuntime scope has no declared set and applies no player field or wall
  effects until its own contract declares one.

### 3.3 Selection

Every capability of the effective set is selected for an admitted, reconnected or recovered
session (FND-02 §9). The server never offers a degraded session in its place.

## 4. The check

The check is one predicate: the client's supported list contains the target scope's effective set.
It runs after authentication and only against the current scope generation's set:

- **Fresh admission** (FND-04A §7 step 11 and §7.1): after the signature (step 6) and after the
  `world_policy_revision` equality of step 11 succeeds, before GrantNonce eligibility (step 12); a
  refusal consumes no nonce and mutates nothing.
- **Same-session reconnect** (FND-04B §12 and §13 item 10): after the reconnect proof is
  authenticated (§9) and the session resolved, against the current scope generation's set.
- **Recovery, same-session or post-grace** (FND-04B §18-§21): after the recovery credential is
  authenticated, ownership and world are classified safe, and §19 dispatch has chosen the path,
  immediately before that path's commit revalidation, against the current scope generation's set;
  a post-grace recovery that creates a new GameSession selects the set (§3.3).
- **Reconciliation:** when a resumed or transferred session's selected set differs from its
  predecessor's (a new scope generation or a transfer added a capability), reconciliation uses a
  replacement snapshot (FND-02 §16), never replay, so no newly selected domain receives deltas
  without a baseline.
- **Transfers into a channel scope:** the same predicate, selection and replacement snapshot before
  the transfer commits (§3.2).
- The check reads declared support only. Support is a claim, not trust: the server still sends and
  enforces everything; the rule guarantees only that the client said it can render what can hurt
  or block it.

## 5. Errors

| Code | Category | Progression | Retry / next authority | Mutation | Public class |
|---|---|---|---|---|---|
| `ADMISSION_CAPABILITY_REQUIRED` | `UNSUPPORTED_REVISION` | `TERMINAL` | a client build supporting the channel's required capabilities; a new grant | `NO_AUTHORITY_MUTATION` | `CLIENT_UPDATE_REQUIRED` |
| `RECONNECT_CAPABILITY_REQUIRED` | `UNSUPPORTED_REVISION` | `TERMINAL` for the candidate | a client build supporting the channel's required capabilities, through recovery | `CURRENT_AUTHORITY_PRESERVED` | `CLIENT_UPDATE_REQUIRED` |
| `RECOVERY_CAPABILITY_REQUIRED` | `UNSUPPORTED_REVISION` | `TERMINAL` | a client build supporting the channel's required capabilities; a new recovery grant | `NO_AUTHORITY_MUTATION` | `CLIENT_UPDATE_REQUIRED` |

- Correlation: the missing registered capability ids, emitted only after the authentication point
  of §4 (FND-04B §24).
- Amended in this PR: FND-04A §11 (rows) and §12 (fixtures), FND-04B §25 (rows) and
  §26 (fixtures), FND-04C §2.1 (the correlation field), §4.1-§4.3 (rows) and §9 (fixtures).

## 6. Activation (ADMIT-CAP-2)

- A channel applies player effects of fields and blocking walls (FIELD-2) only when its scope's
  effective set holds `WORLD_SPATIAL_FIELDS` and `WORLD_INTERACTION_V1`. The set is fixed for the
  ownership generation (§3.2), so the guard never flips while sessions are attached.
- With this decision accepted and ADMIT-CAP-1 landed, `RUNEUSE0-C7` is satisfied for such channels;
  FIELD-2 may then implement Magic Wall, Wild Growth and player-affecting fields
  (WORLD-INTERACTION-0 §8). Amended: RUNE-USE-0 §11 and WORLD-INTERACTION-0 §8.6.

## 7. Rejected options

- **Make the capabilities core protocol v1.** FND-02 §9 keeps core semantics fixed for protocol
  major 1; channels without fields (tests, future profiles) would pay for them.
- **Hide fields from clients that lack the capability.** A player would be hurt or blocked by tiles
  it cannot see (RUNE-USE-0 R5).
- **A separate World policy record with its own publication.** No such record exists; the declared
  `world_policy_revision` of D5 is already bound by every grant and revalidated at every boundary.
- **A durable per-session selected set.** The scope generation's set is fixed and every attach
  checks it, so there is nothing a restart must remember.

## 8. Architect rulings (owner answer 4a; owner rule 5905825574)

- **R1. Where the list lives.** a) Beside the declared `world_policy_revision`, immutable per token
  (recommended: already bound by grants and revalidated, no new record or dimension); b) a new World
  policy record. **Ruled a).**
- **R2. What reconnect checks.** a) The current scope generation's set (recommended: the same
  predicate everywhere, nothing durable); b) the set selected at admission. **Ruled a).**
- **R3. Bound.** a) 16 declared ids (recommended); b) up to `MAX_CAPABILITY_COUNT`. **Ruled a).**

## 9. Owner questions

None. Owner answer 4a gave consent; every other choice applies FND-02 and FND-04.

## 10. Decision test

- **Must decide now:** YES. Without it Magic Wall, Wild Growth and every field that hurts players
  stay creature-only (`RUNEUSE0-C7`).
- **Minimum sufficient:** one declared list per policy token, one predicate at the existing
  revalidation points, three error codes, one guard.
- **Superseding evidence:** a protocol major 2 that makes the capabilities core; measured content
  activation replacing D5's declared revisions (the list then moves with `world_policy_revision`).
- **Deliberately not decided:** the capability numbers, the client integrity attestation
  (SEC-CLIENT-01, owner answer 5b), PvP field rules (PARTY-PVP-0).

## 11. Before-freeze checklist

1. **Contract amendments**, all applied in this PR: FND-02 §9; FND-04A §6, §7 (step 11), §7.1, §11,
   §12; FND-04B §13, §18, §25, §26; FND-04C §2.1, §4.1-§4.3, §7, §9; node-boot D1 and D5;
   RUNE-USE-0 §11; WORLD-INTERACTION-0 §8.6. The registry `requires` field and the D1 configuration key are built by
   ADMIT-CAP-1.
2. **Serialization:** the check runs inside each path's existing atomic revalidation; the set is
   fixed per scope ownership generation, so no publication can race an admission.
3. **Restart:** the set is re-read and re-validated at boot; nothing per session is persisted.
4. **Typed references:** registered FND-02 capability ids; the set is keyed by the channel scope and
   its `world_policy_revision` token.
5. **Wire:** no new message; older clients get `CLIENT_UPDATE_REQUIRED` on a channel that requires
   what they lack.
6. **Split work:** none; one predicate per existing atomic boundary.
