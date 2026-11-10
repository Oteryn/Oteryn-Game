# Successor audit: connection diagnostics and CI capture

## Scope and conclusion

**DELTA_REVIEW_COMPLETE: 53 of 53 assigned paths.** The target is `43f48d1f3bf932bfaef6ca47334aa72117db6c81`, compared with `b028bef4ddcc6d48bdd6cf61479c94e42435ed88`. Every assigned target blob is also identical to its version in intermediate commit `1d9976d190c623cc95aeed0f2db37bbc8809e883`. This is a separate successor review; the original checkout remains at `4f0b6c62a789489fca6c37dd36d5917d096f328e` and receives no coverage credit from these changed versions.

| Evidence category | Exact extent |
|---|---:|
| Full target-file manual reviews | 27 |
| Complete delta reviews with selected target context | 26 |
| Assigned paths with all changed hunks reviewed | 53 |
| Target lines actually reviewed in assigned paths | 9,414 |
| Additional selected consumer cross-references | 7 files, separate ledger |
| Remaining assigned paths | 0 |

The ledger records source SHA-256, target ranges, baseline hunk context, exact diff SHA-256 and individual semantic notes. Its validator checks immutable source identity, valid endpoints, full coverage where explicitly claimed, and coverage of every changed target line. It validates evidence structure; it does not independently prove the semantic conclusions. The 26 selected-context files must not be presented as full current-file reviews. No new qualified finding arose in this successor scope. Existing findings from the original audit were not silently closed by these changes.

## Connection trace and terminal error path

The new connection-trace type is a bounded, non-nil UUIDv7 with the RFC variant. Capability 20 is independently registered and offered by the native session; its catalogue describes diagnostic support with no new command, state domain or gameplay authority. The protocol source, schema, compiled catalogue, golden catalogue and UI registration declarations were reviewed together. The schema digest agrees with both committed locks.

The server creates a diagnostic trace for a connection and carries it through a task-local scope. That scope is installed while polling the owned connection future, so unrelated sockets do not acquire its trace through a globally reused value. Capability selection controls emission of the optional trace field. The trace does not substitute for account authority, lease, actor epoch, connection generation or command sequence. Fresh admission still calls the real owning checks; recovery uses its separate grant and current-fence path. Legacy or recovery responses may omit a trace even when support is offered: the extension is optional, not a new admission prerequisite.

The client decodes the trace with exact cardinality, wire shape and UUID constraints. A trace in a context that did not select capability 20 is refused. The structured `ServerProtocolError` retains a numeric code and optional trace for diagnostics; it does not install server-provided authority or alter replay state. Envelope and current-generation validation precede terminal error interpretation. The existing poisoning behavior remains for terminal failures, with the separately defined activity-scope change behavior preserved.

The actual listener and connection-owner context were reviewed, including bounded admission/liveness polling, current activity lineage, control-loss handling, terminal reconciliation and observer outcome classification. Added malformed-frame/timeout responses are diagnostic paths. The selected `tcp_tls::write_frame` helper was read as a cross-reference; no broader write-deadline conclusion or newly qualified denial-of-service finding is asserted from that helper alone. Duplex test definitions were read, but no Rust or native transport tests were executed here.

Primary reviewed owners are `crates/protocol-oteryn/src/{connection_trace.rs,lib.rs,recovery.rs}`, `crates/session/src/{error.rs,session_exchange.rs,session_access.rs,capability_support.rs}`, and `crates/gameplay/dispatch/src/gameplay_transport/{connection.rs,listener.rs,mod.rs}`. Exact full versus selected status is in the ledger.

## Structured logs and task supervision

The converted native-source logging callsites were traced to their actual argument producers. Account-character retrieval, public projection delivery, runtime status and world-event outcome publication use closed operation/family/classification values for the changed diagnostic lines. The publication token, subject, body and current authoritative fence remain independent of those logs. Retained revision/watermark logic and bounded retry outcomes were not replaced by logging success.

The real structured emitter and line renderer were read separately. They normalize keys and tokens and bound escaped detail; these are formatting controls. They do not provide arbitrary semantic secret redaction. The current changed callers were assessed on the values they pass, rather than assuming the logger automatically makes every possible future argument safe. Event JSON files were reviewed as declarative contracts, not proof of runtime emission or secret filtering.

The full task-supervisor implementation was reviewed with the actual server maintenance/runtime-status, cast-lease and Premium-refresh callers. The registry has a finite capacity; registration overflow omits visibility rather than stopping the underlying operation. Lifecycle state, explicit heartbeat and scoped expected-idle state are distinct. The watchdog periodically inspects age, while completion/drop removes the live record. Join wrappers preserve ordinary output and resume panics. Current callsites supply static task names and explicit progress or expected-idle boundaries. The audit does not infer guarantees for all possible future multi-runtime arrangements or treat a generic task result as an operation-specific success predicate.

The two new local dependency edges, from the server and social crate to task-supervisor, agree between manifests, lockfile and workspace policy. The full policy configuration was understood in terms of package/path membership, production closure, gameplay layers, same-layer exceptions and separate development-only edges. Selected architecture-check consumers confirm the actual role of those declarations. No whole-workspace metadata build or dependency-package source review is implied.

## CI failure logs and workflow pins

The new node-boot shell changes preserve the original exit code and capture failed node output before temporary qualification state is deleted. The stage wrapper retains the first failed incarnation before a later start can truncate its file. Current callers use explicit runner-temporary directories. Selected recursive-entry, startup, cleanup and final-result control flow was reviewed, and exact target shell bytes passed `bash -n`; the complete qualification was not executed.

The protected merge-group and manually dispatched qualification workflows upload only a separate redacted output directory on failure. They retain immutable action/Platform pins, read-only repository permissions, exact candidate checkout and the mandatory qualification success sentinel. The PR heavy workflow retains its eligibility and optional node-boot fan-in behavior and deliberately lacks the protected workflows' artifact tail. The adjusted parity test compares the shared qualification step before that distinct tail. A complete review of the unchanged portions of the two large workflows is not claimed.

The redactor streams decoded lines and drops entire lines containing one of four case-insensitive patterns: `password`, `postgres://`, `token=` or `grant`. Its real workflow callers use different input/output paths. Two original tests passed unchanged: filter selection with surrounding-line preservation, and invalid UTF-8 replacement. This is a limited diagnostic filter. No claim is made that it removes every possible secret representation, and no concrete newly exposed secret producer was established.

The protected workflow's computed Git blob SHA is `7b7f36b197a72a4317d6318b1cc4ec4ab9b602f6`. All three changed pin locations agree. The full merge-authority audit workflow was reviewed: it uses read-only API data, validates current PR identity/head, reads candidate content as inert bytes and recomputes the Git blob without executing candidate code. These static checks do not independently establish historical authorization of the already merged pin rotation; that remains outside this source-only review.

## Diagnostic observation and hypotheses bounded

A supplementary exact-line check found one stale label in `logging-eprintln-baseline.json`: `serve.rs:3412` points into ordinary code, while the corresponding console macro is now at line 3438. The actual consumer in `tools/architecture-check/src/logging.rs` enforces per-owner console-call counts, not exact equality of site labels. The initial strict-line assertion was therefore an invalid assumption about that policy and is retained as an observation in `checks.json`, not a new functional finding. The other 53 listed sites locate console macros at their declared line numbers. Matching these sites gives no full review credit to referenced files.

Unnegotiated diagnostic traces do not become authority; legacy absent traces remain accepted. Error display changes do not bypass generation or envelope checks. Registry overflow does not suppress execution of the task. The redactor's limited pattern set is not treated as proof of a leak without a concrete producer. Metadata hash agreement is not treated as historical approval or deployment evidence. These distinctions prevent diagnostic and qualification changes from being promoted to unsupported security or runtime claims.

## Validation and exact remaining work

`checks.json` records two original redactor tests passing, shell syntax passing, five Python files parsing, exact workflow/schema pin agreement and the two dependency-edge checks. Its overall status includes the stale-line observation. `validation.json` records 53 paths, 27 full files, 26 delta-only files, 9,414 reviewed target lines and zero remaining paths. Seven selected actual consumer reads are separately recorded in `cross-reference-ledger.json`; they do not enlarge the assigned-path count.

No Rust workspace build, PostgreSQL fixture or migration, native node/client process, GitHub Actions run, network integration, concurrency or timing test was executed in this successor review. No source or remote repository write was made. All 53 assigned deltas are complete; any unrecorded unchanged context in the 26 selected files remains outside this successor claim. Further original-commit work proceeds in the separately assigned dispatch social/world scope.
