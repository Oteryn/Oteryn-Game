# LOG-17 successor review

**REVIEWED: all 14 changed files from `43f48d1f3bf932bfaef6ca47334aa72117db6c81` to `c255c6b88dff9467deae5dfc7cd0ff1a0c8fb75f`.** The target has the exact preceding reviewed commit as its sole parent. Twelve target files were reviewed in full; Cargo.lock and identity lib.rs received complete delta review with selected target context. Credited target coverage is 3,320 lines, plus a separate 27-line CLI cross-reference. No coverage is added to immutable audit input `4f0b6c62a789489fca6c37dd36d5917d096f328e`, and the earlier 53-file successor report is unchanged.

## Changes and actual consumers

The new supervisor APIs wrap blocking-pool jobs and the named game-node OS thread while preserving their return values and panic propagation into join handles. The application entrypoint still uses its explicit stack size and exit mapping. Staff verification keeps signature-before-decode ordering and its four-worker permit; Bazaar and bulletin readers retain their single-worker permit through potentially detached blocking IO. Their existing timeouts do not acquire a new ability to cancel already-running filesystem work. Audit journal append still completes and syncs before the source acknowledgement.

The shared registry remains bounded diagnostic state. Lifecycle-only wrappers do not claim heartbeat progress monitoring, and a returned domain error value is not interpreted as a panic. Existing result-converting supervisor APIs remain separate. The new tests assert value/panic preservation, OS thread name, registration and removal; their source was analyzed, but they were not executed.

The architecture checker now applies a per-owner raw-spawn count alongside the console-output count. Its explicit path exclusions and six literal source patterns are heuristic. The current checked-in baseline contains one identity loopback site, and an independent Python reproduction of that exact source selection finds the same site. The identity change itself adds only an explanatory comment. It does not alter authentication or listener lifecycle code.

## Invariants, failure and replay

Staff current-authority fences, signed sequence/roster/activation checks, executor transaction and post-commit independent link retention remain in the reviewed target. Retention failure after a commit is still uncertain to the transport and relies on existing verified repair. Audit append failure still leaves source acknowledgement pending. The diff does not introduce a new session replay protocol, schema migration, ownership-generation rule or authority grant.

Static task labels carry no signed envelopes, player identity or mutable path strings. The two new local crate dependencies agree in manifests, lockfile and allowed workspace edges. No new gameplay finding was qualified. The checker is not reported as a complete syntactic ban: detailed limitations and update-mode behavior are retained in a private observation for separate adjudication.

## Checks and limits

Executed: exact Git/captured SHA and complete changed-hunk coverage validation; Python TOML checks for both dependency edges and boundary membership shape; independent lexical scan reproduction against exact target Git blobs. These are evidence and structural checks. They do not replace execution of Rust architecture-check or Cargo resolution.

Not executed: Cargo build, clippy, Rust tests, native client/server, PostgreSQL, filesystem fault injection, OS thread failure, timing or performance tests. The commit message's claimed earlier validation is not adopted as this audit's result. Deployment and runtime behavior remain unverified. `ledger.json` gives exact per-file review scope; `remaining.json` is empty for this bounded successor delta only.
