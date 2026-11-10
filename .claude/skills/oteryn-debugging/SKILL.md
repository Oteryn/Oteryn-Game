---
name: oteryn-debugging
description: Root-causing a failure in Oteryn-Game before any fix - a failing or flaky test, a red CI leaf job, a PostgreSQL partition timeout, a stack overflow, a red post-merge run on main, or behaviour that differs from the reference. Load when something fails or misbehaves, before writing the fix.
---

# Debugging in Oteryn-Game

No fix without a root cause. A fix pushed on a guess costs a CI cycle, and a
fix copied into many PRs costs one per PR. First decide whose failure it is:
if the latest post-merge run on `main` fails the same way, it is a main
failure (`oteryn-dev` `references/ci-merge.md`, *Reading a red check*).

## 1. Build a tight loop that goes red

This is most of the work. Find one command, run it, and show its output, that:

- goes **red** on this exact symptom (the same test name, panic or assertion
  as the CI log), not on a nearby failure;
- is **tight**: seconds, not a full suite. Narrow to one package, one test
  target and a name filter (`cargo test --locked -p <pkg> --test <target>
  <filter>`), or one PostgreSQL case;
- runs unattended.

Ways to get one, in order: the failing test itself, narrowed; a new test at
the seam that reaches the bug; a replay of the captured input (a frame, a
content record, a row) through the code path; a bisect over commits
(`git bisect run` with the loop) when it broke between two known states.

Read the whole error first: the CI log of the leaf job, the full panic and
backtrace, the failing assertion's values. A PostgreSQL partition prints the
failure near `FAIL`, `panicked at` or `TIMEOUT`, not in its last lines.

## 2. Flaky: raise the reproduction rate

A flaky test is a timing, environment or state problem until shown otherwise.

- **Timing:** run the loop 50-100 times (`for i in $(seq 100); do ... ||
  break; done`), add load (parallel runs, `CARGO_BUILD_JOBS`), and look for a
  fixed sleep, a missing readiness wait, an unbounded wait or a deadline that
  is too short.
- **Environment:** CI runs tests with cargo-nextest, one process per test,
  and some runners have a smaller default stack. Reproduce with
  `cargo nextest run -p <pkg> <filter>`; a stack overflow means the scenario
  needs its own 64 MiB thread (`oteryn-coding`, *Tests*).
- **State:** another test leaves rows, files or globals behind. Run the test
  alone and then with its neighbours; the one that turns it red is the
  polluter. PostgreSQL cases must use their own schema or rows.

Push the failure rate up until most runs fail; only then move on to
hypotheses.

## 3. Hypotheses, then one change at a time

Write 3-5 ranked hypotheses, each with a prediction: "if X is the cause, then
changing Y makes the loop green". Test the top one by changing one variable.
Instrument only to split hypotheses, and tag every debug line with one
prefix per session (`[DBG-<task>]`) so one grep removes them all.

## 4. Fix and keep the proof

- Fix the cause, not the symptom: a longer timeout, a retry or a sleep goes
  in only when the loop showed it is the cause.
- The loop becomes the regression test; it was red before the fix and is
  green after. Remove the tagged debug lines.
- If `main` has the same failure, follow `oteryn-dev`
  `references/ci-merge.md`, *Reading a red check*, step 3 (main fix first,
  one copy).
- Three failed fixes in a row mean the hypothesis is wrong or the design is:
  stop, write down what the three attempts ruled out, and re-plan.

## 5. Keep the lesson

Note the failure and its root cause in the report; the wave retro
(`oteryn-waves`, *Close the wave*) decides whether it becomes a check or a
skill line.

## 6. Red main: targeted fix

A red post-merge run blocks every merge, so the loop above must run in
minutes, not CI cycles. The owner's default (2026-11-02) is a targeted fix,
not a revert. MAINRED13 was red for about 37 hours because three rounds
searched the whole code base and used CI (about 30 minutes per try) as the
loop; the CI history named the culprit commit in minutes.

1. **Bisect the CI history before reading code.** For each failing leaf job,
   find the last green and the first red commit on `main` from the
   post-merge runs (`main-health.sh 30`, then the jobs of each run). A
   cancelled run has no result: dispatch the failing workflow
   (`gameplay-server-seam.yml`, `node-boot-qualification.yml`, or the
   post-merge gate) on the commits in between, or run `git bisect run` with
   the local loop from step 3. One commit in the range is the culprit.
2. **Read the last meaningful log line before the symptom**, not the symptom.
   "connection closed" said nothing; the line before it, `Wheel reset at
   admission did not commit`, pointed at the code in the culprit's diff.
3. **Reproduce locally, red before and green after.** Seam and node boot run
   on a developer machine with Docker and a Platform checkout at the
   harness's `PLATFORM_SHA`:
   - seam: `PLATFORM_SOURCE=<platform> OTERYN_TEST_POSTGRES_ADMIN_URL=<admin
     url> WP5_QUALIFICATION=seam bash tools/qualification/wp5_s3b/run.sh`,
     pass line `S3B_RESULT=SEAM_PASS`;
   - node boot: `PLATFORM_SOURCE=<platform> bash
     tools/qualification/node_boot/run.sh`, pass line
     `NODE_BOOT_RESULT=NODE_BOOT_PASS`;
   - PostgreSQL cases: the failing test filtered by name against a running
     PostgreSQL 18.6, never the full suite.
   Never export or print the admin URL in a shared log.
4. **Scope the fix by the evidence.** The fix packet names the culprit SHA,
   the failing log line, the code it points to and the local command from
   step 3. It does not ask for a search of the whole code base and does not
   restrict the fix to a file list.
5. **Push once.** Before the PR, dispatch seam and node boot on the fix branch
   and wait for both; then the PR, merged before anything else. Waiting PRs
   take `origin/main` afterwards (`oteryn-dev` `references/ci-merge.md`,
   *Reading a red check*, step 3).
6. **Revert is the fallback, not the default.** Offer the owner a revert of
   the culprit when step 3 cannot reproduce the failure within about an
   hour, or when the culprit is too large to fix in place. Name its cost:
   later commits may conflict, and the feature leaves `main` until it comes
   back with the fix.

## 7. Node boot and server diagnostics

<!-- LOGDBG: names below come from the four LOGDBG lanes; re-check them when
the lanes land. -->

Read a red node boot run from its causes, not its last lines.

- **Cause and root.** Every qualification `CHECK ... FAIL` line and every
  `SEAM_EVIDENCE e2e_all scenario=<row> status=FAIL` line ends with
  `cause=<kind> root=<row>/<check-or-precondition>`; kinds are `deadline`,
  `no-progress`, `precondition`, `transport`, `server-refused`, `assertion`
  and `blocked-by`. `failure_summary.py` prints the failed rows grouped by
  `root=` at the top of the stage output and in the job summary: fix the
  roots, the `blocked-by` rows follow them.
- **Fail-fast.** A row whose declared need (a fixture character released, an
  earlier row) already failed does not run: every check prints `FAIL
  cause=blocked-by root=<id> seconds=0`, still `status=FAIL`.
  `NODE_BOOT_NO_FAILFAST=1` runs every row, to see whether a blocked row also
  fails on its own.
- **No progress.** A row with no new PASS for `NODE_BOOT_NO_PROGRESS_S`
  (default 90) seconds ends with `cause=no-progress phase=<last phase>`; the
  row budgets themselves are unchanged.
- **Smaller runs.** `NODE_BOOT_E2E_ONLY=<groups>` (`core`, `f4`, `f5`, `f6`,
  `f7add`, `f7clock`, `f7cont`, `f7forge`, `f7run`) runs those groups and
  their prerequisites. Parallel stacks on one machine take `NODE_BOOT_BASE`,
  `NODE_BOOT_PROJECT`, `NODE_BOOT_SERVICE_USER`, `NODE_BOOT_SKIP_BASIC` and
  the `NODE_BOOT_PG_PORT` / `NODE_BOOT_PLATFORM_PORT` / `NODE_BOOT_GAME_PORT`
  overrides; unset, `run.sh` behaves as in CI.
- **Dev loop.** `NODE_BOOT_REUSE_STACK=1` (refused in CI) reruns only the
  seam stage against a stack that is already up: one hypothesis in minutes,
  not a 35-minute run.
- **PostgreSQL.** The node boot database logs lock waits and slow statements;
  `pg_observe.py` samples blocked, long and idle-in-transaction backends every
  2 s and dumps the top statements per stage into the failure-log directory.
- **Timeline.** `timeline.py --node-log node.log [--pg-activity <file>] --out
  trace.json` turns holder, connection, admission and PostgreSQL samples into
  a trace for <https://ui.perfetto.dev>.
- **Server detail events.** `holder_wait` (waiter, waited_ms, attempts,
  dominant_owner) names who held the durability holder; `release_deferred`
  says why an expired grace session was not released (the f6 cascade);
  `character_authority_check_failed stage=revision_chain` carries the
  receipt family, the violation and the revisions.
- **Client and server join.** A server `ProtocolError`, at join too, reaches
  the client as `E<code> (trace <hex>)`; grep the server log for that trace
  (`transport_close`, `serve_end`, `join_prepare_failed`). A plain EOF at
  join means the server closed without a frame.
- **Stuck tasks.** `tools/diagnostics/console-build.sh` builds the game
  server with tokio-console support into `target/diagnostics`; run it with
  `TOKIO_CONSOLE_BIND=127.0.0.1:6669` and attach `tokio-console`. Supervised
  tasks show their supervisor names. Local debugging only.

## Known signatures in this repository

| symptom | usual cause |
|---|---|
| stack overflow / `SIGSEGV` in a scenario test | default thread stack; needs a 64 MiB thread |
| `busy` holder, lock timeout in one PG partition | contention or a missing retry of a busy durable holder (DEFLAKE-2..4) |
| passes locally, fails in CI only | nextest per-process isolation, runner stack size, or `main` moved (CI tests the merge with `main`) |
| integration count or file-family test red after a content change | a new import batch or spawn file nobody counted (#2684, #2691) |
| red only on the post-merge run | a job the PR gate never runs (`oteryn-dev` `references/ci-merge.md`, *Blind spot*) |
| post-merge red for several commits, mixed with cancelled runs | the culprit is the first red after the last green of that job; bisect with dispatched runs (section 6) |

## Sources

Adapted, not copied verbatim: mattpocock/skills `diagnosing-bugs` (MIT, the
red loop and ranked hypotheses); obra/superpowers `systematic-debugging` (MIT,
root cause before fix, stop after three failed fixes, test-polluter search);
addyosmani/agent-skills `debugging-and-error-recovery` (MIT, the
timing/environment/state split, error output as untrusted data); EveryInc
compound-engineering `ce-compound` (MIT, the lesson test); Oteryn CI history.
