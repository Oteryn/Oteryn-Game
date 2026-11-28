# Owner-funded AI permission boundary

Status: active Game-specific permission extension.

Game resolves review selection and review economy from the policy pinned by
`META_AGENT_POLICY_BINDING.json`. This file controls only permission to consume an
owner's personal, quota-limited or metered AI resources. It is not a review router,
risk classifier or merge gate.

## Default permission

Tool availability, credentials, an alias, a task packet or a recommendation to seek
review does not authorize personal quota or metered spend. An invocation that would
consume those resources requires current owner/user authorization covering that use.
Authorization already established for the same operation in the active session remains
effective; do not ask for it again merely because execution moves between task phases.

## Standing repository review authorization

For `Oteryn/Oteryn-Game`, the owner grants standing authorization to consume
owner-funded, personal-quota or metered AI **only** for one external independent
review of a stable exact-head candidate when the bound META review policy or an
accepted task contract requires that review for qualification.

This standing authorization is current owner/user authorization for that bounded
review invocation and survives chat, worker, coordinator and task-phase handoffs.
Do not ask the owner again for a covered review.

Before invoking a covered review:

- bind the repository, PR and exact candidate head;
- verify deterministic validation required before review is complete when the
  governing task/policy requires that ordering;
- read live PR review state and comments and do not issue another trigger when a
  review for the same exact head is already requested, running or completed;
- use the lightest reviewer/depth selected by the bound review policy;
- count one native `@codex review` request or equivalent provider invocation as
  the single covered invocation for that exact head.

A materially risk-bearing head change may qualify for one new review only when
the bound policy requires re-review. Cosmetic, metadata-only or unchanged-risk
movement does not authorize another paid review. A failed/ambiguous trigger must
be reconciled from live provider/PR readback before any retry; never send duplicate
review requests merely because a response is delayed.

This standing authorization does **not** cover optional/speculative extra reviews,
implementation, code generation or repair, tracked-file mutation, commits, push,
merge/enqueue, production/live-data actions, cross-repository writes, general
Codex usage, repeated reviewer loops beyond the per-PR rule and the owner review loop below or any other owner-funded AI/API operation.
Optional review without separate task-specific authorization is skipped rather than
turned into a new owner prompt.

### Owner review rule for every PR (2026-11-09)

The owner authorizes one manual Codex review (Codex Cloud, read-only, one
holistic whole-diff run, model `gpt-6.2-sol`, reasoning effort `high`, plus,
the `gpt-6.2-astra` runs at effort `high` that the run-selection table of
`.claude/skills/oteryn-review/references/batch-review.md` §1 names for the PR
on the same head, never at effort `medium` and never `codex exec` on the PC,
per owner rule 2026-11-17 and the calibration decision of 2026-11-25; or, from
a session without Codex Cloud, one native `@codex review` PR comment naming
the exact head) of every Game PR before it merges. The sol and astra runs of
one head are together that one review. An astra P0/P1 is fixed only after the
review owner traced it to a REAL verdict (input, caller, wrong result);
an astra finding the review owner disproves is recorded FALSE and closed, and
one it cannot trace either way goes to the P2/P3 backlog. Every session launches, polls and
reads its Codex Cloud tasks from its own chat session; the owner's PC is never
a launch relay (owner rule 2026-11-25). This
replaces the 2026-10-29 trial and its "at most 5 a day" limit. These reviews
are owner-authorized, not optional extras:

- the first review runs once, after the PR's first push, over the whole diff
  against `main`, in parallel with CI;
- a second review is authorized only when a substantial logic change (server
  code, persistence, session fencing, protocol, durable value) lands after the
  first one, and covers only the new diff. At most 2 reviews per PR; a third is
  never started: the coordinator reports to the owner instead;
- P0/P1 findings are fixed at once; P2/P3 findings are collected and fixed
  together in one grouped PR at the end.

The binding, live readback, de-duplication and single review-dispatch owner rules
of this file apply unchanged: one invocation per exact head, never a loop. A head
that only answers this review's findings is reviewed again only when it meets the
second-review rule above. This authorization covers the review alone, not
implementation, repair or any other Codex use.

Owner decision 2026-11-11 (fix of confirmed findings): the review run itself stays
read-only and never edits code. Findings the coordinator has verified against the
code may then be handed, as one list, to a single code-writing Codex run at
reasoning effort `high` for a P0/P1 in a risk class (persistence, session fencing,
protocol, authority, durable value, migrations, several consumers) and `medium` for any other
fix, model `gpt-6.2-sol` (owner rules 2026-11-18 and 2026-11-25), launched as a
Codex Cloud task from the review owner's chat session. That run writes a red test per finding before any fix,
reports a finding it cannot reproduce without changing code, changes no existing
assertion, timeout or budget and nothing outside the list, and commits nothing;
the coordinator reviews its diff before committing. This is ordinary code-writing
Codex use under the 2026-11-11 effort decision, not a further review.

Owner decision 2026-11-18, revised 2026-11-25 (one round per head): one review
of one exact head is one holistic whole-diff run under
`.claude/skills/oteryn-review/references/batch-review.md`, with its chunk runs
for a diff above about 1500 changed lines and the `gpt-6.2-astra` runs of the
same head that its run-selection table names (one dispatch by the single
review-dispatch owner). Those tasks together are the one covered invocation
for that head; they never add a review to the per-PR count.

### Owner review loop (2026-11-18)

The owner authorizes, when the owner asks for it by name for an area or product,
a review loop over merged code on `main`, run as rounds of read-only Codex Cloud
tasks (model `gpt-6.2-sol`, effort `high`) under
`.claude/skills/oteryn-review/references/review-loop.md`. This is the one
exception to "never a loop" above and is limited to it:

- only the active coordinator starts a round, and only for a loop the owner
  asked for; a round is never counted as, or used in place of, the review of an
  open PR, and each fix PR the loop produces gets its own per-PR review;
- the loop's charter (scope, rules, cuts, stop bar) is written before round 1;
- the loop stops at its stop bar, or after 3 rounds in which the number of new
  confirmed P0/P1 has not fallen, and the coordinator reports to the owner;
- it covers the review runs alone, not repair (repair runs under the
  2026-11-11 decision above);
- it also covers verification runs of the loop's own findings (owner,
  2026-11-18: "zgoda na dodatkowe review jest"): read-only Codex Cloud tasks,
  same model and effort, that try to refute or confirm the round's P0/P1
  findings, grouped by area (never one run per finding). Per-PR reviews keep
  the Claude-subagent refute of `batch-review.md` §3.

The owner may revoke or narrow this standing authorization at any time by a later
explicit instruction. A later protected policy may narrow execution further but
cannot broaden this permission.

### Single review-dispatch owner

Standing funding permission is not permission for every worker to emit a provider
trigger. A manual `@codex review` or equivalent owner-funded review invocation is a
GitHub control-plane write and must obey one-writer routing.

- For work governed by an active programme control plane, only the unique active
  control-plane coordinator may emit the owner-funded review trigger.
- A worker/reviewer may determine that review is required and return the exact
  repository/PR/head review packet, but it must not emit the trigger itself.
- For a standalone task with no programme control plane, only the exact task owner
  named by the live allocation may trigger review.
- Owner decision 2026-11-11 (lanes that work alone): a Claude session, child
  session or PC lane that owns a branch or PR, named as its owner in its packet
  or allocation, is the exact task owner for the heads it pushes. It runs that
  head's Codex Cloud review itself under the per-PR rule above and
  reports the result (head SHA, findings, fixes) to its coordinator, which does
  not run a second review of the same head. A Codex lane never reviews code; its
  result is reviewed by the coordinator or the lane that takes it over.
- If trigger ownership is ambiguous, fail closed and reconcile ownership; do not
  ask the owner for funding permission and do not race another agent.
- Provider-native automatic review already running for the same exact head counts
  as the covered invocation and suppresses any manual trigger.

Immediately before the single trigger, the trigger owner must refresh live PR
comments/reviews/provider status for the exact head. Provider-side de-duplication is
a safety backstop, not authority to send concurrent duplicate comments.

Repository-native or platform-provided review must still be both available and
authorized on the actual execution surface. Outside the standing authorization above,
the bound META policy selecting an external review as useful does not create funding,
repository, candidate, mutation, production, secret or merge authority.

## Execution boundary

Use the lightest authorized review mechanism selected by the bound policy. A reviewer
does not receive implementation, tracked-file, commit, push, merge, protection,
production, live-data, credential or cross-repository authority from this document.
Required repository checks and genuinely required independent review are never waived
because an external AI mechanism is unavailable.

If no authorized mechanism can satisfy a material required review, record the exact
missing capability or permission and continue other safe work. Do not invent a review,
use the owner as a message relay when an authorized repository-native route exists, or
treat green CI as independent review.
