# CI, merge conditions and conflicts

Merging, labelling, re-running and pushing stay with whoever the owner
authorised; this file only says how the flow behaves.

## How the gate works now

- `game-gate` (from `.github/workflows/merge-gate.yml`) is the only required
  check. It is an aggregator that turns red within seconds when any selected
  sub-job fails, so the cause is always in the failing leaf job: open that,
  not `game-gate` or `Merge gate / validate`.
- There is no merge queue (owner decision, since 2026-10-23). `merge-group-gate.yml`
  ("Game Merge Queue gate") runs on **push to main** after every merge: full
  workspace tests, every PostgreSQL lane, Windows client, node boot, Server
  Seam, Atlas fullworld. A red post-merge run means `main` is broken for
  everyone: that is a P0 and its fix PR goes first.
- The PostgreSQL harness (`pr-postgres-harness.yml`, checks named
  `PR early signal / Durability PostgreSQL harness (1..6)`) runs on a PR only
  when it carries the `heavy-tests` label and touches a game-server path. Add
  the label to a **finished** mechanic PR or to one batch PR that merges
  several finished ones, never to a work-in-progress head: each labelled push
  re-runs about an hour of runner time.
- Runner routing variables (`OTERYN_SELF_HOSTED_PG`,
  `OTERYN_SELF_HOSTED_RUST`) are read when a run is **created**. After a
  variable changes, runs already queued keep the old routing: cancel them and
  re-run, do not wait for them.
- A job that sits `queued` with no runner is a failure, not progress. Check
  whether the routing variable points at an offline self-hosted runner.

## Blind spot

`Node boot against the real Platform` and the
`Rust Linux workspace (integration-heavy | integration-rest)` shards run
**only after the merge**; the PR `game-gate` never runs them. A PR that
touches node startup, stack size, the dispatch runtime, integration tests,
or `content/` and `imports/` (the integration tests count their files) must
run the matching tests locally (`local-checks.md`) because CI will
only report them once `main` is already red.

## Merge conditions

Start every merge round with the state of `main`:

```sh
.claude/skills/oteryn-dev/scripts/main-health.sh 10
```

If the latest completed post-merge run is red, the next merge is the fix for
it (or the PR that already carries it); other merges wait until a post-merge
run on a later `main` is green or fails only on a job the fix does not touch.

A PR is mergeable when all of these hold on its **current** head SHA:

1. `game-gate` green and the PR `mergeable_state` is `clean`.
2. For a game-server change: all six PostgreSQL partitions green on that same
   head (root `AGENTS.md`: merge only a head whose own CI is green, the
   PostgreSQL legs included). The one exception is the owner's grouping
   decision (2026-10-29, also stated in root `AGENTS.md`): a member of a batch
   PR whose six partitions are green on the same `main`, with its own
   `game-gate` green, merged right after the other members with no other
   merge at all in between, so the final `main` is the tree the batch tested:
   `git rev-parse origin/main^{tree}` equals the batch head's `^{tree}`. A
   member that goes red or changes mid-way stops the landing; rebuild the
   batch from the members left on the new `main` and test it again.
3. Changes whose paths select no PostgreSQL leg (client, docs, tools/CI,
   test-only fixes of `main`) need only (1); their own CI is then complete.

Merge with squash and pass the full 40-character head SHA. After the merge,
watch the post-merge `Game Merge Queue gate` run on the new `main`, and check
for duplicate migration numbers:

```sh
.claude/skills/oteryn-dev/scripts/dup-migrations.sh origin/main
```

Any output is a P0 for the PR that merged second.

## Reading a red check

1. Confirm the run belongs to the current head (`head_sha` of the check run).
   A red result from an older head says nothing about the current one.
2. Open the leaf job log. A PostgreSQL partition prints the failing test near
   `FAIL` / `panicked at` / `TIMEOUT`; the log tail is usually only cluster
   shutdown, so search the whole log, not the last lines.
3. Ask whether `main` has the same failure (latest post-merge run on `main`).
   If yes, it is not this PR's. Land the main fix first and then merge
   `origin/main` into the waiting PRs; say so once on each PR. Port the fix
   into a branch only when that branch must merge before the fix can, and then
   as an exact cherry-pick of the fix commit; a hand-made copy costs a CI cycle
   and a later conflict in every PR it lands in. Before writing a new main
   fix, search the open PRs and recent branches for one that already exists
   (title, changed file, the constant or test name), and claim it in the
   coordinator's STATE so nobody writes a second copy.
4. Otherwise it is this PR's failure. Common causes, in the order they show
   up in this repo's history:
   - `main` moved under the PR (by far the most common): CI tests the PR
     merged with `main`, so something `main` added breaks it. Merge
     `origin/main`, rebuild and reproduce before reading anything else; this
     is one of the merges *Before every push* allows.
   - Migration, capability or command number taken on `main` in the meantime:
     renumber yours (`mechanic.md`, *Before writing code*).
   - Stale content pins or digests (`pin-check`), see `content-world.md`.
   - Lint after a merge: duplicated `mod` lines (E0428), duplicated test
     bodies, `expect`/`unwrap` in production code, `dead_code` on an unwired
     slice.
5. Re-run a job at most once, and only when the log shows an infrastructure
   death (checkout, install, runner lost) or a known lock-timeout signature.
   A second failure is real. "Flake" is never a root cause.

## Merge conflicts

Register the merge drivers once per clone (`tools/merge-driver/install.sh`),
then:

```sh
git merge --no-edit origin/main
cargo fmt --all
python3 tools/content-migration/regenerate_content.py --resolve   # = tools/merge-driver/regen.sh
```

- `oteryn-additive` resolves insert-only hunks (two `mod` lines, two registry
  entries at one place). After it, look for a duplicated import or block: the
  driver has produced duplicates before.
- Generated files (world pins, digests, derived registries) are regenerated,
  never hand-merged or hand-edited.
- Bring `main` in with a merge commit: it keeps every other checkout valid.
- Only a real edit-versus-edit conflict in hand-written logic stops for a
  person.

## Before every push

A red or cancelled run costs a full gate cycle (about 12 minutes) and, with
`heavy-tests`, about 30 more.

- Run the fast checks the gate runs, locally, on the exact tree you will push:
  `cargo fmt --all --check`, strict Clippy and the unit tests of every changed
  crate, plus the content and pin regeneration when you touched content
  (`local-checks.md` has the commands).
- Push once, when all of them pass. Collect follow-up fixes into the next push
  instead of pushing each one; a new push cancels the run in progress.
- Merge `origin/main` into the branch only when it conflicts, when the PR is
  red because `main` moved (*Reading a red check*), or once right before it is
  labelled `heavy-tests` or goes into a batch; not after every merge to
  `main`. Check for conflicts without merging:
  `git merge-tree --write-tree origin/main HEAD` (non-zero exit = conflict).
- Compile and test locally; CI confirms, it does not explore.

## Many PRs at once

- Keep about 10-12 PRs in CI at a time. Beyond that runners queue and every
  PR waits longer.
- Several finished mechanics that each touch game-server code: open one batch
  PR that merges their heads onto a fresh `main` (try the merge locally
  first), label only the batch `heavy-tests`, and once it is green merge the
  members one by one, back to back (*Merge conditions*, 2). Then close the
  batch.
- After each merge to `main`, check every open PR for a new conflict with
  `git merge-tree` before doing anything else, and fix only real conflicts; a
  PR that conflicts gets no CI.
- Every red or conflicted PR you own gets either a pushed fix or one comment
  naming the failing check and what blocks it.
