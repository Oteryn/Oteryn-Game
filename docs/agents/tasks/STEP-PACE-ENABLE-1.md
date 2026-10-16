# STEP-PACE-ENABLE-1 — document the owner-controlled VPS rollout

```yaml
task_id: STEP-PACE-ENABLE-1
title: "STEP-PACE-ENABLE-1: step-wait receipts VPS rollout documentation"
mode: IMPLEMENT
status: authoring
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/step-pace-enable-1
base_sha: 5742f30f3
pr: 2135
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row STEP-PACE-ENABLE-1 (rows 3, 4); §5 Wave 5"
created_at: 2026-10-15
updated_at: 2026-10-16
class: "docs (owner-controlled rollout)"
needs_migration: no
needs_registry: no
needs_content_regeneration: false
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "deploy/synology-game/README.md (Step pacing receipts rollout section only)"
  - "docs/agents/tasks/STEP-PACE-ENABLE-1.md"
slot_only_paths: []
```

## Goal and accepted decisions

Document the immediate owner-controlled enablement of `OTERYN_STEP_WAIT_RECEIPTS=1` on oteryn-vps. The README retains its historical location; Synology is not the deployment target.

- [CP D1066–D1068](https://github.com/Oteryn/Oteryn-Game/pull/2135#issuecomment-6216594441): use the host marker `BASE/run/step-wait-receipts` containing exactly `1`; leave the code default off. Scope is the README rollout section and this record.
- [Owner instruction, 2026-10-16](https://github.com/Oteryn/Oteryn-Game/pull/2135#issuecomment-6216813505): deploy immediately on oteryn-vps, superseding the release/retirement gate in D1066/D1068. Do not await #1942 or invent release/retirement evidence.
- D1067 retains byte-identical results when off and receipts only for capability 13 sessions when on. Serialized transitions and durable resume state are not acceptance criteria for this documentation task.

The old capability 13 builds 9c/6c3 reject field 2. The owner supersession removes their retirement as a prerequisite; it does not make their decoders compatible or claim they were retired.

## Owned scope and boundaries

Update the README rollout section and this task card on #2135 in one authoring batch. No server default flip, supervisor code, test code, protocol, registry, content, migration or shared wiring change is needed. No queue-time slot edits are expected. No Platform writes, production action, review trigger, ready transition, merge or deployment are delegated to the builder.

The prior `supervisor.sh` and `test-supervisor-npc.sh` owned paths are removed by D1067. The previous default-flip alternative is superseded by D1066. The README documents owner actions; it does not execute them.

## Existing implementation and contracts

STEP-PACE-WIRE-1 merged as [#2041](https://github.com/Oteryn/Oteryn-Game/pull/2041). The existing Game implementation reads `OTERYN_STEP_WAIT_RECEIPTS` once per process; only exactly `1` enables it. Off preserves all step-result bytes. On emits field 2 only to sessions that selected capability 13, on `MOVED` and `TOO_EARLY`, never `BLOCKED` or `REJECTED`.

- `apps/game-server/src/gameplay_transport/world_spatial.rs`: switch and receipt boundary.
- `apps/game-server/src/gameplay_transport/connection.rs`: connection-level receipt selection.
- `docs/contracts/protocol-oteryn/v1/world_spatial_v1.proto`: `WorldActorStepResultV1.step_wait_ms`.
- `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json`: capability 13 and command type 1 bounds.
- `docs/agents/tasks/archive/OTV2-20261013-step-pace-wire-1.md`: implementation and historical rollout gate; the owner instruction above supersedes that gate.

## Active controller and remaining operational evidence

[Game #2218](https://github.com/Oteryn/Oteryn-Game/pull/2218) moves current operations to Oracle and the separately installed durable Game incarnation controller. Game `deploy/oracle/README.md` on protected main routes host implementation to Oteryn-Platform `deploy/oracle/`; this task's old base predates that migration.

**PROVEN source gap:** Platform [`7552a5003cf637bee7798743b913d915dd55e825`](https://github.com/Oteryn/Oteryn-Platform/tree/7552a5003cf637bee7798743b913d915dd55e825/deploy/oracle) starts Game `serve` directly in `game-runtime.compose.yml`; its Game environment has no `OTERYN_STEP_WAIT_RECEIPTS`. `game-supervisor.py` has no step-wait marker handling. The historical Game `deploy/synology-game/supervisor.sh` marker implementation does not prove an active VPS integration.

**UNKNOWN installed state:** controller/configuration revision, marker consumption, process environment, enabled receipts and off-state byte identity on the VPS. These are not established by source inspection or by this PR. The marker alone cannot close this evidence gap.

**Owner and trigger:** OWNER-I establishes the supported Platform marker-to-process environment path, applies the owner-authorized rollout and restarts via the durable controller. Any missing controller support is a separately allocated Platform change. Retain links to the installed revision, marker inspection, incarnation/readiness and capability boundary/off-state evidence before claiming operational completion. The owner has already waived waiting for the client release; this gap concerns actual enablement evidence.

## Acceptance criteria

- [x] README and task card record the host-marker decision, default-off behavior, capability 13 boundary and owner supersession of the release/retirement gate.
- [x] Runbook identifies oteryn-vps and Platform controller ownership, historical Synology location, owner-only marker/restart/rollback actions and the exact operational proof gap.
- [x] No production action is taken by the builder; no release, retirement or active marker integration is claimed without evidence.
- [ ] OWNER-I supplies active controller support and rollout evidence described above (operational handback, outside builder authoring scope).

## Validation

Documentation-only local validation:

- `git diff --check`: passed.
- `python tools/agents/validate_governance.py`: passed.
- `python -m unittest discover -s tools/agents/tests -p '*governance_lifecycle*.py'`: 25 passed.
- `python -m unittest tools.agents.tests.test_meta_agent_policy_adoption`: 30 passed.
- Complete diff review and a section-boundary check: only the two owned files change, and README text outside the rollout section is unchanged.

Rust builds, server tests and supervisor tests are not needed because no executable file or contract changes. Existing implementation qualification is recorded under STEP-PACE-WIRE-1; it is not presented as new live VPS validation.

## Builder handback

Keep #2135 draft; the control plane owns qualification and lifecycle transitions. Do not use the README's historical marker command as authority for host access. Report any required edit outside these two owned paths to its owner. This task is not operationally complete until the owner evidence gap is closed.
