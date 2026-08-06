# OTV2-20260806-identifier-owner-baseline

```yaml
task_id: OTV2-20260806-identifier-owner-baseline
title: Record owner-accepted identifier, instance and social-presence baselines
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-id-01-owner-baseline
pr: 56
base_sha: 26b5fa275fba19fdee0e26a6f65263489af3e500
head_sha_before_checkpoint: 55157cc155e84e909e69026606b903a4dfd5154a
owner: ChatGPT architecture coordinator
created_at: 2026-08-06T14:19:00+02:00
updated_at: 2026-08-06T17:29:00+02:00
owned_paths:
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md
  - docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md
  - docs/agents/tasks/active/OTV2-20260806-identifier-owner-baseline.md
public_contracts:
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md
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

Persist the product owner's accepted identifier, channel, world-scoped party, instance runtime, map-template, activity-admission, seamless-handoff and privacy-first social baselines as canonical architecture input without claiming that the complete `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`, Party Finder or instance lifecycle contracts are complete.

## Proven owner decisions

### Identifier and topology

- Durable cross-boundary identities are stable, immutable, non-reused and semantically opaque.
- `WorldId` globally identifies one logical world.
- The canonical channel identity is `WorldId + ChannelId`.
- Channels remain the primary world topology.
- The canonical party identity is `WorldId + PartyId`; a party may organize members across channels of the same world.
- Open-world shared gameplay requires one common channel.

### Instance identity and ownership

- The canonical semantic instance identity is `WorldId + InstanceId`.
- A concrete instance is not semantically owned by its participants' source channel.
- Eligible players may enter one concrete instance from different channels of the same world.
- Cross-world instances are forbidden.
- After commit, one authoritative `InstanceRuntime` owns all admitted characters and instance-local simulation.
- Source channels cannot remain co-owners or mutate instance-local state.
- Each participant retains validated `origin_channel_id` and return metadata for exit, reconnect, audit and recovery.
- Entry and exit are explicit, generation-fenced, idempotent simulation-ownership transitions.

### Map and spatial model

- A channel map, a revisioned activity map template and one concrete instance's mutable state are separate concepts.
- Physical entrance objects such as a boss lever remain channel-owned map state.
- The boss arena or dungeon is created from a revisioned content template.
- Immutable geometry, collision and static assets may be shared across instances.
- Each `WorldId + InstanceId` owns an isolated mutable overlay for players, creatures, doors, effects, timers, mechanics, objectives, corpses and reward state.
- Position identity includes its spatial context: channel space or instance space. Raw `x, y, z` values are insufficient across boundaries.

### Shared admission engine

- Physical levers, Party Finder, quests, events, arena queues and authorized operations are different admission sources for one common activity-instance engine.
- They share capacity reservation, eligibility validation, instance allocation, ownership transfer, snapshot, reward and recovery boundaries.
- A physical fixed-group boss lever validates the complete group before transfer and defaults to strict all-or-nothing admission.
- Party Finder may assemble players from several channels of the same world and transfer them directly into one common instance.
- Party Finder does not first move an instanced group to a temporary common channel.
- Party Finder does not maintain a separate copy of the boss map or a parallel instance implementation.

### Seamless no-relogin transition

- Instance entry does not repeat account authentication or character selection.
- Cross-GameNode movement uses a seamless make-before-break handoff: destination reserve, background destination connection/context, fenced ownership commit, full authoritative snapshot, then source retirement.
- Same-GameNode connection reuse is an optimization and must preserve identical safety semantics.
- Admission material is short-lived, scoped and replay-safe; exact fresh-session versus continuation/grant form remains owned by `FND-04`.
- ADR-0003 remains authoritative: the Gateway stays in the control plane and does not become a permanent gameplay proxy.

### Completion and return

- Encounter results, rewards, lockouts and inventory mutations are settled authoritatively and idempotently before unsafe cleanup.
- Each participant normally returns through validated origin-channel routing to a safe configured exit anchor.
- An unavailable origin channel follows a later explicit recovery policy and never causes silent arbitrary-channel placement.
- The entrance may serve another group after the prior admission commits, subject to activity concurrency and capacity policy.

### Social presence and privacy

- Exact channel, instance, GameNode and map placement are non-public.
- Exact channel visibility is limited by default to current party members and mutually accepted contacts/VIP entries, subject to later user controls.
- Contact/VIP creation requires invitation and explicit acceptance.
- Party invitations and contact invitations remain separate consent lifecycles.
- Character contact is the default relationship scope.
- Account-wide friendship is separate, optional and requires distinct mutual consent.
- Alternate characters remain hidden by default and require deliberate owner-controlled sharing.

## Acceptance criteria

- [x] Record the four identifier classes and cross-cutting invariants.
- [x] Record `WorldId + ChannelId`, `WorldId + PartyId` and `WorldId + InstanceId` semantic scope.
- [x] Record channels as primary topology and instances as optional isolated gameplay contexts.
- [x] Permit same-world cross-channel admission into one concrete instance.
- [x] Require one authoritative `InstanceRuntime` and forbid dual source/destination writers.
- [x] Retain validated origin-channel routing per participant.
- [x] Record explicit fenced entry and exit ownership transitions.
- [x] Record channel map, activity map template and instance-local mutable overlay separation.
- [x] Record channel/instance-scoped spatial identity.
- [x] Record one shared activity-admission engine for physical triggers and Party Finder.
- [x] Record physical fixed-group boss validation and default all-or-nothing barrier.
- [x] Record direct cross-channel Party Finder admission without an intermediate common channel.
- [x] Record seamless make-before-break handoff without user-visible relog.
- [x] Preserve ADR-0003 Gateway control-plane boundary.
- [x] Record authoritative completion, idempotent reward settlement, safe return and cleanup ordering.
- [x] Preserve privacy-first social and alternate-character consent decisions.
- [x] Keep exact token format, protocol schema, map source format, runtime placement, matchmaking and implementation unresolved.
- [x] Make no runtime, protocol, database, schema, migration or external-repository change.
- [x] Maintain documentation-only draft PR #56.
- [ ] Reconcile older wording in `FND-ID-01_OWNER_ACCEPTED_BASELINE.md` that still describes instance scope as unresolved, or explicitly rely on the later-instance-baseline supersession statement.
- [ ] Obtain exact-head repository validation and independent audit before merge.

## Excluded scope

- no `protocol-oteryn` schema or codec;
- no Rust identifier, runtime or map types;
- no PostgreSQL representation;
- no concrete Game Session, continuation or admission-grant format;
- no instance runtime implementation;
- no content/map source-format implementation;
- no Party Finder implementation or matching algorithm;
- no final capacity, placement, migration, reconnect, lockout, reward or spectating policy;
- no client transition UI implementation;
- no write to `blakinio/otclient`;
- no claim that complete foundation or gameplay contracts are accepted or finished.

## Validation

### Focused

- changed scope expected: four Markdown files in PR #56;
- architecture consistency: pending exact-head review after this checkpoint;
- source/runtime behavior: `NOT_APPLICABLE` for documentation-only change.

### Component/integration/E2E

- result: `NOT_APPLICABLE` for this architecture-only PR;
- later contracts must define deterministic E2E evidence listed in `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`.

### Exact-head CI

- exact head: pending after checkpoint update;
- workflow/run: pending;
- result: pending.

## Independent audit

- exact head: pending;
- method/auditor: pending;
- material findings: pending;
- verdict: pending.

## PR and closeout

- PR: #56, draft and open at the last observation;
- changed-file review: pending exact-head refresh;
- unresolved review threads: pending;
- merge result: pending;
- accepted decisions are not yet canonical on `main` until validation, audit and merge complete.

## Context checkpoint

```yaml
last_progress: Owner accepted the shared physical-trigger and Party Finder activity-instance model, revisioned map templates with isolated mutable overlays, direct cross-channel admission, seamless make-before-break no-relogin handoff, authoritative completion and origin-channel return semantics. The instance baseline and task record were updated.
status: validating
branch: docs/fnd-id-01-owner-baseline
head_sha_before_checkpoint: 55157cc155e84e909e69026606b903a4dfd5154a
pr: 56
blocker: PR #56 requires exact-head validation and independent audit before merge; one older FND-ID baseline section still needs wording reconciliation or explicit supersession handling.
next_action: Reconcile identifier-baseline wording, refresh PR description, then validate the exact final head.
```
