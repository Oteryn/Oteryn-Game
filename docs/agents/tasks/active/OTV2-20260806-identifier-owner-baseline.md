# OTV2-20260806-identifier-owner-baseline

```yaml
task_id: OTV2-20260806-identifier-owner-baseline
title: Record owner-accepted identifier, instance and social-presence baselines
mode: CONTRACT
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-id-01-owner-baseline
pr: 56
base_sha: 26b5fa275fba19fdee0e26a6f65263489af3e500
architecture_head_reviewed: 7690f5653a1d5de0aa25528a4185c94618cfbec8
owner: ChatGPT architecture coordinator
created_at: 2026-08-06T14:19:00+02:00
updated_at: 2026-08-06T17:39:00+02:00
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

Persist the product owner's accepted identifier, channel, world-scoped party, instance-runtime, map-template, activity-admission, seamless-handoff and privacy-first social baselines as canonical architecture input without claiming that the complete `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`, Party Finder or instance-lifecycle contracts are complete.

## Delivered owner decisions

### Identifier and topology

- `WorldId` globally identifies one logical world.
- Canonical semantic identities are `WorldId + ChannelId`, `WorldId + InstanceId` and `WorldId + PartyId`.
- Channels remain the primary persistent-world topology.
- Instances are optional isolated gameplay contexts and do not create another world, economy or character namespace.
- Parties may organize members across channels of one world, while open-world simulation remains channel-local and instanced simulation remains instance-local.

### Instance runtime, map and admission

- Eligible players from several channels of one world may enter one concrete instance.
- One authoritative `InstanceRuntime` owns all admitted participants and instance-local simulation.
- Each participant retains validated origin-channel and return metadata.
- Entry and exit are explicit generation-fenced, idempotent ownership transitions.
- Channel map, revisioned activity-map template and instance-local mutable overlay are separate concepts.
- Immutable geometry, collision and static assets may be shared while players, creatures, doors, effects, timers, objectives, corpses and reward state remain isolated per instance.
- Positions are scoped by `ChannelSpace` or `InstanceSpace`; raw coordinates alone do not identify a location across runtime boundaries.
- Physical levers, Party Finder, quests, events and queues consume one shared authoritative activity-instance engine.
- A fixed five-player boss lever validates the complete group and defaults to all-or-nothing admission.
- Party Finder may admit same-world players directly from several channels into one common instance without an intermediate common channel.

### Seamless transition, completion and return

- Instance entry does not repeat account authentication or character selection.
- Cross-GameNode movement uses make-before-break handoff: reserve destination, prepare background destination context, fence and commit ownership, send a full authoritative snapshot, then retire the source path.
- Same-GameNode connection reuse is only an optimization with identical safety semantics.
- Exact fresh-session, continuation or admission-grant representation remains owned by `FND-04`.
- ADR-0003 remains authoritative: Game Gateway stays in the control plane and does not become a permanent gameplay proxy.
- Rewards, lockouts and inventory mutations are settled authoritatively and idempotently before cleanup.
- Players normally return through validated origin-channel routing to a safe configured exit anchor.
- Unavailable origin routing requires an explicit recovery policy and never silently chooses an arbitrary channel.

### Social privacy

- Exact channel, instance, GameNode and map placement are non-public.
- Contact/VIP creation requires invitation and explicit acceptance.
- Character contact is the default relationship scope.
- Account-wide friendship is separate and requires distinct mutual consent.
- Alternate characters remain hidden by default and require deliberate owner-controlled sharing.

## Acceptance criteria

- [x] Record the four identifier classes and cross-cutting invariants.
- [x] Record accepted world-, channel-, instance- and party-scoped identity semantics.
- [x] Record channels as primary topology and instances as isolated optional contexts.
- [x] Permit same-world cross-channel admission into one concrete instance.
- [x] Require one authoritative `InstanceRuntime` and prohibit dual writers.
- [x] Record origin routing and fenced entry/exit ownership transitions.
- [x] Separate channel map, activity-map template and instance-local mutable overlay.
- [x] Record channel/instance-scoped spatial identity.
- [x] Define one shared activity-admission engine for physical triggers and Party Finder.
- [x] Define fixed-group all-or-nothing admission and direct cross-channel Party Finder entry.
- [x] Record seamless no-relogin make-before-break handoff while preserving ADR-0003.
- [x] Record authoritative completion, idempotent settlement, return and cleanup ordering.
- [x] Preserve privacy-first presence, contact and alternate-character decisions.
- [x] Reconcile older `FND-ID-01` wording so accepted instance scope is no longer marked unresolved.
- [x] Keep technical representations and implementation outside this PR.
- [x] Keep the PR documentation-only and limited to four declared Markdown paths.
- [x] Complete adversarial architecture audit with zero open material findings.
- [ ] Require all repository checks to pass on the exact unchanged final head before squash merge.

## Excluded scope

- no Rust implementation, runtime, map types or protocol codec;
- no PostgreSQL schema or migration;
- no concrete Game Session, continuation or admission-grant format;
- no Party Finder implementation or matching algorithm;
- no final capacity, placement, migration, reconnect, reward, lockout or spectator policy;
- no client transition UI implementation;
- no external-repository write;
- no claim that complete foundation or gameplay contracts are finished.

## Validation

### Changed scope

- base: `26b5fa275fba19fdee0e26a6f65263489af3e500`;
- architecture head reviewed: `7690f5653a1d5de0aa25528a4185c94618cfbec8`;
- comparison: 37 commits ahead, 0 behind;
- changed files: exactly four declared Markdown files;
- runtime/component validation: `NOT_APPLICABLE` because this PR contains architecture documentation only.

### E2E

- result: `NOT_APPLICABLE`;
- reason: no executable client, server, protocol, persistence or deployment behavior is changed; deterministic future scenarios are enumerated in `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md`.

### Exact-head CI

- required workflows: Agent governance, Dependency review and CodeQL as selected by live repository configuration;
- result: must be `PASS` on the exact unchanged head produced by this checkpoint before merge;
- live GitHub state is authoritative for the final head and run conclusions.

## Independent audit

- architecture head reviewed: `7690f5653a1d5de0aa25528a4185c94618cfbec8`;
- method: fresh adversarial complete-scope review against root governance, delivery closeout rules, build/test matrix, ADR-0003, ownership/fencing invariants, failure paths, privacy boundaries and all deliberately unresolved items;
- checked for: omitted layers, unsupported completion claims, client-authoritative state, dual writers, stale-source recovery, cross-world leakage, permanent Gateway proxying, map-state sharing, duplicate admission/reward paths, privacy side channels and stale unresolved wording;
- resolved finding: older `FND-ID-01` text incorrectly left accepted instance scope and cross-channel membership unresolved; it was reconciled before this audit verdict;
- open material findings: none;
- verdict: `PASS_ZERO_MATERIAL_FINDINGS`.

The final checkpoint-only task-record commit must receive exact-head CI and a delta audit confirming that it changes no architecture decision.

## PR and closeout

- PR: #56;
- state before final checks: open, draft, mergeable;
- review comments and inline threads: none at audit time;
- merge method: squash after exact-head PASS;
- post-merge requirement: archive this task and release the owned paths.

## Context checkpoint

```yaml
last_progress: All accepted identifier, instance-map, physical-trigger, Party Finder, seamless-handoff and social-privacy decisions were reconciled and passed an adversarial architecture audit with zero material findings.
status: ready
branch: docs/fnd-id-01-owner-baseline
architecture_head_reviewed: 7690f5653a1d5de0aa25528a4185c94618cfbec8
pr: 56
ci_check_generation: pending for checkpoint-only final head
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: 2026-08-06T17:39:00+02:00
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Verify the checkpoint-only final diff, obtain exact-head PASS from all required GitHub checks, record the external audit verdict on PR #56, squash-merge, then archive the task and release ownership.
```
