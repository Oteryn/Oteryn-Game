# Batch review: catch most defects in one round

Read this before you start a Codex review (`SKILL.md` §5) or hand one to a subagent. It
explains how one review round finds most of the defects in a diff at once,
instead of 10 findings, then 5, then 3 over several rounds.

Why rounds shrink: one reviewer with one prompt stops after its first strong
findings, and a line cap cuts the rest. Each fix also adds new code that the
next round reviews. The rules below remove those causes. They do not change
the limits in `SKILL.md` §5: at most 2 reviews per PR, the review never edits code, and
the fix runs at effort `high` for a P0/P1 in a risk class and `medium` for any
other fix (owner decision 2026-11-25).

## 1. One round = one holistic run over the same head

A "review" in `SKILL.md` §5 is one round over one exact head. By default the round is
ONE read-only Codex Cloud run over the whole diff (owner decision 2026-11-25,
after the calibration of the same day). The run reads the whole diff as one
unit, opens the callers and consumers of every changed symbol outside the
diff, and checks the classes of `docs/agents/review/CODEX-CHECKLIST.md`,
including how the parts interact (shared state, registrations, ordering of
handlers and migrations, contracts, fixtures and generated files against
their sources) and each changed player flow end to end (client command,
dispatch, rules, durability, PostgreSQL and back, with two clients at once,
a disconnect midway, a replayed frame, a restart during a write and values at
their limits).

Why holistic: a P0/P1 without a quoted caller is the main source of false
findings (calibration 2026-11-25: at least 9 of the 27 P0/P1 of the astra run
were overstated or false, none of the 6 of the sol run); the evidence and
self-refute rules of §2 target it, and a run that sees the whole diff and its
callers can apply them. One whole-diff round on the integrated head also
found the cross-cutting P0 that per-area loops missed (retro 2026-11-18).

Split only when the diff is too large for one run (above about 1500
hand-written changed lines; generated data, pins and regenerated registries
do not count). Split by area (persistence and migrations, dispatch and
gameplay crates, content data, tools and shared files), not by a fixed line
count. Each chunk prompt names its own files as the focus, carries the whole
diff as read-only context, tells the reviewer to open callers outside the
chunk and includes the severity rubric. The same round also has one
whole-diff integration run by `gpt-6.2-astra` at `high` that skips generated
data and checks the seams: writer keys against migration guards, dispatch
filters, registrations, files shared by the units, migration order
(calibration 2026-11-25: sol alone missed exactly such a seam on a large
diff, astra caught it). Every run of the round starts on ONE frozen head in
one burst (at most 10 tasks); a writer loop on the branch pauses until the
merged packet is ready. Chunk runs alone are not a covered review. The
second review (delta only) is one holistic run over the fix commits and the
code they touch, plus the astra run when the repair is in a risk class.

Model and effort (owner decision 2026-11-25, calibration): every review run
is a Codex Cloud task, read-only, model `gpt-6.2-sol` at reasoning effort
`high`. A PR with code in a risk class (persistence, session fencing,
protocol, authority, durable value, migrations, several consumers) also gets
one `gpt-6.2-astra` run at effort `high` on the same head, with the same
prompt, and so does a content train or a content diff above about 1500
changed lines (calibration 2026-11-25: sol alone lost recall there); the two runs together are the one covered review of that head. An
astra P0/P1 counts only after the review owner traced it (input, caller,
wrong result) to a REAL verdict (§3); one disproved is recorded FALSE and
closed, one that cannot be traced either way goes to the P2 backlog.
Never review at effort `medium`: it lost recall for both models. Name the
task `review-<tag>` (`review-<tag>-astra` for the second run). The review
owner of `SKILL.md` §5 starts it from its own chat session with
`scripts/codex-cloud.mjs` (`oteryn-waves` `references/delegation.md`), never
`codex exec` on the PC and never through the PC as a launch relay. The
native `@codex review` comment route is one run.

## 2. The prompt: all findings, no cap

The review prompt asks for every finding, with no line or count limit.
Use this text in the Codex Cloud review packet or a subagent prompt:

- Report EVERY real defect you find in the diff, P0 to P3. There is no limit
  on the number of findings. Do not stop after the first strong ones.
- For each finding, also list every other place in the diff with the same
  defect class (the pattern sweep).
- Format: `P0|P1|P2|P3 path:line - problem - concrete failure path`.
- Grade with the severity rubric at the top of
  `docs/agents/review/CODEX-CHECKLIST.md`: P1 is a player flow that breaks,
  P2 is content fidelity or a contained defect. A real but non-breaking
  content gap is P2.
- A P0 or P1 states its evidence: `path:line` of the input or state, of the
  caller that reaches the changed code, and of the line that gives the wrong
  result. Without a caller you opened and quoted, it is at most P2.
- Before the output, try to refute each of your P0/P1: look for the guard,
  check, fence or caller that prevents the failure. A finding you refuted is
  dropped; one you cannot settle is reported as `P2 (uncertain)`.
- Mark other uncertain findings with `(uncertain)` instead of dropping them.
- End with `VERDICT: CLEAN` or `VERDICT: N findings`.
- Append `docs/agents/review/CODEX-CHECKLIST.md` and, when the PR fixes a CI
  failure, the failing log lines (test name, panic or assertion, the last 30
  lines of the job). A reviewer that sees the failure finds its cause in the
  diff much faster.

## 3. Merge, verify, sweep

1. Merge the run outputs (the astra run too, on a risk-class PR). Remove
   duplicates by `path:line` and defect class.
2. Give every finding a verdict (`SKILL.md` §6). Verify each P0/P1 against
   the code: trace a real caller or input to the failure. For the ones that
   are hard to call, a second check with the opposite brief helps: a fresh
   Claude subagent (not another paid Codex review) told "try to refute this
   finding; default to refuted when unsure". Keep the finding when the refute
   attempt fails.
3. Pattern sweep: for every confirmed finding, grep the whole tree for the
   same class (same call, same lock, same `_ =>`, same unchecked add, same
   literal that an environment knob overrides), and every test in any crate or
   language that asserts the changed value. Each hit is a candidate,
   verified as in step 2 before it joins the confirmed list; in a delta review
   a hit in unchanged code is confirmed only as a P0, otherwise it goes to the
   backlog. A class fixed at one site and left at another is the main cause of the
   next round: in the retro, rounds that did not converge found one sibling
   per round.
4. P2/P3 go to the grouped list (`SKILL.md` §4). They do not start a fix now.

## 4. One batch fix, one delta review, one CI cycle

1. Write all confirmed P0/P1 findings, sweep instances included, to one file
   and run one fix, at effort `high` for a P0/P1 in a risk class and `medium`
   otherwise, as one Codex Cloud fix packet with the same list, launched
   from the review owner's chat session (`SKILL.md` §5).
   Never one fix run per finding. The packet follows `SKILL.md` §7 (numbered
   items with a status each, sweep list, red test through the real caller,
   class guard, forbidden paths).
2. Review the fix diff yourself (§2-§3) and run the local checks.
3. Push once. One CI cycle covers the whole batch.
4. The second review allowed by `SKILL.md` §5 covers only the fix diff (the delta), as one holistic run (plus astra for a risk-class repair), under the same
   rules.

## 5. When to stop

The diff is done when every run of the last round finished and no
confirmed P0/P1 is open. A failed or empty run means "not reviewed". If
the second review still finds a
P0/P1, `SKILL.md` §5 applies: the design is wrong; stop and report to the owner. Never
start a third round.

## 6. For subagents and child sessions

A packet that delegates review or fixing points to this file and states:

- the head SHA, whether the astra run is due (risk class) and, for big
  diffs, the chunks;
- "report every finding, no cap; sweep each class across the diff";
- the failing CI log lines, when there are any;
- "do not edit code in the review run; fixes go through one fix packet
  after verification".

The merged, verified list with the sweep instances goes to a findings file
(path in the packet), not the raw run logs; the final report (at most 15
lines, `CONTEXT_ROUTING.md`) gives the counts by severity and that path.
