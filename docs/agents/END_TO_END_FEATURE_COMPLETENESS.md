# End-to-end feature completeness

A feature is complete only when its required producer, authoritative domain behavior, transport/API, consumer/UI, persistence and failure behavior form one observable supported workflow.

Canonical native E2E architecture: `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` (`QA-E2E-01`).

## Required feature contract

Record:

- user outcome and supported profiles/capabilities;
- authoritative owner for every state transition;
- inputs, outputs, limits and errors;
- persistence and recovery semantics;
- client/server/Platform revision matrix;
- positive, negative and abuse scenarios;
- required E2E execution tier or tiers;
- telemetry/privacy behavior;
- rollout and rollback.

## Oteryn v2 mandatory concerns

- world/channel/instance scope;
- one-character lease and stale-writer fencing;
- server authority over legality, inventory, loot, damage and resources;
- sequencing, duplicate and lost-baseline behavior;
- channel failure/isolation and shared-world service behavior;
- anti-channel-hopping rewards/PvP/trade rules;
- ruleset capability exposure without protocol forks;
- exact Platform, Gateway, Game Session, protocol, content, ruleset and migration revisions;
- cleanup on success, failure, cancellation and timeout.

## Evidence tiers

- **Tier 1 — headless system E2E:** proves the deployed Platform/Gateway/transport/server/persistence path with a production-protocol test client. It is the preferred broad mechanism for concurrency, negative cases and failure injection. It does not prove native-client UI, rendering or packaging.
- **Tier 2 — instrumented native-client E2E:** proves the real Rust client networking, input, reconciliation, UI and rendering path through a bounded test-only observation adapter. It does not prove the exact production-default binary.
- **Tier 3 — production-binary smoke E2E:** proves the exact release-candidate artifacts and packaging without the in-process test adapter. It is intentionally small and does not replace broad Tier 1 coverage.

A feature must name the tiers required by its supported outcome. A user-visible native-client journey cannot be `PROVEN` from Tier 1 alone. A release claim cannot be based only on an instrumented build when Tier 3 is required.

## Valid E2E attempt

A valid attempt records:

- exact scenario and tier;
- exact client, server and Platform revisions or artifact hashes;
- protocol, ruleset, content, World Bundle and migration revisions;
- deterministic seed, clock mode, topology and fault profile;
- phase outcomes and first divergence;
- required client, server, persistence and audit observations;
- cleanup outcome and retained artifact hashes.

A successful environment startup, process health check or login-port connection is not a successful gameplay scenario.

Direct SQL is acceptable for persistence/migration ownership or an explicitly implementation-coupled assertion. Ordinary gameplay scenarios should prefer stable read-only projections, domain events, audit evidence and invariant reconciliation.

## Flakiness and retries

Hidden retry-until-green is forbidden. Every counted physical attempt remains visible.

Repeated-run populations are classified as:

- `PASS` — every counted attempt completes the required journey and cleanup;
- `UNSTABLE` — mixed outcomes;
- `FAIL` — deterministic product failure or all usable attempts fail acceptance;
- `BLOCKED` — evidence is incomplete, inconsistent, tampered or cleanup-unknown;
- `NOT_EVALUATED` — the declared minimum population was not reached.

A runner or infrastructure repair starts a new population; it does not rewrite historical evidence.

## Completeness states

- `PROVEN` — every required exact scenario and tier passed on named revisions with complete evidence and cleanup.
- `PARTIAL` — bounded implementation exists but a required layer, tier or evidence item is missing.
- `SYNTHETIC_ONLY` — only fake, in-process or fixture-only behavior is proven.
- `UNKNOWN` — evidence absent or stale.
- `BLOCKED` — a named dependency or environment prevents the required evidence.
- `DEFERRED` — owner-approved exclusion.
- `ABSENT` — no owning implementation contract.

`VSL-01` cannot be complete until `QA-E2E-01` is implemented and the minimum evidence named in ADR-0007 exists. A green build, unit test, synthetic integration test or environment-only smoke result is not end-to-end proof.
