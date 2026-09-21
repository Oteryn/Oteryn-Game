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
Codex usage, repeated reviewer loops or any other owner-funded AI/API operation.
Optional review without separate task-specific authorization is skipped rather than
turned into a new owner prompt.

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
