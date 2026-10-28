---
name: steward
description: Drive an Oteryn-Game pull request to a mergeable state - read CI on the current head, tell a PR failure from a main failure, handle merge conflicts with the repo merge drivers, decide when the PostgreSQL harness must run, and keep many open PRs moving without leaving any of them idle. Use when watching, babysitting or fixing CI on Game PRs, or when several PRs are waiting.
---

# Steward: driving Game PRs to green

This skill records how this repository's CI and merge flow behave today. It
does not grant authority: merging, labelling, re-running and pushing stay with
whoever the owner authorised (root `AGENTS.md`, *Repository boundaries*).
Machine contracts win over this text (`docs/agents/BUILD_TEST_MATRIX.md`,
*Machine authority*); if they disagree, fix this file.

## How the gate works now

- `game-gate` (from `.github/workflows/merge-gate.yml`) is the only required
  check. It is an aggregator: it turns red within seconds when any selected
  sub-job fails. Never diagnose `game-gate` or `Merge gate / validate`; open
  the failing leaf job.
- There is no merge queue (owner decision). `merge-group-gate.yml` ("Game
  Merge Queue gate") runs on **push to main** after every merge: full
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

## Merge conditions

A PR is mergeable when all of these hold on its **current** head SHA:

1. `game-gate` green and the PR `mergeable_state` is `clean`.
2. For a game-server change: all six PostgreSQL partitions green on that same
   head, or the PR is a member of a batch PR whose six partitions are green on
   the same `main`.
3. Client-only, docs-only, tools/CI-only and test-only fixes of `main` need
   only (1).

Merge with squash and pass the full 40-character head SHA. After the merge,
watch the post-merge `Game Merge Queue gate` run on the new `main`, and check
for duplicate migration numbers:

```sh
git ls-tree --name-only origin/main apps/game-server/migrations/ | xargs -n1 basename | cut -c1-4 | sort | uniq -d
```

Any output is a P0 for the PR that merged second.

## Reading a red check

1. Confirm the run belongs to the current head (`head_sha` of the check run).
   A red result from an older head says nothing about the current one.
2. Open the leaf job log. A PostgreSQL partition prints the failing test near
   `FAIL` / `panicked at` / `TIMEOUT`; the log tail is usually only cluster
   shutdown, so search the whole log, not the last lines.
3. Ask whether `main` has the same failure (latest post-merge run on `main`).
   If yes, it is not this PR's: port the existing fix into the branch (it is a
   no-op once `main` carries it), or wait for the main-fix PR, and say so once
   on the PR. Before writing a new main fix, search the open PRs and recent
   branches for one that already exists (title, changed file, the constant or
   test name); a second PR for the same fix wastes a CI cycle and conflicts
   with the first.
4. Otherwise it is this PR's failure. Common causes, in the order they show
   up in this repo's history:
   - `main` moved: merge `origin/main` and rebuild before reading anything else
     (about 70% of fix-up commits in October 2026 were this).
   - Migration ordering: a new migration that sorts before one already on
     `main`, or reuses a number. Renumber yours above main's highest.
   - Capability or command number taken on `main` in the meantime.
   - Stale content pins or digests (`pin-check`), see `oteryn-content-world`.
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
tools/merge-driver/regen.sh        # regenerate_content.py --resolve
```

- `oteryn-additive` resolves insert-only hunks (two `mod` lines, two registry
  entries at one place). After it, look for a duplicated import or block: the
  driver has produced duplicates before.
- Generated files (world pins, digests, derived registries) are regenerated,
  never hand-merged or hand-edited.
- Never rebase, amend or force-push a shared branch. A merge commit keeps
  every other checkout valid.
- Only a real edit-versus-edit conflict in hand-written logic stops for a
  person.

## Many PRs at once

- Keep about 10-12 PRs in CI at a time. Beyond that runners queue and every
  PR waits longer.
- Several finished mechanics that each touch game-server code: open one batch
  PR that merges their heads onto a fresh `main` (try the merge locally
  first), label only the batch `heavy-tests`, and merge the members one by one
  once the batch is green on that same `main`. Then close the batch.
- After each merge to `main`, check every open PR for a new conflict before
  doing anything else; a PR that conflicts gets no CI.
- Never leave a red or conflicted PR you own without either a pushed fix or
  one comment naming the failing check and what blocks it.

## Never

- Skip, disable, weaken or quarantine a test to get green.
- Push an empty commit or close and reopen a PR to start CI.
- Edit `.github/workflows/**`, `.github/repository-policy.json` or
  `tools/repository/**` without the owner's consent for that change.
- Edit a PR body after review starts; post the tested head as a comment.
- Treat a green check on an older head as evidence for the current head.
