# Reusable agent prompts

These prompts are execution contracts for recurring Oteryn v2 programmes. They do not replace trusted-base governance, live task checkpoints, accepted ADRs/contracts or live PR/CI state.

## Architecture / decision prompts

- `OTV2_ARCHITECTURE_CONTINUATION_AGENT.md` — iterative Oteryn-v2 architecture work in architecture/analysis-only mode by default. Short invocation: `Oteryn: architektura`.
- `OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md` — staged global architecture decision coordinator.
- `OTV2_DOMAIN_ARCHITECTURE_DESIGN_AGENT.md` — bounded domain architecture design worker allocated by the architecture coordinator.

## Implementation programme

Canonical implementation order and dependencies are defined by:

- `../programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md`.

### Normal entry point

- `OTV2_IMPLEMENTATION_COORDINATOR.md` — implementation coordinator. **Normal short invocation: `Oteryn: implementation coordinator`.**

The coordinator resolves live `main`, performs the serial bootstrap gate first, creates exact worker allocations and only then releases non-overlapping implementation lanes. This is the recommended way to start implementation.

### Direct worker aliases

Direct aliases exist for recovery or an explicitly coordinator-allocated lane. A worker MUST verify a live coordinator allocation naming its lane and exact owned paths before any write. Without that allocation it remains read-only and does not create its own scope.

- `OTV2_IMPL_WORKSPACE_BOOTSTRAP.md` — `Oteryn: impl bootstrap`.
- `OTV2_IMPL_FOUNDATION_RUNTIME.md` — `Oteryn: impl foundation`.
- `OTV2_IMPL_SIMULATION.md` — `Oteryn: impl simulation`.
- `OTV2_IMPL_DOMAIN_CORE.md` — `Oteryn: impl domains`.
- `OTV2_IMPL_DURABILITY.md` — `Oteryn: impl durability`.
- `OTV2_IMPL_VSL_CONTENT.md` — `Oteryn: impl content`.
- `OTV2_IMPL_GAME_ABILITY.md` — `Oteryn: impl ability`.
- `OTV2_IMPL_GAME_INTERACTION.md` — `Oteryn: impl interaction`.
- `OTV2_IMPL_GAME_AI.md` — `Oteryn: impl ai`.
- `OTV2_IMPL_SERVER_SEAM.md` — `Oteryn: impl server seam` (production gameplay listener/client-entry integration; requires the exact #96-derived coordinator allocation).
- `OTV2_IMPL_NATIVE_CLIENT.md` — `Oteryn: impl client`.
- `OTV2_IMPL_QA_E2E.md` — `Oteryn: impl qa`.
- `OTV2_IMPL_VSL_MOVEMENT.md` — `Oteryn: impl movement`.
- `OTV2_IMPL_VSL_COMBAT.md` — `Oteryn: impl combat`.
- `OTV2_IMPL_GAME_CHANNEL.md` — `Oteryn: impl channel` (later multichannel product lane; not a first bootstrap dependency).
- `OTV2_CONTENT_FORMAT_SPIKE.md` — `Oteryn: content format spike` (evidence only; cannot select permanent format by itself).
- `OTV2_IMPL_ANALYTICS.md` — `Oteryn: impl analytics` (later; requires concrete producer event families).

## Safety / authority

A prompt alias grants only the bounded task request represented by that prompt and current coordinator allocation. It never grants production/protected-environment approval, live data/session/account mutation, Platform/external-repository write authority, Reference parity, entitlement activation or owner-funded AI use.

High-risk protocol/session/persistence/item/loot/value/multichannel/fencing work still requires genuinely independent exact-head review under root `AGENTS.md`.

`PROD-ENTITLEMENTS-01` remains excluded from the implementation prompt DAG until separately accepted.

## Reuse rule

Before reuse, evaluate the selected prompt against `../PROMPT_EVAL_STANDARD.md`, read the canonical implementation DAG, and verify all repository state named by the prompt against live GitHub state.

A short invocation is only an alias for resolving the canonical prompt from live `main`; it is not permission to use a cached prompt body or bypass current repository instructions.
