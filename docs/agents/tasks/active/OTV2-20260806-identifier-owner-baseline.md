# OTV2-20260806-identifier-owner-baseline

```yaml
task_id: OTV2-20260806-identifier-owner-baseline
title: Record owner-accepted identifier and social-presence baselines
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-id-01-owner-baseline
pr: 56
base_sha: 26b5fa275fba19fdee0e26a6f65263489af3e500
head_sha: pending-final-validation
owner: ChatGPT architecture coordinator
created_at: 2026-08-06T14:19:00+02:00
updated_at: 2026-08-06T16:38:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md
  - docs/agents/tasks/active/OTV2-20260806-identifier-owner-baseline.md
public_contracts:
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md
depends_on:
  - ADR-0001 through ADR-0011
  - FND-01 and VSL-02 destination cutover
blocks: []
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/otclient
```

## Outcome

Persist the product owner's accepted identifier, channel/instance, world-scoped party, privacy-first presence and consent-based contact baselines as canonical architecture input without falsely claiming that the complete `FND-ID-01` or later social contracts have started or finished.

## Architecture and source of truth

- `PROVEN` — `main` at task start is `26b5fa275fba19fdee0e26a6f65263489af3e500`.
- `PROVEN` — the canonical destination workspace and client cutover are complete.
- `PROVEN` — the global register still requires the source-only `blakinio/otclient` historical marker before the full `FND-ID-01` package begins.
- `PROVEN` — the owner accepted the four-class identifier baseline on 2026-08-06.
- `PROVEN` — the owner accepted that `WorldId` is globally unique and that channels are assigned to their world, making `WorldId + ChannelId` the canonical semantic channel identity.
- `PROVEN` — the owner clarified that channels, rather than instances, remain the selected primary topology for the logical world.
- `PROVEN` — instances remain a useful optional gameplay mechanism that may be available to players on every channel.
- `PROVEN` — the owner accepted that parties are world-scoped and may contain members currently placed on different channels.
- `PROVEN` — open-world shared gameplay still requires one common channel, while future instanced gameplay requires one common instance.
- `PROVEN` — the owner accepted privacy-by-default: exact channel and instance placement are not public.
- `PROVEN` — exact channel visibility is limited by default to current party members and mutually accepted contacts/VIP entries, subject to later user controls.
- `PROVEN` — adding a contact/VIP requires an invitation and explicit acceptance; unilateral VIP tracking is not target behavior.
- `PROVEN` — party invitations remain separate consent operations that may be accepted or declined.
- `PROVEN` — the default accepted contact relationship is between two specific characters.
- `PROVEN` — accepting one character as a contact does not reveal, add or authorize tracking of alternate characters from the same account.
- `PROVEN` — an account-wide relationship is a separate optional relationship requiring its own explicit mutual consent.
- `PROVEN` — alternate characters remain hidden by default even after account-wide friendship is accepted.
- `PROVEN` — alternate-character visibility may be widened only by an owner-controlled privacy setting or explicit sharing action and may be revoked independently.
- `DERIVED` — exact sharing-control granularity, cross-world scope, persistence, protocol and UI behavior remain unresolved.
- `DERIVED` — exact concrete-instance identity, placement, cross-channel membership and migration semantics remain unresolved and must not be inferred from feature availability.

## Acceptance criteria

- [x] Record the four identifier classes and cross-cutting invariants.
- [x] Record `WorldId` as globally unique durable identity.
- [x] Record `ChannelId` as semantically scoped by `WorldId`, regardless of technical global uniqueness.
- [x] Require channel-boundary validation to preserve the world binding.
- [x] Record channels as the primary world topology.
- [x] Record instances as optional isolated gameplay contexts available across the channel topology.
- [x] Keep concrete `InstanceId` scope and channel relationship unresolved.
- [x] Record `PartyId` as semantically scoped by `WorldId`.
- [x] Permit party membership, leadership, invitations, roles, readiness and chat across channels of one world.
- [x] Keep shared open-world simulation channel-local and instanced simulation instance-local.
- [x] Reject cross-channel combat, loot, experience, healing and visibility through party membership alone.
- [x] Keep Party Finder algorithms, channel reservation, transfer and activity rules unresolved.
- [x] Record exact channel, instance, node and map placement as non-public presence data.
- [x] Limit default exact-channel visibility to current party members and mutually accepted contacts/VIP entries.
- [x] Require explicit invitation acceptance before creating an accepted contact/VIP relationship.
- [x] Keep contact and party invitation lifecycles separate.
- [x] Record character-to-character as the default contact scope.
- [x] Reject automatic alternate-character discovery or contact promotion from one accepted character relationship.
- [x] Permit account-wide friendship only as a separate optional relationship with distinct mutual consent.
- [x] Require strong separation between character-contact and account-contact invitation types.
- [x] Keep alternate characters hidden by default even for accepted account-wide contacts.
- [x] Require an explicit owner-controlled setting or sharing action before alternate-character disclosure.
- [x] Permit selective sharing and independent revocation without requiring account-contact removal.
- [x] Require privacy-safe cache invalidation and server-side enforcement of hidden alternate characters.
- [x] Require privacy-safe failure, stale-data handling, rate limits, blocking and abuse controls in later contracts.
- [x] Keep exact sharing controls, cross-world behavior, protocol, persistence and UI details unresolved.
- [x] Keep UUID/ULID/database-column/wire-width and the remaining catalogue unresolved.
- [x] Preserve the historical-marker ordering gate.
- [x] Make no runtime, protocol, schema, migration or external-repository change.
- [x] Open documentation-only draft PR #56.
- [ ] Obtain exact-head repository validation and independent audit before merge.

## Excluded scope

- no `protocol-oteryn` schema or codec;
- no Rust identifier types;
- no PostgreSQL representation;
- no Game Session or lease token format;
- no final `InstanceId` scope or persistence decision;
- no instance runtime, matchmaking, transfer or lifecycle implementation;
- no Party Finder implementation;
- no party size, role, invite, matchmaking, teleport, channel reservation, shared-experience or loot contract;
- no final social-presence service ownership or synchronization protocol;
- no final account-wide contact permission model, cross-world scope or alternate-character sharing-control design;
- no client VIP/social-panel implementation;
- no contact/party invitation persistence or wire schema;
- no legacy VIP migration;
- no write to `blakinio/otclient`;
- no claim that the complete `FND-ID-01` or social-system contract is accepted or complete.

## Implementation / findings

Accepted identifier and topology baseline:

1. durable cross-boundary identities are stable, immutable, non-reused and semantically opaque;
2. scoped identities are meaningful only with their owning scope;
3. runtime-local references use generation-fenced handles and never escape as durable/public identity;
4. revisions, generations and sequences are ordering/fencing values, not entity identities;
5. names, slugs and display numbers are labels or lookup aliases, not canonical identity;
6. `WorldId` globally identifies one logical world;
7. every channel is assigned to one logical world and its canonical semantic identity is `WorldId + ChannelId`;
8. a globally unique technical `ChannelId` representation does not permit dropping the `WorldId` binding;
9. channels remain the primary mechanism for exposing and distributing one logical world;
10. instances do not replace channels and may be provided as optional isolated gameplay contexts for players on any channel;
11. feature availability across channels does not yet prove that one concrete instance is channel-bound, cross-channel shared or portable;
12. the canonical semantic party identity is `WorldId + PartyId`;
13. one party may temporarily contain members placed on different channels of the same world;
14. party organization may remain active across channels, but it does not merge their simulations;
15. open-world shared combat and related mechanics require one common channel;
16. instanced shared gameplay requires one common concrete instance under later contracts;
17. a remote party member receives no cross-channel combat, shared experience, loot, healing, local visibility or proximity effects.

Accepted social presence and consent baseline:

18. exact `ChannelId`, `InstanceId`, node and map placement are not public information;
19. a normal non-contact may see at most coarse presence such as `online on this world`, subject to privacy settings;
20. exact channel is visible by default only to current party members and mutually accepted contacts/VIP entries;
21. the observed player may reduce visibility further, while broader exact-placement disclosure requires explicit opt-in under a later contract;
22. adding a contact/VIP requires a request and authoritative explicit acceptance;
23. contact requests may be declined, ignored, cancelled or expire without creating a relationship;
24. either side may later remove the relationship, and blocking must prevent repeated invitation abuse;
25. contact and party invitations remain separate consent lifecycles;
26. the client must not receive unauthorized exact placement and cannot be trusted as the privacy-enforcement boundary;
27. stale presence must fail toward less disclosure;
28. unilateral legacy VIP tracking with exact presence is rejected;
29. the default contact relationship is between one explicit requester character and one explicit target character;
30. accepting one character does not reveal, discover or add alternate characters from the same account;
31. character contact consent cannot be silently promoted to account-wide consent;
32. account-wide friendship is a separate optional relationship requiring a distinct invitation and explicit mutual acceptance;
33. character-contact and account-contact identities and lifecycle transitions must be strongly separated;
34. account-wide acceptance does not authorize alternate-character disclosure by default;
35. the default alternate-character visibility policy is hidden;
36. widening alternate-character visibility requires an explicit owner-controlled setting or sharing action;
37. selected characters may remain hidden even when broader sharing is enabled;
38. sharing may be revoked independently of the underlying account-wide relationship;
39. hidden alternate characters and ownership linkage must not be discoverable through social-system side channels;
40. newly created characters remain hidden unless the active explicit policy includes them.

Draft PR #56 contains only canonical architecture documents and this task record.

## Validation

### Focused

- command/run: exact branch comparison against task-start `main`
- result: pending final-head refresh; expected scope is three Markdown files only

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only documentation
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable product behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- head: pending after final checkpoint update
- workflow/run: pending
- result: pending

## Independent audit

- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending final-head refresh
- unresolved review threads: pending
- related/superseded PRs: none identified
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner accepted that alternate characters stay hidden by default even for account-wide contacts and become visible only through a deliberate owner-controlled privacy setting or sharing action; canonical social-presence baseline and PR #56 task record were updated.
status: validating
branch: docs/fnd-id-01-owner-baseline
head_sha: pending-final-validation
pr: 56
ci_check_generation: pending after final checkpoint update
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: Full FND-ID-01 remains ordered after the source-only blakinio/otclient historical marker; PR #56 also requires exact-head validation and independent audit before merge.
next_action: Verify the final PR head, changed-file scope and repository validation state.
```
