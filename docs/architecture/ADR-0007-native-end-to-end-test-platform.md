# ADR-0007: Native End-to-End Test Platform

- Status: Accepted
- Date: 2026-08-05
- Decision gate: `QA-E2E-01`
- Coordination ID: `OTV2-QA-E2E-01`
- Supersedes: none
- Related decisions: ADR-0001, ADR-0002, ADR-0003, ADR-0004

## Context

Oteryn v2 is a native Rust gameplay stack whose supported product path crosses multiple independently owned boundaries:

```text
native client
→ Platform Identity and world discovery
→ Game Gateway
→ bounded Game Session
→ protocol-oteryn
→ authoritative Rust game server
→ PostgreSQL
→ transactional outbox and audit evidence
```

A green client build, server build, protocol unit test or database integration test does not prove that this path works as one supported workflow. The project therefore requires end-to-end evidence that includes the real client and server, Platform-owned admission boundaries, persistence, failure behavior and cleanup.

The historical Canary programme demonstrates a useful shared E2E pattern. At `blakinio/canary@f69632104888ece1d9afb801a90a66244694a627`, one platform owns disposable database setup, exact server and client revisions, physical client lifecycle, scenario resolution, evidence collection, timeouts and cleanup. Feature programmes provide declarative scenarios, fixtures and assertions rather than copying complete workflows. The retained suites include login, movement, combat, multiclient, NPC, Platform and recovery scenarios.

Canary also demonstrates the cost and fragility of physical-client-only testing. A retained ten-attempt `login/relog` population contained nine clean passes and one infrastructure/client-configuration failure and was correctly classified as unstable. This is useful evidence, but it shows that a graphical client must not be the sole mechanism for broad protocol, concurrency and failure-injection coverage.

## Decision

Oteryn v2 adopts one shared native E2E platform with three execution tiers.

### Tier 1 — headless system E2E

The primary broad and deterministic tier uses a headless test client that speaks the production transport and `protocol-oteryn` through the supported Platform and Game Gateway path.

The headless client:

- uses the same protocol schemas, codecs, sequencing and admission contracts as the native client;
- sends player intent through the normal product boundary;
- does not link directly to authoritative game-domain mutation APIs;
- does not bypass Game Session validation, character lease acquisition, command sequencing or server legality checks;
- exposes deterministic scenario control and semantic observations suitable for concurrency and failure injection;
- may run many clients and channels in one isolated test environment.

This tier is the main PR-blocking mechanism for:

- login, logout, relog, reconnect and resume;
- one-character lease, double-login and stale-session rejection;
- command duplication, reordering, delay and lost-baseline recovery;
- multiclient and multichannel behavior;
- crash, restart and dependency-loss recovery;
- PostgreSQL transaction rollback and retry behavior;
- item and currency conservation;
- outbox, audit and analytics evidence consistency;
- channel failure, isolation and shared-world service behavior;
- abuse and negative protocol cases.

Tier 1 is real system E2E, not a unit-test substitute, because it crosses the deployed process, transport, admission and persistence boundaries. It does not prove rendering, UI interaction or final client packaging.

### Tier 2 — instrumented native-client E2E

The second tier runs the real native Rust client against the same supported Platform, Gateway, server and PostgreSQL stack.

A bounded E2E adapter may be compiled into an explicitly test-only client profile. The adapter may:

- invoke normal input and client-command paths;
- wait for semantic client states and events;
- expose read-only client observations;
- capture screenshots, frame markers, UI state and client logs;
- report deterministic scenario milestones.

The adapter must not:

- create or forge Platform or Game Sessions;
- teleport a character or mutate server-owned state;
- inject authoritative snapshots or results;
- bypass normal networking, protocol validation or server legality checks;
- exist in production-default builds;
- turn client assertions into evidence of server persistence without independent server-side proof.

Tier 2 proves the actual client networking, input, state reconciliation, UI and rendering path. It is required for user-observable journeys but is intentionally smaller than Tier 1.

### Tier 3 — production-binary smoke E2E

The third tier runs the exact release-candidate client and server artifacts without the test-only in-process client adapter.

It proves at minimum:

1. process startup;
2. Platform authentication and world discovery;
3. Game Gateway and Game Session admission;
4. world entry;
5. movement;
6. one authoritative gameplay action;
7. clean logout;
8. relog;
9. final persisted and offline state;
10. cleanup of all disposable resources.

External OS-level automation or a minimal supported automation surface may be used, but the product artifacts, enabled features, assets and packaging must match the named release candidate.

Tier 3 protects against differences between instrumented and production builds. It is a release gate, not the primary broad regression suite.

## Shared scenario platform

The three tiers use one versioned scenario contract and one shared orchestration platform. Feature programmes own scenario definitions, deterministic fixtures and domain-specific assertions. The E2E platform owns lifecycle and evidence mechanics.

The target logical structure is:

```text
tests/e2e/
├── scenarios/
├── fixtures/
├── worlds/
├── assertions/
├── baselines/
└── schemas/
```

This structure is a contract horizon and does not authorize placeholder directories or crates before an implementation task has an immediate consumer.

A scenario must declare:

- schema version and stable scenario identity;
- required execution tier;
- exact client, server, Platform contract, protocol, content, ruleset and migration revisions;
- topology: worlds, channels, instances and client count;
- deterministic fixtures and generated identities;
- required capabilities and resource limits;
- timeouts and virtual-time policy;
- actions and semantic observations;
- domain, persistence, audit and cleanup assertions;
- required retained artifacts;
- expected failure class where the scenario is negative.

Credentials, production hosts, personal data, proprietary assets and mutable branch references are forbidden in retained scenario manifests.

## Environment topology and ownership

The E2E platform provisions only isolated and disposable resources.

The complete supported topology includes, where required by the scenario:

- a test Identity tenant or repository-owned test Identity fixture;
- Game Gateway and World Registry behavior from an exact named Platform revision or immutable artifact;
- one or more authoritative game-server nodes;
- one or more channels in one logical world;
- PostgreSQL databases with separate Platform and game owners and migrations;
- non-authoritative Redis only when required by the tested configuration;
- exact World Bundle, content and ruleset revisions;
- headless or native clients;
- controlled network and dependency fault injection;
- audit, outbox and observability collectors.

The Platform remains the authority for Identity, reusable credentials, world directory, Game Login Ticket and Game Gateway behavior. The game server remains the authority for Game Session admission, character lease, gameplay legality and durable game mutations. The E2E platform must not introduce a parallel authentication or gameplay authority.

## Determinism and failure injection

Scenarios must explicitly control or record:

- random seeds;
- wall-clock and monotonic-clock behavior;
- virtual time where supported;
- content, ruleset and protocol revisions;
- node, channel and session identities;
- database migrations and fixture hashes;
- network delay, loss, duplication, reordering and disconnect rules;
- process termination and restart points;
- dependency failure and recovery points;
- scenario ordering and concurrency schedule where deterministic scheduling is required.

Failure injection must occur at supported boundaries. It may kill or pause disposable processes, manipulate the isolated test network, revoke test sessions, interrupt database connections or inject supported test clocks. It must not use production systems, credentials or data.

## Assertions and stable probes

Ordinary gameplay scenarios should assert stable domain outcomes rather than internal table layouts.

Preferred evidence:

- semantic client events;
- server-owned read-only test probes;
- versioned domain event and audit records;
- public or test-scoped diagnostics;
- invariant reconciliation results;
- supported persisted-state projections.

Direct SQL is permitted when the scenario owns persistence, migrations or database recovery, or when no stable projection exists and the assertion is explicitly classified as implementation-coupled. SQL assertions in ordinary gameplay manifests are discouraged because they create unnecessary coupling to schema details.

No test-only probe may mutate authoritative runtime state. Mutation occurs only through normal product commands, accepted administrative test setup before the scenario, or bounded fixture loading before authoritative services start.

## Evidence envelope

Every attempt produces one machine-readable result envelope and retains the evidence required to verify it.

The envelope records at least:

- scenario and execution-tier identity;
- exact repository commits and artifact hashes;
- Platform contract and service revision;
- `ProtocolRevision`, `RulesetRevision`, `ContentRevision` and World Bundle hash;
- migration revision and database image identity;
- operating system, target triple and relevant build features;
- random seed, clock mode and fault profile;
- ordered phase results;
- first divergence;
- failure class;
- cleanup result;
- artifact names and hashes;
- start time, duration and attempt identity.

Canonical phases are:

```text
environment
identity
world-discovery
gateway
game-session
transport
admission
character-lease
world-entry
gameplay
persistence
audit-outbox
client-presentation
cleanup
```

A phase may be `NOT_APPLICABLE` only with a scenario-defined reason. A successful environment startup is not a successful gameplay scenario.

## Cleanup contract

Cleanup is part of the result, not a best-effort postscript.

The platform must attempt and report cleanup on:

- success;
- scenario failure;
- assertion failure;
- timeout;
- cancellation;
- client or server crash;
- setup failure after resource creation.

Cleanup evidence covers disposable processes, containers, networks, databases, files, test accounts/sessions, leases and temporary credentials. Unknown cleanup prevents a clean pass.

## Flakiness and repeated-run evidence

Automatic hidden retries are forbidden.

Each physical attempt remains visible with its own result and artifacts. A replacement retry must never erase or convert a failed attempt into a pass.

Repeated-run certification uses an explicit fixed population and exact comparison cell. The cell includes at least:

- scenario;
- execution tier;
- client artifact;
- server artifact;
- Platform revision;
- protocol revision;
- content/ruleset/World Bundle revisions;
- operating system and target triple;
- fault profile.

Population classifications are:

- `PASS` — every counted attempt completed the required journey and cleanup;
- `UNSTABLE` — the population contains mixed outcomes;
- `FAIL` — every usable attempt fails the product acceptance or a deterministic failure is reproduced;
- `BLOCKED` — evidence is incomplete, inconsistent, tampered, cleanup-unknown or cannot be compared;
- `NOT_EVALUATED` — the declared minimum population was not reached.

Infrastructure failures remain failures of the executed attempt and must be classified separately from product divergence. They may justify a repair and a new population, but not rewriting the historical population.

## CI and release placement

The accepted default placement is:

- pull requests: focused Tier 1 scenarios selected by changed paths and risk;
- protected main or merge queue: mandatory Tier 1 foundation journeys and bounded Tier 2 journeys for affected client-facing paths;
- scheduled/nightly: wider Tier 1 concurrency, recovery, fault and soak campaigns plus repeated Tier 2 populations;
- release candidate: required Tier 1 suite, named Tier 2 journeys and Tier 3 production-binary smoke on supported release targets.

High-risk changes may require broader evidence regardless of path selection, including protocol, admission, character lease, inventory, item/currency, market, houses, multichannel runtime, persistence, migrations, updater and client integrity changes.

Historical or parent-commit evidence does not replace exact-head evidence when the changed head can affect the tested path.

## Gate and dependency rules

`QA-E2E-01` is accepted by this ADR.

Implementation may proceed incrementally, but:

- the canonical client must first be present in Oteryn v2 through the accepted `VSL-02` migration/cutover;
- protocol-driving components must not freeze unresolved wire behavior before `FND-02`;
- admission and lease scenarios must follow `FND-04`;
- durable mutation and anti-duplication scenarios must follow `DUR-01` through `DUR-03` and `ANL-01` as applicable;
- broad world/content scenarios follow `DUR-04`;
- a partial runner or synthetic fixture path cannot be represented as complete E2E.

`QA-E2E-01` blocks completion of `VSL-01`. It does not block `FND-01`, `VSL-02` or continued architecture work.

## Consequences

### Positive

- The real client remains an obligatory product proof.
- Broad protocol, concurrency and recovery tests remain fast enough to run frequently.
- Features add scenarios rather than duplicating infrastructure.
- Failures identify the first divergent subsystem and retain comparable evidence.
- Exact revisions and hashes make results reproducible and auditable.
- Flaky and infrastructure failures remain visible instead of being hidden by retries.
- Multichannel, lease, persistence and anti-duplication invariants become first-class E2E concerns.

### Costs

- The project must maintain both a headless production-protocol client and native-client automation.
- Test-only observation surfaces require strict capability and build-profile controls.
- Cross-repository Platform revision pinning adds coordination cost until or unless that boundary changes.
- Physical client and release-binary jobs require more expensive runners and careful artifact retention.
- Deterministic clocks, fault injection and semantic probes require deliberate design in foundation contracts.

## Rejected alternatives

### Only graphical/native-client E2E

Rejected because it is too slow and fragile for broad concurrency, protocol fault and recovery coverage.

### Only headless protocol E2E

Rejected because it cannot prove the supported native-client UI, renderer, input, reconciliation or packaging path.

### Feature-owned workflows

Rejected because lifecycle, security, cleanup and evidence contracts would drift and be copied repeatedly.

### Direct domain API test driver

Rejected as the primary E2E mechanism because it bypasses the supported Platform, transport, protocol and server-authority boundaries.

### Automatic retry until green

Rejected because it hides instability and destroys factual attempt history.

## Non-goals

This ADR does not select the final Rust testing libraries, container orchestration implementation, UI automation library, renderer backend, CI runner vendor, artifact retention duration or exact directory/crate layout. Those choices belong to a bounded implementation package after the relevant workspace and product contracts exist.

This ADR does not authorize production deployment, live account/session mutation, production credentials, proprietary assets or writes to external repositories.

## Acceptance evidence for future implementation

The `QA-E2E-01` implementation is complete only when named exact revisions prove:

- one Tier 1 Platform-to-server login/relog journey;
- one Tier 2 real native-client login/relog journey;
- one Tier 3 production-binary smoke journey;
- one multiclient or multichannel scenario;
- one character lease/double-login scenario;
- one crash/recovery scenario;
- one persistence rollback or dependency-loss scenario;
- one item/currency no-duplication scenario when durable item mutation exists;
- retained result and cleanup envelopes;
- deterministic scenario validation;
- a repeated-run population with no hidden retry;
- exact-head CI and independent audit.

Until those conditions are met, the implementation state must be `PARTIAL`, `SYNTHETIC_ONLY`, `UNKNOWN` or `BLOCKED`, not `PROVEN`.
