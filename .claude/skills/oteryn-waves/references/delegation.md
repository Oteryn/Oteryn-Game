# Delegating work and taking it back

A delegated agent knows only its packet. Most rework in this repository came
from packets that left out the base, the claimed numbers or the definition of
done, and from returned diffs that were pushed without review.

## How much to delegate at once

Send no more delegated tasks than results you can review and land in the
same day; unreviewed results pile up as open PRs that are later closed
unmerged. A delegated result lands on a branch; it becomes a PR only when it
is DONE or an `a` slice (`oteryn-dev` `references/mechanic.md`), and parked
work stays on its branch with a line in the tracking ledger, not as an open
PR.

## The packet

Write it as one self-contained file. Include, in this order:

0. **Consumer map (first, owner decision 2026-11-25):** for each symbol,
   value or behaviour the task changes: every caller and consumer, the
   invariants it touches, the scenario variants (solo and party, every
   consumer, error paths) and every test that asserts the changed value, all
   with `file:line`. The packet's writer reads callers and tests beyond its
   owned paths to confirm the map, within the `CONTEXT_ROUTING.md` read
   budget, and writes only inside the owned paths. A change that touches
   another subsystem's crate gets that owner's review of the map before code
   (`docs/architecture/subsystems/README.md`). For a risk class
   (`oteryn-coding`, *Risk classes*) the half-page plan follows the map and
   is reviewed before code.
1. **Task:** id, one-sentence goal, and whether the result must be DONE or a
   named PARTIAL slice (`oteryn-dev` `references/mechanic.md`, *What "done"
   means*).
2. **Base:** the exact `main` SHA to start from and the branch name. The
   worker's first step is to check that its `HEAD` equals that SHA, and to
   stop and report on a mismatch rather than build on another tree.
3. **Claimed numbers:** migration, capability, command and offer order,
   checked against `main` and open PRs at writing time.
4. **Owned paths:** the files and directories it may write. Everything else is
   read-only (read it freely for callers and tests); shared registration
   files are off limits (use the mounted directories and per-entry
   fragments).
5. **Scope and excluded scope:** what to build and what to leave alone, with
   the reference rules (manual note, TibiaData record, Canary/Crystal file
   and line) it must match.
6. **Success check:** the test or command that is red on the base and must be
   green on the result, named exactly (`oteryn-coding`, *Every task has a
   check*), plus the commands from `oteryn-dev` `references/local-checks.md`
   for the owned paths, and the pre-push list of `oteryn-coding` (*Before you
   push*). It must run where the worker runs: a PostgreSQL check only when
   the packet gives a database route (a PC lane's own database); otherwise a
   non-PostgreSQL test, with the PostgreSQL case run by the coordinator.
7. **Review:** for a Claude lane or child session that owns its branch,
   "you run the Codex review of your head and fix confirmed findings
   (`oteryn-review` §5 and its `references/batch-review.md`: one holistic
   whole-diff run, chunks only above about 1500 lines,
   every finding with no cap, pattern sweep, one batch fix)"; for a Codex lane, "no review, the coordinator
   reviews". Every content train PR and every PR with code in a risk class
   gets that review at effort high, also when it carries `heavy-tests`.
8. **Report:** at most 15 lines: result, head SHA or diff name, commands run
   with pass/fail counts, open questions with options and a recommendation,
   what is still PARTIAL, and the line `Codex review <head SHA>: ...`.
   The launching session reads a Codex Cloud task's final message with
   `poll` or `status` from its own chat. A code-writing task's packet also
   says: "As the last step, write `.codex-status` at the repository
   root: line 1 is the `OTERYN_SETUP` line from
   `/workspace/oteryn-env/logs/setup-status` (or `OTERYN_SETUP MISSING`), line 2 is
   `RESULT DONE|PARTIAL|BLOCKED <one-line reason>`, line 3 the success
   check with pass/fail counts." The launching session reads that file from
   the task's diff and deletes it before committing. A read-only review
   packet writes no file. A task with status `error` and no diff died before
   any work (setup, network or capacity) and is relaunched from the same
   packet.

Keep the packet short and decided: write it for an engineer who has never
seen the repository, so it names the files, the function and type names, the
values taken from the source and the test for each step, and leaves open only
what any competent engineer would choose the same way. Point at repository
files (`AGENTS.md`, task card, README of a directory) instead of pasting them.
State what to do rather than what to avoid; a prohibition belongs only in the
limits below. Send Codex work to Codex Cloud with model `gpt-6.2-sol` and
name the effort by risk (owner decision 2026-11-25): `high` for a risk class
(persistence, session fencing, protocol, durable value, migrations, several
consumers), `medium` for data, content, tests and simple slices. Start each
packet with "Reasoning effort: <high|medium>."; a Codex lane on the PC that
writes code uses the same setting. Codex review runs in Codex Cloud,
read-only, with `gpt-6.2-sol` at reasoning effort `high`, plus
the `gpt-6.2-astra` runs at `high` that the run-selection table
(`oteryn-review` `references/batch-review.md` §1) names, never at `medium` and
never `codex exec` on the PC (`oteryn-review` §5). Its verified findings are
fixed by a separate run (`oteryn-review` §5) at `high` for a P0/P1 in a risk
class and at `medium` for any other fix. Owner rule 2026-11-17 moved review
to Codex Cloud; the calibration decision of 2026-11-25 sets the review models
above and effort by risk for writing.

**Launch route (owner rule 2026-11-25).** Every session that needs Codex
Cloud (the coordinator, a lane, a child session) launches, polls and reads
its tasks from its own chat session with `scripts/codex-cloud.mjs`, using
that session's own `codex login`. The owner's PC is not a launch relay: no
`cxnew`, `cxrev`, `cxgo` or other PC wrapper submits a task for a cloud
session. A session without a login reports that to the coordinator, which
launches for it from its own chat.
Repository subagents use the model and effort
set in their own `.claude/agents/*.md` definition: `oteryn-hard-worker` for a
risk class, `oteryn-impl-worker` for the rest.

Submit a Codex Cloud task with `scripts/codex-cloud.mjs` (2026-11-12: the old
`codex cloud exec --env "Oteryn/Oteryn-Game"` route answers "environment not
found"). It needs an existing `codex login` and the environment variable
`OTERYN_CODEX_ENV_CONFIG_ID` (set in the cloud environment; ask the
coordinator if it is missing). It reads the login file itself; never open that
file. Commands: `node .claude/skills/oteryn-waves/scripts/codex-cloud.mjs`
`check` (environment ready), `submit --prompt-file <packet> --branch <ref>
--title <tag> --model <model> --effort <high|medium>` (prints the thread id;
always pass the model and effort explicitly), `poll <thread>`, `status <thread>`,
`send <thread> --prompt-file <follow-up> --model <thread's model> --effort
<high|medium>` (without `--model` a follow-up runs on `gpt-6.2-sol`), `list`. It calls the desktop
client's internal RPC, not an official API: if it fails, report the error
verbatim and do not retry a submit blindly.

## Limits to paste into every packet

The receiving agent cannot see this skill, so paste these lines verbatim:

- No Docker, no new third-party dependencies, no network access beyond what
  the task names; use only the database route this packet gives.
- Never weaken, skip or delete a test; never hand-edit pins or digests; never
  edit a merged migration.
- Never read credentials, tokens or `.env` files; never print a database URL.
- Do not commit notes, logs, validation JSON, `__pycache__`, evidence scratch
  directories or `*-FINAL.md` working files.
- Codex and child sessions: push only the packet's own branch, and only when
  the packet says so; never open PRs, merge or label. Repository subagents (`.claude/agents/`) push
  their own packet branch and report as their definition says, and never
  merge or label.

## Taking the result back

Before a returned diff goes onto a branch:

1. Apply it to a worktree on the packet's base and check it against current
   `main`: `git fetch origin main`, then
   `git merge-tree --write-tree origin/main HEAD`. Merge only on a conflict
   (exit 1, or GitHub reports the PR conflicted; `oteryn-dev`
   `references/ci-merge.md`, *Before every push*). Any other non-zero exit is
   an error to fix.
2. Read the full diff. Reject or trim: files outside the owned paths, stray
   notes/logs, weakened assertions, `#[ignore]`, hand-edited digests,
   renumbered migrations of other mechanics, unrelated refactors.
3. Check the claimed numbers are still free on `main`.
4. Run the success check and the validation yourself; the agent's own report
   is a claim, not evidence (`oteryn-verify`).
5. Review it (`oteryn-review`), push one commit with the result, then open or
   update the PR (`oteryn-dev` `references/mechanic.md`, *The PR*).

A task that died without a result (capacity error, timeout) is restarted from
the same packet, not silently dropped. Track every delegated task until its
result is merged, rejected or explicitly parked.
