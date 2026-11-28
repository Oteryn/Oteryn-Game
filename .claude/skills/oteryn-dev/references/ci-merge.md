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
  the label to a **finished** mechanic PR or to the content train that merges
  several finished units, never to a work-in-progress head: each labelled push
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

## Protected-main compilation cache

The repository cache eviction limit was verified as 10GB. A successful save of
an 11.26GB cache did not make it a durable reusable entry: the later full API
inventory contained no Rust cache. Keep the four existing cache paths and keys,
prepare a bounded target before recording normalized mtimes, and enforce a
4GiB total raw-file budget including Cargo registry/git data before upload.
Only generated target contents may be pruned; immutable source and dependency
archives remain untouched. Missing build products must rebuild through Cargo.
Report actual before/after bytes and preserve exact-candidate tests independently.
A protected-main warmer finishes before the newest queued main revision starts;
frequent merges must not continually cancel cache preparation.
Warm workspace library tests with `--lib --no-run`: keep their dev-dependency
features, but avoid linking integration executables that the bounded cache
discards. Full selected tests, binary targets and strict all-target Clippy remain
in their qualification lanes. Use the archive producers' checksum-pinned mold
through `LD_PRELOAD`, without adding linker-specific Cargo flags. A cache-only
warmup is not evidence that the candidate's tests passed. On Windows, run strict
Clippy before the release build so a lint failure stops before expensive linking;
retain the release smoke and installer checks after the build.

## Shared build artifacts and partial reruns

The PR fast Rust gate builds each archive in `rust-fast-archive.yml` and starts
that archive's five test workers as soon as its producer finishes. The parent
collects the complete planned archive union before recording a package result.
The reusable producer is a canonical routing control and a consumer root:
changes to its referenced build helpers retain full qualification coverage.
The collector resolves each role's latest actual job attempt: an unchanged
producer may be reused after a partial rerun, but an older success cannot hide
a newer failed, skipped or incomplete attempt. Preserve the exact run, source
SHA, artifact ID, digest, runner identity and test inventory checks.
GitHub may copy a completed job into a later attempt with a new API ID while
retaining its original execution times. Resolve only an unambiguous identical
execution record to its original producer attempt; never pick an older artifact
merely because it exists. Qualify a collector-only partial rerun on the frozen
head and exercise native/World consumers against the resulting copied records.

The three dedicated Dispatch archives use one canonical workspace feature
recipe for both fast tests and PostgreSQL. The PG producer checks for a verified
same-head archive before installing build tools or restoring a Cargo target.
If no compatible producer is available, it builds locally; an invalid selected
artifact fails. Route-less fast results never count as PostgreSQL qualification.
For a deliberate reuse qualification, apply `heavy-tests` to the frozen head
once all three dedicated producers are active or their artifacts are available.
An overall workflow can still report `queued` while its selected producer is
active; honor the bounded active-producer wait in either workflow state.
Queued or undiscovered producers can exhaust the bounded discovery window and
cause local fallback; report those builds separately from successful reuse.
Waiting for producer admission can delay other PG archives, so measure the
complete critical path as well as summed runner time.
The exact MAX501 retirement test reserves all nextest slots on its worker via
the generated planner config. Its production two-second transaction budget and
501-item assertions remain intact; other cases retain ordinary parallelism.
This isolates a hard-budget case from competing fixture processes. Qualify the
actual PG result separately from the scheduler's own protocol tests.
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
migration runs pinned-source contracts, the bounded pool of 40 checks, and the
full validator plus StarterKit seals in three independent exact-head workspaces.
The original named aggregate requires success from all three; child failure,
cancellation or skipping cannot pass it. Native entry's common producer runs
only when its qualification is requested; an
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

## The staging integration branch

Owner decision 2026-11-23. `staging` is a long-lived integration branch
between the lanes and `main`. Create both rulesets below and verify them
before the first lane PR, and in any case before the first promotion. To
verify, read back the effective rules with
`gh api repos/Oteryn/Oteryn-Game/rules/branches/staging` and confirm that
`deletion`, `non_fast_forward`, `pull_request` (squash only) and
`required_status_checks` (`game-gate`) are listed. Then read each ruleset by
id and check its bypass actors: `gh api repos/Oteryn/Oteryn-Game/rulesets/27896004`
(*Protect staging integrity*) must have none, and
`gh api repos/Oteryn/Oteryn-Game/rulesets/27896005` (*Gate staging*) exactly
the admin repository role (`actor_id` 5, `RepositoryRole`, `always`). Never
test them with a real force-push or deletion.

- **Lane PRs go into `staging`.** Each lane works on its own short branch and
  opens its PR with base `staging`. Such a PR runs the light `game-gate`
  (`merge-gate.yml` accepts base `main` and `staging`, about 12 minutes) and
  `codex-review-gate`. The PostgreSQL harness, `heavy-gate` and the PR node
  boot shards stay main-only, so they never run here. Before pushing a
  game-server change, run its PostgreSQL partitions on the PC's staging
  checkout (`STG pg`). Squash-merge once `game-gate` is green on the head and
  the Codex review is in (*Merge conditions*, 4). Archive the task record in
  the PR's own final commit: `auto-archive-merged-tasks.yml` only archives
  records whose PR merged into `main`, because a task completes on `main`.
- **Heavy tests roll on the staging tip.** One standing PR `staging -> main`
  is labelled `heavy-tests` and stays **ready for review, not a draft**: the
  harness skips drafts. Each push to `staging` runs the harness on the new
  tip. For a head `staging`, `pr-postgres-harness.yml` does not cancel the
  run in progress, so a 26-35 minute run finishes even though `staging` moves
  about every 12 minutes. The newest push waits as the one pending run.
- **Promotion happens on the owner's word.** Declare a promotion freeze.
  From then until the promotion has merged, nothing merges into `staging` and
  nothing else merges into `main`, the auto-archive bot PR included. Wait for
  `game-gate` and `heavy-gate` to be green on that exact head. Then
  squash-merge the `staging -> main` PR with its 40-character head SHA, under
  the `main` rules in *Merge conditions*. If `main` moved after the promotion
  head was qualified, run the sync below first: merge `main` into `staging`,
  then qualify the new head again.
- **No retargets.** Do not retarget a PR between `staging` and `main`;
  close it and open a new PR with the right base. Until the retarget run's
  `game-gate` fails (about a minute), the old base's green `game-gate` is
  still the head's check, so a retargeted PR is never merged before a new
  push has been qualified. `game-gate` has one name
  for both bases, so a retarget fails `game-gate` at once (the scope job
  rejects any `edited` event that changed the base, and its run is titled
  `... • retargeted`). The head stays tainted until a new push: every
  `game-gate` attempt (later edits, reruns and job retries included) checks
  the live head and base ref against its own scope and fails on a head that
  has a retargeted run. Other edits (title, body) cancel nothing: in the
  harness they only mirror the `heavy-gate` of the newest real heavy run
  (by run creation, at its latest attempt) on the head, and fail while it is
  still running. Every run's second `heavy-gate` job republishes that verdict
  again once `heavy-tests` is live; in a real run it first waits until every
  other harness run on the head has finished, and fails rather than publish
  while one is still going.
- **Codex review status.** A PR into `staging` publishes the status
  `codex-review-gate (staging)`, so it never replaces the `codex-review-gate`
  of a `main` PR on the same head SHA. The `heavy-tests` exemption applies
  only to PRs into `main`.
- **Sync after promotion, with no force-push.** The squash leaves `staging`
  with history that `main` does not contain. The coordinator then:
  1. `git fetch origin main staging && git switch -c sync/staging-<date> origin/staging && git merge --no-edit origin/main`
     (a real merge commit; with no other commits on `main`, its tree equals
     `main`).
  2. Pushes the sync branch and opens a PR into `staging`. `game-gate` runs on
     that merge commit.
  3. Once `game-gate` is green, `git push origin <sha>:refs/heads/staging`, a
     fast-forward of the same commit. GitHub marks the sync PR merged. Do not
     press a merge button: a squash would drop `main` as a parent and the
     next promotion diff would repeat the last cycle.
  4. Lifts the freeze and opens the next standing `staging -> main` PR
     (ready for review, labelled `heavy-tests`).
- **Keeping `staging` alive.** `delete_branch_on_merge` is on and the
  promotion PR's head is `staging`. The deletion rule below has no bypass
  actor, so nobody's merge, auto-delete included, can delete it. This is not
  verified live (`staging` did not exist when this was written): after the
  first promotion, check that `staging` still exists. If it was deleted,
  restore it with GitHub's *Restore branch* button on the promotion PR.
  `terminal-branch-lifecycle.yml` never deletes a protected branch.

Two rulesets, because a bypass actor bypasses every rule in its ruleset:

```json
{
  "name": "Protect staging integrity",
  "target": "branch",
  "enforcement": "active",
  "bypass_actors": [],
  "conditions": {"ref_name": {"include": ["refs/heads/staging"], "exclude": []}},
  "rules": [{"type": "deletion"}, {"type": "non_fast_forward"}]
}
```

```json
{
  "name": "Gate staging",
  "target": "branch",
  "enforcement": "active",
  "bypass_actors": [{"actor_id": 5, "actor_type": "RepositoryRole", "bypass_mode": "always"}],
  "conditions": {"ref_name": {"include": ["refs/heads/staging"], "exclude": []}},
  "rules": [
    {"type": "pull_request", "parameters": {
      "allowed_merge_methods": ["squash"],
      "dismiss_stale_reviews_on_push": true,
      "require_code_owner_review": false,
      "require_last_push_approval": false,
      "required_approving_review_count": 0,
      "required_review_thread_resolution": true}},
    {"type": "required_status_checks", "parameters": {
      "do_not_enforce_on_create": true,
      "required_status_checks": [{"context": "game-gate"}],
      "strict_required_status_checks_policy": false}}
  ]
}
```

Neither ruleset has `required_linear_history`, so the sync merge commit can
land. The bypass actor (repository role 5, admin: the coordinator's identity)
is used only for step 3 of the sync. `do_not_enforce_on_create` lets the
coordinator create `staging` from `main`. The names differ from `Protect
main`, so `tools/repository/apply_github_settings.py` leaves both rulesets
alone.

## Merge conditions

Start every merge round with the state of `main`:

```sh
.claude/skills/oteryn-dev/scripts/main-health.sh 10
```

If the latest completed post-merge run is red, the next merge is the fix for
it (*Red `main`*); other merges wait until a post-merge run on a later `main`
is green or fails only on a job the fix does not touch.

A PR is mergeable when all of these hold on its **current** head SHA:

1. `game-gate` green and the PR `mergeable_state` is `clean`.
2. For a game-server change: all twenty-four PostgreSQL partitions green on that same
   head (root `AGENTS.md`: merge only a head whose own CI is green, the
   PostgreSQL legs included). The one exception is the owner's grouping
   decision (2026-10-29, also stated in root `AGENTS.md`): a unit of a content
   train (*The content train*) whose twenty-four partitions are green on the
   same `main`, with its own `game-gate` green, merged right after the other
   units with no other merge at all in between, so the final `main` is the
   tree the train tested: `git rev-parse origin/main^{tree}` equals the train
   head's `^{tree}`. A unit that goes red or changes mid-way stops the
   landing; rebuild the train from the units left on the new `main` and test
   it again.
3. Changes whose paths select no PostgreSQL leg (client, docs, tools/CI,
   test-only fixes of `main`) need only (1); their own CI is then complete.
4. The PR has its Codex review (`oteryn-review` §5): a
   `Codex review <SHA>` line in the report or a PR comment, and every
   confirmed P0/P1 from it fixed on the head being merged. A PR without it is
   not merged; the merger runs the review first. The `heavy-tests` label does
   not waive it for a content train PR or a PR with code in a risk class
   (owner 2026-11-25); only a data-only `heavy-tests` PR keeps the automatic
   skip.
5. **Stale base.** The PR's tested merge base is older than the current
   `main`, and `main` has changed a `.rs`, `Cargo.toml` or `Cargo.lock` file
   since then. In that case `cargo check --workspace --all-targets` passes on
   the local merge result (`git merge --no-commit origin/main` in a scratch
   worktree, never pushed). This is not a branch update, so the rule against
   merging `origin/main` below still holds. Two heads that were each green
   can still fail to compile together: #3516 and #3518 broke `main` this way.

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
  for STALE or before labelling. A train branch created from a
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
  PR waits longer. Finish and merge before starting new work (owner
  2026-11-25): at most one content train in CI plus one in build.
- After each merge to `main`, check every open PR for a new conflict
  (*Before every push*, case 1) before doing anything else, and fix only real
  conflicts; a
  PR that conflicts gets no CI.
- Every red or conflicted PR you own gets either a pushed fix or one comment
  naming the failing check and what blocks it.

New shared dispatch PostgreSQL fixtures go into `postgres_tests/support/`, never into a test entry.

## The content train

Owner decision 2026-11-25; it replaces the older package and batch wording.

- **One writer.** The train branch is `claude/happy-davinci-ijz3wy-ctrain-N`,
  cut from a fresh `main` (try each merge locally first). Lanes push unit
  branches; only the writer merges them into the train. While the staging
  flow (*The staging integration branch*) is in use, a unit still reaches
  `main` by the route that section names.
- **Merge and regenerate.** The writer merges the unit branches with the merge
  drivers (`tools/merge-driver/install.sh`), then runs `cargo fmt` and
  regenerates the derived files (pins, registries, goldens) once, on the
  train. Each unit still carries its own regenerated derived files and
  passes its own `pin-check`, so it can land or be dropped alone.
- **A unit is a coherent piece that can be dropped alone.** Its risk class
  (`oteryn-coding`, *Risk classes*) matters more than its size. There is no
  line cap.
- **A red unit leaves the train.** Fix it on its own branch and rebuild the
  train from the units left. A main-red fix goes outside the train.
- **Test and review.** Label only the train `heavy-tests`. Its Codex review
  runs at effort high despite the label (`oteryn-review` §5). When it is
  green, merge the units one by one, back to back (*Merge conditions*, 2).
  Then close the train. When merging the units changed a derived file (pin,
  registry, golden) beyond what the units carry, the units alone cannot
  reproduce the tested tree: the train PR itself lands (squash) and the unit
  PRs are closed with a link to it.

## Red `main`

Owner decision 2026-11-25.

- Revert the culprit PR at once. Find it by bisecting the post-merge runs
  (`oteryn-debugging` section 6). When the culprit is the last merge, the
  revert is good only when its tree equals the last green `main` tree
  (`git rev-parse <revert>^{tree}` and `git rev-parse <last-green>^{tree}`
  are the same). When independent PRs merged after it, check instead that the
  revert is a plain `git revert <culprit>` with no hand edit (`git show
  <culprit> | git patch-id --stable` equals `git diff <revert> <revert>^ |
  git patch-id --stable`) and that the red job is green on the revert head.
- A forward fix is allowed only when a revert is impossible (later merges
  depend on the culprit, a migration is already applied) or the fix is
  already green on its own head. Search the open PRs for an existing fix
  first (*Reading a red check*, step 3). A forward fix that meets the
  hotfix conditions below merges as described there.
- A main-red fix never travels in a content train.
- **Hotfix merge without waiting for CI** (owner 2026-11-26). This is the
  second exception to root `AGENTS.md` "merge only a head whose own CI is
  green" and to *Merge conditions* 1-3 (no wait for `game-gate`, PostgreSQL legs or `heavy-gate`); conditions 4 and 5 still hold. A fix of a red `main` merges as soon as
  all of these hold on its current head:
  - The PR targets `main` and the GitHub PR `additions + deletions` is at most
    20.
  - No changed path matches `^\.github/`, `^tools/repository/`,
    `/migrations/` or `repository-policy`.
  - `cargo fmt --all --check`, Clippy (`-D warnings`, all targets) and the lib
    tests of each changed crate pass locally on that head.
  - *Merge conditions* 4 holds: a clean Codex review receipt for that head.
  - No check run on that head has completed as `failure`. The one exception
    is `heavy-gate`: it never blocks a hotfix, whether it is a mirror or real,
    pending or failed.

  Revert-first and the seam and node-boot wait of `oteryn-debugging` step 5
  do not hold this fix back. The coordinator merges it with
  `C:\Users\Public\fm-hotfix.ps1 -Num <pr> -Sha <head>`. The script re-checks
  these conditions, bypasses ruleset 20991995 for the one squash and
  restores the ruleset in `finally` (`docs/agents/AGENTS.md`, Game override
  of the bypass substitute). The post-merge run on `main` is the
  verification: if it is red, revert the hotfix.
- **Hotfix merge without waiting for CI** (owner 2026-11-26). A main-red fix
  merges as soon as all of the following hold. It is not held for
  `game-gate` or `heavy-gate`.
  - The PR targets `main` and changes at most 20 lines.
  - It touches no `.github/`, `tools/repository/`, migration or policy path.
  - `cargo fmt`, Clippy (`-D warnings`, all targets) and the lib tests of
    each changed crate pass locally on the head.
  - A clean Codex review receipt exists for the head.
  - No check has already completed as a failure. A `heavy-gate` mirror from
    before the label was added does not count.

  The coordinator merges it with `C:\Users\Public\fm-hotfix.ps1 -Num <pr>
  -Sha <head>`, which checks these conditions, bypasses ruleset 20991995 for
  the squash and restores the ruleset in `finally`. The post-merge run on
  `main` is the verification: if it is red, revert the hotfix.
- **Semantic conflicts between green PRs.** Before squashing a PR whose CI
  base is older than `main`, check whether `main` has since changed a crate
  the PR touches. If it has, run `cargo check` on the merge result, or merge
  `main` into the PR and wait for CI. Two heads that were each green can
  still fail to compile together (#3516 and #3518 broke `main` this way).
