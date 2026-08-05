# End-to-end feature completeness

A feature is complete only when its required producer, authoritative domain behavior, transport/API, consumer/UI, persistence and failure behavior form one observable supported workflow.

## Required feature contract

Record:

- user outcome and supported profiles/capabilities;
- authoritative owner for every state transition;
- inputs, outputs, limits and errors;
- persistence and recovery semantics;
- client/server/platform revision matrix;
- positive, negative and abuse scenarios;
- telemetry/privacy behavior;
- rollout and rollback.

## Oteryn v2 mandatory concerns

- world/channel/instance scope;
- one-character lease and stale-writer fencing;
- server authority over legality, inventory, loot, damage and resources;
- sequencing, duplicate and lost-baseline behavior;
- channel failure/isolation and shared-world service behavior;
- anti-channel-hopping rewards/PvP/trade rules;
- ruleset capability exposure without protocol forks.

## Completeness states

- `PROVEN` — required exact scenario passed on named revisions.
- `PARTIAL` — bounded implementation exists but required layer/evidence is missing.
- `SYNTHETIC_ONLY` — only fake/fixture path is proven.
- `UNKNOWN` — evidence absent/stale.
- `BLOCKED` — named dependency prevents completion.
- `DEFERRED` — owner-approved exclusion.
- `ABSENT` — no owning implementation contract.

A green build alone is not end-to-end proof.
