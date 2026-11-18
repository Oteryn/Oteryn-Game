# The owner's review loop: finding logic errors across a whole area

Read this only when running a review loop the owner asked for (`SKILL.md` §9,
authorization in `docs/agents/OWNER_FUNDED_AI_POLICY.md`, *Owner review loop*).
A per-PR review follows `SKILL.md` §0-§8 and `batch-review.md` instead.

A product is far larger than one review run can read, so a loop does not try
to read "all the code". It reviews the code through cuts that each follow one
rule of the game across every module that touches it. Logic errors live
between modules; a file-by-file read misses them.

## 1. Charter (written before round 1, approved by the owner)

One file per loop, not changed between rounds, so rounds are comparable:

- **Scope:** the area (for example Bazaar), its crates and tables, and the
  stop bar: zero confirmed P0/P1, and zero P2 with a traced reachable path, on
  v1-reachable code; P0/P1 only behind an off flag or in tooling.
- **Rules (invariants):** the behaviour the code must have, taken from written
  sources only: the durable invariants of root `AGENTS.md`, `docs/contracts/`,
  `docs/architecture/`, parity tables and `CODEX-CHECKLIST.md`. A rule with no
  written source is a batched owner question, never a reviewer's guess.
- **Map:** for each state the area owns (a table, a ledger, a lifecycle) the
  list of every writer, built by commands (grep of SQL writes, of the dispatch
  registry and of the public functions that call them), one row per writer
  with `file:line`. The coordinator spot-checks it. It is an input, not
  evidence: reviewers report where the code disagrees with it.
- **Cuts and budget:** the runs of one round (section 2) and the round budget
  (default 3 rounds without a fall in new confirmed P0/P1, then stop and
  report the trend to the owner).

## 2. One round: three logical cuts, chunks as the baseline

All runs of a round review one exact head of `main`, started only when every
fix PR of the previous round has merged. They run in parallel, each a
read-only Codex Cloud task (`batch-review.md` §1-§2: every finding, no cap,
sweep each class, the checklist appended).

| Cut | One run per | The run gets | It must return |
|---|---|---|---|
| State | rule, or lifecycle table | the rule, the writer list from the map | a table: writer, `file:line`, holds or breaks, the path when it breaks |
| Flow | end-to-end flow (list, bid, settle, cancel, recover after a crash) | the flow's entry point and the map | the trace across modules, step by step, and each break; brief includes the cheating-player cases (two clients at once, disconnect midway, replayed frame, restart during a write) |
| Boundary | contract (client and server, Game and Platform) | both sides' code, or one side with the contract and fixtures when the other repository is not readable | each field and outcome where the two sides disagree |

A writer, step or field missing from a returned table means "not reviewed",
not clean. Chunk runs (`batch-review.md`, about 400-500 lines of one module)
cover only the files no cut reached, and the files a cut flagged.

## 3. Findings ledger and synthesis

- One ledger per loop lists every finding ever reported, with its verdict
  (`SKILL.md` §6) and `file:line`. Every run gets it as "known; do not
  re-report unless the named code changed". New findings are deduplicated
  against the whole ledger. Only new confirmed findings count toward the stop
  bar and the trend.
- After the merge of a round's results, one synthesis step groups the
  confirmed findings by root cause and lists each state, flow or boundary no
  run covered. A group of P0/P1 with one root cause goes to `SKILL.md` §8; an
  uncovered item gets a run in the next round.

## 4. Between rounds

- Triage per `SKILL.md` §6, sweep per `batch-review.md` §3.
- Verification of a round's P0/P1: read-only Codex Cloud runs grouped by
  area, each told to trace the real caller and to default to refuted when
  unsure (authorized for loops only, `OWNER_FUNDED_AI_POLICY.md`, *Owner review
  loop*); a Claude subagent may take the hard calls as a second opinion.
- A P0 gets its own fix PR at once. Other confirmed P0/P1 go into a few fix
  PRs grouped by area, with disjoint files so their lanes run in parallel.
  P2 go into one grouped P2 PR per round, opened after that round's P0/P1
  PRs merge. Each fix PR is an ordinary PR with its own per-PR review.
- Each confirmed logic error gets a test through the real caller (PG case,
  dispatch path or node-boot row) that fails before the fix.
- Out-of-scope findings go to their owning lanes with their severity.

## 5. Stop

- **Done:** a round with nothing over the stop bar, then one confirmation
  round on the same head with a different cut (by flow instead of by state,
  or refute and completeness briefs), also clean. Every run of both finished.
- **Not converging:** the same P0/P1 class back in the same module or flow,
  or a regression of an earlier fix, sends that area to `SKILL.md` §8 before
  the next round. The round budget (section 1) ends the loop with a report.

## 6. Pilot

Before a loop covers a whole product, run it on one value flow (the first
pilot: Bazaar, owner 2026-11-18) and record for each cut how many new
confirmed findings it produced and what it cost in runs. Keep the cuts that
pay for later loops; write the result into this file.
