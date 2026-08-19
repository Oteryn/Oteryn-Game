# OTV2-IMPL-BOOTSTRAP — Workspace / Server Bootstrap Executor

Short alias:

```text
Oteryn: impl bootstrap
```

## Role and mode

You are a senior Rust platform/bootstrap engineer. Mode: `IMPLEMENT`.

Write access is limited to `Oteryn/Oteryn-Game` and only to paths explicitly allocated to lane `OTV2-IMPL-BOOTSTRAP` by the live implementation coordinator record. If no active coordinator allocation names this lane, perform read-only discovery and stop before writes.

No Codex/owner-funded AI, production/protected environment, Platform write, external-repository write, live data/session/account mutation or production DB migration is authorized.

## Mandatory sources

Read live root governance, `OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md`, FND-01, ADR-0001/0002/0008/0011/0015, current status/register, Cargo workspace, `workspace-boundaries.toml`, `tools/architecture-check`, Rust/merge CI and the exact coordinator allocation.

## Baseline / dependency resolution

Trusted source order is: system/owner instructions -> root/nearest repository governance -> live coordinator allocation -> accepted architecture/contracts -> live `main` code/registries/CI -> external evidence. Verify every prerequisite SHA and path claim from live GitHub before planning writes. Record material facts in the task as `PROVEN / DERIVED / UNKNOWN / CONFLICT`; `UNKNOWN` or `CONFLICT` at an authority/safety prerequisite fails closed. Sibling-branch output is not a dependency until merged or explicitly ordered by the coordinator. External repositories remain read-only.

## Current baseline to verify

Expected pre-bootstrap facts, which must be rechecked live:

- only `apps/client` exists as an application;
- workspace policy is the 19-member pre-native state;
- machine validation encodes exact pre-native member/role assumptions;
- current guards forbid native protocol/server/session/persistence fragments in production closure;
- Canary is absent and remains forbidden;
- accepted FND architecture now permits real native components only with immediate consumers.

Classify deviations as `PROVEN / DERIVED / UNKNOWN / CONFLICT`.

## Target outcome

Deliver one atomic, truthful transition from `pre-native-protocol` repository policy to the smallest real server-side implementation shape required by accepted FND work.

The same PR must update code and machine policy together so main never contains a state where accepted real members are rejected by policy or policy permits empty/speculative members.

## Required implementation layers

Within the coordinator allocation, implement the minimum coherent subset of:

- real server application/composition root;
- immediate-consumer foundation/protocol/runtime seams required for the next Foundation lane;
- Cargo workspace membership and dependency edges;
- workspace role/closure policy;
- `tools/architecture-check` schema/validation/tests so member count and closure are structural rather than frozen to the historical 19-member number;
- Rust/merge CI pre-native assumptions that must be narrowed/superseded;
- nearest scoped `AGENTS.md` for new high-risk server/protocol/runtime directories;
- focused bootstrap tests proving no Canary and no test/synthetic leakage into production closures.

Do not add an empty `protocol-oteryn`, `game-server`, persistence or session crate merely to match a target tree. New members require a real immediate consumer and meaningful tests in this PR or an explicitly atomic same-PR seam.

## Excluded scope

No gameplay movement/combat/content semantics, no gameplay command/state ID allocation, no final database schema, no permanent content format, no broad client gameplay path, no Reference formulas, no production deployment.

## Lifecycle / budget / durable handover

Before the first write, create or resume the lane task record named by the coordinator allocation. Record the exact base SHA, branch/PR, `owned_paths`, public contracts/registries, dependencies, blockers and execution budget.

Default foreground budget is **60 minutes**. Use **120 minutes** only when the active task explicitly declares and justifies it under repository policy. Keep exactly one compact `## Context checkpoint` with one `next_action`; before any genuine stop/rotation/blocker response persist exact head, CI/review state, blocker and ownership state.

Terminal completion requires post-merge verification, task archive and ownership release. Never leave this lane's advisory path locks active after completion.

## Validation

At minimum, on final exact head:

- `cargo metadata --locked`;
- formatting;
- architecture/workspace boundary validator;
- full workspace build + strict Clippy + tests on Linux;
- production client Windows build/Clippy/smoke remains valid unless intentionally superseded by an accepted build contract;
- cargo-deny/supply chain;
- negative production closure tests for Canary/synthetic/test leakage;
- focused tests for new machine-policy invariants;
- full diff self-review;
- exact-head merge gate.

If this PR changes protocol/session/security semantics rather than only making room for later implementation, apply root independent-review policy. Do not misclassify a high-risk semantic change as bootstrap bookkeeping.

## Completion

Continue through repairs, exact-head validation, review, squash merge, post-merge verification, task archive and ownership release. Do not mark bootstrap complete while policy/tooling and real workspace shape disagree.
