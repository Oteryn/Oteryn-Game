# Oteryn v2 — Post-SIM Wave 1 Parallel Worker Launch Pack

- Prepared: `2026-08-22`
- Preparation issue: `#43`
- Preparation base: `main@0161b80c351b644b47c28b290a6f54b44f775de7`
- Mode: `COORDINATE / LAUNCH GUIDANCE ONLY`
- Runtime / production authority: `NONE`
- Implementation write authority from this file: `NONE`

## Purpose

Provide one current launch surface for the next post-SIM implementation wave without duplicating the canonical worker prompt bodies or weakening the live coordinator allocation gate.

The reusable worker prompts already exist on canonical `Oteryn/Oteryn-Game` and were previously evaluated as `PASS` under `PROMPT_EVAL_STANDARD.md`. This launch pack revalidates their applicability to the current post-SIM state and records recommended execution-agent/effort settings only.

A short alias always resolves the canonical prompt from live `main`; never use a cached copy when repository state can be read directly.

## Proven preparation baseline

At preparation time:

- canonical repository: `Oteryn/Oteryn-Game`;
- exact `main`: `0161b80c351b644b47c28b290a6f54b44f775de7`;
- Bootstrap is completed/archived;
- SIM is completed/archived and its production determinism crate is consumed by `apps/game-server`;
- live coordinator state is `SIM_COMPLETED_NEXT_ALLOCATION_PENDING`;
- there is no active implementation worker allocation;
- FOUNDATION, DOMAIN, CONTENT and QA are the next dependency-ready Wave 1 lanes in principle;
- shared workspace/registry/stable-ID mutations remain serialized by the coordinator.

These are preparation-time facts, not a permanent status database. Every launcher must re-read live `main`, current coordinator task and `OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` before acting.

## Required launch order

### 0. Coordinator first

Canonical prompt:

`docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md`

Alias:

`Oteryn: implementation coordinator`

Recommended execution profile: coding/repository agent with **XHigh** reasoning effort.

The coordinator must resolve the then-current exact `main`, publish bounded non-overlapping allocations and serialize shared workspace/registry/stable-ID ownership before any worker receives write authority.

### 1. Parallel workers after allocation

The four workers below may run concurrently only after a live coordinator allocation explicitly names each lane, branch, exact base, owned paths, contracts/registries, dependencies and merge order. Without such allocation each worker remains read-only discovery only.

#### FOUNDATION

Canonical prompt:

`docs/agents/prompts/OTV2_IMPL_FOUNDATION_RUNTIME.md`

Alias:

`Oteryn: impl foundation`

Recommended execution profile: coding/repository agent with **XHigh** reasoning effort.

Reason for elevated effort: protocol, transport, session, admission, reconnect and fencing are high-risk boundaries and require exact negative/security/replay evidence plus genuinely independent exact-head review where repository policy requires it.

#### DOMAIN

Canonical prompt:

`docs/agents/prompts/OTV2_IMPL_DOMAIN_CORE.md`

Alias:

`Oteryn: impl domains`

Recommended execution profile: coding/repository agent with **High** reasoning effort.

#### CONTENT

Canonical prompt:

`docs/agents/prompts/OTV2_IMPL_VSL_CONTENT.md`

Alias:

`Oteryn: impl content`

Recommended execution profile: coding/repository agent with **High** reasoning effort.

#### QA

Canonical prompt:

`docs/agents/prompts/OTV2_IMPL_QA_E2E.md`

Alias:

`Oteryn: impl qa`

Recommended execution profile: coding/repository agent with **High** reasoning effort.

## Prompt revalidation

Current reusable prompt verdict: `PASS` for all four canonical workers.

Launch readiness at preparation time: `BLOCKED_PENDING_COORDINATOR_ALLOCATION`, because the live allocation record explicitly contains no active implementation worker allocation. This is an authority precondition, not a prompt defect.

The launch pack does not grant any worker permission to self-allocate, edit coordinator-owned allocation records, invent shared IDs, change architecture for convenience, use production/protected environments, mutate live data, write external repositories or invoke owner-funded metered AI without the exact authorization required by repository governance.

## Parallelism rule

After allocation, FOUNDATION, DOMAIN, CONTENT and QA may overlap only where exact owned paths and public contracts do not overlap. Any root Cargo/workspace policy, common registry, stable-ID or other shared mutation must be explicitly serialized by the coordinator even if the implementation workers otherwise execute concurrently.

## Completion rule

This launch pack is complete when the canonical prompt references and effort guidance are available on `main`. It does not start implementation and does not make any worker lane complete. Each lane remains governed by its canonical prompt, live task, exact-head validation, review, merge, post-merge verification, archive and ownership release requirements.
