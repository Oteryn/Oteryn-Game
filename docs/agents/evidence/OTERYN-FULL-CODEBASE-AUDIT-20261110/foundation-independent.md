# Independent review — foundation, task supervisor, NTP clock

**Baseline:** `e2e5ced306040113dc9c395251a802f50eb5776a`.
**Reviewer:** `coordination_inventory` (independent of root's prior review).
**Verdict:** No reportable defect with a demonstrated repository caller and wrong result established. No P0/P1 identified.
**Method:** Complete static reading of the seven requested files, including their embedded tests and manifests, plus the selected caller/dependency checks below. **No tests, builds, installs, kernel calls, or SQL were run.** Source checkout verified clean at the same SHA after the review.

## Exact complete-package coverage

`rg --files` in the three directories returned exactly these seven files. All listed ranges were substantively analyzed, not just opened or counted.

| File | Complete analyzed range | Focus |
|---|---:|---|
| `crates/foundation/Cargo.toml` | 1–18 | Dependency and lint boundary |
| `crates/foundation/src/lib.rs` | 1–218 | Generation wrappers, text bounds, cancellation storage/wakeup, cancellation race and tests |
| `crates/foundation/src/time.rs` | 1–219 | Moment/deadline arithmetic, monotonic/manual clocks, lock recovery and tests |
| `crates/task-supervisor/Cargo.toml` | 1–23 | Tokio/error-code dependencies and test features |
| `crates/task-supervisor/src/lib.rs` | 1–663 | Spawn wrappers, registry, heartbeat/watchdog, counters, unwind/RAII behavior and tests |
| `crates/ntp-clock/Cargo.toml` | 1–32 | Linux-only libc pin and isolated unsafe boundary |
| `crates/ntp-clock/src/lib.rs` | 1–168 | Read-only FFI, synchronization status, conversion bounds, unsupported target and tests |

**Totals:** 3 packages, 7 files, **1,341 lines**. This comprises 4 Rust source files/1,268 lines and 3 manifests/73 lines. Complete source coverage here does not mean complete verification of dependencies or deployed behavior.

## Foundation: cancellation and time

**FACT — Early cancellation is retained.** `CancellationToken::cancel` uses `watch::Sender::send_replace(true)` (`lib.rs:149-151`), not a send that requires a live receiver. A waiter subscribes and immediately reads the current value (`158-167`). The pinned dependency explicitly supports sending replacement values without receivers (`vendor/tokio-1.53.1/src/sync/watch.rs:1215-1233`).

**FACT — The suspected subscribe/check/wait missed-wakeup window is guarded.** The subscriber starts with a version (`watch.rs:1387-1393`); `changed_impl` registers notification before checking for a newer version (`983-1001`). If cancellation precedes subscription, the initial borrow observes true. If it follows the initial false observation, the version check/notification delivers it. Every mutation in the token API sets true, so no later false value can erase the event.

**FACT — Cancellation wins when both branches are ready.** `cancellable` polls the cancellation future first (`lib.rs:177-191`). Its work future stays pinned and is dropped when cancellation wins. It does not promise preemption of a future's synchronous poll or rollback of an already issued external operation. The selected real callers use it for a bounded directory GET (`crates/platform-client/src/lib.rs:150-203`) and an admission retry delay (`crates/client-runtime/src/connection.rs:66-102`). The gameplay reset signal waits on the same token (`crates/gameplay/dispatch/src/gameplay_transport/gameplay_drain.rs:27-46`). No cancellation-induced duplicate mutation was established from these callers.

**FACT — The exposed arithmetic does not wrap.** Moment addition/subtraction use checked duration operations (`time.rs:25-41`); deadline remaining time saturates at zero (`80-85`). `ManualClock::advance` updates only after successful checked addition, and `try_set` rejects a backward value under the same write lock (`132-154`). `SystemClock` clones preserve their `Instant` origin (`93-116`). Poison recovery does not introduce a mutation before these checks.

**Caller counterguard:** moments do not encode a clock identity, so comparing different independent clock origins would be caller misuse. The examined world-revert owner retains one `Arc<dyn MonotonicClock>` and uses it for deadline creation and due checks (`crates/gameplay/world/src/world_object_revert.rs:375-407,490-527,595-627,630-661`). A mixed-origin wrong result was not established.

## Task supervisor: registry and panic/cancellation boundaries

**FACT — The retained registry is bounded under one lock.** `register` tests the length and inserts under the same mutex (`251-269`). Full capacity suppresses a diagnostic registry record; it does not limit task execution, which matches the API's stated live-record capacity (`105`, `182`). Each guard records whether it inserted, so an unregistered task's drop does not remove another record (`278-286,402-410`). Heartbeats use weak ownership and do nothing after record removal (`413-428`).

**FACT — Normal future completion, returned error, caught poll panic and async cancellation have distinct terminal paths.** `TaskGuard::run` catches panics from the future's poll (`333-348,431-443`); `finish` sets `finished` before writing the terminal outcome (`370-398`); `Drop` removes the registry record and writes cancellation only if finish was not reached (`402-410`). The registry lock is released before log emission (`270-277`) and watchdog emission (`220-247`). Ordinary poisoning is recovered by the private lock helper (`446-450`).

**FACT — Timing snapshots do not panic if a concurrent heartbeat is newer than the sampled `now`.** The supervisor calls Tokio's `Instant::duration_since` (`452-453`), whose pinned implementation uses saturating subtraction (`vendor/tokio-1.53.1/src/time/instant.rs:70-73`). Milliseconds too wide for `u64` saturate. Atomic task identifiers/counters can theoretically wrap after `2^64` increments; no feasible execution or externally controlled input was found that reaches that state.

### API boundary limitations, without a demonstrated production finding

1. In `spawn_supervised_with_heartbeat`, `make_future(heartbeat)` executes before `guard.run` installs the poll catcher (`139-154`). A factory panic is therefore outside that catcher. RAII still removes the record, but the guard's unfinished path classifies it as cancelled rather than the caught-panic path. Destructors and logging callbacks are likewise not covered by the future-poll catch.
2. `spawn_blocking_supervised` returns an outer async task's handle (`157-179`). Aborting that outer task can drop its guard while the inner blocking operation continues. The pinned Tokio contract explicitly says a dropped join handle detaches its task and started blocking work cannot be aborted (`runtime/task/join.rs:13-36`, `task/blocking.rs:102-120`). This API should not be interpreted as proof that blocking side effects have stopped after the outer handle is aborted.

**Caller check:** repository-wide Rust searches for the supervisor types/spawn/watchdog APIs found calls only in this package's own unit tests. Manifest searches for `oteryn-task-supervisor`/`task-supervisor` found its package and the workspace member entry, with no consuming package dependency. Thus neither limitation currently has a demonstrated production caller or audit-grade impact. They are not promoted to P1/P2 findings. The tests' successful assertions are not claimed as executed evidence.

## NTP clock: FFI and conversion

**FACT — The function only reads the kernel clock state.** The Linux path starts with a zeroed integer `timex`, including zero modes, supplies an exclusive mutable pointer for one `adjtimex` call, and immediately converts the returned fields (`lib.rs:73-91`). A failed state is rejected before use; there is no cached reading. The non-Linux branch returns `None` (`94-101`). This review did not inspect libc's complete implementation or the deployed kernel/time daemon.

**FACT — Known unsynchronized and unrepresentable readings fail closed.** Negative result, `TIME_ERROR`, `STA_UNSYNC`, a converted fractional component outside `[0, 1_000_000)`, negative maximum error, or overflow of seconds-to-microseconds/addition returns `None` (`40-70`). Nanosecond mode truncates to microseconds (`55-59`), as the embedded test specifies (`121-129`). The integer division has no overflow operand or zero divisor.

**Caller counterguards:** the actual consumer `crates/gameplay/social/src/premium/clock.rs:20-48` applies `TrustedNow::new`, serializes the process high-water update, refuses a reading wholly before its previous lower bound, and never lowers its observed upper bound. `premium/mod.rs:97-126` accepts only the configured uncertainty interval and uses saturating endpoint arithmetic. This prevents treating an arbitrary large kernel error as trusted Premium time and prevents a backward clock step from silently resetting that consumer's cutoff.

**Limit:** raw negative nanosecond fractions between -999 and -1 would truncate to zero before the converted range check. `KernelReading` is private and its only non-test producer is `adjtimex`; no repository input path that supplies such a malformed raw fraction was found. No actual clock/entitlement failure is inferred from that synthetic case. Supported cross-target ABI compilation was not run.

## Verification limits and conclusion

The seven requested files received complete static review, and their tests were read. The supplementary caller/dependency ranges above received selected review only. No claim is made about all cancellation callers, all Premium behavior, logging observer implementations, unbounded runtime scheduling, real kernel synchronization accuracy, or native test results.

**Conclusion:** this independent pass agrees with root's no-finding result for these three packages under the audit's real-caller/wrong-result requirement. The task supervisor's factory and blocking-abort boundaries should remain explicit limitations of any broader claim that it catches every panic or cancels all supervised work.
