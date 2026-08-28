# Cross-repository contract policy

## Repositories

Oteryn Game currently interacts with:

- `Oteryn/Oteryn-Platform` — Identity, OAuth/PKCE, Game Login Ticket, Game Gateway, World Registry and Game Session producer;
- `Oteryn/Oteryn-Game` — canonical native Rust game server/client and `protocol-oteryn` consumer/producer;
- `blakinio/Oteryn-v2` — preserved read-only legacy/migration source for historical provenance;
- `blakinio/Otheryn` — read-only behavioral/content migration source;
- `blakinio/otclient` — read-only implementation/migration source until an explicit transfer programme changes ownership.

## Rules

- One repository's task or PR cannot claim authority to mutate another repository.
- Every cross-repository programme uses a shared coordination ID and one local task/branch/PR per authorized repository.
- Identify the canonical contract source, producers, consumers, exact revisions and rollout/rollback order.
- Classify each step as `server-first-safe`, `client-first-safe`, `backward-compatible`, `atomic-required`, `breaking-migration` or `unverified`.
- Do not merge an `atomic-required` side while its paired side is not ready.
- Do not infer implementation from documentation alone; record current versus target state.
- Golden fixtures/schema IDL have one canonical owner and immutable version identifiers.
- `docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json` is the machine-readable index for canonical cross-repository revisions, producers, consumers and rollout order.
- Canonical lock entries may contain only merged commits and immutable schema identifiers. Open PRs and mutable heads remain explicit pending evidence with canonical fields unset.

## Native protocol direction

The accepted target is `protocol-oteryn` only between the Rust client and Rust game server. Historical gameplay profiles are rulesets/content capabilities, not Canary/Tibia wire adapters.

Any proposal to retain or reintroduce `protocol-canary` in the target runtime requires an explicit owner-approved ADR superseding the current architecture. Compatibility code in reference repositories is not automatically migrated.

## Platform session boundary

Preserve the existing authoritative entry direction unless superseded by a cross-repository contract:

```text
Rust client
→ Oteryn Identity OAuth Authorization Code + PKCE
→ one-time Game Login Ticket
→ Oteryn Game Gateway
→ authoritative ticket redemption and World Registry routing
→ Game Session bound to account, character, world, channel, native protocol version, protocol revision, ruleset revision and content revision
→ Rust game server admission
```

Do not create a second Identity authority, password login path, ticket system or direct OAuth-token authentication in the game server.

## Required contract fields

A material cross-repo contract must define:

- stable identities and ownership;
- API/game-session/protocol versions as separate concepts;
- authentication, audience, expiry, replay and revocation behavior;
- world/channel binding and revision fences;
- schemas, limits and failure vocabulary;
- capability/version compatibility matrix;
- observability/redaction requirements;
- rollout, rollback and mixed-version behavior;
- deterministic contract tests and fixture ownership;
- the applicable resource-limit registry entries, stable error categories and named failure scenarios.

## Migration sources

Otheryn and otclient evidence must be pinned to exact commits. Reuse of code/data/assets additionally requires license/provenance review. Behavioral equivalence is accepted through project-owned fixtures and explicit product decisions, not by silently preserving every legacy bug.
