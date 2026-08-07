# OTV2-20260806-identifier-owner-baseline

```yaml
task_id: OTV2-20260806-identifier-owner-baseline
title: Record owner-accepted identifier, instance and social-presence baselines
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/fnd-id-01-uuidv7-baseline
pr: 57
supersedes_pr: 56
final_head_sha: f0294bfa4f67237e9d1ac5115681981dd1b85862
merge_commit: 5fb899bbce999376565d5b0b905c101696891e44
architecture_head_reviewed: 80048871a06dae381c8bcec9e1c8978607236cb8
completed_at: 2026-08-07T08:27:08+02:00
owner: released
owned_paths: []
public_contracts:
  - docs/architecture/FND-ID-01_OWNER_ACCEPTED_BASELINE.md
  - docs/architecture/UUIDV7_DURABLE_IDENTITY_OWNER_BASELINE.md
  - docs/architecture/INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md
  - docs/architecture/SOCIAL_PRESENCE_AND_CONTACT_CONSENT_OWNER_BASELINE.md
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
```

## Outcome

The owner-accepted identifier, UUIDv7, instance-runtime, activity-admission and privacy-first social baselines are canonical on `main`. This task records architecture only and does not authorize runtime, protocol, database or migration implementation.

## Accepted durable identity model

- independently addressable Oteryn-owned durable cross-boundary semantic identities default to strongly typed UUIDv7;
- externally owned identifiers adopt UUIDv7 only through their authoritative contract or coordinated migration;
- UUIDv7 is identity, not authorization, bearer secret, sequencing, fencing or causality;
- canonical channel, instance and party semantics remain `WorldId + ChannelId`, `WorldId + InstanceId` and `WorldId + PartyId`;
- hot simulation uses generation-fenced runtime handles;
- frequent gameplay traffic uses compact session-local handles;
- ticks, sequences, revisions, generations, offsets and fencing remain scoped numeric values (`u64` class by default);
- static content uses stable content keys, revisions and compact bundle/runtime IDs;
- aggregate snapshot transport does not automatically turn every nested runtime object into an independently durable UUID entity;
- adopted UUID identities use native PostgreSQL `uuid` and preserve the full 128-bit value on binary boundaries;
- internal UUIDv7 values are not automatically public references because their time component may reveal approximate creation time;
- capacity planning benchmarks the accepted hybrid rather than UUIDv7 in every hot path.

## Additional accepted topology and privacy consequences

- channels remain the primary persistent-world topology;
- eligible same-world players from several channels may enter one authoritative concrete instance;
- one `InstanceRuntime` owns instance-local simulation and participants after admission;
- entry/exit and GameNode handoff are fenced, idempotent ownership transitions;
- Gateway remains in the control plane per ADR-0003;
- exact channel, instance, node and map placement are non-public;
- contact/VIP and account-wide friendship require explicit mutual consent and alternate characters remain hidden by default.

## Validation

- architecture decision head `80048871a06dae381c8bcec9e1c8978607236cb8`: `PASS_ZERO_MATERIAL_FINDINGS` after resolving Platform-authority ambiguity and snapshot over-identification;
- final update-from-main head: `f0294bfa4f67237e9d1ac5115681981dd1b85862`;
- changed-file review against current main: exactly five declared Markdown files;
- update-from-main exact-head architecture audit: `PASS_ZERO_MATERIAL_FINDINGS`;
- unresolved review threads: none;
- E2E: `NOT_APPLICABLE` — architecture-only documentation;
- required workflow run: `31153919845`;
- required job: `92789187554` — `Agent governance / validate`;
- exact-head required check result: `PASS`;
- PR #57 squash merged to `main` as `5fb899bbce999376565d5b0b905c101696891e44`.

## Closeout

PR #56 remains intentionally superseded and unmerged. PR #57 is canonical. All task-owned paths are released; the four architecture documents listed above are the durable source of truth for subsequent FND-ID-01/FND-02/FND-03/FND-04, durability, analytics, capacity and implementation work.
