# Oteryn GameNode Boot Composition Decision (first Reference slice)

- Status: `PROPOSED — OWNER ACCEPTANCE REQUIRED`; it becomes `OWNER_ACCEPTED ARCHITECTURE DECISION` only after explicit owner acceptance and a merge to protected `main`
- Date: 2026-09-24
- Repository: `Oteryn/Oteryn-Game`
- Decision ID: `OPS-NODE-BOOT-01`. This is the first bounded slice under the `OPS-CHANNEL-01` gate (ADR-0009 §13). It does not satisfy `OPS-CHANNEL-01`.
- Consumes:
  - ADR-0009 (one GameNode = one process);
  - ADR-0015;
  - `GAME-NODE-REGISTRATION-BOOTSTRAP-AUTH-V1` (2026-09-22);
  - the native admission source evidence decision (2026-09-06);
  - the Character authenticated bootstrap intent decision (2026-09-23);
  - FND-04 pre-admission grant profile §11;
  - Server Seam #247 / PR #823.
- Implementation authority granted by this decision: `NONE`
- Production / live-deployment authority: `NONE`

## 1. Decision timing

**Must decide now? `YES`.**

- **Blocked work.** Protected `main` has every owner the fresh-admission path needs:
  - S2 #757;
  - #414;
  - #415;
  - S3-B #815;
  - `serve_gameplay` #823.

  They are composed only inside test harnesses. `apps/game-server/src/main.rs` still exits with `GAMEPLAY_UNAVAILABLE_REASON`, so no startable GameNode exists and no real client can reach the proven seam. Starting a node needs decisions on four points:
  - which inputs a node may take;
  - which actions belong to an operator process rather than the node;
  - how S2 evidence stays inside the five-second source-age bound;
  - where readiness revisions come from.

- **Harder later if over-designed now.** Choosing an orchestrator, health endpoints, dynamic channel placement, secret-delivery products or content-derived revisions now would pull `OPS-CHANNEL-01` and Content activation into the first startable slice.

- **Evidence that may supersede this decision:**
  - an accepted `OPS-CHANNEL-01`;
  - an accepted Content activation authority lane that yields measured revisions;
  - a Platform route/offer export contract;
  - operational evidence that file-based configuration is unsafe.

## 2. Scope of the slice

- Exactly one GameNode process serves exactly one statically configured `(WorldId, ChannelId)` scope.
- Only fresh admission is served, with the #823 behaviour. Resume stays out until #822. Post-admission input remains fail-closed.
- Lifecycle is manual or supervisor-driven. There is no autoscaling, dynamic channel creation, live migration or health API.

## 3. Decisions

### D1 — One explicit configuration file; secrets only by file reference

- **Invocation.** The node starts as `oteryn-game-server serve --config <path>`. With no subcommand, the binary keeps its current fail-closed behaviour, and `--smoke` is unchanged.
- **Parsing.** The configuration is one TOML document with a closed schema:
  - unknown or duplicate keys reject;
  - every value is required;
  - no production defaults exist;
  - limits above the registered maxima (256 connections, 64 handshake units) reject.
- **Contents:**
  - the listener address;
  - the entry deadline and limits;
  - `world_id` and `channel_id`;
  - the PostgreSQL connection reference;
  - the Platform source endpoint and expected peer identity;
  - the Character recovery-fence directory;
  - the readiness revisions (D5).
- **Secrets are never inline values.** The document gives only file paths, each read once at start:
  - TLS certificate chain and key;
  - Platform mTLS client certificate and key;
  - the launch-scoped bootstrap authorization;
  - the PostgreSQL URL.

  Any missing, unreadable, malformed or over-bound input fails before a socket is bound. It exits with a distinct code, and the error names the key but never its value.

Rejected alternatives:
- **Environment variables for everything.** Secret-bearing environment dumps are a common leak path, and file permissions give clearer ownership and rotation.
- **Built-in defaults.** They would silently create a production shape nobody accepted.

### D2 — Operator actions run in a separate operator binary, never in the serving node

- **The binary.** A second binary target in the same crate, `oteryn-game-ops`, performs these control-plane actions:
  - configuring the Character interpretation revision;
  - authorizing the fresh Character recovery store (generation 1);
  - Channel assignment, replace and revoke through `RuntimeScopeAssignmentWriter`, with an explicit control actor;
  - consuming one Platform-authenticated Character bootstrap intent by operation id (the #414 operator variant).
- **Its own process incarnation.** Actions that need a process proof (`NodeIncarnationProof`) register a fresh incarnation for that one invocation, using their own consumed launch authorization. Registration proves process identity only and grants no Channel authority (registration decision §3), so an operator invocation never becomes a serving node.
- **The serving node** never assigns its own scope, authorizes a fresh Character store or consumes bootstrap intents. This keeps the S3-B/#414 trust boundary: operator authority stays out of the server process.

Rejected alternatives:
- **Operator subcommands inside the serving binary.** They ship operator capability in the network-facing artifact.
- **A new crate or service.** There is no boundary yet that justifies it (ADR-0015).

### D3 — Boot sequence of the serving node

1. Load and validate the D1 configuration, and read the referenced files.
2. Connect the durability root and require `maintain_ready_once` to report ready.
3. Generate a fresh UUIDv7 `NodeId`. Register it with the configured bootstrap authorization. The authorization is consumed, and a replay rejects.
4. Establish S2 custody for this incarnation:
   - `initialize_native_admission_source` on the first installation;
   - `claim_native_admission_source_custody` on every later incarnation.

   The S2 registration is a single custody row, so exactly one serving node at a time can ingest Platform evidence. That matches this one-node slice. Several serving nodes need a later S2 decision.
5. Open the Character recovery store in the configured directory, `seal_current`, and `open_character_authority`. Fresh-store authorization is an operator action (D2); the node never performs it.
6. Log the `NodeId`, then wait a bounded, configured time for an operator assignment of exactly the configured scope to this `NodeId`. If none arrives, exit non-zero. A later assignment to another holder fences this node through the existing #415 generation fencing.
7. Publish runtime readiness for the scope, with the assignment's ownership generation and the D5 revisions.
8. Bind the listener and call `serve_gameplay`.
9. On SIGTERM or SIGINT, cancel the shutdown token: entry work is cancelled and in-flight admissions complete. Then publish `ready: false` for the scope before the process exits.

A restart always yields a new `NodeId`. The previous incarnation's assignment must be replaced by the operator before the new node becomes ready. There is no automatic takeover.

### D4 — S2 evidence is fetched on demand inside each admission attempt

- **Why no background refresh.** The accepted source age, including uncertainty, is at most five seconds at the authorization boundary. A periodic per-account refresh loop cannot keep every account current and would re-age nothing safely.
- **The fetch.** For each attempt, after the #414 Character read yields the `AccountId` and the grant header yields the untrusted `kid` selector:
  1. the admission authority performs the two bounded S1 exchanges (`ReadAccountSecurityV1`, `ReadFreshSigningTrustV1`);
  2. the S2 custody accepts both observations;
  3. only then does the #823 composition and commit run.
- **Bounds.** The exchanges run under the existing transient capacity and within the caller's entry deadline. Any source failure, timeout or denial refuses the attempt without authority mutation.
- **No cache.** The node keeps no cache beyond the S2-accepted, revision-guarded projection.

Consequence: Platform source availability and latency are on the admission path, as the evidence decision already accepted.

### D5 — Readiness revisions are declared by the deployment manifest in this slice

- **Declared values.** These values come from the D1 configuration:
  - `route_revision`;
  - `runtime_observation_revision`;
  - `ruleset_revision`;
  - `content_revision`;
  - `map_revision`;
  - `world_policy_revision`;
  - `offer_revision`.

  Each must satisfy the FND-04 grant profile §11 token grammar.
- **Fixed by the build:** `protocol_major` and `transport_profile`.
- **From the #415 assignment:** `ownership_generation`.
- **Why declared rather than measured.** No Content activation authority lane exists yet (`content/activation.rs` requires one), and this slice serves no gameplay content after admission. Declared revisions are therefore acceptable only while post-admission input stays fail-closed.
- **Supersession trigger.** The first slice that serves content-dependent gameplay must replace declared `content_revision`, `map_revision` and `ruleset_revision` with revisions measured from the activated content generation.
- **Cross-repository coordination.** The same declared values must be configured wherever the Platform issues grants for this scope. Any mismatch fails closed, because each dimension is compared independently. This decision grants no Platform mutation.

### D6 — Observability is minimal

- Structured single-line stderr events cover: configuration accepted, registration, assignment awaited or received, readiness published or withdrawn, listener bound, and shutdown.
- No secret, token, grant, key or database URL is ever logged. `NodeId`, `WorldId`, `ChannelId` and ownership generation may be logged.
- Health, readiness and capacity endpoints remain `OPS-CHANNEL-01` scope.

## 4. Required evidence before integration

This is physical qualification with the shipped binaries in the existing WP5 topology (real Platform, PostgreSQL 17.6):

- **Operator setup and the SEAM stages.** `oteryn-game-ops` performs interpretation configuration, fresh-store authorization, assignment and Character bootstrap from real Platform intents. `oteryn-game-server serve` then reproduces every #823 `SEAM_PASS` stage against its own bound port.
- **D4 freshness.** Admission succeeds with no pre-seeded S2 observations, proving the on-demand fetch. It refuses when the Platform source is unavailable.
- **Configuration.** Missing, malformed or unknown keys, over-maximum limits and unreadable secret files each exit non-zero before binding. No secret appears in output.
- **Bootstrap authorization.** A replayed authorization rejects registration.
- **Restart.** A restart yields a new `NodeId`. The old incarnation cannot admit, and the new one is not ready until the operator replaces the assignment.
- **Shutdown.** Graceful shutdown publishes `ready: false`, and later grants for the scope are refused.

## 5. Explicit non-decisions

This decision does not choose or decide any of the following:
- the orchestrator, container image, systemd unit or Kubernetes shape;
- the secret-delivery product;
- production endpoints, certificates or credentials;
- multiple scopes per node, or dynamic channel creation or placement;
- heartbeat or failure detection, RPO/RTO or live migration;
- a Platform route/offer export contract;
- Content activation authority;
- resume/reconnect (#822);
- any gameplay command slice.

`IMPLEMENTATION_AUTHORITY: NONE_BY_THIS_DECISION`
`LIVE_DEPLOYMENT_AUTHORITY: NONE`
