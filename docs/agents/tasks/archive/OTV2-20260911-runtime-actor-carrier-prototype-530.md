# OTV2-20260911-runtime-actor-carrier-prototype-530

```yaml
task_id: OTV2-20260911-runtime-actor-carrier-prototype-530
title: Prove bounded Channel actor-carrier prototype
mode: IMPLEMENTATION_EVIDENCE
status: completed
repository: Oteryn/Oteryn-Game
issue: 530
coordinator_issue: 162
base_branch: main
branch: agent/runtime-actor-carrier-prototype-530
pr: 568
activation_base_sha: 32a055ac9b2c4e69773f1d6cb9746ea412164866
final_head_sha: 3f9eea232e7bd40d2f6a3185b06e7e1d84985328
merge_commit_sha: 90a3f92434e32354ff1aaeac96d038bbc49eba9c
merge_queue_run: 34606613550
owner: OTV2_WORK_COORDINATOR_ACTIVATED_WORKER
created_at: 2026-09-11
completed_at: 2026-09-11
execution_policy: continuous_progress
historical_owned_paths:
  - apps/game-server/examples/runtime_actor_carrier_prototype.rs
  - docs/agents/evidence/OTV2-20260911-runtime-actor-carrier-prototype.json
  - docs/agents/evidence/OTV2-20260911-runtime-actor-carrier-prototype.md
  - docs/agents/tasks/active/OTV2-20260911-runtime-actor-carrier-prototype-530.md
ownership_released: true
implementation_authority: TERMINAL_NON_PRODUCTION_EXAMPLE_ONLY
registry_mutation_authority: NONE
production_authority: NONE
external_repository_write_authority: NONE
```

## Protected activation

PR #564 was protected as `main@32a055ac9b2c4e69773f1d6cb9746ea412164866`.
Coordinator #162 explicitly activated this exact lineage in comment `5632224967`; Issue #530
records the same activation in comment `5632227300`.

No competing runtime-actor worker existed at activation. The worker was limited to the four historical
paths listed above.

## Delivered outcome

PR #568 delivered the bounded `CHANNEL_RUNTIME_ACTOR_CARRIER_V1` prototype using the real public
Foundation `RuntimeScopeRefV1::Channel`, `WorldId`, `ChannelId` and `ScopeOwnershipGeneration` types,
while remaining outside production runtime composition.

The protected candidate demonstrated:

- fixed slot backing with direct `ActorLocalId -> slot/generation` addressing;
- no separately growing actor lookup index or tombstone/retirement-history store;
- exact scope/generation/local-id/local-generation lookup and removal;
- independent wrong-World, wrong-Channel, stale outer-generation, vacant/out-of-range and stale local
  generation rejection;
- complete rollback for injected post-selection failure;
- local generation retention/reuse/exhaustion semantics without wrap;
- move-only namespace continuity authority outside carrier backing;
- same-generation rebootstrap fail-closed and strictly newer generation recovery only;
- checked retained-byte / actor-ID arithmetic;
- independent M/M+1 and deterministic lookup/insertion/removal work evidence for prototype M=1..4;
- explicit non-production/no-registry/no-#508/no-#139 scope.

Those prototype values and measurements are evidence only. They are not a production actor maximum.

## Exact-head qualification and integration

Final PR source head:

`3f9eea232e7bd40d2f6a3185b06e7e1d84985328`

Verified exact-head repository evidence includes:

- Merge Gate run `34605243881`: `SUCCESS`;
- Agent Governance run `34605243742`: `SUCCESS`;
- Architecture Semantic Audit runs `34605243972` / `34606148951`: `SUCCESS`;
- independent Codex review on exact head `3f9eea232e...`, comment `5635298280`: no major issues.

The candidate integrated through the native Merge Queue. Real `merge_group` run `34606613550`
completed `SUCCESS` and produced protected merge commit:

`90a3f92434e32354ff1aaeac96d038bbc49eba9c`

PR #568 is closed/merged. Later protected PR #570 was based on that integrated prototype evidence and
explicitly separated bounded pre-production implementation from future production-capacity acceptance.

## Closeout

The prototype implementation task is terminal. Its four historical writable paths are released by
#162 and must not be treated as an active lease merely because older task prose said `validating`.

The example/evidence files remain historical protected evidence. Reuse or modification requires a new
allocation; this archived task grants no continuing mutation authority.

Any later real carrier implementation must follow its own merged allocation, exact owned paths,
review/CI, native Merge Queue and protected-main readback. It must not infer production capacity,
RESOURCE_LIMITS_REGISTRY mutation, #508 activation or #139 activation from this prototype.
