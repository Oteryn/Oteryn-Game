# OTV2-20261001-charm-native-durability-port

```yaml
task_id: OTV2-20261001-charm-native-durability-port
title: Native coherent Charm progression reader
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/charm-native-progression-port-20261001
branch: codex/charm-native-durability-port-20261001
pr: 1501
issue: 162
base_sha: 3464e4606337e458d24c69893fefd801644b3829
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; transport source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/gameplay_transport/charm.rs
  - apps/game-server/src/gameplay_transport/charm_native.rs
  - apps/game-server/src/gameplay_transport/charm_native_tests.rs
  - apps/game-server/tests/charm_state_postgres.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/charm_state_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-native-durability-port.md
public_contracts: []
depends_on: [CHARM-2, CHARM-3, CHARM-5, 1490]
blocks: []
external_repositories: []
```

## Result and authority qualification

The actual DurabilityRoot snapshot supplies paired Bestiary and Charm views at one revision.
Validated catalogue and race indices, stages, costs, counters, signed balances and slot limits
project through existing protocol encoders. Malformed/obsolete assignments and balances reject;
historical unrequested counters are preserved in storage and omitted from current views.

AuthorityInvariant x ConsumerBoundary x MutationOperator applies at native bind and every view:
independently supplied current session, connection, lease, scope/generation and node evidence
remain fixed expected bindings. Only expected CharacterRevision is refreshed from storage; the
underlying locked snapshot independently rechecks live authority. An old receipt never supplies
current authority. Content revision compares expected binding only; no qualified generation
provenance, capability activation or live effects are claimed. All advertised effect flags are false.

Root inspected the entire delta. The module and library projection tests total489 lines; shared
actual PostgreSQL negatives/linkage bring the coherent read-port batch to603 changed Rust lines.
The slight budget excess retains essential real-consumer authority evidence instead of deleting
negatives or splitting a projection-only result from its actual consumer. No resource, schema,
protocol identifiers or production owner was added.

## Validation and remaining work

Actual PostgreSQL17.6 suite791PASS includes the shared native bind/views case at revision8,
wrong session and stale revision isolated negatives, restart of durable state, and rejection by
the already-bound reader after independently changing live connection generation. The same
case is loaded by both focused Charm and protected Character-authority wrappers. Four projection
unit tests PASS; workspace fmt, diff check, strict workspace all-target Clippy and governance/
lifecycle13PASS. Four narrow test-only dead-code allowances cover native reader items actually
executed in standalone PostgreSQL suites, not in library-only projection tests. No E2E claim.

Write-port unlock/assign dispatch, retained command retries, qualified Content loading, real
commercial/promotion inputs, paid unassign and CHARM-6 activation remain unfinished. Full parent
nine-row task remains IMPLEMENTING. Root publishes actual Git identity with guarded normal
pushes; exact freeze, independent review and CI belong to the external packet. Governance CI
accepts base=main only; CP owns stacked retarget/requalification after PR1490 integrates.
This record reaches main only if PR1501 merges; a commit cannot contain its own final SHA.
