---
name: oteryn-verify
description: Evidence before any claim in Oteryn-Game - before saying done, fixed, passing, green or mergeable, before a report, a commit message, a PR description or a status line, run the command that proves it and read its output. Load whenever you are about to report a result or state of work.
---

# Evidence before claims

A claim without fresh output from the command that proves it is a guess.
Workers' reports, CI from an older head and "should pass" are not evidence.

## The gate

Before stating any result:

1. Name the command that proves it.
2. Run it in full, now, on the exact tree or head the claim is about.
3. Read the whole output: exit code, failure count, the test names.
4. State the result with the evidence (command, head SHA, counts); if the
   output disagrees with the claim, report what it shows instead.

| claim | needs | not enough |
|---|---|---|
| tests pass | the test command's output with 0 failures, on this tree | an earlier run, a run on another head |
| the bug is fixed | the loop that was red on the bug, now green | code changed, "looks right" |
| a regression test works | seen red without the fix and green with it | it passes once |
| CI is green | check runs on the current head SHA | a green run on an older head |
| mergeable | `game-gate` green and `mergeable_state` clean on the current head, plus the PostgreSQL rule (`oteryn-dev` `references/ci-merge.md`, *Merge conditions*) | the PR looked fine an hour ago |
| a worker finished | the diff in the tree and your own run of its checks | the worker's report |
| a mechanic is DONE | the mechanics ledger's definition, element by element | one slice merged |

## Words

- "slice done", never "done", for anything PARTIAL.
- Numbers only when counted, with what was counted.
- A check you could not run is reported as not run, with the reason; it is
  never folded into "passes".

## Sources

Adapted, not copied verbatim: obra/superpowers
`verification-before-completion` (MIT); steipete/agent-scripts `codex-first`
coordination notes (MIT: the worker's report is not evidence).
