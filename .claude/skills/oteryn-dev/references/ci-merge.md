# CI, merge conditions and conflicts

Merging, labelling, re-running and pushing stay with whoever the owner
authorised; this file only says how the flow behaves.

## How the gate works now

- `game-gate` (from `.github/workflows/merge-gate.yml`) and `heavy-gate` are required
  checks; `heavy-gate` enforces the PostgreSQL harness on non-draft,
  non-Dependabot PRs labelled `heavy-tests` and passes for other PRs.
  `game-gate` is an aggregator that turns red within seconds when any selected
  sub-job fails, so the cause is always in the failing leaf job: open that,
  not `game-gate` or `Merge gate / validate`.
- There is no merge queue (owner decision, since 2026-10-23). `merge-group-gate.yml`
  ("Game Merge Queue gate") runs on **push to main** after every merge: full
  workspace tests, every PostgreSQL lane, Windows client, node boot, Server
  Seam, Atlas fullworld. A red post-merge run means `main` is broken for
  everyone: that is a P0 and its fix PR goes first.
- The PostgreSQL harness (`pr-postgres-harness.yml`, checks named
  `PR early signal / Durability PostgreSQL harness (1..24)`) runs on a PR only
  when it carries the `heavy-tests` label and touches a game-server path. Add
  the label to a **finished** mechanic PR or to one batch PR that merges
  several finished ones, never to a work-in-progress head: each labelled push
  re-runs about an hour of runner time (*Before every push*).
- The PR node boot shards (`PR heavy / Node boot ...`) run only on a PR
  labelled `node-boot` (owner 2026-11-18). Node boot is informational, so
  `heavy-gate` does not wait for it. Add the label to a PR that changes node
  startup, the dispatch runtime or the Game-Platform seam, or dispatch the
  `Node boot physical qualification` workflow on the branch.
- Runner routing variables (`OTERYN_SELF_HOSTED_PG`,
  `OTERYN_SELF_HOSTED_RUST`) are read when a run is **created**. After a
  variable changes, runs already queued keep the old routing: cancel them and
  re-run, do not wait for them.
- A job that sits `queued` with no runner is a failure, not progress. Check
  whether the routing variable points at an offline self-hosted runner.
- `auto` (and `auto-split` for PG) routes to Molehill only while the PC
  heartbeat `OTERYN_MOLEHILL_SEEN` (Unix seconds, written every 5 minutes by
  `molehill-heartbeat.sh` while a runner listener is alive) is at most 10
  minutes old; otherwise the run goes hosted. The `Molehill fallback` workflow
  cancels and re-runs, every 15 minutes, runs still waiting on an offline PC.

## Shared build artifacts and partial reruns

The PR fast Rust gate builds each archive in `rust-fast-archive.yml` and starts
that archive's five test workers as soon as its producer finishes. The parent
collects the complete planned archive union before recording a package result.
The collector resolves each role's latest actual job attempt: an unchanged
producer may be reused after a partial rerun, but an older success cannot hide
a newer failed, skipped or incomplete attempt. Preserve the exact run, source
SHA, artifact ID, digest, runner identity and test inventory checks.

The three dedicated Dispatch archives use one canonical workspace feature
recipe for both fast tests and PostgreSQL. The PG producer checks for a verified
same-head archive before installing build tools or restoring a Cargo target.
If no compatible producer is available, it builds locally; an invalid selected
artifact fails. Route-less fast results never count as PostgreSQL qualification.
Use `check_dispatch_pg_shards.py 3 --test <full-test-name>` for the focused
offline-death compiler shard; its workflow measures build and test separately.

Node boot and Server Seam share the pinned Platform executable. The release
World compiler is shared within its workflow run; its consumers use the actual
producer artifact ID and verify source, features, profile and SHA256. Every
consumer still creates its own database and keys. Keep production, spawn, clock,
test and development feature variants separate whenever their closures differ;
the final production smoke build still qualifies its original feature graph.

WorldProject builds its compatible native tools together but still performs two
independent materializations and the completion/idempotence checks. Content
migration uses one bounded pool of 40 checks before the global validator. Native
entry's common producer runs only when its qualification is requested; an
unavailable compatible artifact permits a fallback build alongside real Platform
issuance, while an invalid supplied artifact fails.

After changing pinned workflow/helper bytes, run
`python tools/repository/refresh_ci_workflow_pins.py`, inspect the complete
generated rotation, then run it with `--check` and the repository policy and
gate contract tests. The generator proposes hashes, not authority: the
protected-base audit still rejects changes to its own file and requires the
accepted owner authorization and independent review for a rotation.

## Blind spot

`Node boot against the real Platform`, `Server Seam` and the
`Rust Linux workspace (integration-heavy | integration-rest)` shards run
**only after the merge**; the PR `game-gate` never runs them. A PR that
touches node startup, stack size, the dispatch runtime, integration tests,
or `content/` and `imports/` (the integration tests count their files) must
run the matching tests locally (`local-checks.md`) because CI will
only report them once `main` is already red.
When one of them turns `main` red, fix it with `oteryn-debugging` section 6
(*Red main: targeted fix*): bisect the CI history first, then a local loop.

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
2. For a game-server change: all twenty-four PostgreSQL partitions green on that same
   head (root `AGENTS.md`: merge only a head whose own CI is green, the
   PostgreSQL legs included). The one exception is the owner's grouping
   decision (2026-10-29, also stated in root `AGENTS.md`): a member of a batch
   PR whose twenty-four partitions are green on the same `main`, with its own
   `game-gate` green, merged right after the other members with no other
   merge at all in between, so the final `main` is the tree the batch tested:
   `git rev-parse origin/main^{tree}` equals the batch head's `^{tree}`. A
   member that goes red or changes mid-way stops the landing; rebuild the
   batch from the members left on the new `main` and test it again.
3. Changes whose paths select no PostgreSQL leg (client, docs, tools/CI,
   test-only fixes of `main`) need only (1); their own CI is then complete.
4. The PR has its Codex review (`oteryn-review` §5): a
   `Codex review <SHA>` line in the report or a PR comment, and every
   confirmed P0/P1 from it fixed on the head being merged. A PR without it is
   not merged; the merger runs the review first.

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
   If yes, it is not this PR's. Land the main fix first and say so once on
   each waiting PR; do not merge `origin/main` into them while they wait.
   Recovery for a waiting PR whose head still fails for that defect: once the
   fix is on `main`, merge `origin/main` into it once, on the otherwise-ready
   head, and name the fix SHA. Its rerun is then also the
   final-head run (*Before every push*, case 2). Cherry-pick the fix
   into a branch only when that branch must merge before the fix lands, and then
   as an exact cherry-pick of the fix commit; a hand-made copy costs a CI cycle
   and a later conflict in every PR it lands in. Before writing a new main
   fix, search the open PRs and recent branches for one that already exists
   (title, changed file, the constant or test name), and claim it in the
   coordinator's STATE so nobody writes a second copy.
4. Otherwise it is this PR's failure. Common causes, in the order they show
   up in this repo's history:
   - `main` moved under the PR: CI tests the PR head SHA, not the PR merged
     with `main`, so a test that also fails on `main` is main-red (step 3).
     Merge `origin/main` for it only as step 3 says; otherwise merge only on a
     conflict (*Before every push*).
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

Merge only when *Before every push* allows it. Register the merge drivers
once per clone (`tools/merge-driver/install.sh`), then:

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
- After merging `main` into an older PR, run the architecture check
  (`local-checks.md`, step 4) before pushing: ratchets added on `main` later,
  such as the client size baseline, turn an old green PR red.

## Before every push

A red or cancelled run costs a full gate cycle (about 12 minutes) and, with
`heavy-tests`, about 30 more.

- Run the fast checks the gate runs, locally, on the exact tree you will push:
  `cargo fmt --all --check`, strict Clippy and the unit tests of every changed
  crate, plus the content and pin regeneration when you touched content
  (`local-checks.md` has the commands).
- Run `python3 tools/repository/reliability_checklist.py` and every check it
  lists; it maps the diff to failures that already reached a gate twice.
- Push once, when all of them pass. Collect follow-up fixes into the next push
  instead of pushing each one; a new push cancels the run in progress.
- Never merge `origin/main` after every merge to `main`, to "stay current",
  for STALE or before labelling. A package or batch branch created from a
  fresh `main` is not a merge. Cherry-pick a fix only into a PR that must
  merge before the main fix lands (*Reading a red check*, step 3).
- Merge `origin/main` into the branch only in these two cases (owner
  2026-11-18):
  1. It conflicts. A conflict means GitHub reports the PR conflicted
     (`mergeable_state` `dirty`, "This branch has conflicts"), or, after
     `git fetch origin main`, `git merge-tree --write-tree origin/main HEAD`
     exits exactly 1. Any other non-zero exit is an error to fix, not a
     conflict. A clean local `merge-tree` does not clear a conflict that
     GitHub reports: GitHub ignores the custom merge drivers.
  2. Main-red: the failing test also fails on `main` at the PR's merge base,
     the fix is already on `main` and you name it by its SHA. Merge once, on
     the otherwise-ready head, so that rerun is also the final-head run.
- A `heavy-tests` PR re-runs about an hour of PostgreSQL work on every push.
  Remove the label before any repair push and add it back on the final head
  (adding it starts the run).
- Compile and test locally; CI confirms, it does not explore.

## Many PRs at once

- Keep about 10-12 PRs in CI at a time. Beyond that runners queue and every
  PR waits longer.
- Several finished mechanics that each touch game-server code: open one batch
  PR that merges their heads onto a fresh `main` (try the merge locally
  first), label only the batch `heavy-tests`, and once it is green merge the
  members one by one, back to back (*Merge conditions*, 2). Then close the
  batch.
- After each merge to `main`, check every open PR for a new conflict
  (*Before every push*, case 1) before doing anything else, and fix only real
  conflicts; a
  PR that conflicts gets no CI.
- Every red or conflicted PR you own gets either a pushed fix or one comment
  naming the failing check and what blocks it.

New shared dispatch PostgreSQL fixtures go into `postgres_tests/support/`, never into a test entry.
