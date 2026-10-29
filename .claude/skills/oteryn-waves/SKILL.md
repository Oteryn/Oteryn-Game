---
name: oteryn-waves
description: Planning and running Oteryn-Game mechanics as waves - choosing and slicing mechanics, checking that parallel units share no files or numbers, pre-assigning numbers, writing delegation packets for Codex or subagents, landing a batch, and closing a wave with the ledger update and a retro. Load when planning more than one change, delegating, or landing a batch.
---

# Planning and landing waves

A wave is 3-4 mechanics built in parallel and landed together. Parallel work
is fast only while the units truly share nothing; most lost time in this
repository came from shared files, reused numbers, unreviewed returns and
pushes that CI had to judge.

## 1. Choose and slice

- Take the work from the mechanics ledger (the file
  `docs/agents/tasks/MECHANICS_STATUS.md` points to). Order, by owner
  decision: logic that exists with only test callers first (wire a real
  caller and a test), then PARTIAL rows missing one piece, then MISSING rows.
- One mechanic = one branch = one PR; the `a`/`b` split for a mechanic too
  big for one PR is in `oteryn-dev` `references/mechanic.md`.
- Each slice is a tracer bullet: a thin path through every layer it touches
  (rules, storage, caller, wire, test), not one layer done completely. Write
  the blocking edges between slices explicitly.

## 2. Check that the wave can run in parallel

For each unit list the files it will touch, then look past the list:

- **hot files** no two units may share: routing, dispatch, registries,
  `connection.rs`, golden files, the capability capacity constant;
- **hidden shared surfaces:** migrations, `Cargo.lock`, generated content and
  pins, registry fragments with neighbouring numbers, and one runtime resource
  (a shared PostgreSQL database: each lane gets its own);
- **contracts:** two units that change one type, trait or protocol message
  contend even in different files.

Units that contend move to the next wave; doubt about one unit never holds
back the rest.

## 3. Pre-assign numbers centrally

Run `oteryn-dev` `scripts/next-free-numbers.py` and give each unit its
migration, capability, command and offer-order numbers in its packet. No
worker picks its own number.

## 4. Write the packets and dispatch

One self-contained packet per unit: `references/delegation.md` has the format
(base SHA the worker verifies, numbers, owned paths, the red-then-green success
check, limits to paste) and the rules for taking results back. Dispatch the
whole wave at once, and only as many units as you can review and land the
same day.

## 5. Land the wave through one batch

1. Review each returned unit (`oteryn-review`) and push it to its own branch.
2. Merge the wave's finished game-server heads, plus any other finished ones
   waiting, onto a fresh `main` as one batch branch (4-6 heads, at most 6);
   it is only a test vehicle.
3. Run what the PR gate never runs on the batch tree, on a PC lane when one is
   available: Node boot and the integration shards (`oteryn-dev`
   `references/local-checks.md`, *Jobs that only run after the merge*). Label
   the batch `heavy-tests` once.
4. Batch red: bisect it by member and fix the member, not the batch.
5. Batch green: check `main` health (`oteryn-dev` `scripts/main-health.sh`),
   then merge the members one by one, back to back, with no other game-server
   merge in between (`oteryn-dev` `references/ci-merge.md`, *Merge
   conditions*); then close the batch.

## 6. Close the wave

- One docs PR updates the wave's rows in the mechanics ledger with donor and
  Oteryn file:line evidence; mechanic PRs never edit it.
- Retro, once per wave, at most three lessons. Candidates are the things that
  cost a re-push, a re-run, a review finding or a fix in another PR; keep one
  only if someone reading just the final code would repeat it, and cite its
  evidence (PR or run id). Each kept lesson ends in exactly one of:
  - a proposed automated check, when a script can detect the pattern;
  - one line in the skill that owns that kind of work (`oteryn-coding`,
    `oteryn-debugging`, `oteryn-review`, `oteryn-dev`, this skill);
  - dropped, with the reason.
  Skills change only through a reviewed PR; an agent proposes the line, it
  never edits a skill on its own initiative mid-task.
- Prune at the same time: delete any skill line that, since the last pruning,
  changed nothing an agent did or contradicts a newer rule. A skill that only
  grows is decaying.
- Every second wave, compare the counts against the 2026-10-29 baseline:
  share of red post-merge runs on `main` (about 88%), PRs with follow-up
  commits (161 of 300), duplicated fix commits (68 in 300 PRs), cancelled
  gate runs (83 of 500), PRs with any review (none since 2026-10-21). A
  lesson that moves none of them after two waves is a candidate for pruning.

## Sources

Adapted, not copied verbatim: EveryInc compound-engineering `ce-work`
execution strategy (MIT: parallel safety check, hidden shared surfaces, base
SHA check, bounded batches, abort on unowned writes) and `ce-compound` (MIT:
the lesson test); mattpocock/skills `to-tickets` (MIT: tracer-bullet slices,
blocking edges), `writing-for-agents` (MIT: decided packets, positive
instructions) and `retro` (MIT: mechanical mistakes become checks);
steipete/agent-scripts `codex-first` (MIT: serialized landing, verify the
merged result); the owner's wave decisions of 2026-10-29.
