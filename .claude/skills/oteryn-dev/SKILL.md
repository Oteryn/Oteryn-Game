---
name: oteryn-dev
description: How work lands in Oteryn-Game - from claiming migration/capability/command numbers and building a Tibia mechanic, through local cargo/clippy/PostgreSQL/pin checks, to CI, merge conflicts, batch PRs and merging, plus delegating slices to Codex or subagents. Use for any implementation, fix, review, push, CI or merge work in this repository, including a red game-gate, a pin-check or digest failure, a migration-number clash, a conflicted PR, or writing a packet for another agent, even when the request names none of these.
---

# Landing work in Oteryn-Game

This skill records current repository practice. It grants no authority:
merging, labelling, re-running and pushing stay with whoever the owner
authorised (root `AGENTS.md`, *Repository boundaries*). Machine contracts win
over this text (`docs/agents/BUILD_TEST_MATRIX.md`, *Machine authority*); if
they disagree, fix this skill in the same change.

## The path of one change

| Step | What | Read |
|---|---|---|
| 1 | Task card, nearest `AGENTS.md`, reference rules | `references/content-world.md` (*Reference sources*) |
| 2 | Claim migration / capability / command / offer numbers against `main` **and open PRs** | `references/mechanic.md` |
| 3 | Implement; new capability file set, migrations, tests in mounted dirs | `references/mechanic.md` |
| 4 | Content, imports or pins touched: regenerate, pin-check | `references/content-world.md` |
| 5 | Merge `origin/main`, then local checks for the changed paths | `references/local-checks.md` |
| 6 | PR from the template with a parity table; DONE or a named PARTIAL | `references/mechanic.md` (*The PR*) |
| 7 | CI, red checks, conflicts, `heavy-tests`, batch PRs, merge | `references/ci-merge.md` |
| - | Handing a slice to Codex or a subagent, or taking a diff back | `references/delegation.md` |

Read only the reference for the step you are on. Step 5 is never skipped:
a push without it usually costs a full CI cycle and a second push.

## Rules that hold at every step

These are stated once here; the references do not repeat them.

- A mechanic is DONE only as the mechanics ledger defines it (the file
  `docs/agents/tasks/MECHANICS_STATUS.md` points to); rules, storage, caller,
  wire command, client registration and tests are the checklist. Anything
  less is "slice done" / PARTIAL. Count progress from the ledger's totals and
  update its row in the PR that changes it.
- One mechanic = one branch = one PR.
- Never skip, `#[ignore]`, weaken, quarantine or delete a test to get green.
- Never hand-edit a generated pin, digest or derived registry; regenerate it.
- Never edit a merged migration; fix forward with a new one.
- Never rebase, amend or force-push a shared branch, push an empty commit, or
  close and reopen a PR to start CI. Re-run a job at most once, and only for
  an infrastructure death or a known lock-timeout signature
  (`references/ci-merge.md`).
- A green check on an older head says nothing about the current head.
- Edit `.github/workflows/**`, `.github/repository-policy.json` or
  `tools/repository/**` only with the owner's consent for that change.
- Never read or print credentials, tokens, `.env` files or a database URL
  (`set +x` before exporting one in a script).
- Never connect to or probe CipSoft game or login servers, and never bypass
  a site's protections (captcha, Cloudflare, login, spoofed user agent).
- No new third-party dependency without the custody review its `Cargo.toml`
  change triggers.
- Large data JSON under `content/`, `imports/`, `docs/agents/evidence/` is
  often one multi-megabyte line: locate with `rg -l`, extract with `jq` or
  Python, never print it.
- Evidence and scratch output stay out of git when they are larger than a
  few MB or are archives (`*.tar.gz`): `docs/agents/` already holds about
  540 MB, and every clone and cloud session pays for it.
- Another agent's report is a claim; re-run its validation before it reaches
  a branch.

## Scripts

- `scripts/next-free-numbers.py [--ref origin/main]` - next free migration,
  capability, command, mechanic ordinal and offer order across `main` and
  open PRs; run it when claiming numbers and again before pushing.
- `scripts/main-health.sh [runs]` - last post-merge runs on `main` with the
  failing leaf jobs; run it before any merge round.
- `scripts/dup-migrations.sh [ref]` - prints any migration number used twice
  on `ref` (default `origin/main`); any output is a P0.
