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

Repository-native or platform-provided review must still be both available and
authorized on the actual execution surface. The bound META policy may select an
external review as useful, but that selection does not create funding, repository,
candidate, mutation, production, secret or merge authority.

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
