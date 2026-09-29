# ADR-0020: Native client gameplay entry

- Status: Candidate. Acceptance needs exact-head validation, independent review and protected integration.
- Date: 2026-09-29
- Decision owners: Oteryn project
- Role: Sol Supervising Architect
- Answers: architect item A9 (#162 comment 5879348805)
- Owner decisions: D98 (#162 5879348805), D129 and D138 (#162 5885913325, renumbered in 5886162546)
- Amends: ADR-0011 sections 2 and 3 (the `oteryn-client` edge and the fail-closed rule)
- Defaults: Windows first; synthetic or legally approved assets only
- Runtime, protocol-registry, Platform and production authority: NONE. Each child below needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Context

ADR-0011 keeps the production client fail-closed (`PreNativeProtocol`). Its section 2 amendment says `oteryn-client` must not depend on `oteryn-protocol-oteryn` until a later accepted gate.

The protocol, the server and a working harness now exist. `tools/dev-client` (`oteryn-dev-client`) already does TLS 1.3, ALPN `oteryn-game/1`, admission with a grant, the join snapshot, deltas, `step` and `use`. It is tool-only and outside both production closures.

The owner decided the next step (D98): real graphics and mouse, straight into the production `apps/client`, replacing the fail-closed entry. D129: the first entry includes creatures, other players and chat. D138: login uses a real Platform client grant, and the Platform lane runs in parallel under Platform authority.

This ADR is the accepted gate that ADR-0011 asked for. It does not change protocol, identity or admission contracts.

## Decision

### 1. Crate edge

Extract the session and transport logic from `oteryn-dev-client` into a new production crate, the session crate. Its scope is TLS 1.3, ALPN `oteryn-game/1`, admission, join snapshot, deltas, `step` and `use`. It holds no codec of its own. It uses `oteryn-protocol-oteryn`.

```text
oteryn-client -> session crate -> oteryn-protocol-oteryn
oteryn-dev-client -> session crate      (thin harness)
```

- The crate name is UNKNOWN. It must not contain the fragments `transport` or `game-session`, which `workspace-boundaries.toml` forbids.
- `oteryn-client` gets no direct edge to `oteryn-protocol-oteryn`.
- `oteryn-dev-client` stays tool-only and dev-only. It keeps its public test surface and delegates to the session crate. It is not rewritten.
- The session crate is a production member. It is added to `workspace-boundaries.toml` (production list and edges).

Closure negatives after the change:

- Keep: `protocol-canary`, `protocol-core`, the dev-only and synthetic crates (`oteryn-client-domain`, `oteryn-client-simulation`, `oteryn-synthetic-assets`, `oteryn-test-support`, `oteryn-synthetic-client-harness`), `transport`, `game-session`.
- Replace "`oteryn-client` must not contain `protocol-oteryn`" with "`oteryn-client` contains `protocol-oteryn` only through the accepted session crate". The game-server exception stays.
- `oteryn-dev-client` must stay absent from both production closures.

The CI change (`.github/workflows/merge-gate.yml` closure loop and `workspace-boundaries.toml`) goes into one batched audit rotation with the #1083 game-gate fan-in. It is not a separate rotation.

### 2. Fail-closed rule (ADR-0011 section 3, amended)

New rule: the client is fail-closed unless admission is available.

- No credential is used for a connection that cannot complete. The ordering from ADR-0011 is preserved: check availability first, then request or consume a credential, then connect.
- Any admission error or codec error fails closed with one deterministic result. No retry loop, no guessed framing, no fallback, no late failure after a consumed one-shot grant.
- Success is reported only after the server accepts admission. A transport write is not admission.
- Exact error codes and wording stay with later contracts.

The rest of ADR-0011 (no Canary, no stub protocol adapter, no success claims without admission) stays in force.

### 3. Platform client grant

- The Platform client-grant endpoint is a cross-repository item under Platform authority. This repository has no Platform write authority and creates nothing there.
- The Game side consumes the FND-04 pre-admission grant profile (`docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md`, profile `oteryn-pre-admission-v1`) through `crates/platform-client`.
- The grant is fetched only when the entry check in section 2 says a connection can complete.
- Endpoint shape, transport and timeline on the Platform side: UNKNOWN.

### 4. Renderer

- Minimal 2D tile and sprite batch on the existing `WindowsRenderer` (`crates/renderer`).
- Assets come from `crates/synthetic-assets`. Legally approved assets may be added later.
- No engine, no scene graph and no asset pipeline beyond this slice.

### 5. Input

- A window with mouse: click to tile (move) and click to target.
- Built on `crates/input-platform` and `crates/input-actions`.
- Windows first.

### 6. Scope of the first entry (D129)

- Render the own character, creatures and other players from `WORLD_SPATIAL` visibility.
- This depends on VIS-1 (server interest set) and VIS-2 (schema revision and `WORLD_SPATIAL_ENTITIES` capability) from `docs/architecture/reviews/OTERYN_GAME_MOVE_RL11_VISIBILITY_DECISION_2026-09-28.md`. Until VIS-2 lands, the wire carries only the own actor, so the client shows only the own character.
- Chat needs a chat protocol and a server lane. Neither exists. The design is UNKNOWN. The client shows chat once that lane lands. Chat is a dependency, not part of this ADR's implementation.
- `step` and `use` are the first interactive commands. No other command is added by this ADR.

### 7. Delivery children

Each child needs its own #162 allocation. Numbering is provisional.

| Child | Content | Depends on |
|---|---|---|
| N1 | Session crate extraction; dev-client becomes a thin harness | this ADR |
| N2 | Renderer primitives: tile and sprite batch | this ADR |
| N3 | Mouse input: click to tile, click to target | N2 |
| N4 | Platform grant client (Game side) | FND-04 profile |
| N4-P | Platform client-grant endpoint (Platform lane, Platform authority) | Platform decision |
| N5 | CI closure change, batched with the #1083 game-gate fan-in | N1 |
| N6 | Entity rendering: creatures and other players | VIS-1, VIS-2, N2 |
| N7 | Chat UI | chat protocol and server lane (does not exist) |

The production entry replaces the fail-closed one only when N1, N3, N4, N4-P and N5 have landed and the entry check in section 2 passes. Whether an intermediate build (before N4-P) may ship is UNKNOWN. It must stay fail-closed.

## Decision test

Any later change is inside this ADR only if all of these hold:

- `oteryn-client` reaches `oteryn-protocol-oteryn` only through the session crate.
- The dev-client has no code the session crate could hold.
- No credential is used for a connection that cannot complete.
- Every admission or codec error fails closed.
- The login grant comes from Platform, not from a dev path.
- Assets are synthetic or legally approved.

If one fails, the change needs a new ADR.

## Consequences

Positive:

- One tested session implementation serves the dev harness and the production client.
- The client can reach real gameplay without a new protocol path.
- The fail-closed guarantee stays exact and testable.
- Platform keeps its authority. Game keeps its own.

Costs:

- The closure check and boundaries file change once, in the batched rotation.
- Until VIS-2, the client shows only its own character.
- Until the chat lane exists, there is no chat.
- The first playable entry waits for the Platform endpoint.
- The extraction must keep the dev-client test surface intact.

## Rejected options

- Rewrite dev-client as a new production client: rejected. It duplicates tested session logic.
- Direct `oteryn-client` to `oteryn-protocol-oteryn` edge: rejected. It spreads codec and admission detail into the app. The session crate is the single owner.
- Dev grant path for login: rejected by D138. Login uses the real Platform client grant.
- Keep the fail-closed entry until every feature exists: rejected by D98.

## Deliberately not decided

- Cross-platform clients (Windows first only).
- Real art and a real asset pipeline.
- UI layout beyond this slice.
- Chat protocol and server design.
- Platform endpoint design.
- Session crate name.

## Relationship to existing decisions

- ADR-0011: sections 2 and 3 amended as above. The other sections stand. ADR-0011 carries a one-line pointer to this ADR.
- ADR-0008: `protocol-canary` stays reference-only.
- ADR-0014 and ADR-0016: gameplay transport choice is not changed here.
- ADR-0003: Platform identity and admission boundary stays as accepted.
- This ADR does not change resource values, registry rows or production authority.

## Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#162 5879348805 (A9)"
owner_decisions: [D98, D129, D138]
durable_decision_ref: docs/architecture/ADR-0020-native-client-gameplay-entry.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false   # Platform endpoint stays under Platform authority
implementation_may_resume: true   # N1, N2, N4 may be allocated after acceptance
required_fresh_allocation: true
required_independent_review: "exact-head independent review (crate edge, closure negatives, fail-closed ordering)"
implementation_lanes: [N1, N2, N3, N4, N4-P, N5, N6, N7]
required_revalidation:
  - "N1: dev-client tests pass unchanged over the session crate; the session crate holds no codec of its own"
  - "N5: oteryn-client closure contains protocol-oteryn only through the session crate; canary, dev-only and synthetic negatives still fail the gate; batched with #1083"
  - "N4: no credential is requested for a connection that cannot complete; admission and codec errors fail closed"
remaining_unknowns:
  - session crate name
  - Platform client-grant endpoint shape and timeline
  - chat protocol and server lane design
  - whether an intermediate fail-closed build ships before the Platform endpoint
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates N1, N2 and N4."
```
