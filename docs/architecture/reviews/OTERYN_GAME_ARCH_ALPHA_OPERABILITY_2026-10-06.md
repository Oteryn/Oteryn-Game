# ARCH-ALPHA-OPS-0: alpha operability (observability, time and performance, data continuity, client version)

- Decision id: ARCH-ALPHA-OPS-0.
- Status: the rulings of §1–§4 and the packets are accepted on merge. The owner ruled on
  2026-10-06 (item 1a) that the alpha operability package is authored as one decision set.
  Every contract amendment below is exact text marked pending: it is applied by the named
  packet, because the owning file is a candidate, because the control plane leases registry
  numbers, or because the target is an accepted cross-repository contract
  (`CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`) that needs control-plane and Platform review.
  The owner items of §5.2 are pending; each ruling that depends on one names it, and the
  recommended option is the working assumption until the owner answers.
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
  restore fence directory; its erasure journal is re-applied by §3 ruling 9 step 7.

## Implementation brief

1. Logs stay the ERR-CODES key=value line; `tracing` stays rejected. Levels follow §1 ruling 1;
   no movement history at any level.
2. A resumed connection logs `parent=<trace>`; the admission-outcome line logs
   `attempt=<attempt_ref>`, never the `jti` (§1 rulings 2–3). Both are UUIDv7 and grant nothing.
3. Node→Platform calls send W3C `traceparent` built from the trace (§1 ruling 4).
4. Durability failures carry a 3xxx code and the caller's trace; the DB `application_name` is
   `oteryn-game-server/<version>+<sha12>` (§1 ruling 5).
5. The supervisor writes `log/node-<utc-start>.log` with a `node.log` pointer and prunes by age
   at start, so two restarts no longer erase the panic line (§1 ruling 6, F18).
6. Retention CANDIDATE: logs 14 days, metrics 30 days, bundles until the issue closes (owner 1).
7. Metrics: `metrics` + `metrics-exporter-prometheus` on a loopback listener; the §1 ruling 9
   set only; no player, session or item labels.
8. Alerts live in `deploy/observability/oteryn-alerts.yml`; SLOs have no numbers until measured.
9. `oteryn-game-ops diagnose --bundle` writes an all-or-nothing evidence directory, never
   reading the DB or Platform (§1 ruling 12).
10. There is no global tick. Owners run a work queue and a timer lane; every periodic timer is
    a RESOURCE_LIMITS row with its catch-up class (§2 rulings 1–2).
11. Gameplay reads time only through `OwnerClock`; each rule uses exactly one of four clocks
    (§2 rulings 3–4). Durable time is a remaining duration or a database-time deadline.
12. Hosts run chrony; skew raises an alarm with a 2xxx code and never refuses readiness;
    fencing never compares cross-host time (§2 ruling 7).
13. The current build hosts one channel per node and at most 256 connections; capacity is
    measured by the harness load mode on a 4 vCPU reference, at least 3 repeats, and recorded
    as `PERF01-PLAYERS-PER-CHANNEL` (§2 rulings 9–11). D128's 500 is a target.
14. iai-callgrind benches run as a path-selected job that reports before it gates (§2 ruling 12).
15. sqlx `migrate!` is the only migration tool; merged files never change; new versions exceed
    the highest on `main`; the gaps below 0079 stay gaps forever (§3 ruling 1).
16. One release is one schema version. A release with a migration is stop-the-world: close,
    drain, checkpoint, stop, named backup, migrate, start, validate, open (§3 ruling 3).
17. Durable rows reference content only by `{family, key, revision}`; a referenced definition is
    never removed without a DUR-04 §12 policy (§3 ruling 5).
18. pgBackRest with continuous WAL, weekly full, daily differential and a named pre-migration
    backup; encrypted, off the DB host (§3 ruling 7).
19. The restore fence directory holds every value a restore must not roll back: Character
    recovery fence, LCFA F, `assignment_epoch` high-water and the erasure journal (§3 ruling 8).
20. `oteryn-game-ops restore` runs the twelve steps of §3 ruling 9 in order, re-entrant, with a
    35 s admission gate; clients log in fresh.
21. After a Game restore Platform re-requests by operation identity; Game never writes back
    (§3 ruling 10). Progress after T is lost and never replayed (§3 ruling 11).
22. The client build id is `oteryn-client/<semver>+<sha12>`; a node may set
    `client_build_floor`; below it fresh admission returns 1117
    `ADMISSION_CLIENT_BUILD_UNSUPPORTED` at FND-04A §7 step 11 (§4 rulings 1–3).
23. "Update required" is a latest-only signed feed read before login; assets ship inside the
    build; Velopack installs; an Ed25519 release manifest with offline keys anchors the update
    (§4 rulings 5–9). No browser in alpha.
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
6. **Log storage.** Logs stay on the alpha host's persistent volume as stderr captured by the supervisor (F16), and nothing is shipped off the host in the alpha.
   - The supervisor writes each incarnation to `log/node-<utc-start>.log` and points `log/node.log` at it, so the F16 health greps keep working.
   - At start it deletes logs older than the retention value (R7).
   - The node writes each line with one locked write, so lines do not interleave.

   This resolves F18.
7. **Retention.** The alpha retention of host logs, metrics and debug bundles is **CANDIDATE**: logs 14 days, metrics 30 days, bundles deleted when their issue is closed. Owner question 2 fixes these values within DATA-PRIVACY-01 and ANL-01 §16. A legal or incident hold is explicit and per bundle. No log is kept without a bound.
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
   - `log_lines_total{level,code}`, which lets alerts see coded failures without log shipping.

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
    - `OterynAdmissionRefusals` and `OterynPlatformErrors`, as ratios.

    Every `for:` duration and every ratio threshold is CANDIDATE, set from the first alpha week. There is one dashboard JSON in `deploy/observability/`. Both run only under owner question 1 (a).
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
- **OBS-DEPLOY-4** (supervisor logs, alerts, dashboard). Depends on #1874 merged and OBS-METRICS-3.
  - Builds R6, R7 and R10. Owned paths: `deploy/synology-game/supervisor.sh`, `deploy/synology-game/node.toml.template`, `deploy/synology-game/README.md`, and a new `deploy/observability/`.
  - Tests:
    - a shell test that two restarts keep both earlier logs and that the age prune removes only older files;
    - `promtool check rules` and `promtool test rules` on fixture series;
    - the dashboard JSON parses.

  The alerts and dashboard are built only on owner 1(a).
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
2. Alpha retention; this is privacy scope. Logs hold the trace, `attempt_ref` and scope, and Platform can link `attempt_ref` to an account.
   - (a) CANDIDATE: logs 14 days, metrics 30 days, bundles until the issue closes.
   - (b) Logs 30 days, metrics 90 days.
   - (c) Delete everything at the end of each alpha phase.

   Recommend (a).

### Rejected options

- `tracing`, `tracing-subscriber` or OpenTelemetry spans and an exporter: they rewrite every call site, and three hops are joined by one UUID already (ERR-CODES §3).
- JSON log lines or Loki/Vector shipping in the alpha: there is one host and a handful of testers, the key=value line is greppable, and `EXP-OBS-01` is Track 4.
- The `prometheus` crate instead of `metrics`: it is a heavier registry API, and `metrics` is the facade the ecosystem shares.
- `/metrics` on the control socket: Prometheus cannot scrape a Unix socket.
- `/metrics` on a public interface or with auth: loopback needs neither.
- Per-transaction `SET application_name` with the trace: it adds a statement on the durability path and puts correlation data in DB logs.
- `postgres_exporter` in the alpha: node-side DB latency and errors cover the failure modes. It can be added under 1(a) without a decision.
- A new `oteryn-debug-bundle` tool: ERR-DIAG-4 already owns log reading.
- A `trace` or `attempt` metric label: it has unbounded cardinality (ANL-01 §18).
- Off-host collection of node crash evidence now: a persistent NAS volume meets the DISCONNECT §5 non-ephemeral rule for the alpha.

### Open unknowns

- U1: alpha log volume per tester-hour at `info`. It is measured in the first alpha week and sizes the R7 prune. The node has no size cap within one incarnation.
- U2: whether #1868 keeps the exact `registered`, `node_id=` and `readiness ready=true` tokens that the #1874 health check greps (F22). OBS-LOG-1 locks them with a test.
- U3: whether Synology Container Manager host networking can reach a loopback listener (owner 1a). Fallback: bind to the host's LAN address behind its firewall. That needs an amendment to R8.
- U4: whether Platform accepts the R11 proposal. The Game side works without it, and Gateway↔node joins stay node-side through `attempt`.
- U5: the R10 thresholds and the PERF-01 latency budgets. No value is accepted here.
- Before freeze:
  - Serialization: there is one locked line write, and metrics are atomic counters that are not on the authority path.
  - Restart: metrics reset (ADR-0006), each incarnation has its own log, and `parent` is in memory only.
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
   `max_players_per_channel`, and `max_players_per_world` is that value times the number of
   Channels. ADR-0009 §2 and ADR-0015 still allow several Channels per GameNode (F21). When a
   build hosts more than one, the three ADR-0009 §6 limits are measured separately by PERF-01. This
   ruling does not amend either ADR. Until a value is measured, the alpha ceiling is the registered
   256 connections (F19). D128's 500 per Channel is the target that the measurement is checked
   against (F18). No per-cycle microsecond budget is accepted here. The owner-cycle p99 and
   queue-age objectives are CANDIDATE until PERF-ALPHA-1 measures them.
10. **How the value is measured.** PERF-ALPHA-1 extends `tools/synthetic-client-harness` with a
    load mode. It runs N headless `protocol-oteryn` clients that drive the ADR-0009 §6 workload
    classes, with fixed seeds, against one real node and PostgreSQL. N rises in steps until the
    first objective breaks. The run uses the 4 vCPU reference class (MAP-SPIKE-0, F22), at least
    3 repeats, and a release build. The evidence file under `docs/agents/evidence/` records:
    - command, commit, machine and seeds;
    - p50/p95/p99 owner service time and queue age;
    - CPU per core from `schedstat`;
    - the first violated objective and the breaking N.
11. **How a measured value is accepted.** The accepted `max_players_per_channel` is the breaking
    N divided by 1.3, rounded down (ADR-0009 §6 headroom). It becomes a RESOURCE_LIMITS_REGISTRY
    row whose notes cite the evidence file. The row ships in a PR that passes the normal
    independent review. If the value is below D128's 500, the owner decides (owner question 1). A
    hardware change or a regression gate failure reopens the row.
12. **CI benchmark gate (PERF-01 CI part).**
    - *Benchmarks:* instruction counts with `iai-callgrind`, an upstream crate on valgrind. The
      first set has three benches: the viewport snapshot, the viewport delta, and one seeded
      owner cycle of creature think plus a step with visibility.
    - *Where it runs:* a path-selected merge-gate job on `ubuntu-24.04`. It runs when
      `apps/game-server/src/gameplay_transport/**`, `apps/game-server/src/ai_think.rs`,
      `crates/protocol-oteryn/**` or `crates/simulation-determinism/**` changes. It builds the
      merge base and the head in the same job and compares the two counts.
    - *Noisy runners:* instruction counts do not depend on runner speed, which is why they gate.
      Wall-clock benches never gate a PR. They run as `#[ignore]` release tests and in the
      harness, on the reference class, nightly or on demand (ADR-0007 line 285).
    - *Threshold:* CANDIDATE. PERF-CI-1 runs the benches 20 times on `main`, records the spread,
      and proposes the threshold.
    - *Gate status:* the job reports for its first 20 merges. Making it a required check changes
      repository protection, so the control plane routes that change (owner question 2).

### Contract amendments

- A1. `docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md` §8. Add §8.5 "Clock sources
  (ARCH-ALPHA-OPS-0 §2 ruling 4)" with the four-clock list of ruling 4. Add one sentence to §9: "A
  durable timer stores a remaining duration or an absolute database-time deadline, and its
  decision names which."
- A2. `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`. Add a row `WORLDINT0-RL-19`: unit
  `milliseconds`, `hard_maximum` 1000, failure category per the registry vocabulary, notes
  "CANDIDATE with WORLD-INTERACTION-0; alarm only". Add `PERF01-PLAYERS-PER-CHANNEL` only from
  PERF-ALPHA-1 evidence (ruling 11).
- A3. Node deployment runbook (the OPS-NODE-BOOT-01 operator section). Add: "Run a host time
  daemon (chrony recommended) on every GameNode and database host. The node alarms above
  `WORLDINT0-RL-19`."
- A4. ARCHITECTURE_ANALYSIS_GAP_REGISTER.md §25. Mark tick/scheduling as decided (ruling 1).
  Mark capacity method and CI gate as decided, with values pending PERF-ALPHA-1 and PERF-CI-1.

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
- **PERF-ALPHA-1.** The harness load mode and the first capacity evidence (rulings 10-11).
  Owned: `tools/synthetic-client-harness/src/load/` (new), `tools/synthetic-client-harness/src/main.rs`,
  `docs/agents/evidence/PERF-ALPHA-1-capacity.md` (new). Tests: a 2-client smoke test in CI; the
  full ramp runs on the reference class only. Depends on TIME-CLOCK-1 for queue-age stamps.
- **PERF-CI-1.** iai-callgrind benches and the path-selected job (ruling 12). Owned:
  `apps/game-server/benches/` (new), `apps/game-server/Cargo.toml`, `Cargo.toml` (dev-dependency),
  `.github/workflows/merge-gate.yml`, `docs/agents/evidence/PERF-CI-1-noise.md` (new). Tests:
  the job fails on a deliberate synthetic regression in a test branch. Dependency review covers
  the new dev-dependency. Depends on nothing.
- **PERF-ACCEPT-1.** Writes `PERF01-PLAYERS-PER-CHANNEL` from the PERF-ALPHA-1 evidence, then
  applies A4. Owned: `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, the gap register.
  Depends on PERF-ALPHA-1 and owner question 1.

### Owner questions

1. Context: alpha's ceiling is 256 connections, and the D128 target is 500.
   a) Accept the measured value for alpha even below 500, and log the gap (recommended).
   b) Block the representative-load alpha claim until 500 is measured.
2. Context: making a new check required changes repository protection.
   a) The job reports first and becomes required after PERF-CI-1 sets the threshold (recommended).
   b) Required from the start.
   c) Nightly only, never a PR check.

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
- Writing an NTP client or time daemon. The host daemon is the upstream solution.

### Open unknowns

- U1. The PR objective values for owner service time and queue age. PERF-ALPHA-1 proposes them.
- U2. Whether the GitHub `ubuntu-24.04` runner matches the 4 vCPU reference class for this
  repository's plan. If it does not, reference runs go to a named machine, which is a spend
  question for the control plane.
- U3. The iai-callgrind threshold (PERF-CI-1).
- U4. Status mismatches. FND-03's implementation column says IMPLEMENTED in the programme status
  register (line 51) and NOT_STARTED in the decision register (line 34). SIM-DETERMINISM-01's
  header says PROPOSED and the register says ACCEPTED.
- U5. Map views take 0.41-0.44 of a core against the 0.20 default (F22). This consumes the
  Channel's budget before any other workload. ARCH-MAP-VIEWPORT-BUDGET-V1 owns the fix, and the
  PERF-ALPHA-1 run must include it.

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

### Rulings

1. **One tool, one ledger.** sqlx `migrate!` stays the only migration tool. Releases add files and never edit or delete one. A new migration version must be greater than the highest version on `main`. The gaps up to 0079 stay gaps forever. (F1, F5, F6, F7)
2. **A release is one schema version.** The exact-ledger gate (F3) stays. Alpha supports no mixed-version window. This satisfies DUR-02 rule 6, which tests only the windows a release contract declares. A rolling N/N-1 window needs a later decision on a prefix-plus-declared-expand gate.
3. **Deploy shape.** A release with no new migration rolls channel by channel, using the ADR-0009 §8 sequence for each channel. A release with a new migration is stop-the-world for every node on that `oteryn_game` database. The order: close admission on every channel; drain; checkpoint; stop all nodes; take a named backup (ruling 7); run `oteryn-game-migrate`; start the new binaries; validate; open. Rollback after a migration is never a DOWN script. It is roll-forward, or a restore to the named pre-migration backup (ruling 9).
4. **Expand/contract still applies.** Every migration is written as expand/contract, so that a roll-forward fix is always possible. A CONTRACT step ships in a later release than the cutover that stopped using the old shape.
5. **Persisted content references.** Durable rows refer to content only by `{family, key, revision}` (F10). A compiled or legacy numeric id is never persisted. A release may not remove, rename or reinterpret a definition that live rows reference, unless the change carries a DUR-04 §12 class and, for `EXPLICIT_DATA_MIGRATION` or `REMOVED_WITH_EXPLICIT_POLICY`, an audited domain migration. A referenced registry row is never deleted (F14 pattern). The release gate checks this against a restored copy of real data (packet DATA-CONTENT-REF-5).
6. **Content artifacts and receipts.** World bundles and reference artifacts are immutable and content-addressed. A changed input gets a successor receipt that names what it supersedes, as #1869 did (F13). A node logs its bundle digest and its ledger head on its boot line. `oteryn-game-ops diagnose` reports both.
7. **Backups.** Use pgBackRest (upstream, not forked). Archive WAL continuously. Take a full backup weekly, a differential daily, and a named backup before every schema release. The repository is encrypted (`repo-cipher-type=aes-256-cbc`). Its key and credential are separate from the database credentials and never in the repository. It lives off the database host. HA replicas are not backups. All numbers here are CANDIDATES (Owner question 1).
8. **The restore fence directory.** One directory, outside the database volume and outside every database backup, holds every value a restore must not roll back: the Character recovery fence (F19), the LCFA high-water F (F22), the `assignment_epoch` high-water (F23) and the erasure journal (step 7 of ruling 9; ARCH-LIVE-READINESS-0 §3 ruling 18). Each keeps its own contract semantics. Writes use write, sync, atomic rename and directory sync, as the Character fence does today. Each advance is also copied to an off-host write-once store (Owner question 2). A missing or unreadable value is a refusal, never 0.
9. **Restore runbook (normative order).** A restore is one operator procedure, `oteryn-game-ops restore` (packet DATA-RESTORE-OPS-2):
   1. Close admission on every channel. Stop every node, the LCFA publisher and the status reporter. Verify that no pre-restore process is alive.
   2. Restore with pgBackRest to a named backup and a target time T (`--type=time`). Record the backup label and T.
   3. Start the database with admission and mutation closed. Run the schema gate (F3). A mismatch stops the restore (`SchemaIncompatible`).
   4. Advance the Character recovery fence by CAS (`begin_recovery`). Then run `reconcile_character_recovery`. Restored rows are history, not authority.
   5. Revoke every node registration that is CURRENT in the restored snapshot. New nodes register under fresh NodeIds. All launch and scope authorizations are issued again.
   6. Run the DUR-03 §41 validation and the DUR-02 rule 5 list. Any failure keeps the affected mutation closed until an audited repair.
   7. Re-apply the erasure journal (ARCH-LIVE-READINESS-0 §3 ruling 18). Every erasure recorded after T is applied again before any authority opens. The journal lives in the restore fence directory (ruling 8); a missing or unreadable journal is a refusal.
   8. Raise the LCFA projection epoch strictly above F. Raise `assignment_epoch` strictly above its high-water. Persist both before any publisher or reporter starts.
   9. Drop all Premium evidence caches. No Premium benefit applies until a pull made after the restore succeeds (F26, F29). Restored fence rows are only lower bounds.
   10. Open admission only when at least 35 s (30 s grant lifetime + 5 s skew, F28) have passed since step 4. The command checks this itself.
   11. All pre-restore Game Sessions, leases, transports and reconnect material are dead. Clients log in fresh. The server sends a full snapshot (ADR-0009 §9).
   12. Start the publishers. Publish the Platform reconciliation notice (ruling 10).
   Each failed step exits with an ops status of the model decision and a code registered in block 6000–6999. A fence refusal uses block 3000–3999. The run is re-entrant: a repeated run after a crash resumes at the first step that is not done, and never advances a fence twice for one restore event.
10. **Platform state after a Game restore.** Platform state is not restored with the Game DB. Game is the authority for Character facts. Platform is the authority for accounts, coins and purchases.
    - Character list: the epoch raise invalidates every Platform entry from older epochs (LCFA §5). Characters created after T disappear from Platform's view. Nothing is written back.
    - Runtime routing: the `assignment_epoch` raise invalidates all older runtime state in Platform (status producer §6).
    - Purchases and deliveries: after a Game restore, Platform must re-request every delivery whose Game acknowledgement it received after T, with the same delivery identity. Game delivers through the Character Inbox, which is idempotent per `(item, cause)` (`0076:46`). A delivery that survived the restore answers with the existing receipt. A lost one is delivered once. This is a proposal to Platform under PROD-ENTITLEMENTS-01.
    - Ownership and lifecycle operations (deletion, transfer, Bazaar): Platform re-requests each operation whose Game outcome it received after T, with the same operation identity. Game answers from restored state. If the operation is no longer eligible, Game returns a bounded rejection and Platform compensates on its own side. A Platform workflow row never proves the Game outcome (F27).
    - Game sends Platform one restore notice: restore event id, new recovery generation, T and the backup label. It carries no player data.
11. **Lost player progress.** Gameplay after T is lost. It is never re-executed. Any make-good uses audited idempotent domain transactions (F17). Direct database edits are forbidden.
12. **Restore drills.** A drill restores a named backup into a separate preproduction stack. It runs the full ruling 9 procedure and the content check of ruling 5, and records timings. These timings are the measurement that fixes RTO. One drill must pass before external alpha. Its cadence is a CANDIDATE.

Before-freeze checklist. Concurrent transitions: the migrator's advisory lock, the fence CAS and the epoch row lock serialize them. Restart/resume: rulings 3 and 9 are re-entrant per step. Typed cross-record refs: ruling 5. Older client/peer gating: ruling 2 (exact gate) and ruling 9.11 (fresh login). Multi-component commit and recovery: ruling 8 (fences outside the snapshot) and ruling 10 (idempotent re-request by operation identity).

### Contract amendments

A1. `docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §6, replace the last sentence ("Where the epoch is stored … (U-RS5).") with:
> The epoch is stored as an external high-water in the Game restore fence directory, outside the Game database and its backups, and copied off-host on each raise. The operator raises it only through `oteryn-game-ops restore`, strictly above the stored high-water, after the Character recovery fence advances and before any node reports. A missing or unreadable high-water is a refusal, never 0.
Also mark U-RS5 resolved by this text.

A2. `docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md` §5, replace "The production fence location, durable across host loss, stays open (U-LC2)." with:
> In every profile, F lives in the Game restore fence directory with the Character recovery fence, outside the Character store and its backups, and each new value is copied off-host before the publisher sends. The production restore runbook is `oteryn-game-ops restore` (ARCH-ALPHA-OPS-0 §3 ruling 9).
In §10, set U-LC2 to resolved, pointing at that text.

A3. `docs/contracts/CHARACTER_AUTHORITY_PLATFORM_BOUNDARY.md`, new §13.1 "Game restore" (the contract is Accepted, so this needs the control plane and Platform review):
> After a Game restore to target time T, Game publishes one restore notice (restore event id, recovery generation, T, backup label). Platform then re-requests, with the same operation identity, every operation whose Game outcome it received after T. Game answers from restored state: the existing receipt, a fresh execution if still eligible, or a bounded rejection. On a rejection Platform compensates its own state. Platform never infers a Game outcome from its own workflow row.

A4. A proposal to Platform, routed by the control plane, not a Game file edit. When the PROD-ENTITLEMENTS-01 delivery contract is written, it includes: "Every delivery has a stable delivery identity. Game applies it idempotently. After a Game restore notice, Platform re-requests every delivery acknowledged after T."

### Packets

- **DATA-MIGRATION-GUARD-1.** A CI check that a merged migration file is never changed or deleted, and that every new version is greater than the highest version on `origin/main`. Owned paths: new `tools/repository/check_migration_ledger.py` and its test; a wiring line in the governance/CI workflow that runs repository checks. Tests: changed file fails; deleted file fails; a new 0078 fails; a new 0080 passes. Dependencies: none. P0-adjacent because of F6.
- **DATA-RESTORE-OPS-2.** `oteryn-game-ops restore`, steps 3–10 of ruling 9: the fence CAS and reconcile, the erasure journal re-apply, revocation of CURRENT registrations, the LCFA raise against F, the `assignment_epoch` raise, the 35 s admission gate, per-step resume, and registered codes. Owned paths: `apps/game-server/src/bin/oteryn-game-ops.rs`, `apps/game-server/src/character_recovery_fence.rs`, new `apps/game-server/src/restore_fence.rs`, `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json` (new codes only). Tests: crash after each step and resume; second run advances nothing; F above the computed epoch refuses; a stale NodeId is refused after restore; early admission open is refused. Dependencies: A1, A2; ERR-REGISTRY-0.
- **DATA-BACKUP-PITR-3.** pgBackRest configuration for the test and preproduction stack, an encrypted repository, WAL archiving and the restore fence directory on its own volume. It also reorders the deploy for a release with a migration to stop, named backup, migrate, start (ruling 3; F30). Owned paths: `deploy/synology-game/` and `.github/workflows/synology-game-deploy.yml` once PR #1874 merges; if it does not merge, the control plane names the path (`main` has no deploy directory; `git ls-files` shows only `tools/qualification/wp5_s3a/compose.yml`). Tests: a timed restore of a named backup to T in CI with Postgres; a check that the fence directory is absent from the backup. Dependencies: Owner questions 1 and 2.
- **DATA-RESTORE-DRILL-4.** An automated drill that restores into a separate stack and runs `oteryn-game-ops restore`. It moves the database snapshot and the fence directory independently: DB older, fence newer, fence missing, fence equivocating. It records RTO timings. Owned paths: new drill script and test under `apps/game-server/tests/`; the runbook `docs/operations/` (new). Dependencies: 2, 3.
- **DATA-CONTENT-REF-5.** A release-gate check run on the drill database. It lists every distinct `{family, key, revision}` that live rows reference and fails if the new bundle does not resolve one without an alias or a DUR-04 §12 policy entry. Owned paths: new check module under `apps/game-server/src/content/` and its test. Dependencies: 4.
- **PLATFORM-RESTORE-RECONCILE-P1 (proposal).** The A3 and A4 texts, sent to Platform by the control plane. No Game work until Platform accepts.

### Owner questions

1. Recovery targets for external alpha (all CANDIDATES; RTO is then fixed by the drill measurement).
   a) RPO ≤ 5 min, RTO ≤ 4 h, backup retention 14 days, a drill each month. b) RPO ≤ 1 min, RTO ≤ 1 h, 30 days, a drill each week. c) A nightly dump only (RPO 24 h).
   Recommendation: a. It is cheap with pgBackRest and enough for an alpha.
2. Where the restore fence directory survives host loss.
   a) A separate volume on the node host plus a write-once off-host copy (object storage with object lock) after each advance. b) The node host only; host loss needs a manual reseed. c) A networked compare-and-set service.
   Recommendation: a. It reuses the file register that exists and adds one copy step.
3. Deploy model for alpha.
   a) Stop-the-world only for releases with a migration; rolling per channel otherwise. b) Always stop-the-world. c) Build an N/N-1 schema gate now.
   Recommendation: a.

### Rejected options

- Flyway, Liquibase, refinery or a second migrator: two ledgers break the exact gate (F3). sqlx is upstream and in use.
- DOWN scripts as rollback: rejected by DUR-02 rule 6. They lose data on lossy changes.
- A rolling mixed-version deploy across a migration now: the exact gate refuses it. A wider gate is new risk with no accepted need.
- WAL-G: workable, but pgBackRest has built-in repository encryption, `verify` and time-target restore. One tool is enough.
- Keeping the fences in the database: a restore rolls them back. DUR-02 rule 5 forbids that.
- Storing the fences in Platform: Platform does not own Game recovery authority (ADR-0012), and it adds a cross-system dependency to every restore.
- Re-executing gameplay after T from logs or journals: forbidden by DUR-02 rule 5 and DUR-03 §41.
- Manual SQL fixes for lost purchases or progress: forbidden by Refinements 2026-08-10.
- Reconciling Platform by a Game write into Platform data: Platform owns its data. It re-requests by operation identity instead.

### Open unknowns

- U1 UNKNOWN. Whether PR #1874 merges. If it does, DATA-BACKUP-PITR-3 owns its deploy paths (F30); if not, the control plane names a new path, because `main` has no deploy directory.
- U2 UNKNOWN. Whether any Platform saga that settles value against a Game outcome (Bazaar, store delivery) will be live at external alpha. If none is, A3/A4 are not blocking for alpha, but the restore notice still is.
- U3 CONFLICT (F21). The Character fence decision is still marked candidate but is implemented. The control plane should confirm its acceptance before DATA-RESTORE-OPS-2 builds on it.
- U4 CONFLICT (F12). DUR-04 is marked PROPOSED in its file but accepted in the reconciliation record. Ruling 5 depends only on rules that the code already follows (F10).
- U5 UNKNOWN. How long a full restore of an alpha-sized database takes. DATA-RESTORE-DRILL-4 measures it and fixes RTO.
- U6 UNKNOWN. Whether Platform keeps a Game-acknowledgement time per delivery and per operation, which A3/A4 need in order to select "after T".

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
   - Feed and packages on GitHub Releases of `Oteryn/Oteryn-Game` (owner Q1). One channel,
     `alpha`; no staged rollout. Rollback is a new, higher version built from the earlier source.
     During a rollout the floor stays at the previous build until the operator raises it.
9. **Update integrity.**
   - Every release carries the SEC-CLIENT-01 §3 signed release manifest, extended with: the
     SHA-256 and size of the full package, the asset manifest SHA-256, the channel, and the
     version.
   - Detached Ed25519 over canonical bytes, with the workspace `ed25519-dalek`. The client
     compiles in the trust root public key. Before activation the updater verifies the trust
     record, the manifest, then the package hash, and refuses a version not above its own.
     Velopack's checks add to this; they are not the anchor.
   - Offline keys never enter CI: the workflow publishes a draft, the key holder signs it locally,
     then it is published (owner Q3). OS code signing (Authenticode) is owner Q2.
10. **Browser.** Outside alpha. When a browser profile is registered, its id is
    `oteryn-web/<semver>` with its own floor (ruling 2), and its deployed page and ADR-0018 §9
    manifest replace the updater.
11. **Platform.** Alpha needs no Platform action. The Gateway's `client_build` stays diagnostic,
    and Platform does not gate builds. Platform acts only if the owner picks Q1b.
12. **Before-freeze coverage.** Concurrency: the floor is fixed per ownership generation.
    Resume: a staged release activates only at restart, so an interrupted download keeps the
    prior one. Typed references: the manifest names build id, package digest and asset manifest
    digest. Older-client gating: rulings 3 and 4. Multi-component commit: publish package, then
    signed manifest, then feed entry, then raise the floor.

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
   SemVer version. The client updater verifies it before activation (ARCH-ALPHA-OPS-0 §4 ruling 9)."

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
  Tests: tamper with each field, a revoked key, a version not above the current one, a wrong
  root. Order: with or after SEC-CLIENT-01 acceptance.
- **CLIENT-UPDATE-4** (client; impl, hard). A spike gate first closes the F19 UNKNOWNs; then the
  Velopack integration, the pre-login feed check and verification before activation. Paths:
  `apps/client/src/main.rs`, `apps/client/src/update.rs` (new), `apps/client/Cargo.toml`.
  Tests: a tampered package is not activated; an interrupted download keeps the prior release;
  an unreachable feed lets login continue; user settings survive. Spike failure fallback: a full
  zip, the same verification, and a side-by-side directory switch at restart. Order: after 3.
- **CLIENT-RELEASE-5** (CI; owner workflow authorization). New
  `.github/workflows/client-release.yml`: manual dispatch only, `OTERYN_BUILD_SHA` from an exact
  checkout, the package and a **draft** release; publishing needs the signature (ruling 9).
  Order: after 4, before the first external alpha release.

### Owner questions

1. Where the alpha client is hosted (Platform responsibility versus Game).
   a) GitHub Releases of the public `Oteryn/Oteryn-Game` repository, free. b) A Platform
   download page and storage, a cross-repo Platform task. c) Game-owned object storage or a CDN,
   which costs money. **Recommend a.**
2. Windows code signing (spending).
   a) None for the closed alpha: the Oteryn signed manifest only, with a SmartScreen warning at
   first install, and a purchase decided before open beta. b) Buy a code-signing certificate or
   service now; the cost is UNKNOWN until quoted. **Recommend a.**
3. Custody of the release key (production authority).
   a) The owner holds the offline trust root and the release key, and signs each release locally
   with `oteryn-release-sign`. b) The release key is a secret in a protected GitHub environment
   with owner approval. This conflicts with SEC-CLIENT-01's offline key. **Recommend a.**

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
  hierarchy already gives a root, revocation and a floor.
- **The `self_update` crate.** It replaces the running executable, which ALPHA-CLIENT-01 §17.1
  forbids (DERIVED from the crate's stated purpose; not verified here).
- **A Platform Gateway build gate.** A second authority over the same rule.

### Open unknowns

- Velopack: a verify-before-apply hook, package file access, the CI packaging tool, per-user
  install without admin rights. CLIENT-UPDATE-4's spike closes them.
- Package size about 140 MiB (CANDIDATE: 138 MiB of assets plus the binary); measure the first
  package. Updater package bound 512 MiB (CANDIDATE), fixed after that measurement.
- Feed check timeout 5 s (CANDIDATE); measure against GitHub Releases latency.
- How often the floor must rise depends on how often compiled-in data (F16) and assets change.
  Unmeasured.
- Wire numbers of the FND-04B build rows: open until FND-04B's wire registration.
- SEC-CLIENT-01 and ADMIT-0 are CANDIDATE; RELEASE-MANIFEST-3 and amendment 7 wait on
  SEC-CLIENT-01's acceptance.

## 5. Checklist and owner items

### 5.1 Before-freeze checklist

- Concurrent transitions: the migrator's advisory lock, the fence CAS and the epoch row lock
  (§3); the client build floor is fixed per ownership generation (§4 ruling 12); metrics and
  log writers never change an outcome (§1 ruling 8).
- Restart and resume: per-incarnation log files (§1 ruling 6); timer reload by remaining
  duration or database-time deadline (§2 ruling 6); re-entrant deploy and restore steps
  (§3 rulings 3 and 9); a staged client update activates only at restart (§4).
- Typed cross-record references: `parent`, `attempt` and `traceparent` are diagnostic only
  (§1); content references are `{family, key, revision}` (§3 ruling 5); the release manifest
  names build id, package digest and asset manifest digest (§4).
- Older client and peer gating: no wire change except 1117 (§4 rulings 3–4); a restore forces
  a fresh login (§3 ruling 9 step 11).
- Multi-component commit and recovery: fences live outside the snapshot (§3 ruling 8); Platform
  re-requests by operation identity (§3 ruling 10); a release publishes package, then signed
  manifest, then feed entry, then raises the floor (§4 ruling 12).
- No new authority: every identifier added here is diagnostic, and the only new refusal is the
  build floor.

### 5.2 Owner items

The recommended option is the working assumption until the owner answers.

| Item | Question | Options | Recommendation |
|---|---|---|---|
| §1 Q1 | Where metrics are scraped and alerts sent | a NAS Prometheus/Grafana, email; b no scraper; c hosted (spend) | a |
| §1 Q2 | Alpha log and metric retention | a 14/30 days; b 30/90 days; c delete per phase | a |
| §2 Q1 | Measured capacity below D128's 500 | a accept and log the gap; b block the claim | a |
| §2 Q2 | Making the bench job a required check | a report first, then required; b required now; c nightly | a |
| §3 Q1 | Recovery targets | a RPO 5 min, RTO 4 h, 14 days, monthly drill; b 1 min/1 h/30 days/weekly; c nightly dump | a |
| §3 Q2 | Where the restore fence directory survives host loss | a separate volume plus write-once off-host copy; b host only; c CAS service | a |
| §3 Q3 | Alpha deploy model | a stop-the-world only with a migration; b always; c N/N-1 gate now | a |
| §4 Q1 | Client hosting | a GitHub Releases; b Platform page; c CDN (spend) | a |
| §4 Q2 | Windows code signing | a none for closed alpha; b buy now (spend) | a |
| §4 Q3 | Release key custody | a owner holds offline keys, signs locally; b CI secret | a |
