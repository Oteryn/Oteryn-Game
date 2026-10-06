# ARCH-ALPHA-OPS-0: alpha operability (observability, time and performance, data continuity, client version)

- Decision id: ARCH-ALPHA-OPS-0.
- Status: the rulings of §1–§4 and the packets are accepted on merge. The owner ruled on
  2026-10-06 (item 1a) that the alpha operability package is authored as one decision set.
  Every contract amendment below is exact text marked pending: it is applied by the named
  packet, because the owning file is a candidate, because the control plane leases registry
  numbers, or because the target is an accepted cross-repository contract
  (`CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`) that needs control-plane and Platform review.
  It is a proposed architecture decision and grants no runtime authority; each packet carries
  its own review.
  The owner ruled on every first-round item of §5.2 on 2026-10-06 and the rulings are recorded
  there and in each section's owner questions: every item **a**, except §3 Q2 **b** for the
  testing phase only, which is decided again before external players are admitted (§3 ruling 8).
  The body follows these rulings. §3 Q4 (review round 2) is DECIDED: a, accepted by control
  plane under D607 (D838). No owner item in §1–§4 remains open.
- Origin: owner request (2026-10-06): review what exists and decide what still has to be fixed
  architecturally, because the owner had to point out errors by hand.
- Owning contracts: `docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md`
  (ERR-CODES; the diagnostic line, code blocks and `diagnose`), `FND-03_RUNTIME_EXECUTION_CONTRACT.md`
  (scheduling and clocks), `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, the DUR-02 schema
  gate, the Character recovery fence decision, `FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` and
  `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (wire code 1117), FND-04A/FND-04B
  (admission and resume refusal rows), ALPHA-CLIENT-01 and SEC-CLIENT-01 (updater and release
  manifest). Gap register §25 and §26 point here.
- Related: ARCH-LIVE-READINESS-0 (the "before external players" package: GM tooling, threat
  model and live operations, economy remediation and privacy) relies on §1 retention and §3's
  restore fence directory; its erasure journal and its retained revoke, mute, fence raise and hold
  placement requests are re-applied by §3 ruling 9 step 7.

## Implementation brief

1. Logs stay the ERR-CODES key=value line; `tracing` stays rejected. Levels follow §1 ruling 1;
   no movement history at any level.
2. A resumed connection logs `parent=<trace>`; the admission-outcome line logs
   `attempt=<attempt_ref>`, never the `jti` (§1 rulings 2–3). Both are UUIDv7 and grant nothing.
3. Node→Platform calls send W3C `traceparent` built from the trace (§1 ruling 4).
4. Durability failures carry a 3xxx code and the caller's trace; the DB `application_name` is
   `oteryn-game-server/<version>+<sha12>` (§1 ruling 5).
5. The supervisor pipes stderr into `oteryn-game-server log-sink`, which writes per-incarnation
   segments, rotates them while running at 64 MiB or 24 h, and prunes hourly by age and by a
   1 GiB directory cap, so two restarts no longer erase the panic line and a long-running node
   cannot fill the volume (§1 ruling 6, F18).
6. Retention (owner ruling §1 Q2 a): logs 14 days, metrics 30 days, bundles until the issue
   closes (§1 ruling 7).
7. Metrics: `metrics` + `metrics-exporter-prometheus` on a loopback listener; the §1 ruling 9
   set only; no player, session or item labels.
8. Prometheus and Grafana run on the alpha NAS and alert by email (owner ruling §1 Q1 a). Alerts
   live in `deploy/observability/oteryn-alerts.yml`; SLOs have no numbers until measured.
9. `oteryn-game-ops diagnose --bundle` writes an all-or-nothing evidence directory, never
   reading the DB or Platform (§1 ruling 12).
10. There is no global tick. Owners run a work queue and a timer lane; every periodic timer is
    a RESOURCE_LIMITS row with its catch-up class (§2 rulings 1–2).
11. Gameplay reads time only through `OwnerClock`; each rule uses exactly one of four clocks
    (§2 rulings 3–4). Durable time is a remaining duration or a database-time deadline.
12. Hosts run chrony; skew raises an alarm with a 2xxx code and never refuses readiness;
    fencing never compares cross-host time (§2 ruling 7).
13. The current build hosts one channel per node and at most 256 connections; no capacity value or
    admission limit is published until the separate ARCH-ALPHA-CAPACITY-0 decision merges, which
    fixes its failure objectives before any sweep. The World limit is never multiplied from the
    Channel value (§2 rulings 9–11). D128's 500 is a target.
14. iai-callgrind benches run as a job selected by the `oteryn-game-server` dependency closure; it
    reports first and becomes required after PERF-CI-1 sets the threshold (§2 ruling 12, owner
    ruling §2 Q2 a). A measured capacity below 500 is accepted for alpha and the gap is logged.
15. sqlx `migrate!` is the only migration tool; merged files never change; new versions exceed
    the highest on `main`; the gaps below 0079 stay gaps forever (§3 ruling 1).
16. One release is one schema version. A release with a migration, or one that fails the
    content bundle check (item 17), is stop-the-world: close, drain, checkpoint, stop, named backup,
    migrate, domain migration, live check, activate, start, validate, open (§3 ruling 3). Migration
    waits for the named backup to complete; an audited domain migration runs under a `PENDING`
    activation row; the gating live check reads the final migrated database; no node starts and no
    restore selects the build before its `ACTIVE` row. Deploy, restore and named backup hold one
    exclusive ops lock, which their child `oteryn-game-ops` commands inherit and never weaken.
17. Durable rows reference content only by `{family, key, revision}`. A rolling release must
    still resolve every definition that any build activated on the database resolves, checked
    on the bundles alone. A stop-the-world release is checked against the live database after every
    migration completes, with every node stopped, and a removal needs a DUR-04 §12 policy for each
    referenced definition; a passing live check records a compatibility baseline that retires the older bundle obligations (§3 ruling 5).
18. pgBackRest with continuous WAL, weekly full, daily differential and a named pre-migration
    backup; encrypted, off the DB host. A `pgbackrest check` probe every 120 s alerts when the
    last proven archive is older than 300 s; the 5 min RPO holds only while it is clear (§3
    ruling 7).
19. The restore fence directory holds every value a restore must not roll back: Character
    recovery fence, LCFA F, `assignment_epoch` high-water, the erasure journal and the retained
    signed revoke, mute, fence raise and hold placement requests (§3 ruling 8),
    beside the ops lock and the restore journal. In the testing phase it is a separate NAS volume only (owner ruling §3 Q2 b), and after its loss
    a reseed allowed only under `stack_phase = TESTING` rebuilds every value and the retained fence
    chain above the restored and surviving backup values; its location is
    decided again before external players are admitted. Recovery targets: RPO 5 min, RTO 4 h,
    14-day backup retention, a monthly drill (owner ruling §3 Q1 a).
20. `oteryn-game-ops restore` runs the twelve steps of §3 ruling 9 in order, re-entrant, with a
    35 s admission gate; nodes start and admission opens only after the Platform reconciliation
    and a final validation pass (DUR-02 rule 5); clients log in fresh. It runs the build named by the newest build
    activation row in the restored database, which must be `ACTIVE`, after checking that its recorded ledger digest
    equals the restored ledger's, and bridges the Character fence across every recovery generation
    the restore went back over, from retained fence records.
21. After a Game restore Platform re-requests by operation identity, selected by the Game
    recovery generation and never by comparing clocks; Game re-applies each acknowledged
    delivery absent from the restored database before admission reopens, and Platform
    acknowledges the restore notice; Game never writes back (§3 ruling 10). Progress after T is lost and never replayed (§3 ruling 11).
22. The client build id is `oteryn-client/<semver>+<sha12>`; a node may set
    `client_build_floor`; below it fresh admission returns 1117
    `ADMISSION_CLIENT_BUILD_UNSUPPORTED` at FND-04A §7 step 11 (§4 rulings 1–3).
23. "Update required" is a latest-only signed feed read before login; assets ship inside the
    build; Velopack installs from GitHub Releases; an Ed25519 release manifest with offline keys
    held by the owner anchors the update, and the client persists a trust-record rollback floor
    so a revoked key cannot return; every package also needs a root-signed release
    attestation, so an older build cannot be moved to a package a revoked key signed (§4 rulings 5–9). No Authenticode in the closed alpha. No
    browser in alpha.
24. Packet order: DATA-MIGRATION-GUARD-1 first (P0-adjacent, §3 F6); OBS-LOG-1 after ERR-NODE-1;
    TIME-CLOCK-1 and PERF-CI-1 are independent; VERSION-FLOOR-1 before any floor is set.
25. `deploy/synology-game/` is written by OBS-DEPLOY-4 and DATA-BACKUP-PITR-3 only after PR
    #1874 merges, one writer at a time in the order the control plane leases.
26. New ops (6xxx) and node (2xxx) codes are leased by the control plane at packet time.

## 1. Observability

In this part, ERR-CODES means ARCH-ERROR-CODES-0, and Rn means ruling n of this part.

### Facts

- F1 `docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md` §1.5 (lines 210-243). PROVEN.
  This section fixes the one-line key=value format and its field order, and forbids player-linked ids. `trace` is a UUIDv7 minted per boot and per accepted connection. Admission, durability writes and Platform calls log the connection's trace. This decision builds on that line and does not replace it.
- F2 Same file, §1.10 item 5 (lines 388-397). PROVEN. It defines four levels and `OTERYN_LOG`, read once at start with default `info`. Coded, `warn` and `error` lines are always written, and a malformed value fails the start.
- F3 Same file, §1.9 (lines 297-300). PROVEN. The metrics, tracing and log-shipping backend, alerting and retention are "not decided" and stay under gap §26. This decision fills that for the alpha.
- F4 Same file, §3. PROVEN. `tracing` is rejected "until an observability backend" is chosen, and JSON lines are rejected.
- F5 Same file, §2.5 ERR-DIAG-4. PROVEN. The owned paths are `apps/game-server/src/bin/oteryn-game-ops.rs` and a new `apps/game-server/src/ops_diagnose.rs`. The module is absent on `origin/main` (PROVEN). `diagnose` reads only the named file, with no DB and no Platform access.
- F6 Same file, §1.10 item 6 and ERR-TRACE-5. PROVEN. `CONNECTION_TRACE_V1` (capability 20) carries the trace to the client behind a capability. The client↔node hop is therefore decided.
- F7 Live state, 2026-10-06. PROVEN. `crates/error-codes` is not on `main`, and ERR-NODE-1 is open as PR #1868. Every ruling below follows ERR-NODE-1.
- F8 `OTERYN_GAME_NODE_BOOT_COMPOSITION_DECISION_2026-09-24.md` D6. PROVEN. Observability is minimal: stderr single-line events with no secrets. NodeId, WorldId, ChannelId and generation are loggable. Health, readiness and capacity endpoints are OPS-CHANNEL-01 scope. D3: a DB outage makes admissions refuse. D4: S2 evidence is fetched from Platform per admission.
- F9 `ADR-0009` §2 and §6. PROVEN. The node exposes health, readiness, capacity and lifecycle, and the orchestrator is external. Latency is reported as p50, p95 and p99, and queue age is reported. Neither has a fixed value; PERF-01 owns the numbers (gap §25).
- F10 `ADR-0006` §2 (lines 52-62). PROVEN. Labels must be low-cardinality. No character, GUID, item, transaction or session-generation labels. Metrics may reset on restart, and observability failure must not change gameplay outcomes. Line 392 rejects "only Prometheus and logs" for audit evidence, not for operational health.
- F11 `ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md` §18 (lines 274-276). PROVEN. AccountId, CharacterId, GameSessionId and the other player-linked ids are forbidden as labels, and logs carry only bounded correlation references. §16 (lines 260-266) forbids unlimited retention and does not guess durations.
- F12 `docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md` §15 (lines 403-405). PROVEN. "Authorized diagnostics may include attempt_ref". Line 125: `attempt_ref` is the producer correlation identity and not a GameSessionId. That makes it the Gateway↔node join key.
- F13 `ADR-0003` §3, §4 and §7. PROVEN. The Gateway is a Platform service in Go. The client reaches Identity and the Gateway over HTTP, then connects straight to the Rust node with a grant. The node→Platform hops are registration, `ReportRuntimeStatusV1` and S2 evidence (F8).
- F14 `apps/game-server/src/durability/db.rs:690-706`. PROVEN. `PgConnectOptions` sets TLS, auth and a budget but no `application_name`. The vendored `vendor/sqlx-postgres-0.9.0/src/options/mod.rs:509` has `application_name()`, so no patch is needed. Same file, around line 711: a lazy pool of at most one connection, with serialized root maintenance.
- F15 `Cargo.toml` and `apps/game-server/Cargo.toml`. PROVEN. There are no `tracing`, `metrics`, `prometheus` or OpenTelemetry crates; `reqwest` is already a dependency. `deny.toml` allows MIT and Apache-2.0. DERIVED: `metrics` and `metrics-exporter-prometheus` are MIT; `cargo deny check` confirms this in the packet.
- F16 Open PR #1874, head `a6170c0f`, `deploy/synology-game/supervisor.sh` and its README. PROVEN.
  - stderr goes to `/volume1/oteryn/game-preprod/log/node.log`, a persistent NAS volume with mode 0700 for uid 1001.
  - Each start moves the old log to `node.log.previous`, so only one is kept. There is no size cap.
  - `health` greps `event=registered … node_id=`, `event=awaiting_assignment` and `readiness ready=true`.
  - `oteryn-game-ops` runs only as root.
- F17 `DISCONNECT_LIVENESS_AND_CRASH_EVIDENCE_OWNER_BASELINE.md` §5 (lines 108-124). PROVEN. An external supervisor or collector keeps bounded crash evidence: build, NodeId, panic signature and the logs before the failure. That evidence must not live only on an ephemeral filesystem.
- F18 **CONFLICT** between F16 and F17. Two restarts in a row erase the log that holds the panic line. R6 fixes this.
- F19 `CLIENT_CRASH_DIAGNOSTICS_PRIVACY_OWNER_BASELINE.md`. PROVEN. Client crash upload is automatic with an opt-out and an allowlist, and server evidence is independent of it. Client upload stays under gap §15; this decision covers only server evidence and the F12 report line.
- F20 `ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` §26 (lines 476-490). PROVEN. Logs, metrics, traces, correlation, alerts, cardinality and cost are unresolved. `EXP-OBS-01` (broader observability) is Track 4, so an alpha-minimum set is consistent with the register.
- F21 `FND-ID-01_NODE_ID_PROCESS_INCARNATION_OWNER_BASELINE.md` (around lines 198-207). PROVEN. A NodeId exists per process incarnation and may appear in logs and metrics.
- F22 PR #1868 diff of `native_admission_source/runtime_status.rs`. DERIVED: the runtime-status line keeps `ready=` and `elapsed_ms=` tokens. UNKNOWN: whether the `registered` and `readiness` lines keep the exact tokens the F16 health check greps.
- F23 **CONFLICT** (document status only). The ANL-01 and FND-04 profile headers read "Candidate". `FOUNDATION_PROGRAMME_CURRENT_STATUS.md:57` reads `ACCEPTED`/`LIFECYCLE_CLOSED`. This decision treats both as accepted.

### Rulings

1. **Log levels.** The ERR-CODES §1.5 line and `OTERYN_LOG` stay the only log path, and `tracing` stays rejected (F4). Levels are used as follows:
   - `error`: a coded failure that ends an operation the node cannot retry, or ends the process.
   - `warn`: a coded degraded or retryable state, such as a DB root that is not ready, a refused or failed Platform call, or a refused admission caused by a dependency.
   - `info`: lifecycle (process start, registration, assignment, readiness, shutdown), connection open and close with the close code, and exactly one admission-outcome line per attempt.
   - `debug`: per-command and per-DB-transaction detail. It is never on by default in the alpha.

   There is no per-movement line at any level, because it would breach the forensic baseline of no movement history by default. Runtime level change stays out of scope (ERR-CODES §1.9).
2. **Client→node correlation.** The connection `trace` is the only join key. It reaches the client through `CONNECTION_TRACE_V1` (F6) and the F12 report line, and nothing more is added on the wire. When a node accepts a resume and still holds the previous connection's trace in memory, the resume line logs it as `parent=<uuidv7>`. A trace is never linked to a character in a log (F1).
3. **Gateway→node correlation.** The node's admission-outcome line carries `attempt=<attempt_ref>` from the verified grant (F12). It never logs the `jti`, a nonce or a security generation. A refused grant that fails before verification logs no `attempt`. Gateway-side logging of the same `attempt_ref` is a Platform proposal (R11).
4. **Node→Platform correlation.** Every node HTTP call to Platform sends a W3C Trace Context `traceparent: 00-<trace as 32 hex>-<16 hex random span id>-00`. The UUIDv7 trace is 16 bytes, the same size as a W3C trace-id. The header carries no authority, and Platform must not use it as input. Recipients ignore unrecognized HTTP fields (RFC 9110), so this needs no Platform change. Platform logging the trace-id is a proposal (R11). Each Platform call failure logs `warn` with its 5xxx code and the caller's trace.
5. **Node→DB correlation.**
   - Each durability failure line carries the 3xxx code of its SQLSTATE and the `trace` of the work that issued it. Root-maintenance work uses the boot trace.
   - The node sets `application_name` to `oteryn-game-server/<version>+<sha12>` through the vendored `PgConnectOptions::application_name` (F14). Postgres server logs (`%a`) and `pg_stat_activity` then name the binary and build, never a player.
   - A DB write is attributed by the line that issued it. No trace, CharacterId or session id is sent to Postgres.
6. **Log storage and rotation.** Logs stay on the alpha host's persistent volume as stderr captured by the supervisor (F16), and nothing is shipped off the host in the alpha. Pruning only at start is not enough: a node that runs longer than the retention would keep old lines and grow one file until the volume fills. So the active log rotates while the node runs.
   - The supervisor pipes the node's stderr into `oteryn-game-server log-sink --dir BASE/log` (a subcommand of the same binary, so nothing is installed on the host, which has no root package install and no Docker, F16). It records both pids and checks both command lines before signalling.
   - The sink reads whole lines and writes segments `log/node-<utc-start>-<seq>.log`, with `log/node.log` pointing at the active one. A line is never split across segments; a line longer than 64 KiB is cut with a `truncated=1` marker.
   - **Rotation while running:** a new segment starts when the active one reaches 64 MiB or is 24 h old, whichever comes first.
   - **Pruning:** at sink start and every hour on the sink's own timer, it deletes every closed segment whose newest line is older than 14 days (R7). Then, if the directory holds more than 1 GiB, it deletes the oldest closed segments until it is below that, and writes one `warn` line with a registered 2xxx code into the active segment. The active segment is never deleted.
   - **Bounds:** the log directory holds at most 1 GiB plus one segment (64 MiB). Every line is kept at least 14 days, unless the size cap removes it earlier with a coded line, and is deleted within 15 days and 1 hour (a segment spans at most 24 h, and the prune runs hourly). A lifecycle file line is deleted within 14 days and 1 hour.
   - **Lifecycle file:** the sink also copies the incarnation's `process_start`, `registered`, `awaiting_assignment`, `readiness`, `shutdown` and `panic` lines into `log/node-<utc-start>.lifecycle` (capped at 1 MiB). These lines carry no player-linked field (R1). The F16 health greps, the deploy's `awaiting_assignment` revision read and the bundle's boot-trace lines read this file, so rotation cannot hide them. The hourly prune also ages this file: it rewrites it (temporary file, sync, atomic rename, directory sync) without every line older than 14 days. If a dropped line is the incarnation's latest `registered`, `awaiting_assignment` or `readiness` line, the rewrite appends one `lifecycle_state` line that restates that line's fields with the current time and `since=<its original time>`, so the health greps and the revision read still find the current state. The file is deleted with the incarnation's last segment.
   - If the sink dies, a node write fails with `EPIPE`; the line is dropped and counted in `log_lines_dropped_total`, and gameplay never changes (F10). The supervisor's `health` fails while the sink is gone.
   - The node writes each line with one locked write, so lines do not interleave.

   The 64 MiB, 24 h and 1 GiB values are CANDIDATE until U1 is measured. This resolves F18.
7. **Retention.** Owner ruling 2026-10-06 (owner question 2, option a): the alpha retention is logs 14 days, metrics 30 days (Prometheus `--storage.tsdb.retention.time=30d`), and debug bundles deleted when their issue is closed, within DATA-PRIVACY-01 and ANL-01 §16. A legal or incident hold is explicit and per bundle. No log is kept without a bound (R6).
8. **Metrics stack.**
   - The node uses the upstream `metrics` facade with `metrics-exporter-prometheus` (MIT, F15). The exporter serves Prometheus text on a loopback-only listener set by `[metrics] listen = "127.0.0.1:<port>"` in `node.toml`. The port is CANDIDATE, set in the deploy template.
   - Without the key there is no listener. A configured listener that fails to bind fails the boot with a 2xxx code.
   - A recorder or scrape failure never changes gameplay (F10). There is no OpenTelemetry SDK and no push.
   - Metrics reset on restart (F10).
9. **Minimum alpha metric set (node).** All names are prefixed `oteryn_`. Labels come only from a registered code, a bounded enum or the info metric; there are no player, session, generation or item labels (F10, F11).
   - `node_info{node_id,build,world,channel}` = 1. NodeId is per incarnation (F21), so it is the only per-incarnation label.
   - `node_start_time_seconds` and `node_ready` (0/1).
   - `connections_accepted_total`, `connections_open` and `connections_closed_total{code}`.
   - `admissions_total{outcome}`, where `outcome` is the registered typed outcome.
   - `owner_service_duration_seconds` (histogram: the time an owner spends serving one command or timer batch; there is no tick, §2 ruling 1) and `command_queue_oldest_age_seconds` (ADR-0009 §6).
   - `db_transaction_duration_seconds` (histogram), `db_errors_total{code}` and `db_root_ready` (0/1). The pool holds at most one connection (F14), so there is no pool gauge.
   - `platform_request_duration_seconds{call}` and `platform_errors_total{call,code}`, where `call` is a fixed enum: `register`, `runtime_status` or `s2_evidence`.
   - `log_lines_total{level,code}`, which lets alerts see coded failures without log shipping, and `log_lines_dropped_total`, the lines lost while the R6 log sink was gone.

   Histogram buckets use upstream defaults until PERF-01 measures them. DB-side metrics (`postgres_exporter`) and Gateway metrics are out of the Game set.
10. **SLOs and alerts.** The SLO candidates have **no numbers** until PERF-01 or an alpha baseline sets them:
    - node ready ratio during declared test windows;
    - admission success ratio;
    - owner service time p99 (§2 ruling 1; there is no tick);
    - DB transaction p99.

    The alert rules live in `deploy/observability/oteryn-alerts.yml`:
    - `OterynNodeDown`: `up==0`.
    - `OterynNodeNotReady`.
    - `OterynDbRootNotReady`.
    - `OterynNodeRestarted`: `changes(oteryn_node_start_time_seconds)`, which catches panics.
    - `OterynErrorLines`: `increase(oteryn_log_lines_total{level="error"})>0`.
    - `OterynLogLinesDropped`: `increase(oteryn_log_lines_dropped_total)>0`, which catches a dead log sink (R6).
    - `OterynAdmissionRefusals` and `OterynPlatformErrors`, as ratios.

    Every `for:` duration and every ratio threshold is CANDIDATE, set from the first alpha week. There is one dashboard JSON in `deploy/observability/`. Owner ruling 2026-10-06 (owner question 1, option a): Prometheus and Grafana run as host-network containers on the alpha NAS, Prometheus scrapes the loopback listener, and alerts go to the owner's email.
11. **Platform proposal** (routed by the control plane, not written by Game).
    - The Gateway logs `attempt_ref` and the typed outcome on its `POST /v1/login` line.
    - Platform services log the trace-id of an incoming `traceparent`.
    - Gateway metrics follow the same rules: login and grant counts by outcome, and latency.
    - Neither `attempt_ref` nor any player id is used as a label.
12. **Debug bundle.** `oteryn-game-ops diagnose` gains `--bundle <dir>`. It works with `--report "<line>"` or `--trace <uuid>`, one or more `--log <file>`, and an optional `--metrics-url http://127.0.0.1:<port>/metrics`. It writes a new directory (mode 0700, files 0600):
    - `report.txt`: the input.
    - `lines.log`: the matching trace lines, any `parent` traces, and the `process_start`, `panic` and readiness lines of the same incarnation (its boot trace). Each line is re-parsed against ERR-CODES §1.5, and a non-conforming line is dropped and counted.
    - `codes.txt`: the registry entries for each code.
    - `build.txt`: the ops build and the node `build=` from the logs.
    - `metrics.prom`: the snapshot, or the reason it is missing.
    - `MANIFEST`: sha256 of each file and the inputs.

    The URL must resolve to a loopback address and is time-bounded. The bundle is written to a temporary directory and renamed, so it is either complete or absent. It never reads the DB or Platform (F5).

### Contract amendments

1. `docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md` §1.5, field order item 4.
   - Replace: "4. `code`, `name`, `cat`, `trace` and `wire` (only when present);"
   - With: "4. `code`, `name`, `cat`, `trace`, `parent`, `attempt` and `wire` (only when present);"
   - Add after the `trace` bullet: "- `parent` is the trace of the earlier connection that a resumed connection continues, written only when the node still holds it. `attempt` is the verified grant's `attempt_ref` (FND-04 profile §15), written only on the admission-outcome line. Both are canonical lowercase UUIDv7; a parser rejects any other form. Neither authorizes anything (ARCH-ALPHA-OPS-0)."
2. Same file, §1.9, first bullet.
   - Replace with: "- Log shipping off the host, long-term retention and broader observability (`EXP-OBS-01`). These remain under gap register §26. The alpha log, metrics, alert and debug-bundle rulings are in ARCH-ALPHA-OPS-0."
3. Same file, §3, the `tracing` bullet.
   - Append: "ARCH-ALPHA-OPS-0 chose a Prometheus metrics backend and kept this rejection: traces cross hops through the §1.5 `trace`, `attempt` and `traceparent`, not through spans."
4. `docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` §26.
   - After "Existing gates" add: "- Alpha partial resolution: ARCH-ALPHA-OPS-0 (log levels, correlation, host-local logs, loopback Prometheus metrics, alerts, debug bundle). Off-host shipping, durable SLO store and production retention remain open."

### Packets

- **OBS-LOG-1** (log levels, correlation fields, DB name). Depends on ERR-NODE-1 (#1868).
  - Builds R1, R2, R3 and R5 and amendment 1.
  - Owned paths: `apps/game-server/src/node/serve.rs`, `apps/game-server/src/durability/db.rs`, the admission-outcome call site under `apps/game-server/src/native_admission_source/`, and the line writer in `crates/error-codes` once merged.
  - Tests:
    - the admission line carries `attempt` and never `jti`;
    - a resume line carries `parent` only when it is held;
    - the parser rejects `parent` or `attempt` out of order or not UUIDv7;
    - a durability failure carries a 3xxx code and the caller's trace;
    - `application_name` equals the build string (connect test against `tools/qualification/node_boot/run.sh`);
    - `OTERYN_LOG=info` emits no per-command line;
    - the `registered` and `readiness` lines keep the F16 health tokens.
  - Checks: `cargo test -p oteryn-game-server` and `game-gate`.
- **OBS-CORR-2** (`traceparent`). Depends on OBS-LOG-1.
  - Builds R4. Owned path: the Platform HTTP client module under `apps/game-server/src/native_admission_source/`.
  - Tests:
    - the header has 55 characters and its trace-id equals the trace hex;
    - the span id is random and never all zero;
    - no header is taken from a response;
    - a mock server sees the header on all three calls.
- **OBS-METRICS-3** (node metrics). Depends on OBS-LOG-1.
  - Builds R8 and R9. Owned paths: `Cargo.toml`, `apps/game-server/Cargo.toml`, `Cargo.lock`, a new `apps/game-server/src/node/metrics.rs`, the metric call sites, and `node/config.rs` (`[metrics]`).
  - Tests:
    - a missing key opens no listener;
    - a non-loopback `listen` is refused at config parse;
    - a bind failure gives a 2xxx code;
    - the scrape label keys match an allowlist exactly, with no unknown label;
    - the label values are registered codes or enum names;
    - a recorder failure does not change an admission outcome.
  - Checks: `cargo deny check` (licences and sources) and `game-gate`.
- **OBS-DEPLOY-4** (log sink, supervisor logs, alerts, dashboard). Depends on #1874 merged and OBS-METRICS-3.
  - Builds R6, R7 and R10. Owned paths: a new `apps/game-server/src/node/log_sink.rs`, the `log-sink` dispatch in `apps/game-server/src/main.rs`, `deploy/synology-game/supervisor.sh`, `deploy/synology-game/node.toml.template`, `deploy/synology-game/README.md`, and a new `deploy/observability/` (Prometheus, Grafana and email alerting on the NAS, owner ruling 1a).
  - Tests:
    - with a 4 KiB segment size, the sink rotates at the size bound and never splits a line; with an injected clock it rotates at 24 h with no new input size;
    - with an injected clock, the hourly prune removes only closed segments whose newest line is older than 14 days, never the active segment, and the lifecycle file of the running incarnation survives;
    - with an injected clock, the hourly prune drops every lifecycle file line older than 14 days and writes one `lifecycle_state` line for each dropped latest `registered`, `awaiting_assignment` or `readiness` line, and after 20 days `health` and the `awaiting_assignment` revision read still work;
    - over the 1 GiB cap (scaled down in the test), the oldest closed segments go first and one coded `warn` line is written;
    - a node that runs past the retention with a steady line rate keeps the directory within its bound and keeps no line older than 15 days and 1 hour, and no lifecycle file line older than 14 days and 1 hour;
    - a shell test that two restarts keep both earlier incarnations' logs, and that `health` and the `awaiting_assignment` revision read still work after several rotations;
    - a killed sink makes `health` fail and does not change an admission outcome;
    - `promtool check rules` and `promtool test rules` on fixture series;
    - the dashboard JSON parses.
- **OBS-BUNDLE-5** (`diagnose --bundle`). Depends on ERR-DIAG-4 and OBS-LOG-1.
  - Builds R12. Owned paths: `apps/game-server/src/ops_diagnose.rs` and `apps/game-server/src/bin/oteryn-game-ops.rs`.
  - Tests:
    - a report finds the trace and its `parent`, and the boot-trace lines of that incarnation;
    - a forged `trace=` in `detail` is not matched;
    - a non-loopback URL is refused;
    - an unreachable URL still writes the bundle with `metrics.prom` stating the reason;
    - an injected write failure leaves no partial directory;
    - MANIFEST hashes verify;
    - no line in the bundle holds an ANL-01 §18 id (fixture scan).

### Owner questions

1. Where metrics are scraped and alerts sent; this is production hosting and possible spend.
   - (a) Prometheus and Grafana as host-network containers on the alpha NAS. There is no spend, and alerts go to the owner's email.
   - (b) No scraper in the alpha. `/metrics` feeds only the debug bundle, and outages are found by deploy health and tester reports.
   - (c) A hosted metrics service, which is spend.

   Recommend (a): a node crash otherwise goes unseen until a tester complains.

   **Owner ruling 2026-10-06: a.** R10 and OBS-DEPLOY-4 follow it.
2. Alpha retention; this is privacy scope. Logs hold the trace, `attempt_ref` and scope, and Platform can link `attempt_ref` to an account.
   - (a) CANDIDATE: logs 14 days, metrics 30 days, bundles until the issue closes.
   - (b) Logs 30 days, metrics 90 days.
   - (c) Delete everything at the end of each alpha phase.

   Recommend (a).

   **Owner ruling 2026-10-06: a.** R6 and R7 follow it.

### Rejected options

- `tracing`, `tracing-subscriber` or OpenTelemetry spans and an exporter: they rewrite every call site, and three hops are joined by one UUID already (ERR-CODES §3).
- JSON log lines or Loki/Vector shipping in the alpha: there is one host and a handful of testers, the key=value line is greppable, and `EXP-OBS-01` is Track 4.
- The `prometheus` crate instead of `metrics`: it is a heavier registry API, and `metrics` is the facade the ecosystem shares.
- `/metrics` on the control socket: Prometheus cannot scrape a Unix socket.
- `/metrics` on a public interface or with auth: loopback needs neither.
- Per-transaction `SET application_name` with the trace: it adds a statement on the durability path and puts correlation data in DB logs.
- `postgres_exporter` in the alpha: node-side DB latency and errors cover the failure modes. It can be added to the owner ruling 1a NAS stack without a decision.
- A new `oteryn-debug-bundle` tool: ERR-DIAG-4 already owns log reading.
- A `trace` or `attempt` metric label: it has unbounded cardinality (ANL-01 §18).
- Off-host collection of node crash evidence now: a persistent NAS volume meets the DISCONNECT §5 non-ephemeral rule for the alpha.
- Pruning only at start, or `logrotate` with `copytruncate`: the first leaves a long-running incarnation unbounded; the second loses the lines written between copy and truncate.
- An upstream pipe logger (`svlogd`, `s6-log`, `rotatelogs`) on the host: none is guaranteed on the NAS, and installing one needs root. The sink is a subcommand of the binary already deployed; if the host later ships one of them, OBS-DEPLOY-4 may use it instead under the same bounds and tests.

### Open unknowns

- U1: alpha log volume per tester-hour at `info`. It is measured in the first alpha week and fixes the R6 segment size and directory cap.
- U2: whether #1868 keeps the exact `registered`, `node_id=` and `readiness ready=true` tokens that the #1874 health check greps (F22). OBS-LOG-1 locks them with a test.
- U3: whether Synology Container Manager host networking can reach a loopback listener (owner ruling 1a). Fallback: bind to the host's LAN address behind its firewall. That needs an amendment to R8.
- U4: whether Platform accepts the R11 proposal. The Game side works without it, and Gateway↔node joins stay node-side through `attempt`.
- U5: the R10 thresholds and the PERF-01 latency budgets. No value is accepted here.
- Before freeze:
  - Serialization: there is one locked line write, and metrics are atomic counters that are not on the authority path.
  - Restart: metrics reset (ADR-0006), each incarnation has its own segments and lifecycle file, and `parent` is in memory only.
  - Long-running node: the sink rotates and prunes while it runs, so the R6 bounds hold without a restart.
  - Typed refs: `trace`, `parent` and `attempt` are UUIDv7.
  - Peer gating: there is no wire change, and `traceparent` is ignorable. Ops and server ship in one bundle, so the parser and writer match.
  - Multi-component commit: none, because the bundle is published by an atomic rename.

## 2. Time, scheduling and performance

### Facts

Classes: PROVEN (read in an accepted source or in code on `main`), DERIVED (follows from PROVEN
facts), UNKNOWN, CONFLICT.

Tick and scheduling
- F1 PROVEN. No universal fixed tick. Each owner schedules its own semantic deadlines.
  FND-03 §8.4. SIM-DETERMINISM-01 §15 and its deferral list (line 419) leave a global tick rate undecided.
- F2 PROVEN. FND-03 names three time domains: wall clock, process-local monotonic time, and
  authoritative execution order. A monotonic instant is never persisted (§8.1-8.3, §9).
- F3 PROVEN. Timers have a scheduling key (scope, ownership generation, entity generation,
  monotonic deadline, deterministic order) and a catch-up class: `DEADLINE_STATE`,
  `RUN_EACH_BOUNDED`, `COALESCE_ELAPSED`, `SKIP_TO_LATEST`, `EXPIRE_OR_CANCEL`. FND-03 §10.1-10.5.
  Tests use deterministic clocks (§11).
- F4 PROVEN. Code has a Channel owner timer lane with an `OwnerClock` trait and a
  `VirtualOwnerClock` for tests. It is fenced by `ScopeRuntimeFence` and `RuntimeWorkStamp`.
  `apps/game-server/src/foundation/owner_timer.rs`.
- F5 PROVEN. The gameplay path has its own owner clock. It uses `clock_origin = Instant::now()` per
  Channel runtime and `owner_now() = clock_origin.elapsed()`
  (`gameplay_transport/mod.rs:486`, `monk_save.rs:40-45`). Each connection also runs
  `tokio::time::interval_at` timers with `MissedTickBehavior::Delay`
  (`connection.rs:1366-1405`), for example combat refresh at 250 ms (`attack.rs:55`) and quest
  log refresh at 1 s. Creature think runs at 1,000 ms (`ai_think.rs:65`, D115), with at most
  1,024 thinks per 50,000 us window (CREATUREAI0-RL-05).
- F6 CONFLICT (minor). There are two `SemanticTimeMicros` types:
  `foundation/owner_timer.rs` and `crates/simulation-determinism/src/lib.rs:366`. The gameplay
  path does not read through `OwnerClock`.
- F7 PROVEN. ADR-0009 §13 requires PERF-01 to define the tick or scheduling model. F1 already
  answers it: there is no tick, and the scheduling model is per-owner deadlines.

Clocks in accepted and candidate rules
- F8 PROVEN. Admission decision time is the database clock:
  `SELECT ... clock_timestamp()` (`durability/fresh_admission.rs:1812`, `:2694`). FND04B-SAME-SESSION-GRACE-S
  fixes the grace deadline at decision time + 60 s, never extended (RESOURCE_LIMITS_REGISTRY.json).
- F9 PROVEN (CANDIDATE source). D3 binds gameplay-bounding timestamps to the database clock
  (`clock_timestamp()` at commit), never a client clock or a runtime wall clock
  (D3 corpse decay decision, lines 36 and 121). OFFLINE-0 repeats this for offline time
  (lines 67-68 and 222). TIMED-ITEM-0B puts a lit item on the ground under a durable `deadline_at`
  in database time, and stores `remaining_ms` for a worn or held item
  (TIMED-ITEM-0B lines 104-127). All three are CANDIDATE documents.
- F10 PROVEN. Monk serene state persists `serene_forced_remaining_micros`, which is a remaining
  duration (`monk_save.rs`).
- F11 PROVEN. Premium time is the trusted node clock: `ntp_adjtime` read only,
  `uncertainty = maxerror`, fail closed when unsynchronized, with `MAX_CLOCK_SKEW_US` = 5 s
  (PREMIUM-ACTIVATION-0 §1.1; `premium/mod.rs:51`). Oteryn writes no time daemon.
- F12 PROVEN (CANDIDATE source). The World clock is
  `floor((t − world_clock_epoch_utc_ms) / 2,500 ms) mod 1,440`. Here `t` is the recorded
  normalized UTC fact, and the epoch is durable and immutable after activation. Channels agree
  within `WORLDINT0-RL-19` = 1,000 ms and raise an alarm above that (WORLD-INTERACTION-0 §11.1-11.2,
  lines 596 and 648). `WORLDINT0-RL-19` is not in RESOURCE_LIMITS_REGISTRY.json (0 matches).
- F13 PROVEN. UUIDv7 time order is never authority, freshness or lease evidence
  (FND-ID-01 contract line 526). Platform must not issue on a favorable wall-clock timestamp
  (FND-04 pre-admission refinement line 350). Fences are generations and revisions (FND-04 line 80).
- F14 PROVEN. The diagnostic `ts` is UTC wall-clock time and "orders nothing"
  (ARCH-ERROR-CODES-0, line 237).
- F15 UNKNOWN. No document requires NTP or chrony on GameNode or database hosts beyond the
  Premium fail-closed rule (F11). No code measures skew between the node and the database.

Capacity and performance
- F16 PROVEN. ADR-0009 §6 defines `max_players_per_channel`, `_game_node` and `_world`. Each is
  set by PERF-01 on named hardware with exact artifacts and at least 30% headroom. A claim records
  p50/p95/p99, queue age and the first violated objective. §6 also names ten workload classes.
- F17 PROVEN. PERF-01 is PLANNED/NOT_STARTED (gap register §25, lines 457-473).
- F18 PROVEN (CANDIDATE source). D128 sets a design target of 500 concurrent players per
  Channel, to be verified by PERF measurements (owner batch D118-D128, line 32). This is a target,
  not an accepted capacity.
- F19 PROVEN. Node config holds one `world_id` and one `channel_id`, and the node accepts at
  most 256 connections and 64 handshake units (OPS-NODE-BOOT-01 D1, line 58; `node/config.rs:78`).
- F20 DERIVED. One alpha GameNode therefore serves one Channel, and at most 256 players.
  That is below D128's 500.
- F21 PROVEN. ADR-0009 §2 (line 47) says one channel per GameNode is "not the default
  architectural requirement". ADR-0015 says one process "may host multiple ChannelRuntimes". Both
  allow several Channels per GameNode. The current build hosts one (F19).
- F22 PROVEN. Measurement precedent: MAP-VIEWPORT-MEASURE-1 (#1873) used a release
  `#[ignore]` test (`world_map_tests.rs`). The evidence names the command, the commit, the
  machine (4 vCPU) and fixed seeds, and measures CPU with `/proc/thread-self/schedstat`. It made one
  run and no repeat for variance. Map views cost 0.41-0.44 of a core at 500 players, against the
  0.20 architect default (ARCH-MAP-VIEWPORT-BUDGET-V1 §1.2).
- F23 PROVEN. `tools/synthetic-client-harness` exists. It is synthetic-only, has a `--live`
  mode, and links `protocol-oteryn`. ADR-0007 puts wide concurrency and soak campaigns in
  scheduled runs (line 285).
- F24 PROVEN. No workspace crate depends on criterion, iai-callgrind or divan. No workflow runs a
  performance benchmark. `rust-cache-pilot.yml` benchmarks build caching only.

### Rulings

1. **No tick.** The alpha keeps FND-03 §8.4: there is no global tick. A Channel is scheduled by
   its owner work queue and its owner timer lane (F4). Connection pacing timers stay
   per connection (F5). This ruling answers ADR-0009 §13 "tick/scheduling". PERF-01 measures owner
   service time and queue age, not tick overrun.
2. **Every periodic timer is registered.** A gameplay timer with a fixed cadence has a
   RESOURCE_LIMITS_REGISTRY row (unit `milliseconds`). Its notes name the FND-03 §10.5 catch-up
   class. Existing rows already do this (CREATUREAI0-RL-06, ATTACK0-RL-03). Missed ticks never
   burst: `MissedTickBehavior::Delay` and `SKIP_TO_LATEST` remain the defaults (F5).
3. **One owner clock per Channel.** Gameplay rules read time only through `OwnerClock`. In
   production it is backed by the Channel's monotonic `clock_origin`. Tests use
   `VirtualOwnerClock`. Cooldowns, attack and think cadence, conditions, in-fight, regeneration
   and overlays use this clock. Gameplay code never calls `SystemTime::now()`. The two
   `SemanticTimeMicros` types (F6) become one type when TIME-CLOCK-1 touches them, and the change
   ends there.
4. **Which clock decides what.** Every rule uses exactly one clock:
   - owner monotonic clock: volatile timers and online-only durations (ruling 3);
   - database `clock_timestamp()` at commit: every durable deadline, every elapsed-offline
     fact, and every admission, grace and decay time (F8, F9);
   - trusted node clock (`TrustedClock`, `ntp_adjtime`): entitlement windows (F11);
   - recorded UTC fact from the trusted node clock: the World clock (F12). The fact is read once
     per owner input that needs it and recorded with that input (SIM-DETERMINISM-01 §15). When
     `TrustedClock` returns `None`, the World clock still uses the raw realtime reading, and the
     node raises the skew alarm (ruling 7). Light is not an entitlement, so it does not fail
     closed.
   A new decision names its clock from this list. A rule that needs another clock needs its own
   decision.
5. **What is persisted.** A monotonic instant is never persisted (FND-03 §9). Durable timed state
   takes one of two forms:
   - **remaining duration**, for time that counts only while the owner runs, such as worn timed
     items and monk serene (F9, F10);
   - **absolute deadline in database time**, for time that runs in the real world, such as ground
     items, corpses, grace and offline training.
   Each timed decision states which form it uses and whether offline time counts. Those that do
   not yet say must do so before acceptance.
6. **Restart and reconnect.**
   - *Reconnect within grace:* the actor stays in its Channel (FND-04B), so owner timers keep
     running unchanged.
   - *Restart:* volatile timers are cleared, as for overlays in WORLD-INTERACTION-0.
     A remaining duration is loaded from the last checkpoint and rescheduled at `owner_now() + remaining`.
     An absolute deadline is compared with database time, so a deadline that passed while the
     node was down fires once, under its catch-up class.
   - The family's decision says who gets the time lost between the last checkpoint and a
     crash; TIMED-ITEM-0B §6 already does.
   - A restarted Channel gets a new `clock_origin`. Owner instants are never compared across
     process incarnations.
7. **Clock sync and skew.** Every GameNode host and the database host run a host time daemon.
   chrony is recommended. systemd-timesyncd and ntpd are accepted, because `ntp_adjtime` reads
   all three (F11). The node checks two things, at boot and on each maintenance poll (`serve.rs`):
   - local sync state and `maxerror` through `TrustedClock`;
   - node-to-database offset: `clock_timestamp()` minus node realtime, corrected by half the
     round trip.
   Above the bound, the node emits one diagnostic line with a new registered 2xxx code
   (ARCH-ERROR-CODES-0 §1.2 assigns the number). It does not refuse readiness. Every rule that
   needs trusted wall time already fails closed or uses database time (ruling 4). The bound is
   `WORLDINT0-RL-19`, CANDIDATE 1,000 ms. TIME-SKEW-1 registers it, and its value is accepted
   when WORLD-INTERACTION-0 is. The 5 s Premium bound stays unchanged.
8. **Fencing never depends on cross-host time.** Session generation, lease generation and
   ownership generation are integers compared inside a database transaction (F13). Expiry
   compares database time with database time. A node never compares its own clock with a
   timestamp written by another host to decide authority. A skew alarm therefore has no fencing
   consequence.
9. **Players per Channel is measured, not chosen.** In the current build a node holds one
   `world_id` and one `channel_id` and accepts at most 256 connections (F19, F20). So per node
   equals per Channel until multi-channel hosting is built: `max_players_per_game_node` equals
   `max_players_per_channel`. `max_players_per_world` is never derived by multiplying the
   per-Channel value: the Channels of one World share PostgreSQL, its I/O and the world services,
   which a one-Channel run does not load (ADR-0009 §5–6). It is measured by its own World run
   (ARCH-ALPHA-CAPACITY-0, ruling 10), and a second Channel in a World adds no claimed capacity.
   ADR-0009 §2 and ADR-0015 still allow several Channels per GameNode (F21). When a
   build hosts more than one, the three ADR-0009 §6 limits are measured separately by PERF-01. This
   ruling does not amend either ADR. Until a value is measured, the alpha ceiling is the registered
   256 connections (F19). D128's 500 per Channel is the target that the measurement is checked
   against (F18). No per-cycle microsecond budget and no service objective is accepted here (ruling 10).
10. **Capacity is decided by ARCH-ALPHA-CAPACITY-0.** The load method, the objectives and
    their acceptance move to that separate architecture decision packet. Until it merges, no
    `max_players_per_channel`, `max_players_per_game_node` or `max_players_per_world` value is
    published or registered. The admission limit is unpublished, and alpha runs at the
    registered 256 connection ceiling (F19), which is not an accepted capacity. That packet must:
    - fix every pass and fail objective, and have it accepted in review, before any capacity
      sweep: owner service time p99, queue age, memory growth, network, and Channel and World
      persistence pressure. An objective that changes after a sweep reruns the sweep; no
      threshold is set after a result;
    - record every ADR-0009 §6 claim field in both the Channel run and the World run: hardware,
      artifacts, behaviour model, p50/p95/p99 latency, queue age and rejections, CPU, memory,
      network and persistence pressure (PostgreSQL transaction p99, pool wait, WAL and I/O),
      and the first violated objective;
    - include a soak of a fixed duration at the candidate value in both runs, with failure
      objectives for memory growth and persistence backlog (ADR-0009 §6 test class 10);
    - keep ruling 9: per node equals per Channel in this build, and the World limit comes from
      its own run of at least two Channels on one PostgreSQL, never by multiplying;
    - apply the 30% headroom to the highest N that passes in every repeat, never to the first
      failing N;
    - carry owner ruling 1a (§2 Q1): a value below D128's 500 is accepted for alpha and the gap is
      logged in gap register §25.
11. **How a value is accepted.** A capacity value becomes a RESOURCE_LIMITS_REGISTRY row only
    from ARCH-ALPHA-CAPACITY-0's accepted evidence, in a PR that passes the normal independent
    review, with notes citing the evidence file. A hardware change or a regression gate failure
    reopens the row.
12. **CI benchmark gate (PERF-01 CI part).**
    - *Benchmarks:* instruction counts with `iai-callgrind`, an upstream crate on valgrind. The
      first set has three benches: the viewport snapshot, the viewport delta, and one seeded
      owner cycle of creature think plus a step with visibility.
    - *Where it runs:* a path-selected merge-gate job on `ubuntu-24.04`. It builds the merge
      base and the head in the same job and compares the two counts.
    - *What selects it:* the set is derived from the code, not listed by hand. The measured
      paths cross most of the crate: the viewport plan calls `crate::map::view`
      (`gameplay_transport/world_map.rs:36`, `:291-339`); creature think uses
      `crate::movement::step_cardinal` and `crate::foundation::owner_timer`
      (`ai_think.rs:47`, `:54`); the owner cycle reads `owner_timer::SemanticTimeMicros`
      (`gameplay_transport/monster_ai_cycle.rs:15`); movement and the transport use
      `crate::world_runtime` (`movement/speed.rs:13`, `gameplay_transport/mod.rs:363`). A file
      list would miss the next such edge. So the job runs when a changed path is owned by
      `oteryn-game-server` (including `apps/game-server/benches/`) or by any workspace package in
      its local dependency closure (today `crates/error-codes`, `crates/foundation`,
      `crates/protocol-oteryn`, `crates/simulation-determinism`, `crates/world-bundle`;
      `apps/game-server/Cargo.toml`), or is a Cargo build input, `vendor/**` (the patched
      `tokio` and `sqlx` crates) or the workflow itself. The closure is computed by
      `tools/repository/classify_pr_test_lanes.py` from `cargo metadata`, which already builds the
      local reverse-dependency graph (`graph`, lines 231-268) and the build-input set (lines
      30-33). An incomplete path enumeration or a classifier error runs the job. The benches read
      only fixtures under `apps/game-server/benches/`, so no `content/` change can move a count
      unseen.
    - *Noisy runners:* instruction counts do not depend on runner speed, which is why they gate.
      Wall-clock benches never gate a PR. They run as `#[ignore]` release tests and in the
      harness, on the reference class, nightly or on demand (ADR-0007 line 285).
    - *Threshold:* CANDIDATE. PERF-CI-1 runs the benches 20 times on `main`, records the spread,
      and proposes the threshold.
    - *Gate status:* owner ruling 2026-10-06 (owner question 2, option a): the job reports for its
      first 20 merges and becomes required after PERF-CI-1 sets the threshold. Making it a
      required check changes repository protection, so the control plane routes that change.

### Contract amendments

- A1. `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md` §8. Add §8.5 "Clock sources
  (ARCH-ALPHA-OPS-0 §2 ruling 4)" with the four-clock list of ruling 4. Add one sentence to §9: "A
  durable timer stores a remaining duration or an absolute database-time deadline, and its
  decision names which."
- A2. `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`. Add a row `WORLDINT0-RL-19`: unit
  `milliseconds`, `hard_maximum` 1000, failure category per the registry vocabulary, notes
  "CANDIDATE with WORLD-INTERACTION-0; alarm only". No capacity row is added here;
  ARCH-ALPHA-CAPACITY-0 owns the `PERF01-PLAYERS-PER-*` rows (rulings 10-11).
- A3. Node deployment runbook (the OPS-NODE-BOOT-01 operator section). Add: "Run a host time
  daemon (chrony recommended) on every GameNode and database host. The node alarms above
  `WORLDINT0-RL-19`."
- A4. ARCHITECTURE_ANALYSIS_GAP_REGISTER.md §25. Mark tick/scheduling as decided (ruling 1).
  Mark the CI gate as decided, with its threshold pending PERF-CI-1, and the capacity method as
  pending ARCH-ALPHA-CAPACITY-0.

### Packets

- **TIME-CLOCK-1.** Gameplay reads time through `OwnerClock` and uses one `SemanticTimeMicros`
  type. Owned: `apps/game-server/src/foundation/owner_timer.rs`,
  `apps/game-server/src/gameplay_transport/mod.rs`, `.../monk_save.rs`,
  `crates/simulation-determinism/src/lib.rs`. Tests: one virtual-clock test per migrated
  family; a grep check that `SystemTime::now` stays out of `gameplay_transport/**`.
  Depends on nothing.
- **TIME-SKEW-1.** Boot and maintenance-poll skew check (ruling 7), the 2xxx code, row A2 and
  runbook A3. Owned: `apps/game-server/src/node/serve.rs`, `apps/game-server/src/node/clock_check.rs`
  (new), `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json`,
  `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`. Tests: a fixed `TrustedClock` with unsync,
  over-bound and in-bound cases, and a PostgreSQL offset case with an injected node clock.
  Depends on PREMIUM-ACTIVATION-0's `TrustedClock`.
- **ARCH-ALPHA-CAPACITY-0.** The capacity decision of ruling 10, an architecture decision
  packet routed by the control plane. It names the load, acceptance and World-run implementation
  packets. Its load packet depends on TIME-CLOCK-1 for queue-age stamps.
- **PERF-CI-1.** iai-callgrind benches and the path-selected job (ruling 12). Owned:
  `apps/game-server/benches/` (new), `apps/game-server/Cargo.toml`, `Cargo.toml` (dev-dependency),
  `.github/workflows/merge-gate.yml`, `tools/repository/classify_pr_test_lanes.py` and its test,
  `docs/agents/evidence/PERF-CI-1-noise.md` (new). Tests: the job fails on a deliberate
  synthetic regression in a test branch; the classifier selects the job for a change confined to
  each of `apps/game-server/src/map/view.rs`, `movement.rs`, `world_runtime.rs`,
  `foundation/owner_timer.rs`, `ai_think.rs`, a file under each dependency crate, `Cargo.lock`
  and `vendor/`, and for an incomplete enumeration; it skips the job for a change confined to
  `docs/` or `apps/client/`. Dependency review covers the new dev-dependency. Depends on nothing.

### Owner questions

1. Context: alpha's ceiling is 256 connections, and the D128 target is 500.
   a) Accept the measured value for alpha even below 500, and log the gap (recommended).
   b) Block the representative-load alpha claim until 500 is measured.
   **Owner ruling 2026-10-06: a.** It carries into ARCH-ALPHA-CAPACITY-0 (ruling 10).
2. Context: making a new check required changes repository protection.
   a) The job reports first and becomes required after PERF-CI-1 sets the threshold (recommended).
   b) Required from the start.
   c) Nightly only, never a PR check.
   **Owner ruling 2026-10-06: a.** Ruling 12 follows it.

### Rejected options

- A fixed global tick, for example 50 ms. FND-03 §8.4 rejects it, and per-owner deadlines
  already work.
- Persisting monotonic instants, or node wall-clock deadlines. Both break across restart and
  across hosts (FND-03 §9, D3).
- Refusing readiness on a skew alarm. Nothing that needs trusted time is left unprotected, so a
  clock fault would become an outage for no gain.
- Wall-clock criterion benches as a PR gate. Shared runners are too noisy. The existing
  `#[ignore]` release-test method already gives p99 and CPU, so criterion is not added now.
- An external load tool (k6, Locust, goose). `protocol-oteryn` is a custom binary protocol over
  TLS, and the repository already has a Rust client stack in the synthetic harness.
- Accepting D128's 500 or the 256 connection maximum as measured capacity. Neither has evidence.
- Keeping the capacity method in this decision with candidate objectives. Objectives set after a
  sweep fit the threshold to the result, and the ADR-0009 §6 memory, network, persistence and soak
  evidence needs design this decision does not have.
- Writing an NTP client or time daemon. The host daemon is the upstream solution.

### Open unknowns

- U1. The objective values for owner service time, queue age, memory growth, network and
  persistence pressure, and the soak duration. ARCH-ALPHA-CAPACITY-0 fixes them before any sweep.
- U2. Whether the GitHub `ubuntu-24.04` runner matches the 4 vCPU reference class for this
  repository's plan. If it does not, reference runs go to a named machine, which is a spend
  question for the control plane.
- U3. The iai-callgrind threshold (PERF-CI-1).
- U4. Status mismatches. FND-03's implementation column says IMPLEMENTED in the programme status
  register (line 51) and NOT_STARTED in the decision register (line 34). SIM-DETERMINISM-01's
  header says PROPOSED and the register says ACCEPTED.
- U5. Map views take 0.41-0.44 of a core against the 0.20 default (F22). This consumes the
  Channel's budget before any other workload. ARCH-MAP-VIEWPORT-BUDGET-V1 owns the fix, and the
  ARCH-ALPHA-CAPACITY-0 load run must include it.

## 3. Data continuity

Scope: schema migration across releases, deploy shape, Character and content versioning, backups
and PITR, the restore runbook, and Platform state that a Game restore does not restore.
This section builds on `docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md`
(registry blocks 3000–3999 durability and 6000–6999 ops, the key=value line, `trace`,
`oteryn-game-ops diagnose`). It does not change any rule there.

### Facts

Migrations and the schema gate
- F1 PROVEN. The tool is upstream sqlx 0.9.0 `migrate!`. The vendored sqlx patch changes only the pool/TLS budget. `Cargo.toml` (`sqlx = "=0.9.0"`, `[patch.crates-io]`); `docs/superpowers/plans/2026-09-06-sqlx-driver-budget.md`.
- F2 PROVEN. Only `oteryn-game-migrate` runs DDL, with its own credential `OTERYN_GAME_MIGRATION_DATABASE_URL`. `apps/game-server/src/durability/schema.rs` (`MigrationExecutor`); `apps/game-server/src/bin/oteryn-game-migrate.rs`.
- F3 PROVEN. Readiness needs the DB ledger to equal the embedded set exactly: same count, versions, checksums, all successful. Otherwise the node reports `SchemaIncompatible`. `schema.rs` `inspect_executor`; `apps/game-server/src/durability/db.rs` ~958–971.
- F4 DERIVED from F3. The compatibility window between a binary and the schema has zero width. An old binary is not ready after any new migration. A new binary is not ready before it. All nodes on one `oteryn_game` database therefore run the same migration set.
- F5 PROVEN. sqlx refuses a dirty, changed-checksum or unknown applied version. It does not refuse a new version that is lower than an applied one; it applies it out of order. sqlx-core 0.9.0 `migrate/migrator.rs` (`ignore_missing=false`, no order check).
- F6 CONFLICT (hazard). `apps/game-server/migrations/` has 68 files, 0001–0079, with gaps 0027, 0037, 0053, 0061–0067 and 0078. 0079 is merged (#1850). A later 0078 would apply out of order on every database that already has 0079. No CI check prevents it: `tools/repository/classify_pr_test_lanes.py:81` only routes the path.
- F7 PROVEN. Accepted evolution rule: EXPAND → MIGRATE/BACKFILL → VALIDATE → CUT OVER → CONTRACT. No simplistic DOWN scripts. Released history is immutable. Tests cover each mixed-version window "required by the release contract". `docs/architecture/DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` §8 (rule 6).
- F8 PROVEN. No live migration of an active channel. Relocation runs: stop admissions → drain → checkpoint → fence old generation → start → validate → ready → fresh Game Sessions. `ADR-0009` §8; failure path §9.

Character and content versioning
- F9 PROVEN. Values that cannot be recomputed keep the authoritative value plus its interpretation revision. Skill state is definition-keyed and revisioned. `docs/architecture/DUR-02_PROFILE_NEUTRAL_CHARACTER_SCHEMA_DECISION_PACKET.md` §4.5 (lines ~226–232).
- F10 PROVEN. Persisted rows refer to content by text key plus revision, not by numeric id: items use `definition_family`, `definition_production_key`, `definition_revision_ref`, `map_revision`, `content_revision` (`migrations/0010_item_mint_ground.sql:15–38`). Proficiency uses `definition_key` plus `definition_revision` (`0032`). A search of all migrations found no persisted numeric content id.
- F11 PROVEN. Canonical identity is a stable `ContentKey`. Compiled numeric ids live only inside one artifact. Aliases are explicit and acyclic. A content change is classed `COMPATIBLE_NO_MIGRATION`, `READ_COMPATIBLE_NORMALIZE`, `EXPLICIT_DATA_MIGRATION`, `INCOMPATIBLE_REQUIRES_PRODUCT_DECISION` or `REMOVED_WITH_EXPLICIT_POLICY`. A value-bearing removal never silently drops value. Bundles are immutable and content-addressed. `docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` §2 (line 29–31), §9, §11 (148–150), §12.
- F12 CONFLICT (status only). The DUR-04 file header says `PROPOSED / IN_REVIEW`. `docs/architecture/OTERYN_V2_POST_WAVE_A_F_RECONCILIATION_20260816.md:39` says DUR-01..04 are accepted "where recorded in current status".
- F13 PROVEN. Evidence receipts are immutable. A changed input gets a successor receipt with `supersedes` and provenance, and v1 stays untouched. Commit `6f16b269e` (#1869 ITEM-MOD17-RECEIPT-REPIN-1), `docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v2.json`.
- F14 PROVEN. Registries that persisted rows point at are never deleted or renamed (FK RESTRICT); counters only rise. `migrations/0076_character_inbox.sql:11–16`.

Backup and restore baseline
- F15 PROVEN. The accepted PITR envelope requires base backup plus WAL, encrypted least-privilege backups, and the rule that HA is not a backup. Restore drills run on named artifacts. Validation covers the ledger, fences, receipts and journal. The service starts closed. A strictly newer fence lives outside the snapshot. Pre-restore authority cannot resurrect. Replay never resubmits gameplay. RPO/RTO, cadence, retention, provider and drill frequency are not decided. `DUR-02_PERSISTENCE_V1_OWNER_BASELINE.md` §7 (rule 5), §13.
- F16 PROVEN. `oteryn_game` and `oteryn_platform` are separate databases with separate backup policies and no cross-DB FKs. `ADR-0004` §2, §5, §8.
- F17 PROVEN. Before external alpha, RTO/RPO, PITR, restore drills, expand/contract and version-skew/rollback must be defined and proven. Kubernetes is not required. Incident compensation uses audited idempotent domain transactions, never manual DB edits. `docs/architecture/ARCHITECTURE_REVIEW_REFINEMENTS_2026-08-10.md` ~185–220.
- F18 PROVEN. Item restore validation: ledger, unique live ids, one location per item, receipt consistency, audit sets, a newer recovery fence, no remint. `DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md` §41. Item ids are UUIDv7 (`0010:13`), so a restore cannot reissue an id that was lost.

Restore fences (three exist, in three places)
- F19 PROVEN. Character recovery fence: an external file register `character-recovery-fence-v1.record` in `character.fence_directory`, advanced by CAS (`begin_recovery`), checked by every Character writer (`assert_recovery_fence`). `apps/game-server/src/character_recovery_fence.rs:11,174`; `apps/game-server/src/durability/character_authority.rs:319,390`.
- F20 PROVEN (gap). `oteryn-game-ops` exposes `character fresh-store` but no command that calls `begin_recovery` or `reconcile_character_recovery`. Only tests call `begin_recovery` (`tests/character_authority_postgres.rs:128`). No operator path exists to finish a restore.
- F21 CONFLICT (status only). The fence decision says "candidate … no implementation authority until independently reviewed". Its DB side and the file register are on main. `docs/architecture/reviews/OTERYN_CHARACTER_RESTORE_NONROLLBACK_FENCE_DECISION_2026-09-23.md` status line; `migrations/0005`, `0007`.
- F22 PROVEN. LCFA `projection_epoch`: the SQL raise `game_character_account_projection_resync(true)` exists (`0024:94`, `0028:55`). The contract requires an external high-water F and a raise strictly above it (`docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` §5). The production fence location is open (§10 U-LC2). A search of the publisher (`native_admission_source/account_characters.rs`) found no F file; the producer is not enabled (§11).
- F23 PROVEN. `assignment_epoch` (runtime status producer) is raised by an operator after a restore. Where it is stored and how it is raised is open (U-RS5). `docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §6 (Candidate).
- F24 PROVEN. Node registration revisions are "never reset or deleted" (`0003:15`). A NodeId is a fresh UUIDv7 per process incarnation (`0003:39–41`). DERIVED: a restore resets the high-water and marks registrations CURRENT that were CURRENT at the target time.

Platform-held state
- F25 PROVEN. Platform keeps coin balances and the purchase ledger. Delivery ownership, entitlement lifecycle and idempotent delivery are open under §32 PROD-ENTITLEMENTS-01. `docs/architecture/OTERYN_STORE_CATALOG_OWNER_DECISION_2026-09-28.md` §2, §3.
- F26 PROVEN. Game holds high-water marks of Platform `authority_revision` and `lifecycle_revision` (`0029`). A Platform restore needs no Game change: Premium fails closed until Platform issues higher revisions. `docs/architecture/reviews/OTERYN_GAME_PREMIUM_DELIVERY0_PREMIUM_EVIDENCE_TRANSPORT_DECISION_2026-09-30.md` §12.3 (line ~575). A Game restore is not covered there.
- F27 PROVEN. Cross-boundary Character workflows use operation identity, idempotent receipts, and authoritative reread after an ambiguous result. A Platform workflow row is not proof of a Game outcome. `ADR-0012` §9, §11; `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md` §8, §13 (Accepted).
- F28 PROVEN. Admission grants live at most 30 s with 5 s verifier skew (`docs/contracts/FND-04_PRE_ADMISSION_GRANT_PROFILE_V1.md` §7). Recovery grant nonces are stored in the Game DB (`0001:128`). A restore rolls their consumption back.
- F29 PROVEN. A restart must rebuild a current non-rollback floor from authoritative evidence before it authorizes anything, or it fails closed. `FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md` ~256; `FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md:370`.
- F30 CONFLICT (open PR #1874, not on `main`). The proposed Synology preproduction deploy (`deploy/synology-game/deploy-ops.sh`, `.github/workflows/synology-game-deploy.yml`) runs `oteryn-game-migrate` before it stops the node and takes no named backup. Ruling 3 requires stop, then a named backup, then migrate. The same PR places the Character fence at `BASE/fence-parent/fence`, outside the database volume, which matches ruling 8. Packet DATA-BACKUP-PITR-3 reorders the deploy.

Build identity
- F31 PROVEN (gap). Nothing in the database records which build ran when. A serving node registers one incarnation before it opens Character authority or serves (`src/node/serve.rs:4-7`, register call at `:1442`). Registration revisions come from a writer high-water taken `FOR UPDATE` (`0006:254-263`), and registrations are never deleted (`0003:15`, `0003:87`). A registration row has no build id (`0003:40-52`). The build id is kept only on Character operation receipts and audit rows (`0005:80`, `0005:102`), which a quiet build never writes. Releases without a migration share one ledger digest (ruling 3), so the digest alone cannot name the build.
- F32 PROVEN. The only literal seed row in the migrations on `main` is one wheel ruleset revision (`0070:127`); no migration inserts a `{family, key, revision}` literal, so definition references reach rows only through the server's writes (for example `0049:15`, `0049:94`). DUR-04 makes an unresolved reference a compile-time failure and incompatible artifact pairs fail closed (`DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` §7, line 102).

### Rulings

1. **One tool, one ledger.** sqlx `migrate!` stays the only migration tool. Releases add files and never edit or delete one. A new migration version must be greater than the highest version on `main`. The gaps up to 0079 stay gaps forever. (F1, F5, F6, F7)
2. **A release is one schema version.** The exact-ledger gate (F3) stays. Alpha supports no mixed-version window. This satisfies DUR-02 rule 6, which tests only the windows a release contract declares. A rolling N/N-1 window needs a later decision on a prefix-plus-declared-expand gate.
3. **Deploy shape.** A release with no new migration that passes ruling 5's bundle check rolls channel by channel, using the ADR-0009 §8 sequence for each channel. During the roll two builds share the database, so such a release must read every row its predecessor writes and write nothing its predecessor cannot read; a release that cannot meet this ships stop-the-world. A release with a new migration, or one that fails the bundle check, is stop-the-world for every node on that `oteryn_game` database. The order: close admission on every channel; drain; checkpoint; stop all nodes; take a named backup (ruling 7) and wait until pgBackRest reports it complete, its WAL archived (`archive-check=y`, the upstream default); run `oteryn-game-migrate`; record the build activation (ruling 9 step 3) as `PENDING`; run any audited domain migration of ruling 5 to completion; run ruling 5's live check on this final database; record the `ACTIVE` activation only after that check passes; start the new binaries; validate; open. No check gates a state that a later step still changes, and no node of the new build starts before its `ACTIVE` row commits. A crash anywhere in this order leaves every node stopped; the rerun deploy holds the ops lock again and resumes from the database itself: a ledger that already holds the new migration means the named backup completed, and a newest activation row that is `PENDING` for this build means any domain migration reruns, idempotent, and the live check runs again, before the `ACTIVE` row. Every stop-the-world release writes `PENDING` first, with or without a domain migration, so its `ACTIVE` row never commits before its final live check passes; a crash or a PITR between the two leaves a `PENDING` newest row, which nodes and a restore refuse. "Stop all nodes" means the supervisor stops them and keeps them stopped, and the deploy verifies that no node process is alive before the next step, as in ruling 9 step 1. A rolling release records its `ACTIVE` activation before its first node starts. Rollback after a migration is never a DOWN script. It is roll-forward, or a restore to the named pre-migration backup (ruling 9). Rolling back a release without a migration is a deploy of the older build under these same rules, ruling 5's check included. A release that shipped stop-the-world because its rows are unreadable to its predecessor cannot be rolled back by a deploy; like a migration, it rolls forward or restores to its named backup.
   **Ops lock.** A deploy (its named backup included) and `oteryn-game-ops restore` each hold an exclusive, non-blocking `flock` on `ops.lock` in the restore fence directory (ruling 8, on the node host) for their whole run; every other `oteryn-game-ops` write command holds it shared. A held lock is a refusal with a 6xxx code, never a wait. The kernel drops the lock when the process dies, so a crash leaves no stale lock. Two operators, or a deploy and a restore, therefore never interleave.
   **Lock handoff to child commands.** A holder of the exclusive lock runs some steps as child `oteryn-game-ops` commands: the deploy runs `release activate`, ruling 5's checks and the audited domain migration, and the restore runs the selected build's `oteryn-game-ops` (ruling 9 step 3). The child must neither wait on its parent's lock nor weaken it, so it inherits it. The parent passes its open `ops.lock` descriptor to the child without close-on-exec and puts its number in `OTERYN_OPS_LOCK_FD`. When that variable is set, the child never opens `ops.lock` itself and never takes a shared lock. Only a command that holds the lock exclusively sets the variable. It first checks with `fstat` that the descriptor is the `ops.lock` of its own fence directory (same device and inode as a `stat` of that path). It then calls `flock(LOCK_EX | LOCK_NB)` on the inherited descriptor. `flock` locks belong to the open file description, so this call is a no-op when the parent holds the lock exclusively, and it fails while any other process holds the lock. Either way the child runs only under an exclusive lock, and a shared request, which would downgrade the parent's lock, is never made. The child never unlocks or closes the descriptor early. The lock is released only when the parent, which holds the last copy of the descriptor, unlocks or exits. A refusal with a 6xxx code follows if the variable names a descriptor that is not open, is not that file, or cannot be locked exclusively. A command that requires the exclusive lock (`release activate`, ruling 5's checks, the audited domain migration, the restore) and runs without the variable takes the lock itself and refuses if it is held.
4. **Expand/contract still applies.** Every migration is written as expand/contract, so that a roll-forward fix is always possible. A CONTRACT step ships in a later release than the cutover that stopped using the old shape.
5. **Persisted content references.** Durable rows refer to content only by `{family, key, revision}` (F10). A compiled or legacy numeric id is never persisted. A referenced registry row is never deleted (F14 pattern). A release may not remove, rename or reinterpret a definition that live rows reference, unless the change carries a DUR-04 §12 class and, for `EXPLICIT_DATA_MIGRATION` or `REMOVED_WITH_EXPLICIT_POLICY`, an audited domain migration. Every reference a row holds was written by a build whose bundle resolved it (F32), so two checks enforce this (packet DATA-CONTENT-REF-5). Neither uses an old snapshot:
   - **Bundle check, every deploy.** Under the exclusive ops lock (ruling 3), the deploy reads the bundle digest of every build activation row, `PENDING` or `ACTIVE`, in the live database from the newest compatibility baseline onward (ruling 9 step 3; below) and requires the new bundle to resolve every `{family, key, revision}` that any of those bundles resolves, to a definition with the same content digest. It reads no player row. Every row, written before or during the deploy, holds a reference that some activated bundle resolves, and only a deploy holding the lock adds an activation, so the input cannot go stale. A pass lets the release roll (ruling 3). The new bundle's own additions need no check. Rows written before the activation table existed were written by builds with no activation row, so on a database with no activation row the bundle check has no input and the deploy runs the live check instead; the release that adds the table carries a migration and is stop-the-world anyway.
   - **Live check, every stop-the-world release, on the final database.** A failing bundle check, or one with no input, makes the release stop-the-world (ruling 3), and every stop-the-world release runs this check, because a migration can rewrite stored references. With every node stopped, after `oteryn-game-migrate` and after any audited domain migration has completed, and before the `ACTIVE` row, the deploy runs the check on the live database with a read-only role: it lists every distinct `{family, key, revision}` that rows reference and fails if the new bundle does not resolve one directly, by an explicit alias, or under a DUR-04 §12 class that needs no further data migration. A reference that the new bundle classes `EXPLICIT_DATA_MIGRATION` means the domain migration is incomplete, and fails; `INCOMPATIBLE_REQUIRES_PRODUCT_DECISION` always fails. A failure writes no `ACTIVE` row and starts no node; the release rolls forward or restores to its named backup (ruling 9). The deploy may run the same check before the named backup to fail early; that run gates nothing and writes no report. Any audited domain migration runs under the build's `PENDING` activation and completes before its `ACTIVE` row (ruling 3); it is idempotent, so a deploy resumed after a crash runs it again to completion. With every node stopped and the ops lock held, no Game writer runs, so the database the final check read is the one the new build starts on.
   - **Compatibility baseline after a passing live check.** An intentional removal fails the bundle check, because an older activated bundle still resolves the removed definition. Without a baseline, that older row would fail every later bundle check and make every later release stop-the-world. So when the final live check passes, `release activate` writes the new build's `ACTIVE` row as a **baseline**: the row carries `baseline = true` and `live_check_digest`, the SHA-256 of the live check's report. The report lists every distinct reference and its `resolution`: `DIRECT`, `ALIAS` or the name of one DUR-04 §12 class that needs no further data migration; any other value fails the check. It is stored write-once as `releases/<build id>/live-check-<live_check_digest>.json` in the release index (activation revisions repeat across timelines after a restore, so they never name a file), synced before `release activate` runs, and never pruned in the alpha. The baseline is written in the same exclusive lock hold as the live check that passed on that database, after any migration and any domain migration have completed and before any node starts. A deploy resumed after a crash runs the live check again in its own hold. `release activate` refuses `baseline = true` without such a report, and refuses a report digest that does not match. The bundle check then reads only the baseline row and the rows after it. This drops the older obligations safely. The live check proved, with every node stopped, that the baseline bundle resolves every reference in the database. Every row written later is written by a build activated at or after the baseline. The activation table stays append-only: the older rows remain for restore (ruling 9 step 3), and only the bundle check skips them. A PITR to a T before the baseline restores the older rows and the older baseline with them. A database with no activation row gets a baseline as its first row, because the live check runs there.
   A drill database is never the gate. A drill runs both checks only as a rehearsal (ruling 12).
6. **Content artifacts and receipts.** World bundles and reference artifacts are immutable and content-addressed. A changed input gets a successor receipt that names what it supersedes, as #1869 did (F13). A node logs its bundle digest and its ledger head on its boot line. `oteryn-game-ops diagnose` reports both.
7. **Backups.** Use pgBackRest (upstream, not forked). Archive WAL continuously. Take a full backup weekly, a differential daily, and a named backup before every schema release. The repository is encrypted (`repo-cipher-type=aes-256-cbc`). Its key and credential are separate from the database credentials and never in the repository. It lives off the database host. HA replicas are not backups. Owner ruling 2026-10-06 (owner question 1, option a): RPO ≤ 5 min (`archive_timeout` ≤ 300 s), RTO ≤ 4 h, backup retention 14 days (`repo1-retention-full-type=time`, `repo1-retention-full=14`) and a restore drill each month (ruling 12). The drill measurement confirms the RTO.
   **Archive health.** `archive_timeout` only forces segment switches; it does not prove that a segment reached the repository, and a failing archive keeps WAL locally while the recoverable point ages. So a probe run by the deploy supervisor every 120 s runs `pgbackrest check`, which writes a restore point, switches the segment and waits until that segment is in the repository. A pass at time t proves every commit before t is archived, also on an idle database. The probe writes `oteryn_wal_archive_check_last_success_timestamp_seconds` and `oteryn_wal_archive_check_last_run_timestamp_seconds` to a textfile that upstream `node_exporter` (textfile collector) serves to the NAS Prometheus (§1 ruling 10). Two alerts join `deploy/observability/oteryn-alerts.yml`: `OterynWalArchiveBehind` when `time() - oteryn_wal_archive_check_last_success_timestamp_seconds > 300`, and `OterynWalArchiveProbeMissing` when the run timestamp is absent or older than 300 s. The 5 min RPO holds only while both are clear; while either fires, the recoverable point is the last success, the owner is emailed, and no release with a migration starts (the deploy checks the probe before its named backup). DECIDED (§3 Q4: a, accepted by control plane under D607 (D838)): fresh admission stays open during an archive alert in the testing phase, and the choice is decided again with ruling 8 before external players.
8. **The restore fence directory.** One directory, outside the database volume and outside every database backup, holds every value a restore must not roll back: the Character recovery fence (F19), the LCFA high-water F (F22), the `assignment_epoch` high-water (F23) and the erasure journal (step 7 of ruling 9; ARCH-LIVE-READINESS-0 §3 ruling 18). It also holds the retained signed request files of every staff roster revoke, staff mute, economy fence raise and case or privacy legal hold placement (ARCH-LIVE-READINESS-0 §1 ruling 2, §3 rulings 2, 5 and 20), written before submit, with an outcome record for each; step 7 of ruling 9 replays them by the replay rule below. Each keeps its own contract semantics. The directory also holds the ops lock (ruling 3) and the restore journal (ruling 9 step 2). Writes use write, sync, atomic rename and directory sync, as the Character fence does today. A missing or unreadable value is a refusal, never 0. Owner ruling 2026-10-06 (owner question 2, option b, with the owner's refinement): **for the testing phase only**, the directory lives on its own volume of the alpha NAS (the node host), with no off-host copy. If the NAS is lost, the fences and the erasure journal are lost with it. A restore after that loss needs an explicit, logged operator reseed, `oteryn-game-ops restore --reseed`, which rebuilds the whole directory inside the restore run, after step 3 and before step 4 of ruling 9, and never outside one; it refuses while any fence value of the directory is still present, so it never moves a surviving fence. The operator supplies, for each value, the highest one found in any surviving backup; the run logs them in the restore journal. (a) **Character fence.** Let H be the highest admitted generation in the restored database and B the highest in any surviving backup, and G = max(H, B). The reseed writes the retained records H+1 to G as step 4 requires them: record H+1 has `predecessor_digest` equal to the digest of the restored admission row H, every later record the digest of the one before, each with a fresh UUIDv7 `recovery_event_id`, written write-once as in step 4; record G is installed as the current record (with G = H, the restored row H itself is installed). It then reads every record back and checks the whole chain from row H, as reconcile will, and refuses on any difference. Step 4 then advances G to G+1 and bridges H+1 to G+1 as usual, so the fence ends strictly above every restored and backup value and no generation is skipped. (b) **LCFA F and the `assignment_epoch` high-water** are set strictly above every value in the restored database and in any surviving backup; step 8 then raises above them. (c) **Erasure journal, retained requests and outcome records.** All are written empty, so step 7 re-applies nothing, and the run records that erasures, revokes, mutes, fence raises and hold placements after T are lost. (d) The ops lock and the restore journal are created by steps 1 and 2 as in a new directory. The journal records the event's closed `fence_source`: `RETAINED` for an ordinary restore, `RESEED` for this one, with H, B, G, the minted event ids and the digest of every record written. A later restore to a T whose admission rows above H differ from the rebuilt chain refuses at step 4 (`Conflict`); such a backup is unusable after a reseed. The reseed is refused unless the stack's deploy configuration sets `stack_phase` to `TESTING`; `stack_phase` is one of `TESTING` and `EXTERNAL`, and a missing or other value refuses. `EXTERNAL` is set only after the re-decision below. This is acceptable only because testing-phase data belongs to no external player. **Re-decision gate:** before any external player is admitted, the owner decides again where the directory survives host loss (options a and c of owner question 2 stay open), and external admission stays closed until that ruling is recorded, deployed and drilled (ruling 12).
   **Replay after restore.** This one rule selects the retained requests for ruling 9 step 7 and for ARCH-LIVE-READINESS-0 §1 ruling 2 and §3 rulings 5 and 20. A request file is written before submit, so it proves intent, not commit.
   - **Outcome record.** Once the submitting `oteryn-game-ops` process holds a request's final outcome, it appends an outcome record to the directory: operation_id, request digest, `COMMITTED` or `REFUSED`, and the outcome digest. The outcome is the node's answer, or the tool's own transaction once it has committed or refused. The record is write-once and idempotent by operation_id, and it is written before the tool reports. A crash before the record leaves the request ambiguous. The tool's ordinary reconciliation submits the same bytes again, gets the stored outcome and appends the record.
   - **Selection.** At restore, selection reads only these records and the restored database. It compares no `issued_at`, file time or other clock with T, because an issuer clock that lags would drop a control committed after T (ruling 10 for the same reason).
     - An operation_id with a stored outcome in the restored database committed before T. Nothing changes, so a control unmuted, lifted or released before T stays that way.
     - A `COMMITTED` record with no stored outcome is a commit after T that the restore lost. It is applied again, verified against the roster revision it names, without the freshness window, idempotent by operation_id.
     - A `REFUSED` record changes nothing.
     - A request file with no record is ambiguous: it was never submitted, or it crashed before its record. The restore stops at step 7 with a 6xxx code. It replays none of these requests, starts no node, keeps admission and mutation closed, and lists the ambiguous ones. Each is resolved by a signed `oteryn-game-ops restore resolve --operation-id <id> --apply | --abandon`, verified like the request itself, in the request's namespace. The resolution is written as that request's outcome record (`--apply` as `COMMITTED`, `--abandon` as `REFUSED`), and the run resumes. A request is never applied or dropped without a commit record or a signed decision.
   - **Order and scope.** Every replayed request only narrows and none depends on another, so the order does not change the result; they run in the order the store holds them. No grant, unmute, lift or hold release is replayed. One committed after T is lost and is signed again.
   - **Retention.** The request files, the outcome records and the stored outcomes are not pruned in the alpha.
9. **Restore runbook (normative order).** A restore is one operator procedure, `oteryn-game-ops restore` (packet DATA-RESTORE-OPS-2):
   1. Take the exclusive ops lock (ruling 3). Close admission on every channel. Stop every node, the LCFA publisher and the status reporter, and have the supervisor keep them stopped until this run starts them. Pause the scheduled backups and their expiry until step 3 has finished, so no backup runs against the half-restored database and no expiry removes the backup being restored. Verify that no pre-restore process is alive.
   2. Open the restore journal `restore-journal-v1` in the restore fence directory, written by write, sync, atomic rename and directory sync. A new run mints a restore event id (UUIDv7) and records it with the backup label and T before the database changes. Restore with pgBackRest to that named backup and T (`--type=time`, `--delta`, so a rerun after a crash overwrites a partial restore). Record its annotations.
   3. Start the database with admission and mutation closed. Select the build, then run the schema gate (F3) with it. The ledger digest is SHA-256 over the ordered `(version, checksum)` pairs of the `_sqlx_migrations` rows. Every server release records the digest of its embedded `migrate!` set and its world bundle digest. Releases without a migration share one digest (ruling 3), so the digest cannot name the build (F31). A new append-only table of **build activation rows** (packet DATA-RESTORE-OPS-2) does: `(activation_revision, build_id, ledger_digest, bundle_digest, phase, baseline, live_check_digest)`; `phase` is `PENDING` or `ACTIVE` (ruling 3) and nothing else, and the last two are for ruling 5's compatibility baseline. Revisions come from a writer high-water taken `FOR UPDATE`, and a guard trigger refuses update and delete, as for node registrations (`0006:254-263`, `0003:87`). Only a deploy writes a row, through `oteryn-game-ops release activate` under the exclusive ops lock, after any migration and after ruling 5's check on that final database, and only if the build's embedded digest equals the live ledger digest and its `releases/<build id>/release.json` (and, for a baseline, its live-check report) is already synced in the release index with the same values. `release activate` connects with the migration credential, which no node holds; the node runtime role has `SELECT` only on the table. Every stop-the-world release writes a `PENDING` row right after `oteryn-game-migrate`; a rolling release (ruling 3) writes `ACTIVE` alone. While the newest row is `PENDING`, `release activate` refuses every row except that build's `ACTIVE` row, written after any domain migration has completed and the final live check has passed. A node refuses to register (`src/node/serve.rs:1442`) unless an `ACTIVE` row names its build and the newest row is `ACTIVE`. Once any row exists, an `oteryn-game-ops` write command refuses the same way, except the audited domain migration, which runs only under the inherited exclusive lock while the newest row is `PENDING` for its own build. So every row a build writes commits after that build's activation row, and a PITR to T that holds the row also holds the activation: WAL order decides, not a clock. The restore takes the activation row with the highest revision in the restored database, which is the build most recently deployed at T; it must be `ACTIVE`. It checks that the row's ledger digest equals the digest of the restored rows, that every restored row is successful, and that `releases/<build id>/release.json` is retained with the same build id, ledger digest and bundle digest. Steps 4–12 run with that build's `oteryn-game-ops` and node binaries, and their exact gate must pass; the invoking command keeps the ops lock and runs the selected `oteryn-game-ops` as its child, handing the lock down as ruling 3 describes. The journal format is versioned, and a build refuses a journal version it does not know. Other retained builds may share the digest; the restore never chooses among them by digest or by backup annotation. Any older build that wrote rows before T was deployed before or beside the selected one, so ruling 3 and ruling 5's check already require the selected build to read those rows. The restore stops with a 6xxx code, and no node starts, on any of these: no activation row; a newest row whose digest differs from the restored ledger (T after a migration and before the activation that follows it); a newest row that is `PENDING` (T inside a stop-the-world deploy, before its final live check passed); a build missing from the index; an unsuccessful ledger row; or a partial ledger. The operator then chooses another T. The current binary never runs against an older ledger, except R1's `oteryn-game-ops` in the bounded legacy restore below, and the restore never migrates. Moving the restored database forward to the current release is afterwards an ordinary release under ruling 3, with its named backup. This is what makes a restore to the named pre-migration backup (ruling 3) a working rollback. The release index (`releases/<build id>/` with the binaries, the bundle and `release.json`) is never pruned in the alpha (U8). Every backup carries `--annotation=oteryn-build=<build id>` and `--annotation=oteryn-ledger=<digest>` from the newest activation row, read before the backup starts; they help the operator choose a backup and select nothing. **First rollout (legacy restore).** The release that adds the activation table, R1, is the first with a `restore` command, and its own named pre-migration backup has no activation table. Only that one transition gets a bounded legacy path. (a) R1's migration set adds only the activation table, its writer high-water and its guard trigger to the ledger of the release it replaces, R0; every other schema change of the restore packets ships in a later release. (b) Before its named backup, R1's deploy retains R0 in the release index as `releases/<R0 build id>/` with R0's binaries, bundle and a `release.json`, and R1's own `release.json` names R0 as `legacy_predecessor` with R0's build id, ledger digest and bundle digest. The deploy refuses unless the live ledger digest equals that ledger digest and the stopped nodes' boot lines (ruling 6) name that bundle digest. (c) A restore that finds no activation table accepts the restored database only if its ledger digest equals the `legacy_predecessor` ledger digest of the invoking release's chain, and then selects R0; the earlier builds that share R0's digest wrote only rows that R0 reads (ruling 3), as above. Any other ledger without the table fails closed and starts no node. (d) R0 has no `restore` command, so R1's `oteryn-game-ops` runs steps 4–12 itself, in a legacy mode whose exact gate is R0's ledger digest. By (a), every table those steps read or write exists unchanged in R0's ledger. The restore never creates the activation table in the restored database. If the restore fence directory holds an erasure record, a retained request or a delivery after T whose re-apply needs a table R0's ledger lacks, the legacy restore stops with a 6xxx code and starts no node. Step 12 starts R0's node binaries, which predate the activation check and register as before. (e) Leaving R0 is an ordinary stop-the-world deploy of R1 or a later release under ruling 3. A table-less ledger older than R0's fails closed as before. Record the selected build in the journal.
   4. Advance the Character recovery fence by CAS (`begin_recovery`) from the external generation G to G+1, with the journal's restore event id. On a resumed run, a fence record that already carries this event id (`recovery_event_id`, `character_recovery_fence.rs:29`) means the CAS is done, and it is not repeated. Then run `reconcile_character_recovery`. Restored rows are history, not authority. If T predates an earlier recovery, the restored admitted generation H is below G. Today reconcile accepts only H = G or H = G+1 (`durability/character_authority.rs:411-417`), and admission rows are contiguous (`migrations/0005_character_authority.sql:40`, `recovery_generation = predecessor_generation + 1`), so such a restore would conflict for ever. Therefore the fence register retains every record it supersedes: before it replaces the current record, `begin_recovery` writes that record write-once as `character-recovery-fence-v1.<generation>.record` in the fence directory (write, sync, atomic rename, directory sync; an existing file with other bytes is a conflict). Reconcile bridges H < G: it reads the retained records H+1 to G, checks that each one's `predecessor_digest` is the digest of the one before, starting from the database's admission row H (the `assert_predecessor_admission` check), and inserts H+1 to G and then G+1 in one transaction. A missing, unreadable or mismatched record refuses (`Conflict`); a generation is never skipped. No admission row, or H above G+1, stays a contradiction as the fence decision requires. After a NAS loss the current and retained records come from ruling 8's reseed, checked the same way.
   5. Revoke every node registration that is CURRENT in the restored snapshot. New nodes register under fresh NodeIds, and only in step 12. All launch and scope authorizations are issued again.
   6. Run the DUR-03 §41 validation and the DUR-02 rule 5 list on the restored rows, so a broken restore stops early. Any failure keeps the affected mutation closed until an audited repair. Step 12 runs it again on the final state; DUR-02 rule 5 keeps admission and mutation closed until reconciliation passes.
   7. Re-apply the erasure journal (ARCH-LIVE-READINESS-0 §3 ruling 18) and the retained requests (ruling 8). Step 7 first checks the journal against Platform's signed erasure head, bound to a fresh nonce and the environment id, so every `erasure_seq` from its `floor_seq` to its `head_seq` has a record (ARCH-LIVE-READINESS-0 §3 ruling 18), and then verifies the Platform signature of every erasure journal record. If a sequence number has no record, or any record cannot be read, parsed or verified, or the head cannot be obtained or does not verify, the whole restore stops with a 6xxx code: it applies no record, starts no node, keeps admission and mutation closed, and the event stays `IN_PROGRESS` at step 7. A record is never skipped, because a skipped erasure would bring back the Character state it erased. The run resumes only after an audited repair appends Platform's current issue of each missing or failing erasure request, with the same operation_id and `erasure_seq`; `--abandon` does not help, because every later restore reads the same journal. Then every erasure in the journal is applied again before any authority opens, idempotent by operation_id; one already complete in the restored database changes nothing. Then it replays the retained staff roster revoke, staff mute, economy fence raise and case or privacy legal hold placement requests by the replay rule of ruling 8. Only a request with a retained `COMMITTED` outcome record and no stored outcome in the restored database is applied again. A request file with no outcome record stops the restore here until a signed resolution. Neither the erasure selection nor the request selection compares `issued_at`, a file time or any other clock with T. Both live in the restore fence directory (ruling 8); a missing or unreadable directory, journal or request store is a refusal.
   8. Raise the LCFA projection epoch strictly above F. Raise `assignment_epoch` strictly above its high-water. Each raise first writes its target value into the journal, then applies it, so a resumed run re-applies the same value and never raises twice. Persist both before any publisher or reporter starts.
   9. Drop all Premium evidence caches. No Premium benefit applies until a pull made after the restore succeeds (F26, F29). Restored fence rows are only lower bounds. A pull that Game refuses with `ACCOUNT_ERASED` (ARCH-LIVE-READINESS-0 §3) is recorded for that account, and the step continues.
   10. All pre-restore Game Sessions, leases, transports and reconnect material are dead. Clients log in fresh. The server sends a full snapshot (ADR-0009 §9).
   11. Start the publishers. Publish the Platform reconciliation notice and run the delivery reconciliation of ruling 10 in the selected build's `oteryn-game-ops`, with every node still stopped and admission closed. The step is done only when every listed delivery is applied, returned from its receipt or rejected, and Platform has acknowledged the notice (ruling 10).
   12. Run the validation of step 6 again on the final database, after the step 7 re-applies, the raises and the reconciliation; a failure keeps admission and the affected mutation closed until an audited repair. Then set the event to `OPENING` (write, sync, atomic rename, directory sync) before any node starts. Then start the nodes; they register under fresh NodeIds (step 5). Open admission only after that, and only when at least 35 s (30 s grant lifetime + 5 s skew, F28) have passed since step 4. The command checks this itself on the ops host's monotonic clock. It trusts no time read from the journal: a resumed run that finds step 4 done waits the full 35 s from its own start. After admission opens, set the event to `COMPLETE` the same way, then release the ops lock.
   Each failed step exits with an ops status of the model decision and a code registered in block 6000–6999. A fence refusal uses block 3000–3999. The run is re-entrant: the journal records each completed step, and a repeated run after a crash resumes the journaled event at the first step that is not done, and never advances a fence twice for one restore event. A repeated run whose backup label or T differ from an unfinished journal refuses, unless the operator passes `--abandon`, which sets that event to `ABANDONED`; fences it advanced stay advanced, and the new event advances them again from there (step 4 bridges the gap). Each journal event has a closed `state`: `IN_PROGRESS`, `OPENING`, `ABANDONED` or `COMPLETE`. `IN_PROGRESS` resumes at the first step that is not done. `OPENING` means nodes may run and admission may be open, so the restored state is live: a run with the same backup label and T only finishes step 12 (start any stopped node, wait the full 35 s, open admission, which is idempotent, and set `COMPLETE`), and a run with another label or T refuses, with or without `--abandon`. `ABANDONED` and `COMPLETE` are terminal, never change and are never resumed. **Journal gate.** Live authority never coexists with an unfinished restore: when the restore journal holds an event, a node refuses to register (`src/node/serve.rs:1442`), the supervisor refuses to start a node, and every `oteryn-game-ops` write command other than `restore` refuses, each with a 6xxx code, unless the newest event is `OPENING` (nodes only; ops write commands wait for `COMPLETE`) or `COMPLETE`. A crash anywhere in steps 1–11 therefore leaves the stack stopped, and an `ABANDONED` newest event (a crash between `--abandon` and the new event) keeps it stopped until a restore completes. R0's nodes in the legacy restore predate the check, so for them the supervisor's check alone applies.
10. **Platform state after a Game restore.** Platform state is not restored with the Game DB. Game is the authority for Character facts. Platform is the authority for accounts, coins and purchases.
    - Character list: the epoch raise invalidates every Platform entry from older epochs (LCFA §5). Characters created after T disappear from Platform's view. Nothing is written back.
    - Runtime routing: the `assignment_epoch` raise invalidates all older runtime state in Platform (status producer §6).
    - **No selection by time.** Nothing here compares a Platform time with a Game time or with T. Platform and Game clocks are independent, so a Platform acknowledgement time says nothing about whether the Game commit is before or after T. Selection uses Game-issued order only.
    - **Game-issued watermark.** Every Game outcome returned to Platform (a delivery receipt, a Character operation receipt) carries the Character recovery generation under which Game committed it. Every Character write checks the fence in its own transaction (`assert_recovery_fence`, `durability/character_authority.rs:533`; for example `durability/item_decay_retire.rs:341`), and the delivery packet's write does the same. Generations only rise (`game_character_recovery_admissions`, `0005:30-42`). Platform stores the generation with its acknowledgement. After a restore, H is the highest admitted generation in the restored database (ruling 9 step 4). Every commit after T carries a generation of at least H, so the set "generation ≥ H" contains every lost outcome. It also contains outcomes of generation H committed before T; those are replays and are no-ops. A finer Game sequence is not used, because a sequence value is not commit order.
    - Purchases and deliveries. The identity is Platform's `delivery_operation_id`: every retry reuses it and a duplicate cannot double-grant (Platform `docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md:218-237`; Game `docs/architecture/PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md` §13, lines 329-340). Game places the item through the Character Inbox with `cause_ref` = `delivery_operation_id` (`game_character_inbox_deliver`, `0076:215-277`). The Inbox record key is `(item_instance_id, cause_kind, cause_ref)` (`0076:61`), so it only stops a second record for the same item; a lost delivery mints a new item. The delivery packet therefore keeps a receipt keyed by `delivery_operation_id` alone, written in the same transaction as the Inbox row, and a repeat of that id returns the receipt and creates nothing. After a restore Game runs one reconciliation (ruling 9 step 11), before any node starts or admission opens: (1) read the set R of `delivery_operation_id` receipts in the restored database; (2) page through Platform's list of acknowledged deliveries for each World in the database whose recorded generation is ≥ H, using Platform's own list cursor (an empty bound, "every acknowledged delivery", is also correct); (3) for each id not in R, apply the delivery from Platform's envelope through the normal path under the same id; an id in R returns its existing receipt. A listed delivery whose target Character is erased, in the restored database or by the step 7 re-apply, gets a bounded rejection and creates nothing; it is never moved to another Character, and Platform compensates. R is only a shortcut: each apply is itself idempotent by id, so a delivery that races the reconciliation is still applied once. The step is re-entrant: a crash mid-run repeats it and only no-ops follow. Deliveries Platform still holds as PENDING or AMBIGUOUS are retried by Platform under the same id, as before. This is a proposal to Platform under PROD-ENTITLEMENTS-01 (A4).
    - Ownership and lifecycle operations (deletion, transfer, Bazaar): Platform re-requests, with the same operation identity, each operation whose Game outcome carries a generation ≥ H. Game keys these by `operation_id` (`game_character_operation_receipts`, `0005:90-111`). Game answers from restored state after the step 7 re-apply: the existing receipt, a fresh execution if still eligible, or a bounded rejection, which is always the answer for an erased Character; on a rejection Platform compensates on its own side. A Platform workflow row never proves the Game outcome (F27).
    - Game sends Platform one restore notice: restore event id, restored generation H, new recovery generation, T and the backup label. T is for the record only; nothing selects by it. It carries no player data. Platform acknowledges it, naming the restore event id, once every re-request of the bullet above has a Game answer; while no Platform workflow that settles against a Game outcome is live (U2), the acknowledgement only confirms receipt. Game opens admission only after it (ruling 9 steps 11–12); a re-request that arrives later is still answered by operation identity. So a Platform outage, or a slow acknowledgement, keeps admission closed and lengthens the restore past the RTO of ruling 7; Game never opens without the acknowledgement. This risk is an explicit Platform acceptance item of A3 (PLATFORM-RESTORE-RECONCILE-P1).
11. **Lost player progress.** Gameplay after T is lost. It is never re-executed. Any make-good uses audited idempotent domain transactions (F17). Direct database edits are forbidden.
12. **Restore drills.** A drill restores a named backup into a separate preproduction stack, with that stack's own restore fence directory and ops lock; it never opens the production ones. It runs the full ruling 9 procedure, with step 11 sent to a preproduction Platform or a stub and never to production Platform. It rehearses both checks of ruling 5 on the restored data, and records timings. A drill never approves a release. These timings are the measurement that fixes RTO. One drill must pass before external alpha. Owner ruling 2026-10-06 (owner question 1, option a): a drill runs each month and must finish within the 4 h RTO; a miss blocks the next schema release until a drill passes. Entry condition for external players: the ruling 8 re-decision is recorded, its store is deployed, and one drill has restored with it.

Before-freeze checklist. Concurrent transitions: the ops lock (ruling 3) serializes deploy, restore and named backup, and their child commands inherit it exclusively through `OTERYN_OPS_LOCK_FD`, never re-taking it or taking it shared; the migrator's advisory lock, the fence CAS, the activation and registration high-waters and the epoch row lock serialize the rest. Restart/resume: ruling 3 resumes from the live ledger and the newest activation row, whose `PENDING` phase keeps nodes and a restore off a database whose stop-the-world deploy has not passed its final live check; ruling 9 resumes per step through the restore journal, whose event `state` is a closed set, and the event id on the fence record; the journal gate lets a node register only under an `OPENING` or `COMPLETE` newest event, and `OPENING` is written before any node starts, so an unfinished event never coexists with live authority and `--abandon` never undoes a live restore. Typed cross-record refs: ruling 5, checked on bundles from the newest compatibility baseline onward or on the final migrated live database with every node stopped, never on an old snapshot or before a migration. Gate order: no check gates a state that a later step still changes (ruling 3 live check after every migration; ruling 9 step 12 validation after the reconciliation), and no node registers and no admission opens before every reconciliation completes (ruling 9 steps 11–12). Older client/peer gating: ruling 2 (exact gate), ruling 9.3 (a restore runs the newest activated build in the restored database, checked against the restored ledger, or, for the first rollout's backup only, R1's `legacy_predecessor`) and ruling 9.10 (fresh login). Multi-component commit and recovery: ruling 8 (fences outside the snapshot), ruling 9.4 (the fence chain is bridged from retained records, never skipped; after a NAS loss ruling 8's reseed rebuilds and verifies that chain from the restored row H) and ruling 10 (idempotent re-request by operation identity).

### Contract amendments

A1. `docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §6, replace the last sentence ("Where the epoch is stored … (U-RS5).") with:
> The epoch is stored as an external high-water in the Game restore fence directory, outside the Game database and its backups. In the testing phase that directory is on its own NAS volume only; where it survives host loss is decided again before external players are admitted (ARCH-ALPHA-OPS-0 §3 ruling 8). The operator raises it only through `oteryn-game-ops restore`, strictly above the stored high-water, after the Character recovery fence advances and before any node reports. A missing or unreadable high-water is a refusal, never 0.
Also mark U-RS5 resolved by this text.

A2. `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` §5, replace "The production fence location, durable across host loss, stays open (U-LC2)." with:
> F lives in the Game restore fence directory with the Character recovery fence, outside the Character store and its backups. In the testing phase that directory is on its own NAS volume only and host loss needs a manual reseed; where it survives host loss is decided again before external players are admitted (ARCH-ALPHA-OPS-0 §3 ruling 8). The production restore runbook is `oteryn-game-ops restore` (ARCH-ALPHA-OPS-0 §3 ruling 9).
In §10, set U-LC2 to resolved for the testing phase, pointing at that text; it reopens at the external-player re-decision.

A3. `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`, new §13.1 "Game restore" (the contract is Accepted, so this needs the control plane and Platform review):
> Every Game outcome carries the Character recovery generation under which Game committed it, and Platform stores it. After a Game restore, Game publishes one restore notice (restore event id, restored generation H, new recovery generation, T, backup label). Platform then re-requests, with the same operation identity, every operation whose Game outcome carries a generation ≥ H. Platform never selects by comparing its own clock with T. Game answers from restored state: the existing receipt, a fresh execution if still eligible, or a bounded rejection, which is always the answer for an erased Character. On a rejection Platform compensates its own state. Platform acknowledges the notice, naming the restore event id, once every such re-request has a Game answer; Game opens admission only after that acknowledgement. Platform never infers a Game outcome from its own workflow row. Platform accepts that the Game RTO includes this acknowledgement: while Platform cannot acknowledge, Game admission stays closed and the restore may exceed the RTO, and Platform states the time within which it acknowledges a restore notice.

A4. A proposal to Platform, routed by the control plane, not a Game file edit. When the PROD-ENTITLEMENTS-01 delivery contract is written, it includes: "Every delivery has a stable `delivery_operation_id`. Game applies it idempotently and returns the recovery generation it committed under; Platform stores that generation. Platform offers Game a read-only, cursor-paged list of acknowledged deliveries per World, filtered by generation ≥ H. After a Game restore notice, Game re-applies every listed id absent from the restored database; a present id is a no-op; a delivery to an erased Character is a bounded rejection that Platform compensates. No step compares Platform and Game clocks."

### Packets

- **DATA-MIGRATION-GUARD-1.** A CI check that a merged migration file is never changed or deleted, and that every new version is greater than the highest version on `origin/main`. Owned paths: new `tools/repository/check_migration_ledger.py` and its test; a wiring line in the governance/CI workflow that runs repository checks. Tests: changed file fails; deleted file fails; a new 0078 fails; a new 0080 passes. Dependencies: none. P0-adjacent because of F6.
- **DATA-RESTORE-OPS-2.** `oteryn-game-ops restore`, steps 1–12 of ruling 9 (step 11's reconciliation lands with PLATFORM-RESTORE-RECONCILE-P1): the ops lock and the restore journal, the ledger digest (also printed by `oteryn-game-ops schema-digest`), the build activation table (with the `phase` column of ruling 3, the `baseline` and `live_check_digest` columns of ruling 5, and `SELECT` only for the node runtime role) with `oteryn-game-ops release activate`, the lock handoff of ruling 3, the node's and the ops write commands' activation check, the build selection, the fence CAS with the restore event id, the retained fence records and the multi-generation reconcile bridge, the erasure journal re-apply (every record's Platform signature verified before any is applied; one that fails stops the restore) and the re-apply of retained revoke, mute, fence raise and hold placement requests, the NAS-loss reseed of ruling 8, revocation of CURRENT registrations, the LCFA raise against F, the `assignment_epoch` raise, the 35 s admission gate, per-step resume, and registered codes. Owned paths: `apps/game-server/src/bin/oteryn-game-ops.rs`, `apps/game-server/src/character_recovery_fence.rs`, `apps/game-server/src/durability/character_authority.rs` (reconcile only), `apps/game-server/src/durability/schema.rs` (digest only), new `apps/game-server/src/restore_fence.rs`, new `apps/game-server/src/durability/build_activation.rs` and its `mod` line, one new migration for the activation table (version above the highest on `main` at authoring, ruling 1), `apps/game-server/src/node/serve.rs` (the activation check and the journal gate before `register` only), `deploy/synology-game/supervisor.sh` (the journal gate before a node start only, after OBS-DEPLOY-4), `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json` (new codes only). Tests: crash after each step and resume; a crash after the fence CAS resumes without a second CAS; a resumed run waits the full 35 s; a second restore, or a deploy, while one holds the ops lock refuses; the restore's selected-build child runs steps 4–12 under the inherited lock, the lock stays exclusive for the whole run, and a shared `oteryn-game-ops` write command started during the child or between steps refuses; a child whose `OTERYN_OPS_LOCK_FD` names a closed descriptor, another file, or a separately opened `ops.lock` while the parent holds it refuses; no child takes a shared lock or unlocks; `release activate` refuses `baseline = true` without a matching live-check report; an unfinished journal with another T refuses without `--abandon`, and an abandoned event followed by a new one bridges the fence; builds A and B share one digest: activate A, write, activate B, write, then a PITR between the two activations selects A and one after B selects B, also when no B node has started; a redeploy of A after B selects A; a node or ops write command of a build with no activation row refuses; a T between a migration and the next activation refuses; a node of a build whose newest row is `PENDING`, and any ops write command except that build's domain migration while the newest row is `PENDING`, refuse; `release activate` for another build while the newest row is `PENDING` refuses; a PITR to a T after a `PENDING` row and before its `ACTIVE` row refuses and starts no node, also for a stop-the-world release without a domain migration; the node runtime role cannot insert into the activation table; `release activate` refuses a build whose `release.json` is not synced in the release index; recoveries to generations 2 and 3, then PITR to a T before the first, so the database is at 1 and the fence at 3: the restore advances to 4 and bridges 2, 3 and 4; a missing, altered or reordered retained record refuses; a PITR to a T before a migration selects the previous release and passes its gate, a T after it selects the current release, and a ledger that no retained release matches (including a partial one) refuses and starts no node; second run advances nothing; F above the computed epoch refuses; a stale NodeId is refused after restore; early admission open is refused; no node starts or registers and admission stays closed before step 11 completes with Platform's acknowledgement and step 12's validation passes, also when 35 s have passed; a step 12 validation failure keeps admission closed; the reseed (ruling 8) with the restored database at H = 2 and the newest surviving backup at B = 5 writes records 3 to 5 chained from row 2, step 4 advances to 6 and reconcile bridges 3 to 6; a reseed with B = H installs row H and step 4 advances to H+1; a reseed whose read-back chain differs, or that runs while any fence value is present, refuses; F and the `assignment_epoch` high-water end above every restored and backup value; the reseed refuses unless `stack_phase` is `TESTING`, also when it is missing; a later restore whose admission rows above H differ from the rebuilt chain refuses; an `ABANDONED` or `COMPLETE` journal event never resumes; a crash after `OPENING` and before `COMPLETE`, both before and after admission opens: the supervisor restarts the nodes, `--abandon` and a run with another T refuse, and a run with the same T waits the full 35 s, opens admission once and sets `COMPLETE`; while the newest event is `IN_PROGRESS` or `ABANDONED`, a node registration, a supervisor node start and an ops write command each refuse, also after a crash in each of steps 1–11; while it is `OPENING`, an ops write command other than `restore` refuses; a restore to a T before a roster revoke, a staff mute, a fence raise and a hold placement re-applies each once, and a second run changes nothing; with the issuer's clock 10 min behind, so a control committed after T has an `issued_at` before T, it is still re-applied; with the issuer's clock 10 min ahead, a mute committed and unmuted before T, with an `issued_at` after T, returns its stored outcome and stays unmuted; a retained request that never reached submit, and one whose node commit was followed by a crash before its outcome record, each stop the restore at step 7 with nothing replayed, until a signed `restore resolve` applies or abandons it; a resolution that is unsigned, signed in another namespace or by a principal not on the roster refuses; a `REFUSED` record is not replayed; a `COMMITTED` record whose operation_id has a stored outcome changes nothing; the tool appends the outcome record before it reports, and its reconciliation of an ambiguous outcome appends the missing record; a grant, unmute, lift or hold release after T is not re-applied; every erasure in the journal, before or after T, is applied once and a second run changes nothing; an erasure journal record with a bad Platform signature, or one that cannot be parsed, a deleted `PENDING`/`COMPLETE` pair and a journal truncated at a record boundary each stop the restore at step 7 before any record is applied, start no node and keep admission closed, a resumed run stops again, and the run completes once Platform's current issue of each missing or failing request is appended; a head for another environment, a replayed head or an unreachable Platform stops step 7; a missing request store refuses; a reseed records that these requests are lost; a step 9 pull refused with `ACCOUNT_ERASED` is recorded and the step continues; first rollout R0 to R1, then a restore to R1's named pre-migration backup selects R0 through `legacy_predecessor`, runs steps 4–12 with R1's `oteryn-game-ops` against R0's ledger, starts R0's nodes and opens admission; a table-less ledger that differs from `legacy_predecessor` refuses; a legacy restore with an erasure, retained request or delivery after T that R0's ledger cannot apply refuses; R1's deploy refuses when the live ledger or the nodes' bundle digest differ from `legacy_predecessor`; a check on the migration files that R1 adds no table other than the activation table, its high-water and its guard. Dependencies: A1, A2; ERR-REGISTRY-0; the re-apply of retained requests lands with the ARCH-LIVE-READINESS-0 packets that write them (OPS-GM-AUDIT-1 roster and signature module).
- **DATA-BACKUP-PITR-3.** pgBackRest configuration for the test and preproduction stack per ruling 7 (`archive_timeout` 300 s, 14-day retention), an encrypted repository, WAL archiving and the restore fence directory on its own NAS volume (testing phase, ruling 8). It also reorders the deploy for a release with a migration to stop, named backup, migrate, live check, start (ruling 3; F30). Owned paths: `deploy/synology-game/` and `.github/workflows/synology-game-deploy.yml` once PR #1874 merges; if it does not merge, the control plane names the path (`main` has no deploy directory; `git ls-files` shows only `tools/qualification/wp5_s3a/compose.yml`). It adds the ops lock to the deploy, the `release activate` calls before a build's first node starts (for a stop-the-world release `PENDING`, any audited domain migration, the final live check, then `ACTIVE`; for a rolling release `ACTIVE` alone; ruling 3), the wait for the named backup to complete before migrating, the `stack_phase` setting (ruling 8), the call to ruling 5's checks, the backup annotations and the release index of ruling 9 step 3 (`releases/<build id>/` on the NAS with the binaries, the bundle and a `release.json` holding the build id, ledger digest and bundle digest, never pruned in the alpha), and the archive probe, the `node_exporter` textfile collector and the two archive alerts of ruling 7 (written to `deploy/observability/oteryn-alerts.yml` after OBS-DEPLOY-4, one writer at a time). Tests: a timed restore of a named backup to T in CI with Postgres; a check that the fence directory is absent from the backup; a deploy records its `ACTIVE` activation before its first node starts and refuses while the ops lock is held; migration never starts before the named backup reports complete; the gating live check runs after `oteryn-game-migrate` and the domain migration and before the `ACTIVE` row, and a failing one writes no `ACTIVE` row and starts no node; a crash after the named backup, after the migration, after the `PENDING` row and inside the domain migration leaves every node stopped, and the rerun takes no second named backup once the migration is applied and writes the `ACTIVE` row only after the domain migration completes; a stop-the-world release without a domain migration writes `PENDING` before its final live check, a crash between that row and the check leaves every node stopped and the newest row `PENDING`, and the rerun runs the check again before the `ACTIVE` row; its `release activate` and ruling 5's checks run as children under the inherited exclusive lock and succeed, and a shared `oteryn-game-ops` write command started while any of them runs, or between them, refuses; the lock is released only when the deploy exits; a check that the configured `archive_timeout` and retention match ruling 7; every backup carries both annotations, taken from the newest activation row; a `promtool test rules` case where the repository becomes unreachable (bad credential, then no network) and `OterynWalArchiveBehind` fires within 300 s, one where the probe stops and `OterynWalArchiveProbeMissing` fires, and one where an idle healthy database fires neither; a release with a migration refuses to start while either alert fires. Dependencies: OBS-DEPLOY-4 for the alert file (owner questions 1 and 2 are ruled).
- **DATA-RESTORE-DRILL-4.** An automated drill that restores into a separate stack and runs `oteryn-game-ops restore`. It moves the database snapshot and the fence directory independently: DB older, DB older by two recovery generations, fence newer, fence missing, fence equivocating, fence lost and reseeded with a surviving backup two generations ahead of T, a T before the latest migration, a T inside a domain migration, a T between the `PENDING` row and the live check of a release without one, and a restore to the first rollout's named pre-migration backup (ruling 9 step 3). It records RTO timings and the size of the release index (U8). Owned paths: new drill script and test under `apps/game-server/tests/`; the runbook `docs/operations/` (new). Dependencies: 2, 3.
- **DATA-CONTENT-REF-5.** The two release-gate checks of ruling 5, run by the deploy under the ops lock. The bundle check compares the new bundle with the bundle of every activation row in the live database from the newest compatibility baseline onward and reads no player row. The live check runs on every stop-the-world release, on the live database with a read-only role, with every node stopped, after `oteryn-game-migrate` and any domain migration have completed and before the `ACTIVE` row: it lists every distinct `{family, key, revision}` that rows reference and fails if the new bundle does not resolve one directly, by an explicit alias or under a DUR-04 §12 class that needs no further data migration. Owned paths: new check module under `apps/game-server/src/content/` and its test; DATA-BACKUP-PITR-3 adds the call from the deploy. Tests: a definition that an activated bundle resolves and the new bundle drops fails the bundle check, even when no row in an older snapshot references it; a reference written after a drill snapshot is still covered, because its bundle is activated; the same revision with changed content fails; a rollback to a bundle that lacks a definition the newer build added fails the bundle check and passes the live check only when no row references it; an alias or a §12 class passes the live check; the live check refuses to run unless its caller holds the exclusive ops lock, itself or through the handoff (ruling 3); an intentional removal fails the bundle check, passes the live check and writes a baseline activation row with the report digest, and the next release that keeps every definition of the baseline bundle passes the bundle check and rolls, although an older activated bundle still resolves the removed definition; the bundle check ignores rows before the newest baseline; a baseline is refused without a live check that passed in the same lock hold; a PITR to before a baseline restores the older obligations; a drill database is never accepted as gate input; the bundle check includes the bundles of `PENDING` rows; a report `resolution` other than `DIRECT`, `ALIAS` or a DUR-04 §12 class fails; the report is named by its digest, and an existing file with other bytes refuses; a deploy resumed after a crash runs the live check again before writing the baseline; an incomplete domain migration (rows still referencing a definition the new bundle classes `EXPLICIT_DATA_MIGRATION`, with the `PENDING` row newest) fails the final live check, writes no `ACTIVE` row or baseline and starts no node; a schema migration that rewrites a stored reference to one the new bundle lacks fails the final live check although a run before the migration passed; an early run before the named backup writes no report and no baseline; a stop-the-world release without a domain migration whose final live check fails leaves its `PENDING` row newest and writes no `ACTIVE` row. Dependencies: 2 (activation rows), 3 (deploy).
- **PLATFORM-RESTORE-RECONCILE-P1 (proposal).** The A3 and A4 texts, sent to Platform by the control plane. No Game work until Platform accepts. Explicit acceptance item: Platform accepts that its acknowledgement of the restore notice is on the Game RTO path (a Platform outage keeps Game admission closed and can exceed the 4 h RTO) and states its acknowledgement time; the drill (ruling 12) measures the acknowledgement time. The Game side then lands with the PROD-ENTITLEMENTS-01 delivery packet (the `delivery_operation_id` receipt and the generation on every outcome) and in `oteryn-game-ops restore` step 11 (the reconciliation of ruling 10). Tests: Platform's clock 10 min behind Game, so a delivery committed after T has a Platform time before T: it is still listed (generation ≥ H) and delivered once; Platform's clock 10 min ahead, so a delivery that survived the restore has a Platform time after T: it returns the existing receipt and no second item; a duplicate replay of one id, twice and again after a crash mid-reconciliation, leaves one Inbox item and one receipt; a restore to a T before an earlier recovery (H < G) lists every generation from H; one id reused with another target fails closed; a missing acknowledgement of the restore notice keeps every node stopped and admission closed.

### Owner questions

1. Recovery targets for external alpha (all CANDIDATES; RTO is then fixed by the drill measurement).
   a) RPO ≤ 5 min, RTO ≤ 4 h, backup retention 14 days, a drill each month. b) RPO ≤ 1 min, RTO ≤ 1 h, 30 days, a drill each week. c) A nightly dump only (RPO 24 h).
   Recommendation: a. It is cheap with pgBackRest and enough for an alpha.
   **Owner ruling 2026-10-06: a.** Rulings 7 and 12 and DATA-BACKUP-PITR-3 follow it.
2. Where the restore fence directory survives host loss.
   a) A separate volume on the node host plus a write-once off-host copy (object storage with object lock) after each advance. b) The node host only; host loss needs a manual reseed. c) A networked compare-and-set service.
   Recommendation: a. It reuses the file register that exists and adds one copy step.
   **Owner ruling 2026-10-06: b, with a refinement.** The fence directory on the NAS is used only for the testing phase, and the choice is decided again before external players are admitted. Ruling 8, A1, A2, ruling 12 and U7 follow it.
3. Deploy model for alpha.
   a) Stop-the-world only for releases with a migration; rolling per channel otherwise. b) Always stop-the-world. c) Build an N/N-1 schema gate now.
   Recommendation: a.
   **Owner ruling 2026-10-06: a.** Ruling 3 follows it.
4. Admission while the WAL archive alert fires (ruling 7; added in review round 2).
   a) Alert the owner and block releases with a migration; fresh admission stays open in the testing phase, and the choice is decided again with ruling 8 before external players. b) Also close fresh admission on every channel until the probe passes again.
   Recommendation: a. Testing-phase data belongs to no external player, and closing admission turns a backup fault into an outage.
   **DECIDED: a, accepted by control plane under D607 (D838).** Ruling 7 follows it.

### Rejected options

- Flyway, Liquibase, refinery or a second migrator: two ledgers break the exact gate (F3). sqlx is upstream and in use.
- DOWN scripts as rollback: rejected by DUR-02 rule 6. They lose data on lossy changes.
- A rolling mixed-version deploy across a migration now: the exact gate refuses it. A wider gate is new risk with no accepted need.
- WAL-G: workable, but pgBackRest has built-in repository encryption, `verify` and time-target restore. One tool is enough.
- Keeping the fences in the database: a restore rolls them back. DUR-02 rule 5 forbids that.
- Storing the fences in Platform: Platform does not own Game recovery authority (ADR-0012), and it adds a cross-system dependency to every restore.
- Re-executing gameplay after T from logs or journals: forbidden by DUR-02 rule 5 and DUR-03 §41.
- Manual SQL fixes for lost purchases or progress: forbidden by Refinements 2026-08-10.
- Selecting the restore build by ledger digest alone, or by the backup's build annotation: releases without a migration share a digest, and the annotation names the build at backup start, not at T. Ruling 9 step 3 selects the newest activation row in the restored database instead.
- Recording the build on each node registration instead of on a deploy-written activation row: a deploy that has not yet started a node, and `oteryn-game-ops` writes, would leave no record, and ruling 5's bundle check needs the activated set.
- One activation row written before the audited domain migration: a crash or a PITR between the two leaves a newest row with a valid digest on a half-migrated database. Ruling 3 writes `PENDING` first and `ACTIVE` only after the domain migration completes.
- A NAS-loss reseed that sets only the current fence record: reconcile needs every retained record from the restored generation upward and would refuse for ever. Ruling 8 rebuilds and verifies the chain.
- Running the content check on the monthly drill database: rows written after its snapshot escape it. Ruling 5 checks bundles, or the live database with every node stopped.
- Gating on a live check run before the migration: a schema or domain migration can rewrite or leave stored references, so that run proves nothing about the database the build starts on. Ruling 5 gates on the run after every migration.
- Reopening restore admission at the 35 s gate alone, before the Platform reconciliation: DUR-02 rule 5 keeps admission closed until reconciliation passes. Ruling 9 opens it in step 12.
- Pruning the release index by backup age or by activation revision: revisions repeat across timelines after a restore. The alpha keeps every release (U8).
- Selecting the retained revoke, mute, fence raise and hold placement requests by `issued_at` after T: `issued_at` is the issuer's clock, so a lagging clock drops a control committed after T. Ruling 8's replay rule selects by retained outcome records instead.
- Replaying every retained request with no stored outcome in the restored database: a request file is written before submit, so a restore would apply for the first time a request that was never submitted. Replay needs a retained `COMMITTED` outcome record; a request without one stops the restore until a signed resolution (ruling 8).
- Selecting the re-request set by Platform acknowledgement time after T: it compares two independent clocks, so a lagging Platform clock drops a lost delivery while Platform keeps the settled purchase. Ruling 10 selects by Game recovery generation instead.
- Reconciling Platform by a Game write into Platform data: Platform owns its data. It re-requests by operation identity instead.

### Open unknowns

- U1 UNKNOWN. Whether PR #1874 merges. If it does, DATA-BACKUP-PITR-3 owns its deploy paths (F30); if not, the control plane names a new path, because `main` has no deploy directory.
- U2 UNKNOWN. Whether any Platform saga that settles value against a Game outcome (Bazaar, store delivery) will be live at external alpha. If none is, A3/A4 are not blocking for alpha, but the restore notice still is.
- U3 CONFLICT (F21). The Character fence decision is still marked candidate but is implemented. The control plane should confirm its acceptance before DATA-RESTORE-OPS-2 builds on it.
- U4 CONFLICT (F12). DUR-04 is marked PROPOSED in its file but accepted in the reconciliation record. Ruling 5 depends only on rules that the code already follows (F10).
- U5 UNKNOWN. How long a full restore of an alpha-sized database takes. DATA-RESTORE-DRILL-4 measures it and fixes RTO.
- U6 UNKNOWN. Whether Platform can store the Game recovery generation per acknowledged delivery and operation and list them per World by its own cursor, which A3/A4 need. Until it can, the reconciliation lists every acknowledged delivery for the World.
- U7 OPEN (owner re-decision). Where the restore fence directory survives host loss once external players are admitted. Owner ruling 2b covers the testing phase only; external admission waits for the new ruling (rulings 8 and 12).
- U8 UNKNOWN. How large the never-pruned release index grows. DATA-RESTORE-DRILL-4 records its size; a pruning rule needs its own decision before it matters.
- U9 RISK (A3). The restore notice acknowledgement is on the RTO path: a Platform outage keeps Game admission closed and can exceed the 4 h RTO. Recorded as an explicit Platform acceptance item of A3 (PLATFORM-RESTORE-RECONCILE-P1); Game never opens admission without the acknowledgement.

## 4. Client version and update

### Facts

Classes: PROVEN (read in the file), DERIVED (follows from PROVEN facts), UNKNOWN, CONFLICT.

- F1 PROVEN. Protocol major 1, profile 1 `tcp_tls13_alpn_v1`. `schema_revision` is diagnostic,
  never an equality gate; the schema SHA-256 is not a compatibility token. (FND-02 §4)
- F2 PROVEN. `ClientBootstrap.client_build_id` (field 7) is a "bounded diagnostic/build
  identity, never authority", 1..=128 bytes; `ClientResume` carries it as field 8.
  (`docs/contracts/protocol-oteryn/v1/foundation.proto`; FND-02 §11;
  `crates/protocol-oteryn/src/lib.rs` lines 348, 594, 603; `FND02-CLIENT-BUILD-ID-BYTES`)
- F3 PROVEN. The client sends `oteryn-client/<CARGO_PKG_VERSION>`, today `oteryn-client/0.1.0`,
  with no source SHA. (`apps/client/src/lib.rs` line 155; `Cargo.toml` line 41)
- F4 PROVEN. The Gateway login sends the same value as `client_build`, 1..=64 visible ASCII,
  "diagnostics only". (`crates/platform-client/src/native_login.rs` lines 395, 636)
- F5 PROVEN. ARCH-ERROR-CODES-0 build identity is `<version>+<sha12>`, `+<sha12>.local` or
  `+unknown`, from `crates/error-codes/build.rs`; that crate is not on `main` yet (ERR-NODE-1).
  (`reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md` §1.10 item 2, §2.2; working tree)
- F6 PROVEN. Nothing defines a minimum client build or a code that refuses one. 1100–1116 are
  assigned; 1112 `ADMISSION_GRANT_REVISION_UNSUPPORTED` is an unsupported Platform grant profile
  (FND-04A §7 step 8), not a client build. (`PROTOCOL_OTERYN_V1_REGISTRY.json` `error_codes`;
  FND-04A §7, §11)
- F7 PROVEN. N8 sends one `ProtocolError` (1100..1199) before admission, then closes; before
  authentication only 1100 and 1101 are reachable. An unknown code in the range maps to
  `TEMPORARILY_UNAVAILABLE`, `TERMINAL`, no retry (`admission_refusal_class`, implemented).
  1117..1199 are reserved for admission rows; 1200–1999 for future wire rejections.
  (`reviews/OTERYN_GAME_ARCH_LOGIN_FIRST_PACKETS_2026-10-05.md` N8; `crates/protocol-oteryn/src/lib.rs`
  line 292; ARCH-ERROR-CODES-0 §1.2)
- F8 PROVEN. ADMIT-0 (CANDIDATE) checks capabilities after authentication at fresh admission
  (FND-04A §7 step 14), reconnect and recovery, with three `CLIENT_UPDATE_REQUIRED` codes. Its
  set is fixed per ownership generation; node-boot D3 is stop-then-start. The reconnect and
  recovery codes have no wire number. (`reviews/OTERYN_GAME_ADMIT0_WORLD_REQUIRED_CAPABILITIES_DECISION_2026-10-01.md`
  §3.2, §4, §5)
- F9 PROVEN. `PROD-COMPAT-01` (`REQUIRED_FOR_ALPHA`) must decide min/max clients, forced update,
  release channels and signed manifests; gap register §24 also lists delta updates and CDN as
  unresolved. (`GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md`; `ARCHITECTURE_ANALYSIS_GAP_REGISTER.md` §24)
- F10 PROVEN. SEC-CLIENT-01 (CANDIDATE) §3: a release manifest (build id, platform, hash of every
  executable and library, corpus reference) signed by an offline release key under a separate
  offline trust root; OS signatures belong to `EXP-UPDATE-01`. SEC-REL-1 depends on
  "PROD-COMPAT-01's release train"; §4.1 expects a version rule that refuses builds before
  admission. (`reviews/OTERYN_GAME_SEC_CLIENT01_CLIENT_INTEGRITY_AND_ANTI_BOT_DECISION_2026-10-01.md`)
- F11 CONFLICT. FND-02 §11 says `client_build_id` is "never authority"; SEC-CLIENT-01 says it
  "selects only the challenge corpus". A floor uses it to refuse. Ruling 2 resolves this: it may
  refuse and never grant.
- F12 PROVEN. ALPHA-CLIENT-01 (CANDIDATE): no in-place replacement, activation at a safe
  boundary (§17.1); signed-artifact verification before external alpha (§17.2); atomic
  activation, rollback never bypasses compatibility (§17.3); user state separate (§17.4);
  updater technology deferred (§14, §17).
- F13 PROVEN. ADR-0018 is Proposed and does not authorize browser distribution; §9 asks for an
  immutable manifest binding build, asset digest and file digests.
- F14 PROVEN. The server streams the map. Payloads carry `content_generation`, `bundle_digest`,
  `reset_epoch`; a mismatched delta is `STATE_REVISION_MISMATCH` and the client sends
  `ResyncRequest`. `appearance_id` names a 15.30 appearance; 0 draws a placeholder.
  (`docs/contracts/protocol-oteryn/candidates/MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md` §3)
  The server loads a bundle only if digest, checksums, schemas and content revision match the
  World's pins. (`ADR-0021-world-map-runtime-loading.md` §4, line 215)
- F15 PROVEN. The 15.30 assets in `content/assets/files/` are 138 MiB (`du`). The client reads
  the manifest (`imports/official/client-assets/15.30/manifest.json`, 1,158,899 bytes) from a
  path at run time; an unindexed id returns `AssetError::UnknownAppearance`.
  (`crates/client-assets/src/store.rs` line 40; `appearance.rs` line 252)
- F16 PROVEN. The client compiles in `content/movement/step_speed_v1.json`
  (`apps/client/src/input.rs` line 62).
- F17 DERIVED. No workflow publishes client binaries (`.github/workflows/` listing). ADR-0011
  line 146 requires release automation to prevent accidental public distribution; workflow
  changes need owner authorization (D158 pattern).
- F18 PROVEN. The workspace pins `ed25519-dalek = "=2.2.0"` (`Cargo.toml` line 52); `semver`
  1.0.28 is in `Cargo.lock` (line 2744). The repository is public (GitHub API `visibility`).
- F19 PROVEN (external). Velopack has a Rust crate (`velopack`), builds installers and update
  packages, and serves updates from any static host, GitHub Releases included
  (docs.velopack.io). UNKNOWN: deltas, a verify-before-apply hook, CI tooling, install without
  admin rights.

### Rulings

1. **Build identity.** A client build id is `oteryn-client/<semver>`. Release builds use the
   ARCH-ERROR-CODES-0 form `<CARGO_PKG_VERSION>+<sha12>`; local builds use `+<sha12>.local`.
   Both are valid SemVer, and the workspace pins the `semver` crate (`=1.0.28`) to parse them.
   Every client release raises the workspace version. Comparison uses SemVer precedence, which
   ignores the `+` metadata. The same string goes to the Gateway (64 characters at most) and to
   the bootstrap. No wire change.
2. **Client build floor.** A node may set `client_build_floor`, a SemVer version, per transport
   profile. In alpha only profile 1 exists.
   - Unset means no check; development, test and fixture configs leave it unset, so harness ids
     keep working. When set, a profile 1 session is refused if its id does not parse as
     `oteryn-client/<semver>` or its version is below the floor.
   - The floor is fixed for each ownership generation; raising it needs a restart (node-boot D3).
     There is no maximum: a newer build is compatible under FND-02's additive rules.
   - The floor compares a self-reported id, so it is a compatibility gate, not an integrity
     control. Package integrity rests on the updater's checks (ruling 9), including the root
     release attestation.
   - The id may only refuse. It never grants, selects or widens anything. FND-02 §11 is amended
     to say so, which resolves F11. SEC-CLIENT-01 may still use the id to select a corpus.
3. **Where the check runs.**
   - **Fresh admission:** at FND-04A §7 step 11, after authentication, binding and the
     grant-revision checks, and before the nonce step. A refusal is code
     **1117 `ADMISSION_CLIENT_BUILD_UNSUPPORTED`**, with no nonce or authority mutation.
   - The check runs before ADMIT-0's capability check. The floor is a public value (ruling 5),
     so it does not need ADMIT-0's ownership-first order. It is still after authentication, so
     the N8 disclosure rule holds.
   - **Reconnect and recovery:** the same predicate runs on `ClientResume.client_build_id` at
     ADMIT-0's points. The rows are `RECONNECT_CLIENT_BUILD_UNSUPPORTED` and
     `RECOVERY_CLIENT_BUILD_UNSUPPORTED`. They get the next free numbers in 1200–1999 when FND-04B's wire rows
     are registered (1200 and 1201 are assigned by ARCH-LIVE-READINESS-0). Until then a resume refusal stays frameless (N8).
4. **What an older client receives.**
   - A client with N8 but without 1117 shows the generic text with "E1117" and does not retry
     (F7); a client without N8 sees an I/O error. Both are acceptable backstops, but the first
     floor ever set is at least the first build that maps 1117 (VERSION-FLOOR-1 ships it).
   - A client that knows the public class `CLIENT_UPDATE_REQUIRED` (1112, 1116, 1117) runs the
     update check from ruling 5 and offers "Update and restart". It never retries automatically.
5. **"Update required" is announced before login, not on the wire.**
   - At start and before every Gateway login, the client reads its channel's signed release feed.
     If a newer release exists, the client installs it before login. The alpha policy is
     **latest only**: every update is mandatory.
   - An unreachable feed does not block login; the server floor is the backstop.
   - No in-session push: a floor raise restarts the node, and sessions come back through
     admission or recovery. Apart from the 1117 row: no wire change, message or capability.
6. **Map and world content.** The MAP-WIRE-1 binding is the content-version rule (F14): a
   digest or epoch change brings a snapshot, a mismatched delta a resync. The client keeps no map
   bundle and caches no map payload across sessions. ADR-0021 pinning stays. Nothing is added.
7. **Client assets and compiled-in data belong to the build.**
   - The 15.30 asset set ships inside the client release. The server never serves assets; there
     is no asset negotiation on the wire.
   - The client compiles in the asset manifest SHA-256 and refuses another manifest: the asset
     store fails closed with a client error code. The release manifest (ruling 9) covers the
     package, so it covers the assets too.
   - An `appearance_id` missing from the local index draws the placeholder and increments a
     diagnostic counter. It is never fatal.
   - A new asset version, or a server change to data the client compiles in (F16), ships as a new
     client release, with the floor raised in the same rollout.
8. **Distribution for alpha.**
   - Native Windows `x86_64-pc-windows-msvc` only. Every release is a full package; no deltas.
   - The updater is Velopack (`velopack` crate): it installs, downloads and stages, and activates
     only at client restart (ALPHA-CLIENT-01 §17.1, §17.3, §17.4).
   - Feed and packages on GitHub Releases of `Oteryn/Oteryn-Game` (owner ruling 2026-10-06,
     Q1 a). Each release also carries the trust log and a root-signed checkpoint (ruling 9),
     issued with every trust record and every release. One channel, `alpha`; no staged rollout. Rollback is a new, higher version built from the earlier source.
     During a rollout the floor stays at the previous build until the operator raises it.
9. **Update integrity.**
   - Every release carries the SEC-CLIENT-01 §3 signed release manifest, extended with: the
     SHA-256 and size of the full package, the asset manifest SHA-256, the channel, and the
     version.
   - Detached Ed25519 over canonical bytes, with the workspace `ed25519-dalek`. Before
     activation the updater verifies the trust records and checkpoint (below), the manifest,
     the root release attestation, then the package hash, and refuses a version not above its own. Velopack's checks add to
     this; they are not the anchor.
   - **Client trust records and their rollback floor.** Release keys change only through
     SEC-CLIENT-01 §3 trust records `{revision, previous_revision, active_keys, revoked_keys}`
     and checkpoints `{revision, issued_at}`, both signed by the offline trust root. That
     section defines the rollback floor for the node only. Without a client floor, a feed that
     re-presents an older signed record, which still lists a now-revoked key as active, would
     let the client accept a manifest signed by that key. `main` has no client updater or trust
     store yet (`git ls-tree origin/main apps/client/src`), so the client floor is defined here:
     - *Build floor:* each client build compiles in the trust root public key and the trust
       snapshot current at build time: the head record's revision, its active and revoked key
       sets, and the checkpoint that names that head.
     - *Persisted floor:* `trust-floor.v1` in the per-user client state directory
       (`%LOCALAPPDATA%\Oteryn\state\`), outside the Velopack install root, so no update or
       rollback package replaces it (ALPHA-CLIENT-01 §17.4). It holds the highest applied record
       revision with its active and revoked key sets, the highest accepted checkpoint
       `{revision, issued_at}`, and a SHA-256 over those bytes. The effective floor is the
       higher of the persisted and the build floor, field by field; the revoked set is the union
       of both and never shrinks.
     - *Verification:* the updater fetches the trust log and the latest checkpoint published
       with the release (ruling 8). It accepts the checkpoint only if the root signature
       verifies, its revision is not below the floor's checkpoint revision, and its `issued_at`
       is not earlier than the floor's. It applies records in order only: each needs
       `revision = current + 1`, `previous_revision = current`, a valid root signature, and no
       key in `active_keys` that is in the revoked set. A record at or below the floor revision
       is rejected. A log prefix without a checkpoint is never the head. A manifest verifies only
       under a key that is active at the head and has never been revoked.
     - *Durability and monotonicity:* after applying records and before verifying any manifest,
       the updater writes the new floor with write, fsync, atomic rename and directory fsync. It
       never writes a value below the one it replaces. A crash before the rename keeps the old
       file, which only delays the raise.
     - *Failure:* a missing file uses the build floor (a fresh install). A file that fails its
       digest refuses every update until a reinstall; login continues (ruling 5), and the server
       floor (ruling 2) stays the backstop.
     - *Root-bound release (a revocation that older builds keep):* the build and persisted
       floors protect a client that has seen a revocation. They do not protect a fresh install
       of an older retained build, which starts from its own older build floor. The holder of a
       revoked release key who controls the feed could re-present an old root-signed checkpoint
       under which the key was still active, and sign a package with any version. The server
       floor (ruling 2) compares only a self-reported SemVer and would admit it. So a release
       key alone never authorizes a package. With every release the offline root also signs a
       **release attestation** `{channel, version, package_sha256, package_size,
       manifest_sha256, trust_revision}`. Its `trust_revision` is the head record revision the
       manifest verifies under. The updater activates a package only if a root-signed
       attestation names that package's channel, version, exact SHA-256 and size, and its
       manifest's SHA-256, and only if the attestation's `trust_revision` is not above the head
       the client has applied. The attestation's security is the root signature over the exact
       package digest, so a trust record issued after a release (one that revokes another key,
       say) leaves that release installable. This check is in addition to the manifest check, not
       instead of it. A replayed old checkpoint or attestation names only a package the root
       released, never one that a revoked key signs later, so no older build can be moved to
       such a package. `main` has no updater yet, so the first published updater build already
       enforces this. No build without it is published or retained: CLIENT-RELEASE-5 refuses
       to publish one.
     - *Residual risk (accepted, bounded):* the root is offline (owner ruling Q3 a), so the
       client has no online freshness bound like SEC-CLIENT-01's 24 hours. A client that has not
       seen the revocation, facing a feed controlled by the holder of the revoked key, can be
       held at an older release that the root attested. It is never moved to a package the root
       did not attest. The hold ends at the first later checkpoint the client sees, every new
       build carries the revocation in its build floor, and once the server floor is above the
       held release, login shows the update screen (1117). The emergency path is an
       out-of-train client release with a raised build floor plus a raised server floor
       (ruling 2).
   - Offline keys never enter CI: the workflow publishes a draft, the key holder signs the
     manifest, the release attestation, any new trust record and the checkpoint locally, then it is published. Owner
     ruling 2026-10-06 (owner Q3, option a): the owner holds the offline trust root and the
     release key and signs with `oteryn-release-sign`. The root's release attestation is signed
     in that same local session, so the owner's steps do not grow. Owner ruling 2026-10-06 (owner Q2,
     option a): no OS code signing (Authenticode) in the closed alpha; the manifest is the only
     signature, users see a SmartScreen warning at first install, and a purchase is decided
     before open beta.
10. **Browser.** Outside alpha. When a browser profile is registered, its id is
    `oteryn-web/<semver>` with its own floor (ruling 2), and its deployed page and ADR-0018 §9
    manifest replace the updater.
11. **Platform.** Alpha needs no Platform action. The Gateway's `client_build` stays diagnostic,
    and Platform does not gate builds. Owner ruling Q1 a hosts the client on GitHub Releases,
    so Platform has no hosting task.
12. **Before-freeze coverage.** Concurrency: the floor is fixed per ownership generation.
    Resume: a staged release activates only at restart, so an interrupted download keeps the
    prior one; a crash while applying trust records never lowers the persisted trust floor
    (ruling 9). Typed references: the manifest names build id, package digest and asset manifest
    digest. Older-client gating: rulings 3 and 4; an older build is moved only to a package the root
    attested (ruling 9). Multi-component commit: publish the trust log and checkpoint, then
    package, then signed manifest and root release attestation, then feed entry, then raise the
    floor.

### Contract amendments

1. **`docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md` §11**, in the `client_build_id`
   sentence. Replace "bounded diagnostic/build identity, never authority" with: "bounded
   build identity. It never grants, selects or widens authority. A node may refuse it against
   its client build floor (ARCH-ALPHA-OPS-0 §4 ruling 2); it may also select a challenge corpus
   (SEC-CLIENT-01)."
2. **`docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` `error_codes`**: add
   `{"code": 1117, "name": "ADMISSION_CLIENT_BUILD_UNSUPPORTED", "category": "UNSUPPORTED_REVISION", "default_disposition": "TRANSPORT_FATAL", "progression": "TERMINAL", "public_class": "CLIENT_UPDATE_REQUIRED"}`.
3. **`docs/architecture/FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT.md` §7 step 11.** Append:
   "; then, when the node sets a client build floor for the transport profile, the bootstrap's
   `client_build_id` parses as `oteryn-client/<semver>` with a version at or above the floor ->
   else `ADMISSION_CLIENT_BUILD_UNSUPPORTED`".
   **§11**, a new row:
   `| ADMISSION_CLIENT_BUILD_UNSUPPORTED | UNSUPPORTED_REVISION | TERMINAL | a client build at or above the floor; a new grant | no nonce/authority mutation | CLIENT_UPDATE_REQUIRED | fresh admission client build below the supported floor | the floor version, after authentication (step 11) |`
4. **`docs/architecture/FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT.md` §25**: two rows,
   `RECONNECT_CLIENT_BUILD_UNSUPPORTED` (mutation `CURRENT_AUTHORITY_PRESERVED`) and
   `RECOVERY_CLIENT_BUILD_UNSUPPORTED` (`NO_AUTHORITY_MUTATION`). Both are
   `UNSUPPORTED_REVISION`, `TERMINAL`, `CLIENT_UPDATE_REQUIRED`, and both are numbered when the
   FND-04B wire rows are registered.
5. **`MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md` §3, Appearance.** Add: "An `appearance_id` that
   the client's pinned asset index does not hold is drawn as the placeholder and counted. It is
   never fatal."
6. **`ALPHA-CLIENT-01_NATIVE_CLIENT_ARCHITECTURE_CONTRACT_CANDIDATE.md` §17.4.** Replace
   "Installer/updater technology remains deferred." with: "The alpha installer and updater is
   Velopack with a full package per release, verified against the signed release manifest before
   activation (ARCH-ALPHA-OPS-0 §4 rulings 8 and 9)."
7. **SEC-CLIENT-01 §3, Release manifest.** Append: "The manifest also binds the full update
   package SHA-256 and size, the client asset manifest SHA-256, the release channel and the
   SemVer version. The client updater verifies it before activation (ARCH-ALPHA-OPS-0 §4 ruling 9).
   The client keeps its own trust-record rollback floor: each build embeds the trust snapshot of
   its build time, and the updater persists the highest applied revision, the revoked key set
   and the highest checkpoint outside the install root and rejects any record or checkpoint
   below that floor. Each release also carries a trust-root-signed release attestation naming
   the channel, version, exact package SHA-256 and size and manifest SHA-256; the updater
   activates no package without one (ARCH-ALPHA-OPS-0 §4 ruling 9)."

### Packets

- **VERSION-FLOOR-1** (protocol; protocol review). Builds the build id format (ruling 1),
  `client_build_floor` in node config, code 1117 on server and client, and the client's
  `CLIENT_UPDATE_REQUIRED` screen. Paths: `apps/client/src/lib.rs`;
  `apps/game-server/src/node/config.rs`, `foundation/fnd04_verifier.rs`,
  `gameplay_transport/connection.rs`; `crates/protocol-oteryn/src/lib.rs`;
  `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`; FND-02 §11; FND-04A §7, §11; workspace
  `Cargo.toml` (`semver`). Tests: unset floor admits a free-form id; a version below the floor
  and an unparseable id each give 1117; at the floor admits; `+meta` ignored; a bad
  signature still gives 1101; no nonce consumed; the client maps 1117 to the update screen.
  Order: after ERR-NODE-1 and N8.
- **VERSION-ASSET-PIN-2** (client; impl). Compiled-in asset manifest digest; placeholder and
  counter for unknown ids. Paths: `crates/client-assets/src/store.rs`, `appearance.rs`;
  amendment 5. Tests: a changed manifest byte fails closed; an unknown id draws the placeholder
  and is counted; the real 15.30 manifest passes. Order: in parallel with 1.
- **RELEASE-MANIFEST-3** (security; security review). New crate `crates/release-manifest`: the
  ruling 9 format, canonical bytes, Ed25519 and trust record verification, and the local signing
  tool `oteryn-release-sign`. With SEC-REL-1 accepted, this is its manifest half, one owner.
  It also holds the trust-floor logic of ruling 9 as a pure library, and the signing tool
  issues the checkpoint. Tests: tamper with each field, a revoked key, a version not above the
  current one, a wrong root; after revocation record n+1, re-presenting record n is rejected and
  a manifest signed by the revoked key fails; a revision gap or a wrong `previous_revision` is
  rejected; a checkpoint with a lower revision or an earlier `issued_at` than the floor is
  rejected; a log prefix without a checkpoint is not the head; a record that reactivates a
  revoked key is rejected; the release attestation format, its root signature and its signing
  by `oteryn-release-sign`; a package and manifest valid under a revoked key but named by no
  root attestation are rejected; a replayed old attestation names only its own package, so a
  substituted package fails; an attestation for another channel, version, size or manifest, or
  with a `trust_revision` above the applied head, is rejected; a trust-only record issued after
  a release leaves that release installable. Order: with or after SEC-CLIENT-01 acceptance.
- **CLIENT-UPDATE-4** (client; impl, hard). A spike gate first closes the F19 UNKNOWNs; then the
  Velopack integration, the pre-login feed check, the persisted trust floor and verification
  before activation. Paths: `apps/client/src/main.rs`, `apps/client/src/update.rs` (new),
  `apps/client/src/trust_floor.rs` (new), `apps/client/Cargo.toml`. Tests: a tampered package
  is not activated; an interrupted download keeps the prior release; an unreachable feed lets
  login continue; user settings survive; after the revocation record is applied and the client
  restarts, a re-presented older record is rejected and a higher-version package signed by the
  revoked key is not activated; a fresh install of a build whose build floor predates the
  revocation, given the old checkpoint that lists the revoked key as active and a
  higher-version package and manifest signed by that key, activates nothing, because no root
  attestation names the package; the same install updates to a root-attested release; a missing floor file uses the build floor; a file below the
  build floor is raised to it; a corrupt file refuses updates while login continues; a kill
  between apply and rename never lowers the floor; an update or rollback package leaves the
  floor file unchanged. Spike failure fallback: a full
  zip, the same verification, and a side-by-side directory switch at restart. Order: after 3.
- **CLIENT-RELEASE-5** (CI; owner workflow authorization). New
  `.github/workflows/client-release.yml`: manual dispatch only, `OTERYN_BUILD_SHA` from an exact
  checkout, the package and a **draft** release; publishing needs the signature and the root
  release attestation (ruling 9), and the workflow refuses to publish a build whose updater
  lacks the attestation check (CLIENT-UPDATE-4's test passes on that exact checkout).
  Order: after 4, before the first external alpha release.

### Owner questions

1. Where the alpha client is hosted (Platform responsibility versus Game).
   a) GitHub Releases of the public `Oteryn/Oteryn-Game` repository, free. b) A Platform
   download page and storage, a cross-repo Platform task. c) Game-owned object storage or a CDN,
   which costs money. **Recommend a.**
   **Owner ruling 2026-10-06: a.** Rulings 8 and 11 follow it.
2. Windows code signing (spending).
   a) None for the closed alpha: the Oteryn signed manifest only, with a SmartScreen warning at
   first install, and a purchase decided before open beta. b) Buy a code-signing certificate or
   service now; the cost is UNKNOWN until quoted. **Recommend a.**
   **Owner ruling 2026-10-06: a.** Ruling 9 follows it.
3. Custody of the release key (production authority).
   a) The owner holds the offline trust root and the release key, and signs each release locally
   with `oteryn-release-sign`. b) The release key is a secret in a protected GitHub environment
   with owner approval. This conflicts with SEC-CLIENT-01's offline key. **Recommend a.**
   **Owner ruling 2026-10-06: a.** Ruling 9 follows it.

### Rejected options

- **Delta updates for alpha.** The asset set changes rarely; a full package is the minimum.
  Revisit when measured package size and release cadence show a cost.
- **Announcing the floor on the wire** before authentication or in `ServerAccepted`: a new field
  with older-client gating and a pre-authentication disclosure. The feed and 1117 cover it.
- **An in-session "update required" push.** A floor raise already restarts the node.
- **Gating builds by capability ids.** Capabilities name features; ADMIT-0 covers feature gaps.
- **The schema SHA or `schema_revision` as a version token.** FND-02 §4 forbids it.
- **A maximum client version.** Newer clients are compatible under the additive rules.
- **The server serving assets** to the native client: a new wire domain for no alpha gain.
- **TUF (`tough`).** One channel and one key holder do not need its roles; the SEC-CLIENT-01 key
  hierarchy already gives a root and revocation, and ruling 9 adds the client rollback floor.
- **An online timestamp key (a TUF timestamp role) for client freshness.** It needs a new
  key and a signing host the feed holder does not control. The root release attestation already
  keeps a revoked key from installing code on any build; what remains is a hold at an attested
  release, which the server floor makes visible (ruling 9).
- **Trusting the newest presented trust record without a persisted floor.** An older signed
  record re-presented after a revocation would trust the revoked key again (ruling 9).
- **The `self_update` crate.** It replaces the running executable, which ALPHA-CLIENT-01 §17.1
  forbids (DERIVED from the crate's stated purpose; not verified here).
- **A Platform Gateway build gate.** A second authority over the same rule.

### Open unknowns

- Velopack: a verify-before-apply hook, package file access, the CI packaging tool, per-user
  install without admin rights. CLIENT-UPDATE-4's spike closes them.
- Package size about 140 MiB (CANDIDATE: 138 MiB of assets plus the binary); measure the first
  package. Updater package bound 512 MiB (CANDIDATE), fixed after that measurement.
- Feed check timeout 5 s (CANDIDATE); measure against GitHub Releases latency.
- Whether Velopack's uninstall or repair touches `%LOCALAPPDATA%\Oteryn\state\`. The
  CLIENT-UPDATE-4 spike checks it; if it does, the trust floor moves to a path it never touches.
- How often the floor must rise depends on how often compiled-in data (F16) and assets change.
  Unmeasured.
- Wire numbers of the FND-04B build rows: open until FND-04B's wire registration.
- SEC-CLIENT-01 and ADMIT-0 are CANDIDATE; RELEASE-MANIFEST-3 and amendment 7 wait on
  SEC-CLIENT-01's acceptance.

## 5. Checklist and owner items

### 5.1 Before-freeze checklist

- Concurrent transitions: the ops lock around deploy and restore, the migrator's advisory lock,
  the fence CAS, the activation high-water and the epoch row lock (§3); the client build floor is fixed per ownership generation (§4 ruling 12); metrics and
  log writers never change an outcome (§1 ruling 8).
- Restart and resume: rotating log segments and a lifecycle file per incarnation (§1 ruling 6);
  timer reload by remaining duration or database-time deadline (§2 ruling 6); a deploy resumes
  from the live ledger and the `PENDING`/`ACTIVE` activation phase, and a restore per step through
  the restore journal and the event id on the fence record (§3 rulings 3 and 9); a staged client update activates only at restart, and
  the client trust floor never moves down (§4 ruling 9).
- Typed cross-record references: `parent`, `attempt` and `traceparent` are diagnostic only
  (§1); content references are `{family, key, revision}`, checked on bundles or on the stopped live
  database after every migration completes, never on an old snapshot (§3 ruling 5); a restore selects its build from the
  restored activation rows (§3 ruling 9 step 3); the release manifest
  names build id, package digest and asset manifest digest, and a root release attestation
  binds the package to the trust root (§4 ruling 9).
- Older client and peer gating: no wire change except 1117 (§4 rulings 3–4); a restore forces
  a fresh login (§3 ruling 9 step 10).
- Gate order: no check gates a state that a later step still changes, and no node registers and
  no admission opens before every reconciliation completes (§3 ruling 3 live check, ruling 9
  steps 11–12).
- Multi-component commit and recovery: fences live outside the snapshot (§3 ruling 8), on a
  NAS volume only for the testing phase with a re-decision before external players, and a
  NAS-loss reseed rebuilds the retained fence chain from the restored row H; Platform
  re-requests by operation identity (§3 ruling 10); a release publishes the trust log and
  checkpoint, then package, then signed manifest and root release attestation, then feed
  entry, then raises the floor
  (§4 ruling 12).
- No new authority: every identifier added here is diagnostic. The only new player-facing
  refusal is the build floor; the activation check and the ops lock refuse only boot and
  operator actions.

### 5.2 Owner items

The owner ruled on every item of the first round on 2026-10-06. Every item is ruled **a**, except §3 Q2, which is
ruled **b** with an owner refinement: the fence directory on the NAS is used only for the testing
phase, and the choice is decided again before external players are admitted (§3 ruling 8, U7).
§3 Q4 was added in review round 2 and is DECIDED: a, accepted by control plane under D607 (D838).

| Item | Question | Options | Recommendation | Ruling 2026-10-06 |
|---|---|---|---|---|
| §1 Q1 | Where metrics are scraped and alerts sent | a NAS Prometheus/Grafana, email; b no scraper; c hosted (spend) | a | a |
| §1 Q2 | Alpha log and metric retention | a 14/30 days; b 30/90 days; c delete per phase | a | a |
| §2 Q1 | Measured capacity below D128's 500 | a accept and log the gap; b block the claim | a | a |
| §2 Q2 | Making the bench job a required check | a report first, then required; b required now; c nightly | a | a |
| §3 Q1 | Recovery targets | a RPO 5 min, RTO 4 h, 14 days, monthly drill; b 1 min/1 h/30 days/weekly; c nightly dump | a | a |
| §3 Q2 | Where the restore fence directory survives host loss | a separate volume plus write-once off-host copy; b host only; c CAS service | a | b, testing phase only; decided again before external players |
| §3 Q3 | Alpha deploy model | a stop-the-world only with a migration; b always; c N/N-1 gate now | a | a |
| §3 Q4 | Admission while the WAL archive alert fires (review round 2) | a alert and block migrations, admission open in testing; b also close admission | a | DECIDED a, accepted by control plane under D607 (D838) |
| §4 Q1 | Client hosting | a GitHub Releases; b Platform page; c CDN (spend) | a | a |
| §4 Q2 | Windows code signing | a none for closed alpha; b buy now (spend) | a | a |
| §4 Q3 | Release key custody | a owner holds offline keys, signs locally; b CI secret | a | a |
