# FND-02 Platform Protocol Reconciliation Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: input to `FND-02`; does not start or complete `FND-02`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- External evidence: `blakinio/Oteryn-Platform@c0b8703d326a04b43ae8e06f6192b0cb91c859b7`, `docs/contracts/OTERYN_NATIVE_GAMEPLAY_PROTOCOL_CONTRACT.md`, contract revision `2`, schema revision `2`, schema SHA-256 `9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9`
- Applies to: interpretation of the existing Platform native gameplay contract by `blakinio/Oteryn-v2`, future `FND-02` protocol work, and later coordinated Platform reconciliation

## Purpose

Record the project owner's accepted resolution of the conflict between the existing merged Oteryn Platform native gameplay protocol contract and the newer accepted Oteryn v2 architecture.

The existing Platform contract is retained as **bounded reconciliation input and historical cross-repository evidence**. It is **not** accepted wholesale as the final `protocol-oteryn` contract for Oteryn v2 and it does not complete or pre-empt `FND-02`.

This baseline preserves useful Platform-side authentication, routing and fail-closed security concepts while preventing stale Canary compatibility, C++ Otheryn runtime assumptions and prematurely frozen wire details from becoming implicit authority over the native Rust stack.

This document does not mutate or supersede any source in `blakinio/Oteryn-Platform`. Platform-side canonical correction requires its own explicitly authorized repository task, branch, PR and accepted authority.

## Owner-accepted decision

The owner accepts the following interpretation:

```text
existing Platform native gameplay contract
    = merged external evidence + reconciliation input
    != final Oteryn-v2 protocol-oteryn contract
    != permission to implement its conflicting runtime assumptions

FND-ID-01
    freezes identifier vocabulary required across the boundary

FND-02
    later freezes the canonical native protocol contract
    after consuming accepted Oteryn-v2 architecture and reconciled Platform requirements

FND-03 / FND-04
    later freeze runtime and admission/session/lease behavior in their own scopes
```

No field, transport, schema, capability, listener, protocol family, session claim or runtime role from the external Platform document becomes final Oteryn-v2 authority merely because that external revision is merged and immutable.

## Why reconciliation is required

### Accepted Oteryn-v2 target

The accepted Oteryn-v2 architecture requires:

- one native Rust client;
- one authoritative Rust game server;
- one production gameplay protocol family, `protocol-oteryn`;
- `protocol-canary` and Tibia/Canary wire compatibility excluded from the production runtime;
- Otheryn C++ retained only as behavioral/content migration evidence;
- Platform Identity, Game Login Ticket, Game Gateway and World Registry retained as the external authentication/routing control plane;
- identifier meanings frozen before the dependent protocol schema;
- the protocol contract frozen before authoritative runtime and production admission implementation.

### Conflicting external assumptions

The existing Platform contract was produced under an earlier architecture state and contains assumptions including:

- `canary` and `oteryn` as gameplay protocol families offered/selected by Platform policy;
- the Rust client implementing independent `protocol-canary` and `protocol-oteryn` adapters;
- Otheryn C++ as authoritative character-admission/gameplay runtime;
- native Oteryn implemented by a separate TLS/ASIO listener in Otheryn;
- a frozen native transport identifier `tcp.tls13.protobuf.be32.v1`;
- native protocol version `1` already fixed;
- protobuf schema revision `2` and a specific canonical schema digest already fixed;
- an exact native capability set and digest already fixed;
- concrete command, sequencing and state-revision vocabulary already placed in the normative protocol contract;
- concrete Game Session v2 fields and protocol binding written before Oteryn-v2 completes its own identifier and admission gates.

Those statements are not silently imported into Oteryn-v2.

## Reconciliation classification

### A. Preserve as required architectural inputs

The following concepts remain valid requirements or strong inputs where they are consistent with accepted Oteryn-v2 ADRs and are later re-expressed by the owning gate:

1. **Single Identity authority**
   - Platform Identity authenticates the user.
   - Passwords and reusable OAuth credentials are not passed to the game server.
   - Oteryn-v2 must not create a second login authority.

2. **One-time Game Login Ticket boundary**
   - the ticket is short-lived, one-time and audience-bound;
   - ticket replay, expiry, generation mismatch and ambiguous consumption fail closed;
   - Gateway consumes the ticket before creating the gameplay session boundary.

3. **Game Gateway and World Registry control-plane role**
   - Platform controls world/channel discovery and route policy;
   - World Registry remains the accepted topology/route authority unless later superseded;
   - Gateway coordinates admission/routing but does not become gameplay simulation authority.

4. **Exact session/route binding principle**
   - an admitted gameplay session must be bound to the intended world/channel/endpoint and accepted revisions;
   - stale, contradictory or ambiguous routing/readiness must fail closed;
   - exact field ownership and token semantics remain `FND-04` work.

5. **No silent downgrade after security-sensitive progression**
   - once a gameplay path/session/credential is selected or issued, failures must not silently reinterpret the same authority as permission to use a weaker or unrelated protocol path;
   - because Oteryn-v2 has only one production gameplay protocol family, this principle is retained without production Canary fallback.

6. **Separate version dimensions**
   - Platform API/session contract versions, gameplay protocol version, schema/content/ruleset revisions and policy revisions are different concepts and must not be overloaded;
   - exact field names, widths and compatibility policy remain owned by their later contracts.

7. **Bounded, deterministic compatibility checks**
   - malformed, duplicate, contradictory or unsupported compatibility declarations fail closed;
   - exact capability/revision mechanics remain `FND-02` work.

8. **Security and observability discipline**
   - secrets remain opaque and minimally exposed;
   - protocol/admission failure categories must be diagnosable without leaking credentials or hidden topology;
   - replay, expiry, downgrade and cross-world/channel misuse require explicit negative testing in the later implementation programme.

### B. Retain only as research/design evidence

The following are useful examples but are not yet accepted final design:

- the idea of a native protocol version number;
- capability negotiation as a concept;
- command/result correlation;
- server sequencing;
- state revisions;
- snapshot/delta/reconciliation concepts;
- exact route readiness matching;
- schema digest pinning;
- explicit transport identity;
- immutable selection after session issuance.

`FND-02` may adopt, modify or reject their exact shape after evaluating the complete Oteryn-v2 requirements.

### C. Explicitly not accepted as final Oteryn-v2 authority

The following external-contract assumptions are rejected as final target authority unless a later owner-approved ADR explicitly changes the current architecture:

- production `canary` as an offered gameplay protocol family;
- a production Rust `protocol-canary` adapter;
- dual-protocol `Auto` selection;
- Canary fallback or downgrade;
- Otheryn C++ as the target authoritative gameplay server;
- an Otheryn TLS/ASIO native gameplay listener;
- protocol translation between Canary and Oteryn;
- Canary opcode/framing/crypto/serializer compatibility requirements;
- the exact external transport string `tcp.tls13.protobuf.be32.v1` as already-final Oteryn-v2 transport;
- protobuf as already-final Oteryn-v2 IDL solely because the external contract uses it;
- external schema revision `2` and its digest as already-final Oteryn-v2 schema;
- the external fixed capability catalogue/digest as already-final Oteryn-v2 capability contract;
- external `command_id = UUIDv4` as already-final Oteryn-v2 command identity policy;
- any exact Game Session v2 claim set that bypasses the later `FND-ID-01` and `FND-04` ownership decisions.

## Gate ordering

This decision does not reorder the foundation programme.

The accepted sequence remains:

```text
FND-ID-01
    -> FND-02
    -> FND-03
    -> FND-04
```

### FND-ID-01

Must first complete the cross-boundary identifier catalogue and freeze the required owner/issuer/scope/lifecycle/visibility/comparison/representation constraints.

The Platform contract may be consulted for names and boundary requirements, but it cannot override later owner-accepted Oteryn-v2 identifier baselines.

### FND-02

Must then create the canonical `protocol-oteryn` contract from the accepted Oteryn-v2 architecture plus reconciled Platform requirements.

`FND-02` owns at least the protocol-level decisions for:

- transport and connection model;
- framing;
- IDL/serialization;
- wire representation of accepted identifiers;
- protocol/schema/capability versioning;
- message envelope and resource limits;
- commands/results;
- sequencing and revision semantics;
- snapshots, deltas and reconciliation;
- reconnect/protocol continuity behavior within the boundary assigned to it;
- downgrade prevention;
- golden fixtures and compatibility validation.

It must not silently treat the old Platform tuple as a pre-approved answer.

### FND-03 and FND-04

`FND-03` remains the authority for runtime execution semantics.

`FND-04` remains the authority for Identity-to-game admission, Game Session, one-time gameplay credentials, character binding/leases, fencing, revocation, reconnect and related failure semantics.

The old Platform contract can provide requirements to those gates but cannot collapse them into `FND-02`.

## Cross-repository contract-lock interpretation

The existing `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` entry for coordination ID `OTS-20260804-native-protocol-selection` records a real immutable merged Platform revision. That historical fact remains true and must not be erased.

For Oteryn-v2 interpretation:

- `status = LOCKED` means the referenced external revision is an immutable merged contract revision;
- it does **not** mean Oteryn-v2 has accepted that revision as `FND-02`;
- `accepted_for_fnd02 = false` remains mandatory;
- the entry must explicitly identify the revision as `RECONCILIATION_INPUT_ONLY` for future FND-02 work;
- no Oteryn-v2 producer or consumer implementation may claim conformance to that external revision as the final native protocol contract;
- a later accepted FND-02 contract must update/supersede the cross-repository lock through its own coordinated programme.

## Platform-side follow-up requirement

A complete cross-repository resolution eventually requires a separate explicitly authorized task in `blakinio/Oteryn-Platform`.

That Platform-side work must:

- apply Platform's own architecture-authority and ADR rules;
- mark or supersede conflicting runtime/protocol assumptions without erasing history;
- retain valid Identity/Gateway/World Registry security and ownership invariants;
- coordinate against the accepted Oteryn-v2 `FND-ID-01` and later `FND-02` contract;
- not activate production gameplay merely because documentation is reconciled.

Until then, Oteryn-v2 records the conflict and fails closed against treating the external document as its final native protocol contract.

## Consequences

### Positive

- prevents an earlier cross-repository document from silently overriding newer accepted Oteryn-v2 ADRs;
- preserves useful security and Platform boundary work instead of discarding it;
- keeps `FND-ID-01` and `FND-02` meaningful rather than rubber-stamping a pre-existing schema;
- keeps the Rust server/client architecture independent from Otheryn/Canary runtime compatibility;
- makes later cross-repository migration explicit and auditable;
- prevents accidental implementation against the wrong protocol contract.

### Costs

- the Platform contract requires a later coordinated correction;
- native protocol implementation remains intentionally blocked until the ordered contracts are complete;
- some already-written Platform protocol/schema work may become historical or require migration;
- cross-repository E2E cannot claim final native protocol conformance until both sides implement the later accepted contract.

## Rejected alternatives

### Accept the existing Platform contract wholesale as FND-02

Rejected because it would silently reintroduce Canary compatibility, Otheryn runtime assumptions and wire choices frozen before the required Oteryn-v2 gates.

### Discard the Platform contract entirely

Rejected because it contains useful Identity, ticket, routing, fail-closed and downgrade-prevention requirements that should be reconciled rather than rediscovered.

### Modify Platform from this Oteryn-v2 task

Rejected because cross-repository writes require explicit authorization and separate task/branch/PR ownership in the external repository.

### Begin FND-02 immediately

Rejected because `FND-ID-01` remains the next ordered gate and must freeze identifier semantics required by the wire boundary first.

## Non-authorization

This baseline does not authorize:

- writes to `blakinio/Oteryn-Platform`;
- completion or implementation of `FND-ID-01`;
- start, completion or implementation of `FND-02`;
- protocol codec, listener, transport or schema implementation;
- authoritative game runtime work;
- Game Session/admission/lease implementation;
- persistence/database changes;
- Canary production compatibility;
- deployment, rollout or production activation.
