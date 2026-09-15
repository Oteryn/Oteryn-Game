# ADR-0011: Native client pre-protocol migration state

- Status: Accepted foundation
- Date: 2026-08-05
- Decision owners: Oteryn project
- Applies to: `FND-01`, `VSL-02`, the atomic Rust-client destination migration, `FND-02`, `FND-03` and `FND-04`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`

## Context

ADR-0002 requires the existing Rust client to be classified under `FND-01`, cut over under `VSL-02` and imported through one atomic destination migration/workspace pull request before the native protocol, authoritative runtime and admission contracts are frozen.

The source client currently contains a production `protocol-canary` adapter. ADR-0008 fixes that subsystem as `REFERENCE_ONLY` and prohibits it from entering the Oteryn v2 production workspace, binaries, negotiation, fallback or translation paths.

At the same time, `protocol-oteryn` does not yet exist as an accepted implementation. Its exact framing, schemas, limits, identifiers, sequencing, compatibility and failure behavior belong to `FND-02` and related foundation gates. Creating an empty adapter, speculative codec or compatibility placeholder during migration would silently freeze unresolved public contracts and create false evidence that the migrated client can enter gameplay.

The migration therefore needs an explicit, safe destination state between:

```text
client source imported and governed by the canonical workspace
```

and:

```text
validated native gameplay connection available
```

Without a named transition state, implementers could retain Canary temporarily, invent a partial native adapter, hide a broken gameplay path behind retries or present a login flow that consumes credentials before failing.

## Decision

### 1. The atomic destination migration may end in `pre-native-protocol`

The accepted immediate post-migration state is named:

```text
pre-native-protocol
```

In this state the native Rust client:

- is part of the canonical `blakinio/Oteryn-v2` workspace;
- compiles, launches and shuts down deterministically on the accepted target matrix;
- retains only the migrated subsystems accepted by `FND-01` and `VSL-02`;
- may expose non-gameplay product surfaces, diagnostics, settings, asset validation and bounded development/test harnesses where separately accepted;
- does not claim that a production gameplay connection is available.

`pre-native-protocol` is a programme state and migration invariant. It is not a player-selectable protocol mode, compatibility profile, release channel or permanent product variant.

### 2. No production gameplay protocol adapter exists in this state

The production dependency graph and release artifacts in `pre-native-protocol` contain neither:

- `protocol-canary`; nor
- a placeholder, stub, incomplete or speculative production `protocol-oteryn` adapter.

The migration must not preserve Canary as a temporary fallback and must not rename, wrap or translate Canary behavior into a nominally native crate.

A future `protocol-oteryn` member may enter the canonical workspace only when its immediate consumer and accepted gate authorize real contract work. An empty layering crate created only to make the target tree look complete is forbidden.

### 3. Production gameplay entry is explicitly unavailable and fail-closed

Every production path that would begin gameplay must stop before any irreversible or security-sensitive boundary, including before:

- requesting or consuming a gameplay credential for a connection that cannot complete;
- binding a Game Session to an unavailable native adapter;
- opening a gameplay endpoint under an unsupported protocol assumption;
- advertising a world/channel as enterable by this client revision;
- reporting connection, authentication, admission or gameplay success.

The client must expose one deterministic unavailable result rather than:

- retrying until a hidden path succeeds;
- silently selecting Canary;
- connecting with guessed framing or schemas;
- consuming a one-shot ticket and failing later;
- presenting an endless loading state;
- reporting transport write completion as gameplay admission.

Exact error codes and UI wording remain owned by later contracts, but the state must be unambiguous to users, tests, telemetry and support tooling.

### 4. The transition is visible in build and runtime evidence

The migration acceptance evidence must prove all of the following on the exact destination head:

- the canonical workspace and client build without a production gameplay protocol adapter;
- `protocol-canary` is absent from workspace membership, production dependency edges, features and release packaging;
- no production `protocol-oteryn` stub is used to satisfy compilation;
- the client launches and reaches its accepted non-gameplay ready state;
- an attempted production gameplay entry returns the explicit unavailable result before credential consumption and gameplay endpoint connection;
- no UI, log, metric or test result claims successful gameplay capability;
- development-only fixtures cannot be enabled accidentally in production artifacts.

Synthetic and compile-time evidence is allowed where authorized, but it must be labeled as such and cannot be presented as native client/server compatibility.

### 5. `FND-02` ends the no-native-protocol design state, not all gameplay gates

`FND-02` is necessary to replace the no-native-protocol state with an accepted native protocol contract and implementation programme.

It is not sufficient by itself to authorize production gameplay. Production entry remains blocked until the applicable contracts and implementations are complete, including at minimum:

- `FND-ID-01` for the identifier vocabulary required by the wire and admission boundaries;
- `FND-02` for `protocol-oteryn`;
- `FND-03` for authoritative runtime execution and lifecycle behavior;
- `FND-04` for Identity/Game Session admission and character lease behavior;
- the exact validation and rollout evidence required by those gates.

A later gate may split implementation and rollout into additional bounded stages, but it cannot weaken the fail-closed behavior accepted here without a new owner-approved ADR.

### 6. Non-gameplay capabilities remain explicitly bounded

This ADR does not require the migrated client to provide a complete launcher, account portal, character selection, world browser, editor or offline game.

`FND-01` and `VSL-02` must classify which existing non-gameplay capabilities have immediate consumers and remain useful in the destination. Retained surfaces must not depend on Canary semantics or imply that the client can enter a world.

Development harnesses may exercise renderer, assets, input, diagnostics or protocol-neutral domain behavior only when:

- they are excluded from production defaults;
- their data is synthetic or legally approved;
- their purpose is named in tests or documentation;
- they cannot consume live gameplay credentials;
- they cannot be mistaken for end-to-end gameplay evidence.

## Consequences

### Positive

- the client can be migrated before the native wire contract is implemented without preserving a forbidden legacy adapter;
- the atomic destination PR remains buildable and reviewable;
- unresolved protocol decisions are not frozen by placeholders;
- one-shot credentials and admission boundaries are protected from knowingly incomplete clients;
- users and operators receive an explicit unavailable state instead of misleading connection behavior;
- `FND-02`, `FND-03` and `FND-04` retain clear ownership of their contracts;
- migration evidence can distinguish a healthy canonical client shell from a gameplay-capable release.

### Costs

- the migrated client may temporarily launch without allowing players to enter gameplay;
- some existing source-client integration tests will need to be reclassified, replaced with synthetic fixtures or deferred;
- application composition must tolerate the intentional absence of a gameplay adapter;
- product and support messaging must distinguish migration readiness from gameplay readiness;
- release automation must prevent accidental public distribution as a gameplay-capable build unless later gates pass.

## Rejected alternatives

### Keep `protocol-canary` until `protocol-oteryn` is ready

Rejected because it violates ADR-0008, preserves a second production protocol path and encourages domain/runtime decisions around legacy wire behavior.

### Create an empty `protocol-oteryn` crate during migration

Rejected because an empty crate has no immediate contract-valid consumer, hides the intentional capability gap and invites speculative schemas or fake success paths.

### Copy the current source workspace unchanged and clean it up later

Rejected because the atomic destination migration must apply `FND-01` dispositions and accepted dependency boundaries on the same destination head. An intermediate canonical `main` with Canary or an unmanaged graph is forbidden.

### Allow gameplay attempts to fail naturally after connection

Rejected because late failure may consume one-shot credentials, create ambiguous telemetry, expose unsupported parsing paths and mislead users about admission state.

### Block the entire client process from launching

Rejected as a universal requirement. A launchable, protocol-free client shell provides useful migration, renderer, asset, settings, diagnostics and packaging evidence. Individual retained surfaces still require an accepted `FND-01` disposition and immediate consumer.

## Relationship to existing decisions

- ADR-0001 remains authoritative for the native Rust client/server and single `protocol-oteryn` direction.
- ADR-0002 remains authoritative for repository ownership, migration order and the one atomic destination PR.
- ADR-0008 remains authoritative for the `REFERENCE_ONLY` disposition of `protocol-canary`.
- This ADR defines the safe transition state created when ADR-0002 and ADR-0008 are applied before `FND-02` implementation.
- This ADR does not authorize runtime, protocol, admission or external-repository implementation.

## Acceptance impact

`FND-01` must classify the existing source workspace so the destination application can compose and launch without a production protocol adapter.

`VSL-02` must include this state in its path mapping, migration acceptance, rollback and exact-head validation.

The atomic destination migration is incomplete if it:

- contains a production Canary dependency;
- contains a speculative native protocol stub;
- cannot launch because protocol code is absent;
- permits gameplay credential consumption or endpoint connection despite the unavailable state;
- presents itself as gameplay-capable.

Any future change that reintroduces a temporary production protocol, silent fallback or late-failure gameplay path requires a new owner-approved ADR.
