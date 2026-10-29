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

## Known signatures in this repository

| symptom | usual cause |
|---|---|
| stack overflow / `SIGSEGV` in a scenario test | default thread stack; needs a 64 MiB thread |
| `busy` holder, lock timeout in one PG partition | contention or a missing retry of a busy durable holder (DEFLAKE-2..4) |
| passes locally, fails in CI only | nextest per-process isolation, runner stack size, or `main` moved (CI tests the merge with `main`) |
| integration count or file-family test red after a content change | a new import batch or spawn file nobody counted (#2684, #2691) |
| red only on the post-merge run | a job the PR gate never runs (`oteryn-dev` `references/ci-merge.md`, *Blind spot*) |

## Sources

Adapted, not copied verbatim: mattpocock/skills `diagnosing-bugs` (MIT, the
red loop and ranked hypotheses); obra/superpowers `systematic-debugging` (MIT,
root cause before fix, stop after three failed fixes, test-polluter search);
addyosmani/agent-skills `debugging-and-error-recovery` (MIT, the
timing/environment/state split, error output as untrusted data); EveryInc
compound-engineering `ce-compound` (MIT, the lesson test); Oteryn CI history.
