# ADR-0003: Platform Identity, Game Gateway and game admission boundary

- Status: Accepted foundation
- Date: 2026-08-05
- Decision owners: Oteryn project
- Applies to: `blakinio/Oteryn-v2` integration with `blakinio/Oteryn-Platform`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`

## Context

The historical Open Tibia stack often uses a distinct login server or a login protocol embedded in the game server. Current Canary/Otheryn-compatible deployments may still expose native password login, an external `opentibiabr/login-server`, DB-backed account sessions or other compatibility paths.

Oteryn v2 is a new native stack. Its accepted architecture already assigns web identity, OAuth/PKCE, Game Login Tickets, Game Gateway and World Registry to `blakinio/Oteryn-Platform`. The new Rust server must not become a second credential authority.

`Oteryn-Platform` currently contains a standalone Game Gateway implemented in Go. It exposes the game-login orchestration boundary, redeems a Game Login Ticket through private Platform APIs, obtains an authorized login context, selects an allowed route/protocol candidate and requests bounded **pre-admission Game Session material** for the selected route. This pre-admission material is an authorization/admission capability and is not the canonical logical gameplay `GameSessionId`.

The existing native gameplay work is incomplete. Platform contains contract and producer-side support, but the native candidate is disabled by default. The current Rust client has no `protocol-oteryn` adapter, and no authoritative Rust game server listener exists. Therefore native client-to-server gameplay is not currently proven.

## Decision

### 1. No new classic login server

Oteryn v2 does not introduce a separate classic login-server component that verifies account passwords and returns a character list in the legacy model.

The target control-plane path is divided into:

- **Platform Identity** — verifies reusable account credentials and account-security policy;
- **Game Gateway** — performs bounded game-route and pre-admission session orchestration;
- **Rust game server / game-domain Game Session authority** — validates/consumes the pre-admission material, acquires the character lease, performs final admission and establishes the canonical logical gameplay session.

Legacy Canary/Otheryn login paths may exist during compatibility migration, but they are not part of the target Oteryn v2 runtime and must not be treated as its final authentication architecture.

### 2. Platform remains the credential authority

`blakinio/Oteryn-Platform` remains the sole target owner of:

- account Identity;
- password verification and migration;
- OAuth Authorization Code + PKCE;
- MFA, recovery and account-security policy;
- web/launcher authorization state;
- one-time Game Login Ticket issuance and redemption authority;
- World Registry and channel-route policy.

The Rust client and Rust game server must not introduce:

- another password database;
- direct password verification by the game server;
- direct OAuth authentication to the game server;
- another MFA or recovery authority;
- a second ticket format with equivalent authority;
- direct client selection of a route not authorized by Gateway/Registry.

### 3. Game Gateway remains in Oteryn Platform

Game Gateway remains owned and deployed from `blakinio/Oteryn-Platform`.

It is a control-plane service, not part of the authoritative gameplay runtime. Its responsibilities include:

1. accepting one bounded game-login request and one opaque Game Login Ticket;
2. redeeming the ticket through a private Platform Identity API;
3. resolving authorized account, character, world and channel context;
4. consulting authoritative World Registry policy;
5. selecting one allowed gameplay route/protocol revision;
6. invoking one pre-admission Game Session material issuance path;
7. returning only sanitized endpoint, selection and short-lived admission/session material;
8. failing closed on invalid, ambiguous or unavailable dependencies.

The Gateway must not own character simulation, inventory, combat, world state, durable gameplay persistence or the canonical logical gameplay `GameSessionId` lifecycle.

### 4. Keep the initial Gateway implementation in Go

The current Go Game Gateway remains the initial implementation used by Oteryn v2.

It is not moved into `blakinio/Oteryn-v2` and is not rewritten in Rust as part of foundation work. A rewrite would repeat a security-sensitive component without delivering a required gameplay capability.

A later rewrite requires a separate ADR and must be justified by measured evidence such as:

- material operational complexity caused by maintaining Go;
- inability to satisfy required latency or resource targets after profiling;
- a concrete security/isolation improvement;
- a significant maintenance benefit that outweighs migration and re-audit cost.

Any replacement must preserve or version the public/private contracts, failure semantics, security invariants and rollout/rollback behaviour. Language uniformity alone is not sufficient justification.

### 5. Rust client responsibility

The native Rust client:

1. authenticates through the approved Platform Identity flow;
2. obtains a one-time Game Login Ticket;
3. calls Game Gateway with a bounded supported protocol offer where required;
4. receives the selected endpoint, channel, revisions and short-lived pre-admission session/admission material;
5. establishes the `protocol-oteryn` gameplay connection to the selected Rust game server;
6. never sends the reusable account password to the game server.

### 6. Rust game-server responsibility

The Rust game server:

- accepts only the approved pre-admission Game Session/admission contract;
- validates issuer, audience, expiry, revisions and route/channel binding;
- atomically consumes or validates replay protection according to the future session contract;
- acquires the current character lease;
- rejects stale generation, wrong character/world/channel, incompatible revision and duplicate admission;
- performs final game-owned checks such as character state, ban/disabled policy where contracted and safe entry conditions;
- establishes the canonical logical gameplay session through the game-domain Game Session / Admission authority only after successful final admission;
- starts authoritative gameplay only after successful admission.

It does not issue reusable credentials or choose a different channel than the authorized session material.

The canonical `GameSessionId` is owned and logically issued by the game domain according to `FND-ID-01_GAME_SESSION_ID_OWNER_ISSUER_BASELINE.md`; Platform/Gateway pre-admission material must not be treated as that identity.

### 7. Target flow

```text
Rust client
→ Platform Identity: OAuth Authorization Code + PKCE
→ one-time Game Login Ticket
→ Game Gateway (Go, Oteryn-Platform)
→ ticket redemption + World Registry route selection
→ short-lived pre-admission Game Session material bound to account/character/world/channel/revisions
→ Rust game server admission
→ character lease / final game-owned admission checks
→ game-domain canonical GameSessionId
→ protocol-oteryn gameplay
```

Channel switching repeats the safe admission boundary. A successful destination transition establishes the fresh logical Game Session required by the accepted multichannel architecture and therefore receives a fresh canonical `GameSessionId`; it is not an in-place protocol or adapter switch.

### 8. Current protocol truth

As of this decision:

- the canonical native gameplay contract and review IDL exist in `blakinio/Oteryn-Platform`;
- Platform/Gateway producer-side support exists and is disabled by default;
- `blakinio/otclient/oteryn-client` has no implemented `protocol-oteryn` crate;
- the new authoritative Rust server has not been implemented;
- no complete native login-and-gameplay E2E is proven.

Documentation must continue to distinguish contract/producer readiness from actual runtime compatibility.

### 9. Existing native contract requires reconciliation

Before implementing `protocol-oteryn` in Oteryn v2, the protocol contract programme must explicitly decide whether the existing Platform-owned native gameplay contract is:

- adopted as the canonical v1 contract;
- revised and re-pinned for the Rust server target;
- or explicitly superseded through coordinated ADRs and contracts.

A second silent native protocol must not be created.

The existing correspondence built around the C++ Otheryn producer and older profile terminology must be audited against the accepted Rust-server target and current single-native-version decision.

### 10. Game Session terminology refinement

The owner-accepted `FND-ID-01` decision now distinguishes two concepts that earlier revisions of this ADR described generically as “Game Session”:

1. **pre-admission Game Session material** — short-lived Platform/Gateway-produced authorization and routing material presented to the game server; and
2. **canonical logical gameplay session** — a game-domain lifecycle entity identified by `GameSessionId` after successful authoritative admission.

This refinement does not move reusable credential authority, Game Login Ticket authority, World Registry or Gateway routing out of Platform. It clarifies that Platform authorization to attempt admission is not proof that a canonical gameplay session already exists.

Where historical text or external Platform contracts still use the generic term `Game Session`, `FND-ID-01_GAME_SESSION_ID_OWNER_ISSUER_BASELINE.md` is authoritative for the semantic identity/issuer distinction and `FND-02`/`FND-04` must reconcile final names and schemas before implementation.

## Consequences

### Positive

- reusable credentials never enter the gameplay runtime;
- Platform policy can be enforced once at a clear authority boundary;
- game nodes can scale and fail independently from web Identity;
- Gateway can route among channels without becoming a world-state or canonical gameplay-session owner;
- the Rust server receives a narrow, short-lived admission capability;
- failed/pre-admission attempts do not become canonical logical gameplay sessions;
- no unnecessary Gateway rewrite blocks the first playable slice.

### Costs

- Platform availability is part of new-login admission;
- cross-repository contracts require coordinated revisions;
- pre-admission material, canonical Game Session, key rotation, replay prevention and lease failure semantics must be precisely defined;
- compatibility login paths must be removed or fenced before claiming one globally enforced production login policy.

## Rejected alternatives

### Put Identity/login inside the Rust game server

Rejected because it couples credential authority to gameplay failure domains and creates duplicate security policy.

### Move Game Gateway into Oteryn v2

Rejected because Gateway is part of Platform routing/session orchestration, not the authoritative gameplay runtime.

### Rewrite Gateway in Rust immediately

Rejected because the existing Go service is already isolated and tested, while the missing product capability is the native client/server runtime.

### Let the client select any server/channel directly

Rejected because route, capacity, revision and rollout policy belong to World Registry and Gateway.

### Let Platform issue the canonical GameSessionId

Rejected because Platform can authorize an admission attempt but cannot prove that final game-owned checks, character lease acquisition and authoritative gameplay admission succeeded. Canonical logical gameplay-session identity therefore belongs to the game-domain Game Session / Admission authority.

## Not performed by this ADR

- no Platform or Gateway code is changed;
- no legacy login path is disabled;
- no pre-admission session/admission token format is finalized;
- no canonical `GameSessionId` wire encoding is finalized;
- no lease implementation is chosen;
- no native protocol is activated;
- no runtime E2E claim is made.

## Required follow-up

1. Reconcile or supersede the existing native gameplay contract for the Rust server target, including the pre-admission-material versus canonical-`GameSessionId` distinction.
2. Accept the exact Identity, Game Session, admission and lease contract across repositories.
3. Implement the Platform client and pre-admission material consumer in the Oteryn v2 workspace.
4. Implement game-domain `GameSessionId` issuance only after `FND-04` freezes the admission/session state machine.
5. Implement `protocol-oteryn` client/server adapters and golden compatibility fixtures.
6. Prove exact-revision E2E before enabling native routing.
7. Remove or network-fence legacy authentication paths before claiming global Identity enforcement.
