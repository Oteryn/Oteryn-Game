# Oteryn GameNode Boot Composition Decision (first Reference slice)

- Status: `OWNER_ACCEPTED ARCHITECTURE DECISION` after merge to protected `main`. The owner accepted D1–D6 on 2026-09-24, in the session that authored PR #830, after three independent review rounds. Later review corrections only align D1–D4 with already accepted decisions: the WP3 durability configuration profile, scope-assignment control-actor authentication, the #414 recovery admission and receipt reconciliation, and S2 source ordering.
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
  - the explicit durability-root connection fields that `DurabilityRootConfig` requires:
    - literal transport IP and port;
    - DNS TLS server name;
    - database;
    - username.

    A generic PostgreSQL URL is not accepted, matching the accepted WP3 configuration profile;
  - the Character recovery-fence directory, together with the recovery store's `authority_scope_id` and `issuer_identity`. `CharacterRecoveryStore::open` requires both, and a retained record created under other values rejects. `oteryn-game-ops` takes the same three values;
  - the bounded assignment wait (D3);
  - the control-socket path (D2). The node binds it; `oteryn-game-ops` takes the same path as an explicit argument. Its parent directory must be owned by the node's service user, with mode 0700 and not writable by anyone else;
  - the readiness revisions (D5);
  - the Runtime readiness source authority: one non-empty namespace per deployment and scope, kept identical across node replacements so the guard's source-revision chain stays monotonic, and never equal to a Platform source authority;
  - the S2 descriptor registration revision and its `installed_at` timestamp (D3). Both are copied from the operator-issued S2 fresh-store authorization and changed only together with a descriptor change;
  - two separate Platform routes, each with its own source authority namespace, endpoint, expected peer name and service trust roots. The source authority is the value S1 responses are bound to, and it is distinct from the TLS peer name:
    - the admission evidence source (S1/S2);
    - the Character bootstrap intent issuer (#414).
- **Secrets and trust material are never inline values.** The document gives only file paths, each read once at start:
  - gameplay TLS certificate chain and key;
  - per Platform route: the operator-provisioned service trust roots, plus that route's mTLS client certificate and key. The two routes use distinct client identities. The trust roots are separate from the gameplay certificate chain, and an empty root set rejects, as `ProducerDescriptor::new` already requires;
  - the launch-scoped bootstrap authorization (D3);
  - the PostgreSQL password, and the database root-CA PEM (bounded), each as its own file;
  - only while the S2 store is uninitialized: the one-time S2 fresh-store authorization (D3).

  Every secret file is opened without following symbolic links and must be a regular file owned by the reading process's user, with no group or other permission bits (mode 0600 or 0400). Its parent directory must be owned by that user or root and not writable by group or others. The checks run on the opened descriptor, not on the path, and any failure rejects before the content is read. `oteryn-game-ops` applies the same checks to its own credential files.

  Any missing, unreadable, insecure, malformed or over-bound input fails before a socket is bound. It exits with a distinct code, and the error names the key but never its value.

Rejected alternatives:
- **Environment variables for everything.** Secret-bearing environment dumps are a common leak path, and file permissions give clearer ownership and rotation.
- **Built-in defaults.** They would silently create a production shape nobody accepted.

### D2 — Operator actions are control-plane actions; only the serving node holds a process proof

A second binary target in the same crate, `oteryn-game-ops`, performs control-plane actions.

**Control-actor authentication.** The scope-assignment decision requires an independently authenticated and authorized control actor. A name, configuration file or network access is insufficient. The authentication is the database credential:
- **Group roles.** A forward migration defines two least-privilege PostgreSQL group roles:
  - a GameNode runtime role;
  - a control-plane role.
- **Login roles.** Deployment creates distinct login roles as members of these groups.
- **Control-plane privileges.** Only the control-plane role may:
  - issue or revoke registrations;
  - submit or reconcile assignments;
  - configure the Character interpretation;
  - admit the fresh Character recovery generation;
  - record the S2 fresh-store issuance.
- **Runtime privileges.** The runtime role may only perform the fenced runtime work: registration consumption, S2 custody and observations, readiness, Character reads and bootstrap, and admission.
- **Recorded actor.** The recorded `ControlActor` is derived from the authenticated database session role, never from a caller-supplied label.
- **Exact-scope authorization.** Group membership alone never authorizes an assignment.
  - A new control-scope grant table holds one row per (control login role, `WorldId`, `ChannelId`) with the permitted operations. The operations are initial assign, replace and revoke.
  - Only the database owner can write that table, through deployment administration outside both Game roles. The control-plane role can read it but never write it.
  - Every assignment command checks, in its own transaction, a grant row for the exact session role, scope and operation, and rejects otherwise. A grant that permits initial assign is the independently authorized fresh-scope bootstrap that the scope-assignment decision requires.
  - §4 requires the negative tests.
- **Node isolation.** The serving node holds only runtime credentials.

The exact grant lists are part of the implementation. Negative tests must prove that a runtime credential cannot assign, revoke, issue authorizations, configure the interpretation or admit a fresh Character generation.

`oteryn-game-ops` uses the same explicit durability-root connection fields as D1, with its own control-plane credential files.

**Durable request and authorization files.** Before creating any of them, the tool validates the output file's parent directory: owned by the invoking user or root, not writable by group or others, and opened without following symbolic links. Otherwise it refuses. Every file the tool writes (launch authorization, assignment request, Character recovery request) is created as a new file (exclusive create, no symbolic-link following, mode 0600), written completely, synchronized, and then its parent directory is synchronized. Only after all of this succeeds does the tool submit the database or fence mutation that the file guards. Any failure before that point aborts without the mutation, so a crash can never leave a committed or ambiguous mutation whose exact request was lost. It never registers a GameNode incarnation, so `NodeId` keeps its ADR-0009 meaning: the identity of a running game-server process. Its actions, all through existing Game-owned control-plane operations:
- **Launch authorization.** It issues the launch-scoped bootstrap authorization for one serving launch through `issue_node_bootstrap_authorization`. On a replacement launch, the authorization names the prior `NodeId` in `supersedes`. Before issuing, it durably writes one authorization file for the node containing the complete issuance request: the freshly generated secret, the exact non-secret `LaunchBinding` and the `supersedes` value. `register_node_incarnation` requires the secret and the binding, and a changed binding rejects.
  - **Exact-replay issuance.** The implementation changes `issue_node_bootstrap_authorization` so an existing row with the identical secret digest, binding and `supersedes` returns success instead of `Rejected`; any difference still rejects. A consumed authorization is not reissued: the replay only reports that the exact row exists.
  - **Ambiguous issuance.** After a lost response, `authorization issue --reconcile <file>` replays the retained request, and its definite result says whether the authorization exists. The tool never generates a second authorization for the same launch while a retained file has no definite result.
- **Registration revocation** through `revoke_node_registration`.
- **Channel assignment**, replace and revoke through `RuntimeScopeAssignmentWriter`, with an explicit control actor and a stable writer name:
  - Before submitting, the tool writes the exact request to a request file: a freshly generated operation key and the canonical command. It never regenerates or edits an existing request file.
  - A lost acknowledgement leaves the durable writer slot occupied, and the writer rejects new work until that exact operation is reconciled. The tool therefore offers `assignment reconcile --request <file>`, which replays the retained key and command through `reconcile`.
  - Any later mutating invocation first reads `unreconciled()` and refuses new work while a slot is occupied. It names the operation key so the operator can reconcile it.
- **The Character interpretation revision.** On a new database it is configured only after the fresh Character store is authorized and admitted, because `admit_fresh_character_recovery` requires the Character tables to be empty, including `game_character_interpretations`. The tool refuses to configure the interpretation until generation 1 is admitted.
- **Fresh Character recovery store:** authorize generation 1 in the configured fence directory (`authorize_fresh_store`), then admit it into the database (`admit_fresh_character_recovery`) while the tool still holds that transition. The serving node's `open_character_authority` requires the admitted row.
  - **Request file first.** Before advancing the external fence, the tool writes a request file with the freshly generated `recovery_event_id` and `issued_at`. It never regenerates or edits an existing request file.
  - **Recovery after a failure.** After an ambiguous admission or a lost acknowledgement, the operator re-runs the action with that request file. The same inputs reproduce the same generation-1 transition for the retry or reconciliation; new inputs would conflict.
  - **Idempotence.** The re-run returns the already-admitted generation-1 record and never authorizes a second fresh store.
- **The S2 fresh-store authorization.** It is required by the S2 evidence decision: a genuinely new store initializes only under an independently authorized fresh-store provenance record. The tool issues it as a file holding the complete `FreshStoreProvenance`: namespace, authorization reference, source authority and the exact `initialized_at` timestamp fixed at issuance. The file also holds the descriptor registration revision, `installed_at` and facts. Because every field is fixed at issuance, an ambiguous initialization can be compared exactly on restart. Issuing it never initializes the store itself.
  - **Durable, control-plane-authenticated issuance.** The file alone proves nothing. The tool first writes it durably, then records the exact canonical content through a new control-plane operation into a new issuance table keyed by descriptor revision. Only the control-plane role may insert, and the recorded actor comes from the authenticated session role. An exact replay returns success; different content rejects.
  - **Initialization binds to it.** `initialize_native_admission_source` is changed to require, in the same transaction, a recorded issuance whose content equals the supplied provenance and descriptor exactly; otherwise it rejects. The runtime role can read that row but never write it, so a process holding only runtime credentials cannot fabricate or select the initial source and trust descriptor.
  - §4 requires the negative tests.

Mutations that require a current process proof (`NodeIncarnationProof`) run only inside the serving node, under its own registration:
- S2 initialization and custody;
- S2 observation acceptance;
- runtime readiness;
- Character bootstrap.

Character bootstrap works as follows:
- The operator supplies only the intent's operation id, over a node-local Unix-domain control socket. The socket is created mode 0600, owned by the node's service user and never network-reachable.
- For each request, the node:
  1. first checks `reconcile_character_bootstrap(operation_id)`; an already-committed result is returned without any external read, which covers a lost socket response after the Platform intent has expired;
  2. otherwise reads the intent itself over the Platform intent route (D1);
  3. fetches `ReadAccountSecurityV1` for the intent's `AccountId` from the evidence route and accepts it into S2 custody, because `bootstrap_character` requires current account-security evidence no older than the five-second bound, and on a new installation no admission has fetched it yet;
  4. only then commits the Character with its own proof.
- A missing or stale account-security observation, or an authenticated denial, rejects without a Character mutation. An authenticated denial is still accepted into S2 custody. The operator retries with the same operation id, which stays idempotent under #414.
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
3. **Register.** Generate a fresh UUIDv7 `NodeId` and register it with the configured launch authorization. The `NodeId` is logged, since it is not secret, before the first attempt. The process retains it until registration has a definite outcome.
   - **Ambiguous failure** (lost response, unknown commit, database outage): the process replays the exact same request (secret, binding, `NodeId`) with bounded backoff until the outcome is definite. `register_node_incarnation` returns the original proof for an exact replay. The process never abandons an ambiguous registration and then registers a different `NodeId` under the same authorization.
   - **Definite rejection:** boot fails.
   - **Process killed mid-attempt:** each restart still creates a new `NodeId`, per the registration decision §4. The operator therefore issues a new launch authorization superseding the logged `NodeId`. If that `NodeId` never registered, issuance with `supersedes` rejects. That proves only that the killed process never registered, so the prior holder it was meant to replace may still be current. The operator therefore issues the new authorization with the `supersedes` value retained in the killed launch's authorization file: the original replacement target, or none only when that launch itself had none. Explicitly revoking that original target is the alternative. Supersession is never silently dropped.

   The authorization is consumed, and a replay under a different `NodeId` or binding rejects. When that authorization names a superseded `NodeId`, registration makes the prior incarnation non-current in the same step. A replacement launch without a superseding authorization, or without an operator revocation of the prior incarnation, cannot pass step 4, because a live prior holder keeps S2 custody.
4. **Establish S2 custody** for this incarnation:
   - **First installation:** `initialize_native_admission_source` with the provenance and descriptor from the operator-issued S2 fresh-store authorization. That authorization is required when the store is uninitialized.
     - When the store is already initialized and the authorization is still supplied (for example after an initialization whose response was lost), the node compares the stored provenance and descriptor with the supplied authorization, through a new read-only durability API.
     - An exact match means initialization already completed. The node continues with the custody claim, which is a no-op when this incarnation already holds custody.
     - Any difference fails boot.
   - **Every later incarnation:** `claim_native_admission_source_custody`, which succeeds only when the prior holder is no longer current.
   - **Descriptor check:** `register_native_admission_descriptor` runs on every boot with the configured revision and `installed_at`, and the facts derived from the configured route: source authority, endpoint, peer name, trust-root digest and client-identity digest.
     - A higher revision registers the new facts only if a control-plane issuance for exactly that revision, `installed_at` and those facts is recorded; `register_native_admission_descriptor` checks this in the same transaction and rejects otherwise. The operator records a later revision with `oteryn-game-ops` in the same way as the first (D2), with no fresh-store provenance. A runtime configuration alone can never replace the Platform trust descriptor.
     - An equal revision must match the stored facts exactly, so changed facts retained under an old revision fail boot.
   - **Pending publications:** before any evidence fetch or readiness, read `pending_native_source_publications` and reconcile each retained slot by its exact operation binding (checkpoint or clear, under the S2 resource contract). Boot fails if a slot stays unresolved after a bounded number of attempts.

   The S2 registration is a single custody row, so exactly one serving node at a time can ingest Platform evidence. That matches this one-node slice. Several serving nodes need a later S2 decision.
5. **Open Character authority.** Open the Character recovery store in the configured directory, `seal_current`, and `open_character_authority`. Fresh-store authorization and its database admission are operator actions (D2); the node never performs them. A store that is not admitted fails boot.
6. **Await assignment.** Log the complete non-secret registration fact: `NodeId` together with its database-allocated `registration_revision`. The operator passes exactly this `NodeRegistrationFact` to the assignment `Assign` or `Replace` command. Wait the configured bounded time for an operator assignment of exactly the configured scope to this `NodeId`. If none arrives, exit non-zero. A later assignment to another holder fences this node through the existing #415 generation fencing.
7. **Bind.** Bind the gameplay listener and the control socket. A failed bind exits before any readiness is published.
   - **Pre-existing path.** Before binding, the node checks the configured socket directory's ownership and mode (D1). A pre-existing path in it is removed only if it is a Unix socket owned by the service user. Any other file type or owner fails boot and is never unlinked.
   - **Graceful shutdown** unlinks the socket path. A crash leaves the socket to this check at the next launch.
8. **Publish readiness** for the scope, with the assignment's ownership generation and the D5 revisions:
   1. Read the current Runtime guard publication chain for the scope under the guard serialization. This is a new read-only durability API. A replacement assignment has already written a `ready = false` successor.
   2. Publish with `CompareAndSet` from that exact publication revision, with a source revision strictly greater than the current one.
   3. Use `Bootstrap` with the restored high-water only when no Runtime guard exists.
   4. A stale or conflicting CAS rereads once; if it still fails, boot fails.
   5. **Source metadata.** The node is the source of its own readiness, so nothing is invented:
      - authority: the configured Runtime readiness source authority (D1);
      - source revision: the current guard's source revision plus one, or 1 under `Bootstrap`;
      - decision identity: derived deterministically from the `NodeId`, ownership generation, source revision and `ready` value, so it is unique per revision;
      - observed at: the node's clock at publication, with clock uncertainty 0 because the node observes its own state. The publication validator rejects both a decreasing value and a value later than the node's `now`. If the current guard's `source_observed_at` is ahead of the node's clock by at most the accepted five-second uncertainty bound, the node waits until its clock passes that value and then publishes, so the value never decreases and is never future-dated. A larger gap fails boot with a clock-skew error;
      - restored high-water under `Bootstrap`: the highest publication revision the new read API finds in the guard history for the key, or 0 when there is none.

      The `ready = false` shutdown publication (step 10) uses the same rules; if its bounded wait would exceed the shutdown budget, shutdown proceeds without it, as step 10 already allows for a failed non-ready publication.
9. **Serve.** Run two loops under the same shutdown token:
   - `serve_gameplay` on the already-bound gameplay listener;
   - the bounded control-socket accept/handler loop (D2).

   If either loop fails, the node follows the step 10 shutdown order.
10. **Shut down.** On SIGTERM or SIGINT:
    1. publish `ready: false` for the scope first;
    2. then cancel the shutdown token, which stops gameplay and control-socket acceptance and cancels entry work, while in-flight admissions and an in-flight bootstrap complete;
    3. then unlink the control-socket path and exit.

    If the non-ready publication cannot be written, shutdown still proceeds, and every later admission for the scope refuses because this incarnation stops serving.

**Database outage while serving.** Writing `ready: false` needs the same database, so readiness cannot be withdrawn during an outage. Every admission then refuses without authority mutation (#823). When maintenance reports the root ready again, admissions resume under the unchanged readiness publication. Signalling this node's health to routing stays `OPS-CHANNEL-01` scope.

A restart always yields a new `NodeId`. Before the new process launches, the operator issues its launch authorization superseding the prior `NodeId`, or revokes the prior registration. The operator then replaces the assignment. There is no automatic takeover.

### D4 — S2 evidence is fetched on demand inside each admission attempt

- **Why no background refresh.** The accepted source age, including uncertainty, is at most five seconds at the authorization boundary. A periodic per-account refresh loop cannot keep every account current and would re-age nothing safely.
- **The fetch.** For each attempt, after the #414 Character read yields the `AccountId` and the grant header yields the untrusted `kid` selector:
  1. the admission authority performs the two bounded S1 exchanges (`ReadAccountSecurityV1`, `ReadFreshSigningTrustV1`);
  2. the S2 custody accepts both observations, each under the `NSRC-PENDING-PUBLICATION` lifecycle of the resource envelope:
     - the exact observation binding is checkpointed into one of the two durable publication slots (`checkpoint_native_source_publication`) before the SQL acceptance;
     - the slot is cleared only after a definite outcome;
     - an unknown commit keeps the slot;
     - every evidence demand first reconciles any occupied slot by its fixed identity before checkpointing new work. It replays the retained binding through the idempotent acceptance, then clears the slot on a definite outcome. The D3 step 4 boot reconciliation is the same operation, so a running node recovers its slots once the database recovers, without a restart;
     - with both slots occupied, evidence demand is unavailable and the attempt refuses;
     - all S2 publications of the node, including that reconciliation, run serially in one publication lane. The lane is entered within `NSRC-QUEUE-WAIT`, otherwise the attempt refuses. Its guard is owned by the publication task spawned on the durability root, not by the caller, so a cancelled attempt cannot release it while its SQL operation is outstanding. A slot seen by reconciliation is therefore never live: its original operation has ended in this process with an ambiguous outcome, or it belongs to an earlier incarnation whose custody is fenced. That ended pass ran under the server-side transaction, statement and lock timeouts bounded by its deadline, and the replay reaches its authoritative result through the idempotent acceptance;
  3. only then does the #823 composition and commit run.
- **Bounds.** The exchanges run under the existing transient capacity and within the caller's entry deadline.
- **Failures and denials.**
  - A transport failure, timeout or unauthenticated or undecodable response refuses the attempt, and that failed exchange causes no mutation.
  - The two exchanges are independent. Each authenticated response is checkpointed and accepted into S2 on its own, even when the paired exchange fails. The account exchange completes its acceptance before the trust exchange starts, so a newer denial is never discarded because of the other fetch.
  - An authenticated observation is accepted into S2 custody whatever its facts, including a newer `allowed = false` or `trusted = false` denial. S2 therefore advances its revision floor and a delayed lower-revision allow cannot enter later. The composition then refuses the attempt, and no GameSession, nonce or lease is created.
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

- **Operator setup and the SEAM stages**, in this order:
  1. Before the node starts, `oteryn-game-ops` issues the launch authorization (secret, binding and `supersedes`) and the S2 fresh-store authorization, authorizes and admits the fresh Character store, then configures the interpretation. Configuring the interpretation before the admission is refused.
  2. `oteryn-game-server serve` starts, registers and logs its `NodeRegistrationFact`, then waits for its assignment (D3 step 6).
  3. While it waits, `oteryn-game-ops` assigns the scope to that logged fact. The node then binds its listener and control socket and publishes readiness.
  4. Characters are bootstrapped from real Platform intents through the node control socket.
  5. The running node reproduces every #823 `SEAM_PASS` stage against its own bound port.
  - No operator invocation creates a `game_node_registrations` row.
- **D4 publication slots.**
  - A crash between checkpoint and clear leaves a slot that the next boot reconciles.
  - An ambiguous acceptance while serving is reconciled by the next evidence demand after the database recovers, without a restart. No evidence acceptance happens outside a checkpointed slot.
- **D4 freshness.** Admission succeeds with no pre-seeded S2 observations, proving the on-demand fetch. It refuses when the Platform source is unavailable.
- **Configuration.** Each of the following exits non-zero before binding, with no secret in output:
  - missing, malformed or unknown keys;
  - over-maximum limits;
  - unreadable secret or trust-root files;
  - empty trust roots;
  - an S2 fresh-store authorization that differs from an already-initialized store's stored provenance or descriptor, or that is missing for an uninitialized store.
- **S2 issuance binding.** Initialization without a recorded issuance, or with content that differs from it, rejects.
- **Ambiguous S2 initialization.** A retry after an initialization whose response was lost, with the identical authorization, completes boot.
- **Registration outage.** A database outage during registration is ridden out by exact replay, without exiting or changing the `NodeId`.
- **Launch authorization.**
  - An issuance whose response was lost is reconciled from its retained file: the exact replay succeeds, and a changed binding or `supersedes` rejects.
  - A replayed authorization under a different `NodeId` or binding rejects registration.
  - An exact replay after a lost registration response returns the original proof, and boot continues.
- **Scope grants.** A control-plane login is refused an assignment for a scope or operation it has no grant for, and is refused writing a grant.
- **Descriptor revisions.** A higher configured descriptor revision without a matching recorded issuance fails boot.
- **Readiness clock.** A replacement node whose clock is behind the prior publication's `source_observed_at` by less than the uncertainty bound waits and then publishes readiness; a larger gap fails boot.
- **Mixed S1 results.** When the account exchange returns an authenticated denial and the trust exchange fails, the denial is accepted into S2 and a later delayed lower-revision allow is refused.
- **Credential separation.** With the runtime credential, each of the following is refused by the database:
  - assignment;
  - revoke;
  - authorization issuance;
  - interpretation configuration;
  - S2 fresh-store issuance;
  - fresh Character admission.

  The recorded control actor equals the authenticated control-plane session role.
- **Fresh Character store.** Boot fails until the operator has authorized and admitted generation 1. Re-running the operator action after a lost acknowledgement does not create a second fresh store.
- **Restart.**
  - A replacement launch under a superseding authorization yields a new `NodeId`, claims S2 custody, and becomes ready after the operator replaces the assignment. The old incarnation can no longer admit.
  - A replacement launch without supersession or revocation fails at S2 custody and never becomes ready.
- **Control-socket restarts.** A restart after a graceful exit or a crash binds the control socket again. A foreign file at the socket path, or a wrongly owned or permissive directory, fails boot without being removed.
- **Fresh-store retry.** After an ambiguous fresh Character admission, a re-run with the retained request file completes it without conflict.
- **S2 restart safety.**
  - Changed descriptor facts under an unchanged revision fail boot.
  - After a crash that leaves S2 publication slots occupied, the next boot reconciles them before admitting.
- **Readiness after replacement.** After a replacement assignment, the new node publishes readiness by CAS from the writer's `ready = false` successor.
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
