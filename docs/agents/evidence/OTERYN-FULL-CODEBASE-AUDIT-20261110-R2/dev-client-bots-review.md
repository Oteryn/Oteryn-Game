# Dev client and synthetic player orchestration

Input: `Oteryn/Oteryn-Game@4f0b6c62a789489fca6c37dd36d5917d096f328e`.
Status: both packages **REVIEWED**, not runtime VERIFIED. All tracked files, production/tool code, manifests and test bodies were substantively inspected. Native Cargo/TLS tests were not executed.

## oteryn-dev-client — 11 files / 3,166 lines

This is a tool-only adapter over the existing Session and TLS/TCP crates. Connect and resume use those actual implementations; no second codec, grant authority or permissive admission bypass exists in this wrapper. The caller supplies a trust root, hostname, grant and timeout; the wrapper retains the exact session error and adds only elapsed phase/last-frame context for I/O/timeouts. Other protocol errors remain typed. Every exposed gameplay adapter delegates to the same Session methods. Disconnect consumes the transport before returning continuity, and disconnect-after-step deliberately leaves a sent outcome unread for recovery qualification.

The complete scripted TLS fixtures and assertions were inspected: admission success/closure, no-ALPN rejection before any bootstrap byte, snapshot id/type/order/size mismatches, two-chunk assembly, invalid generation/sequence, command correlation and poisoning after protocol errors, business refusals that leave the session usable, actual post-result deltas, idle/interleaved liveness, duplicate histories, local encoding refusal without consuming an id, and new-transport recovery. The spell-targeting integration fixture actually decodes the outgoing combat command for current-target, directional fallback and cursor-position cases, then checks ordinary refusal feedback. These are real adapter/Session test paths against scripted peers; they are not a live GameNode or PostgreSQL execution result.

The root workspace boundary declaration classifies this package as tool-only and permits the expected qualification dependencies. No additional confirmed production defect was established. Existing platform/session/runtime findings remain in their owning reports and are not duplicated here.

## oteryn-player-bots — 3 files / 1,329 lines

The synthetic-only supervisor drives the dev client's real session path. It refuses duplicate bot IDs and zero liveness slices before starting workers; a semaphore bounds active runners. Specs explicitly redact grant, seed, certificate and character identity in their Debug view. Command and latency counters saturate, event drains are bounded by the Session owner, and final commands receive a bounded liveness window so delayed deltas are not silently skipped.

The shutdown future polls cancellation before the operation. Cancellation drops the exchange and the owning runner immediately drops its Session, so a partially consumed frame is not reused. Run ownership retains JoinHandles while collecting: cancellation of finish/shutdown still triggers Drop to abort all workers. Cooperative shutdown reaches queued live runners after permits release; custom injected runners retain their explicit cooperation responsibility. Reports are sorted by bot id, task panics/cancellation become typed failures, and one failed runner does not terminate peers.

All mock supervision tests and three scripted real-TLS-runner tests were inspected, including concurrency, per-bot failure isolation, cancellation of collection, delayed final delta, liveness traffic during shutdown and floor-changing movement. A normal game refusal is intentionally not a transport failure: the scripted Blocked test explicitly expects a completed run. Metrics therefore do not prove that each gameplay action succeeded or changed state. No separate production caller of BotSupervisor was found in the searched game-server/synthetic-harness source; no deployment prevalence is inferred.

No new confirmed finding was established. No live bot session, network connection, server mutation or native test run occurred during this review.
