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
  - the Character recovery-fence directory;
  - the bounded assignment wait (D3);
  - the readiness revisions (D5);
  - the S2 descriptor registration revision (D3);
  - two separate Platform routes, each with its own endpoint, expected peer name and service trust roots:
    - the admission evidence source (S1/S2);
    - the Character bootstrap intent issuer (#414).
- **Secrets and trust material are never inline values.** The document gives only file paths, each read once at start:
  - gameplay TLS certificate chain and key;
  - per Platform route: the operator-provisioned service trust roots, plus that route's mTLS client certificate and key. The two routes use distinct client identities. The trust roots are separate from the gameplay certificate chain, and an empty root set rejects, as `ProducerDescriptor::new` already requires;
  - the launch-scoped bootstrap authorization (D3);
  - the PostgreSQL URL;
  - only while the S2 store is uninitialized: the one-time S2 fresh-store authorization (D3).

  Any missing, unreadable, malformed or over-bound input fails before a socket is bound. It exits with a distinct code, and the error names the key but never its value.

Rejected alternatives:
- **Environment variables for everything.** Secret-bearing environment dumps are a common leak path, and file permissions give clearer ownership and rotation.
- **Built-in defaults.** They would silently create a production shape nobody accepted.

### D2 — Operator actions are control-plane actions; only the serving node holds a process proof

A second binary target in the same crate, `oteryn-game-ops`, performs control-plane actions. It never registers a GameNode incarnation, so `NodeId` keeps its ADR-0009 meaning: the identity of a running game-server process. Its actions, all through existing Game-owned control-plane operations:
- **Launch authorization.** It issues the launch-scoped bootstrap authorization for one serving launch through `issue_node_bootstrap_authorization`. On a replacement launch, the authorization names the prior `NodeId` in `supersedes`. It writes the secret to a file for the node.
- **Registration revocation** through `revoke_node_registration`.
- **Channel assignment**, replace and revoke through `RuntimeScopeAssignmentWriter`, with an explicit control actor and a stable writer name:
  - Before submitting, the tool writes the exact request to a request file: a freshly generated operation key and the canonical command. It never regenerates or edits an existing request file.
  - A lost acknowledgement leaves the durable writer slot occupied, and the writer rejects new work until that exact operation is reconciled. The tool therefore offers `assignment reconcile --request <file>`, which replays the retained key and command through `reconcile`.
  - Any later mutating invocation first reads `unreconciled()` and refuses new work while a slot is occupied. It names the operation key so the operator can reconcile it.
- **The Character interpretation revision.**
- **Fresh Character recovery store authorization** (generation 1) in the configured fence directory.
- **The S2 fresh-store authorization.** It is required by the S2 evidence decision: a genuinely new store initializes only under an independently authorized fresh-store provenance record. The tool issues it as a file holding the provenance namespace, authorization reference, source authority, and the descriptor registration revision and facts. Issuing it never initializes the store itself.

Mutations that require a current process proof (`NodeIncarnationProof`) run only inside the serving node, under its own registration:
- S2 initialization and custody;
- S2 observation acceptance;
- runtime readiness;
- Character bootstrap.

Character bootstrap works as follows:
- The operator supplies only the intent's operation id, over a node-local Unix-domain control socket. The socket is created mode 0600, owned by the node's service user and never network-reachable.
- For each request, the node:
  1. reads the intent itself over the Platform intent route (D1);
  2. fetches `ReadAccountSecurityV1` for the intent's `AccountId` from the evidence route and accepts it into S2 custody, because `bootstrap_character` requires current account-security evidence no older than the five-second bound, and on a new installation no admission has fetched it yet;
  3. only then commits the Character with its own proof.
- A missing, denied or stale account-security observation rejects without a Character mutation. The operator retries with the same operation id, which stays idempotent under #414.
- The socket loop is bounded: one request at a time, a bounded request size and a per-request deadline. It answers with a closed result (committed, rejected or unavailable) and never with the intent contents.
- The operator's input is a pointer, not authority. The Platform-authenticated intent, current account security and the recovery fence decide the result, as in #414.
- The socket accepts no other command.

Rejected alternatives:
- **Registering each operator invocation as a GameNode incarnation.** This redefines `NodeId`. Because V1 has no expiry, it would also leave an exited process current and assignable.
- **A new operator-process authority.** It needs its own owner decision, and this slice needs it only for Character bootstrap.
- **Operator subcommands inside the serving binary.** They ship control-plane capability in the network-facing artifact.
- **A new crate or service.** There is no boundary yet that justifies it (ADR-0015).

### D3 — Boot sequence of the serving node

1. Load and validate the D1 configuration, and read the referenced files.
2. Connect the durability root and require `maintain_ready_once` to report ready. From then until exit, a bounded maintenance task calls `maintain_ready_once` in two cases: at a fixed interval well inside the 30-minute holder lifetime, and immediately after any operation reports `RootUnavailable`. The durability holder is therefore re-established after retirement or loss, rather than only at boot.
3. **Register.** Generate a fresh UUIDv7 `NodeId` and register it with the configured launch authorization. The authorization is consumed, and a replay rejects. When that authorization names a superseded `NodeId`, registration makes the prior incarnation non-current in the same step. A replacement launch without a superseding authorization, or without an operator revocation of the prior incarnation, cannot pass step 4, because a live prior holder keeps S2 custody.
4. **Establish S2 custody** for this incarnation:
   - **First installation:** `initialize_native_admission_source` with the provenance and descriptor from the operator-issued S2 fresh-store authorization. That authorization is required when the store is uninitialized and rejected when it is already initialized.
   - **Every later incarnation:** `claim_native_admission_source_custody`, which succeeds only when the prior holder is no longer current.
   - **Descriptor changes:** `register_native_admission_descriptor` runs when the configured descriptor revision advances.

   The S2 registration is a single custody row, so exactly one serving node at a time can ingest Platform evidence. That matches this one-node slice. Several serving nodes need a later S2 decision.
5. **Open Character authority.** Open the Character recovery store in the configured directory, `seal_current`, and `open_character_authority`. Fresh-store authorization is an operator action (D2); the node never performs it.
6. **Await assignment.** Log the complete non-secret registration fact: `NodeId` together with its database-allocated `registration_revision`. The operator passes exactly this `NodeRegistrationFact` to the assignment `Assign` or `Replace` command. Wait the configured bounded time for an operator assignment of exactly the configured scope to this `NodeId`. If none arrives, exit non-zero. A later assignment to another holder fences this node through the existing #415 generation fencing.
7. **Bind.** Bind the gameplay listener and the control socket. A failed bind exits before any readiness is published.
8. **Publish readiness** for the scope, with the assignment's ownership generation and the D5 revisions.
9. **Serve.** Run two loops under the same shutdown token:
   - `serve_gameplay` on the already-bound gameplay listener;
   - the bounded control-socket accept/handler loop (D2).

   If either loop fails, the node follows the step 10 shutdown order.
10. **Shut down.** On SIGTERM or SIGINT:
    1. publish `ready: false` for the scope first;
    2. then cancel the shutdown token, which stops gameplay and control-socket acceptance and cancels entry work, while in-flight admissions and an in-flight bootstrap complete;
    3. then exit.

    If the non-ready publication cannot be written, shutdown still proceeds, and every later admission for the scope refuses because this incarnation stops serving.

**Database outage while serving.** Writing `ready: false` needs the same database, so readiness cannot be withdrawn during an outage. Every admission then refuses without authority mutation (#823). When maintenance reports the root ready again, admissions resume under the unchanged readiness publication. Signalling this node's health to routing stays `OPS-CHANNEL-01` scope.

A restart always yields a new `NodeId`. Before the new process launches, the operator issues its launch authorization superseding the prior `NodeId`, or revokes the prior registration. The operator then replaces the assignment. There is no automatic takeover.

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

- **Operator setup and the SEAM stages.**
  - `oteryn-game-ops` issues the launch authorization and the S2 fresh-store authorization, configures the interpretation, authorizes the fresh Character store, and assigns the scope.
  - Characters are bootstrapped from real Platform intents through the node control socket.
  - `oteryn-game-server serve` then reproduces every #823 `SEAM_PASS` stage against its own bound port.
  - No operator invocation creates a `game_node_registrations` row.
- **D4 freshness.** Admission succeeds with no pre-seeded S2 observations, proving the on-demand fetch. It refuses when the Platform source is unavailable.
- **Configuration.** Each of the following exits non-zero before binding, with no secret in output:
  - missing, malformed or unknown keys;
  - over-maximum limits;
  - unreadable secret or trust-root files;
  - empty trust roots;
  - an S2 fresh-store authorization supplied for an already-initialized store, or missing for an uninitialized one.
- **Launch authorization.** A replayed authorization rejects registration.
- **Restart.**
  - A replacement launch under a superseding authorization yields a new `NodeId`, claims S2 custody, and becomes ready after the operator replaces the assignment. The old incarnation can no longer admit.
  - A replacement launch without supersession or revocation fails at S2 custody and never becomes ready.
- **Readiness ordering.**
  - When the listener bind fails, no readiness is published.
  - On shutdown, `ready: false` is published before acceptance stops.
- **Durability maintenance.** After the holder is forcibly retired or the database restarts, the node refuses admissions during the outage and admits again after recovery, without a process restart.
- **Assignment handoff.** The operator assigns using only the logged registration fact (`NodeId` and revision), and a wrong revision rejects.
- **Shutdown.** Graceful shutdown publishes `ready: false`, and later grants for the scope are refused.
- **Control socket.**
  - It rejects any command other than a bootstrap operation id, and it is not reachable over the network.
  - On a fresh installation, the first Character bootstraps through it with no prior admission, which proves the account-security fetch.
  - A denied account rejects without a Character.
- **Assignment reconciliation.** When an assignment's acknowledgement is lost, later invocations refuse new work until `assignment reconcile` with the retained request file reports the exact outcome.

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
