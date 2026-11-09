# Logging and error diagnostics in Oteryn: design and architecture

Status: accepted by the owner on 2026-11-09 (all §8 points; metrics backend later). The scope
was extended to the whole server (§3.0). The code state was checked at the head of #3057
(`a44b7ec`).

**Relation to ARCH-ERROR-CODES-0.** This document extends
`docs/architecture/reviews/OTERYN_GAME_ARCH_ERROR_CODES_2026-10-05.md`; it does not replace it.
It decides three items that §1.9 deliberately left open: in-process metrics, task supervision,
and the event registry. The external observability backend, retention, and remote log-level
control remain open.

## 0. Why this design exists

Node boot on `main` is red. The test client receives "transport I/O failed: early eof" about
15 seconds after the server logged `event=connection_admitted ms=5557…6610`. The server does
not record why it closed the connection, so the cause is invisible in both the CI log and a
local run. Finding that one of seven silent exit paths in `serve_admitted_actor` caused it,
including `hold_refused` in
`crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs`, which closes the
connection after 15 seconds without a frame,
required reading the code and several 45-minute runs.

The goal is: **every failure can be diagnosed from one run, without reading code or rerunning.**

## 1. Current state: facts from the code

What already exists (ERR-NODE-1, in part):
- the `oteryn-error-codes` crate: one line format (`Line`, §1.5), `E####` codes with a category
  and progression, and the `docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json` registry;
- a panic hook, a `process_start` line with `build`, one trace for the whole process, and the
  `OTERYN_LOG` level filter (`apps/game-server/src/node/serve.rs`);
- privacy rules: a line contains no player identifiers, tokens, or grants; `detail` is escaped
  and is at most 512 bytes.

What is missing:

| Area | State |
|---|---|
| Connection trace (`trace` for every accepted connection) | absent; the listener logs without a trace |
| Connection end after admission | not logged; `hold_refused` and six other exits are silent |
| Admission refusal reason | only `refused_classified`, without a code |
| Shared writer | `oteryn_error_codes` is used by 11 files; `eprintln!` occurs about 920 times (668 in `crates/gameplay/dispatch`) |
| `oteryn-game-ops diagnose` (ERR-DIAG-4) | not implemented |
| Trace on the wire (`CONNECTION_TRACE_V1`, ERR-TRACE-5) | number 20 reserved; capability not registered |
| Metrics (counters, histograms) | absent; backend deliberately undecided (§1.9) |
| CI report | `tools/qualification/node_boot/run.sh` does not summarize the node log; the full job log must be downloaded |

## 2. Rules

1. **One event model.** Every diagnostic line goes through `oteryn_error_codes::Line`. New code
   does not add `eprintln!`.
2. **Every end is logged exactly once.** Every operation with a lifecycle (connection,
   admission, durable write, Platform call, qualification stage) ends with one `*_end` line
   containing the outcome, code, and duration. A silent close is an error.
3. **Correlate by trace, not identity.** Lines for one connection share a `trace` (UUIDv7).
   Player identifiers never enter the log (ANL-01 §18).
4. **Code instead of text.** Every failed outcome has a registered code. Text goes only in
   `detail`.
5. **Cheap when healthy.** Debug details are collected in memory and written only on failure
   (the flight recorder, §3.4).
6. **Testable.** A test confirms that every exit path emits its end line.

## 3. Architecture

```text
 event ──► Span / Line (oteryn-error-codes) ──► OTERYN_LOG filter ──► stderr (key=value)
              │                                                   │
              ├─► flight recorder (per-connection ring) ──────────┘ dumped on failure
              └─► in-process counters and histograms ──► `oteryn-game-ops stats` (control socket)

 node stderr ──► generated node log ──► `oteryn-game-ops diagnose` (--trace / --report / --summary)
                                 └─► node-boot script: summary in $GITHUB_STEP_SUMMARY on failure
```

### 3.0 Scope: the whole server, not only connections

Every server subsystem has the same contract: a registered module, registered events, a `Span`
for every operation with a lifecycle, a code for every failure, and a trace that links work to
its cause.

**Trace hierarchy.** Every piece of work has a parent:

```text
boot trace (process)
 ├─ startup/<stage>           configuration, content, world, migrations, node registration
 ├─ job trace                 one periodic job run (world pass, scheduler)
 │   └─ durability / platform operations for that run
 ├─ task trace                each long-lived tokio task (tick loop, collector, listener)
 ├─ connection trace          one player connection
 │   └─ command (seq)         one command and its outcome
 │       └─ durability / platform / channel-owner work item
 └─ ops trace                 one control-socket command (oteryn-game-ops)
```

A child line carries its own `trace` and `parent=<parent trace>`, so `diagnose --trace` can show
the whole tree.

**Subsystem catalogue and required events.** Module names are tokens from the event registry
(§3.9):

| Module | Location in code | Required events (`Span` → `*_end`) |
|---|---|---|
| `node` | `apps/game-server/src/node/serve.rs`, `apps/game-server/src/node/config.rs`, `apps/game-server/src/node/cancellation.rs` | `process_start`, `startup_stage` (each stage with duration), `readiness`, `drain`, `shutdown`, `config_rejected` |
| `content` | `crates/gameplay/content`, `crates/gameplay/content-promotion`, `apps/game-server/src/node/quest_log_content.rs` | `content_load` (revision, counts, rejected entries with a code), `content_activation` |
| `world` | `crates/gameplay/world`, `crates/world-bundle`, `crates/gameplay/world-change`, `crates/gameplay/weather` | `world_bundle_load`, `world_change_apply`, `tick` (threshold breaches only, §3.10) |
| `durability` | `crates/gameplay/durability` | `db_tx` (operation, outcome, duration, retries), `db_pool` (exhaustion, wait), `migration`, `fence_conflict` |
| `platform` | `crates/platform-client` | `platform_call` (endpoint, status, duration, retries), `platform_unavailable` |
| `admission` | `crates/gameplay/native-admission-source`, `crates/gameplay/admission-evidence`, `crates/gameplay/character-recovery-fence` | `admission` (outcome, code, duration), `character_bootstrap`, `recovery_fence` |
| `gameplay_transport` | `crates/gameplay/dispatch` | `connection`, `command` (type, outcome, code, duration; success at debug), `liveness_lost`, `handoff` |
| `channel` | channel owner and actors in `crates/gameplay/dispatch` and `crates/gameplay/entities` | `work_item` (threshold breaches), `actor_spawn`/`actor_despawn` (debug), `channel_owner_change` |
| `combat`, `spell`, `items`, `economy`, `housing`, `social`, `quests`, `npc`, `progression`, `encounters` | corresponding crates under `crates/gameplay` | rule refusal with a code (`*_refused`), state inconsistency (`invariant_violation`, always error) |
| `monster_ai` | `crates/gameplay/monster-ai` | `ai_budget_exceeded`, `pathfinding_failed` (counted, sampled) |
| `scheduler` | scheduler and job modules under `apps/game-server/src/node` | `job` (name, outcome, counts, duration), `job_skipped` (reason), `job_overrun` |
| `ops` | `apps/game-server/src/bin/oteryn-game-ops.rs`, control socket | `ops_command` (command, outcome, code, duration) |
| `runtime` | task supervisor (§3.11) | `task_start`, `task_end`, `task_panic`, `task_stalled` |
| `stats` | §3.5 | `stats` every 60 seconds |

**Rules for the whole server:**
- A failure at a subsystem boundary (an error returned outside its crate) has a registry code.
  Boundary error enums gain `fn code()` when the module is first touched (§1.4 of
  ARCH-ERROR-CODES-0).
- A game-rule refusal caused by a player, such as insufficient mana, is an outcome rather than
  a server error: debug level plus a counter. A server-state inconsistency is always `error`
  with a code.
- A retry is logged as one `*_end` with `attempts=N`, not as N lines.
- High-frequency events (tick, AI, individual frames) are never info. They report only
  threshold breaches and counters.

### 3.1 Span: lifecycle with a guaranteed end line

A small RAII type in `oteryn-error-codes`, with no external dependencies:

```rust
let mut span = Span::start(Level::Info, "gameplay_transport", "connection", trace);
// ... work ...
span.outcome("admitted_disconnected").code(E3xxx).field("reason", "not_playable");
// drop(span) always writes:
// event=connection_end outcome=admitted_disconnected code=E3xxx ms=21043 trace=… detail="reason=not_playable first_entry=RefusedUnavailable"
```

- `drop` writes the end line **always**: on `return`, `?`, and future cancellation. If no one
  sets an outcome, it is `unset`. In tests, `unset` is an error, so a new exit path without an
  outcome fails tests.
- The start line (`*_start`) is debug. The end line is info on success and warn or error with a
  code on failure, so it is always recorded (§1.10 item 5).
- A child span inherits its parent's trace, for example for admission, a durable write, or a
  Platform call within a connection.

Why not the `tracing` crate: ARCH-ERROR-CODES-0 §3 rejects it because it would rewrite every
call and change the line format. `Span` provides the needed subset (start, end, duration,
outcome, trace) in about 150 lines.

### 3.2 Connection trace on the server, without a protocol change

- The listener assigns a UUIDv7 to each accepted connection (`uuid_v7()` already exists) and
  passes it through `admit_frame`, `serve_admitted`, and authority calls.
- Every line for that connection has `trace=…`.
- Sending the trace to the client is a separate step (ERR-TRACE-5, §3.7), because it changes
  the protocol.

### 3.3 End and refusal codes

- `ConnectionEnd` and `AdmissionRefusal` gain `fn code(&self) -> ErrorCode` (the Game registry,
  range 3xxx for transport). Every variant has a code. An exhaustive test using `match` without
  `_` enforces this.
- The seven `serve_admitted_actor` exits set `reason` on the span: `resolve_native_scope`,
  `staff_refresh`, `remember_activity`, `quest_callback`, `not_playable(first_entry)`,
  `observe_none`, and `setup_prepare`.
- `hold_refused` is no longer silent on the server. Before closing, it records
  `connection_end outcome=hold_expired` with the reason.

### 3.4 Flight recorder

- A bounded ring of the last N (for example, 64) debug events per connection is held in memory.
- On an abnormal end (error code, `ControlLost`, `hold_expired`, or task panic), the ring is
  written as `event=flight trace=… seq=…` lines. It is discarded on a normal end.
- This provides debug detail exactly for failures, without the cost of debug logging all traffic.
- The ring follows the same privacy rules: only tokens, numbers, and redacted `detail`.

### 3.5 In-process metrics

- Atomic counters and simple fixed-bucket histograms in `oteryn-error-codes::metrics`:
  - `connection_end_total{outcome}`;
  - `admission_refused_total{code}`;
  - `admission_ms` (p50/p95/p99);
  - `durability_write_ms`;
  - `platform_call_ms{endpoint}`;
  - `liveness_missed_total`.
- Read them with `oteryn-game-ops stats` over the existing Unix control socket. No network port
  is opened.
- Write a periodic `event=stats` line every 60 seconds so the trend is visible in the log itself.
- Export to Prometheus/OpenTelemetry remains an owner decision (§1.9 is still open). The data
  model is compatible with Prometheus text format, so a later export needs one adapter.

### 3.6 Tools

- **`oteryn-game-ops diagnose`** (ERR-DIAG-4, already designed) plus a new `--summary` mode:
  - counts ends by outcome and code;
  - shows duration distributions;
  - prints the five traces with the worst outcomes and their lines.
- **Node boot `tools/qualification/node_boot/run.sh`:** after failure, runs
  `diagnose --summary` on the generated node log and appends the
  result to `$GITHUB_STEP_SUMMARY`. The job summary then shows, for example,
  "12× admitted_disconnected reason=not_playable(RefusedUnavailable)" without opening logs.
- **CI artifact:** upload the generated, redacted node log on failure, with seven-day retention. This is
  a workflow edit and therefore requires owner approval.
- **Test client (`tools/dev-client`, qualification):** an error includes the phase (`connect`,
  `join_snapshot`, `command`), time since connection, last received frame, and `trace` when
  known. Instead of "early eof", it shows, for example, "early eof in join_snapshot after
  21.0 s, last frame ServerAccepted".

### 3.7 Trace and close reason on the wire: protocol change

- ERR-TRACE-5 is unchanged: capability `CONNECTION_TRACE_V1` (id 20), and a
  `connection_trace` field in `ServerAccepted`, `ServerResumeAccepted`, and `ProtocolError`.
- Extension: instead of silently closing after admission, the server sends `ProtocolError`
  with a code such as `NOT_PLAYABLE` or `HOLD_EXPIRED` if the client negotiated the capability.
  An older client notices nothing.
- This requires protocol review and owner approval. It is a separate packet.

### 3.8 Ratchet on `eprintln!`

- `architecture-check` counts `eprintln!` and `println!` in production crate and application
  source, excluding tests and examples, and compares the count with a baseline file.
- The count may only fall. A new call without an exception entry fails CI.
- Migration is opportunistic: when a module changes, its lines move to `Line`/`Span` (§1.10
  item 5).

### 3.9 Event registry

- Registry fragments follow the capability and code pattern. Each module has a JSON fragment
  under a logging event registry directory. Each entry gives the event name, level, fields
  (tokens), whether it has a `Span`, failure code or codes, and owner.
- `architecture-check` verifies three things:
  - every `event=` in code is in the registry;
  - every registry entry is emitted;
  - every subsystem in the §3.0 table has its required events.
- A `log-coverage` architecture report shows the percentage of subsystems and exits covered by
  spans. It rises with every packet.

### 3.10 Slow-operation and tick thresholds

- `Span` may have a threshold: `Span::start(...).slow_after(Duration)`. Crossing it raises the
  end line to warn with `slow=true`, even for a successful outcome.
- Default thresholds are in one configuration file, not code:
  - DB transaction: 250 ms;
  - Platform call: 1 second;
  - admission: 2 seconds (currently 5.6–6.6 seconds, so the problem is immediately visible);
  - periodic job: 30 seconds;
  - channel work item: 50 ms.
- The world tick has a tick counter and duration histogram. Write `warn tick_overrun` only when
  a tick exceeds its budget, at most once per 10 seconds, with the number of breaches in that
  period.

### 3.11 Background task supervisor

- The code has about 160 `tokio::spawn`/`spawn_blocking` sites. One
  `spawn_supervised(name, trace, future)` function replaces them.
- The function logs `task_start` at debug and `task_end` with the outcome. A task panic produces
  `task_panic` with code `E4001`, task name, and trace instead of disappearing silently.
- Long-lived tasks such as loops report a heartbeat. No heartbeat for longer than the threshold
  produces `task_stalled`, detecting deadlocks and stuck loops.
- `oteryn-game-ops stats` shows live tasks and the age of their last heartbeat.

### 3.12 Node health

- `oteryn-game-ops health` over the control socket returns:
  - readiness;
  - DB pool state;
  - Platform availability;
  - tick latency;
  - connection count;
  - stalled tasks;
  - the last 10 errors with codes.
- The same set is periodically included in the `event=stats` line.

## 4. Privacy and security

- The §1.5 rules are unchanged: no AccountId, CharacterId, GameSessionId, grants, tickets,
  tokens, passwords, credential-bearing URLs, or payloads; `detail` is redacted and limited to
  512 bytes.
- The trace does not come from the client and authorizes nothing.
- The flight recorder and metrics obey the same rules. Metric labels are registered tokens only,
  with no unbounded cardinality.
- Logs are not sent to an external service such as Sentry without an owner decision.

## 5. Performance

- Check the level filter before formatting. Call `format!` only if the line passes.
- A connection span writes one end line. Start lines are debug only.
- The ring has a fixed bound. Atomic counters are lock-free.
- There are no info lines for an individual frame or tick.

## 6. Implementation plan

| Packet | Scope | Protocol/workflow | Size |
|---|---|---|---|
| **LOG-0** (with the #3057 fix) | connection trace; `connection_end` with outcome, `ms`, and reason; code on admission refusal; summary in `tools/qualification/node_boot/run.sh` | no | small |
| **LOG-1 Span** | `Span` in `oteryn-error-codes`; admission, serving, durable writes, Platform; "every exit has an outcome" test | no | medium |
| **LOG-2 diagnose** | ERR-DIAG-4 plus `--summary`; `tools/qualification/node_boot/run.sh` uses it in CI | no | medium |
| **LOG-3 ratchet** | `eprintln!` counter in `architecture-check`; migrate when touched | no | small |
| **LOG-4 flight recorder** | per-connection ring, dump on failure | no | medium |
| **LOG-5 metrics** | counters and histograms, `oteryn-game-ops stats`, `event=stats` line | no | medium |
| **LOG-6 client context** | phase, duration, and last frame in dev-client and qualification errors | no | small |
| **LOG-7 wire** | ERR-TRACE-5 plus `ProtocolError` with a code instead of silent close | **yes** (protocol review) | medium |
| **LOG-8 CI artifact** | upload the generated, redacted node log on failure | **yes** (workflow) | small |
| **LOG-9 event registry** | event registry fragments, `architecture-check` check, coverage report | no | medium |
| **LOG-10 task supervisor** | `spawn_supervised` instead of about 160 `spawn` calls; `task_panic`, `task_stalled`; `oteryn-game-ops health` | no | medium |
| **LOG-11 node + scheduler + ops** | startup stages, schedulers and world passes, ops commands as spans with codes | no | medium |
| **LOG-12 durability + platform** | transaction and call spans, DB pool, retries, slow-operation thresholds | no | medium |
| **LOG-13 admission + transport** | full dispatch migration (668 `eprintln!` calls) and admission sources to `Line`/`Span` | no | large (split by module) |
| **LOG-14 world, channel, AI** | tick, work items, AI: counters and threshold breaches | no | medium |
| **LOG-15 game rules** | `fn code()` for boundary enums in gameplay crates, `invariant_violation` | no | large (by crate) |

Order:
1. LOG-0 immediately, because it fixes the current `main` failure.
2. LOG-1, LOG-9, and LOG-10: the Span, registry, and task-supervisor foundation.
3. LOG-2 (diagnose) and LOG-8 (CI artifact), so CI benefits immediately.
4. Subsystems: LOG-11, LOG-12, LOG-13, LOG-14, LOG-15. Each packet raises coverage and lowers
   the LOG-3 ratchet count.
5. LOG-4, LOG-5, LOG-6, LOG-7.

Subsystems proceed in parallel on Codex lines, one module per line, without shared files.

## 7. How this would work for the current incident

Without this design: "early eof", followed by several hours of code reading and runs.

With LOG-0 through LOG-2, the CI job summary would immediately show something like the following.
**This example is illustrative; only the diagnostic run in progress can establish the real
cause.**

```text
connection_end outcomes (12):
  12× admitted_disconnected  reason=hold_expired  cause=not_playable first_entry=RefusedUnavailable  ms≈21000
admission_ms p50=5.8s p99=6.6s
```

That line points directly to the path and cause. It also shows the second, already measured
problem: admission takes 5.6–6.6 seconds.

## 8. Owner decisions

The owner accepted all four points below on 2026-11-09. The metrics backend comes later, and
the scope is extended to the whole server.

1. Accept LOG-0 through LOG-6 as the direction, without protocol or workflow changes.
2. LOG-7: approve the protocol change, carrying the trace and a code instead of a silent close.
3. LOG-8: approve the workflow edit, uploading the generated node-log artifact on failure for
   seven days.
4. Choose the Prometheus/OpenTelemetry metrics backend later. Implement
   `oteryn-game-ops stats` first.

## 9. Rejected options

- **`tracing` plus `tracing-subscriber` across the codebase:** this would rewrite every call and
  change the format, consistent with ARCH-ERROR-CODES-0 §3. Reconsider it with the
  observability backend.
- **JSON logs:** node-boot D6 fixes the key=value format, which is easy for people to grep.
- **Debug logging always enabled:** it has cost and leak risk. The flight recorder provides the
  same detail only for failures.
- **External error SaaS such as Sentry:** sending data externally requires a privacy decision
  under the client diagnostics baseline.
