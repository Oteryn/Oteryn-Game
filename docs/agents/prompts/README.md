# Reusable agent prompts

These prompts are execution contracts for recurring Oteryn v2 programmes. They do not replace trusted-base governance, live task checkpoints, accepted ADRs/contracts or live PR/CI state.

## Architecture / decision prompts

- `OTV2_ARCHITECTURE_CONTINUATION_AGENT.md` — iterative Oteryn-v2 architecture work in architecture/analysis-only mode by default. Short invocation: `Oteryn: architektura`.
- `OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md` — staged global architecture decision coordinator.
- `OTV2_DOMAIN_ARCHITECTURE_DESIGN_AGENT.md` — bounded domain architecture design worker allocated by the architecture coordinator.
- `OTV2_SOL_EXECUTION_ARCHITECTURE_CONTINUATION.md` — continuation and packaging of the owner-approved Work-control-plane + Sol-lane-lead + selective-Codex execution model. **Short invocation: `Oteryn: sol execution architecture`.**
- `OTV2_SOL_SUPERVISING_ARCHITECT.md` — material cross-lane Game architecture decision role for durable escalation packets. **Short invocation: `Oteryn: sol supervising architect`.**

The Sol execution architecture prompt is governance/architecture only. It validates the written execution-model spec against live GitHub, requires owner confirmation of that written spec before authoring the adoption plan, and then packages the future Sol lane-lead prompt family. It grants no gameplay/runtime, production or cross-repository authority.

The Sol Supervising Architect is not a routine coding lane. It resolves `ARCHITECTURE_ESCALATION_REQUIRED` within existing owner-approved architecture authority and returns `OWNER_DECISION_REQUIRED` when product/scope/authority decisions exceed that boundary. Architecture resolution grants neither implicit runtime write authority nor merge authority: the architect cannot merge, auto-merge, close out as canonical or otherwise integrate any PR/decision it authored or materially changed, and must hand the exact artifact to the uniquely active control plane or another separately authorized merge role.

## Implementation programme

Canonical implementation order and dependencies are defined by:

- `../programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md`.

### Normal entry point

- `OTV2_IMPLEMENTATION_COORDINATOR.md` — implementation coordinator. **Normal short invocation: `Oteryn: implementation coordinator`.**

The coordinator resolves live `main`, performs the serial bootstrap gate first, creates exact worker allocations and only then releases non-overlapping implementation lanes. This is the recommended way to start implementation.

### Work delivery profile

- `OTV2_WORK_DELIVERY_COORDINATOR.md` — ChatGPT Work execution coordinator/subagent dispatcher with fail-closed material architecture escalation. **Short invocation: `Oteryn: work coordinator`.**

The Work profile does not supersede or widen `OTV2_IMPLEMENTATION_COORDINATOR`. It is a stricter execution profile: Work coordinates exact allocations, path-disjoint subagents and integration, while material architecture/API/schema/security/persistence/resource/cross-repository conflicts become durable `ARCHITECTURE_ESCALATION_REQUIRED` handoffs to the owner-designated Supervising Architect rather than worker-selected architecture.

### Single active control-plane rule

`Oteryn: work coordinator` and `Oteryn: terra game coordinator` may both remain `reusable`, but they are **mutually exclusive for mutating control-plane work inside one programme lifecycle**. Reusability permits resolution from live `main`; it does not activate a second scheduler/integrator.

The active profile is resolved from the current coordinator Issue/task. An explicit `active_control_plane_profile` wins. For a legacy task without that field, the profile already named as canonical coordinator prompt/owner remains active and every other reusable control-plane profile is read-only recovery. Switching profiles requires a durable docs/governance transition merged to protected `main`; alias invocation, chat instruction, model selection or tool availability is not a transfer.

If exactly one active profile cannot be proven, all control-plane mutation fails closed as `POLICY_CONFLICT`. The inactive profile may inspect live state and prepare a recovery/transfer packet, but it may not allocate workers, grant shared leases, integrate/merge, mutate coordinator status or close/archive the programme.

The existing #162 lifecycle remains under `OTV2_WORK_DELIVERY_COORDINATOR` unless a later merged coordinator transition explicitly selects Terra. This package therefore adds the safer Terra profile without silently stealing the live coordinator lifecycle.

### Terra High deterministic control plane + Sol leads

- `OTV2_TERRA_GAME_CONTROL_PLANE.md` — ChatGPT Work / Terra High deterministic control plane with **zero technical or architecture discretion**. **Short invocation: `Oteryn: terra game coordinator`.**
- `OTV2_SOL_DURABILITY_LEAD.md` — deep Durability lane reasoning/implementation. **`Oteryn: sol durability lead`.**
- `OTV2_SOL_SERVER_SEAM_LEAD.md` — production Server Seam lead; read-only until exact durable prerequisite/allocation is ready. **`Oteryn: sol server seam lead`.**
- `OTV2_SOL_CLIENT_QA_LEAD.md` — native Client + truthful Tier 1/Tier 2 QA lead. **`Oteryn: sol client qa lead`.**
- `OTV2_SOL_MOVEMENT_LEAD.md` — Movement lead gated by current Client/QA and #139 resource closure. **`Oteryn: sol movement lead`.**
- `OTV2_SOL_COMBAT_LEAD.md` — Combat/death/loot/XP/pickup lead gated by merged Movement and current prerequisites. **`Oteryn: sol combat lead`.**
- `OTV2_SOL_POST_VSL_EXPANSION.md` — read-only-by-default decomposition of remaining accepted Game work after terminal Movement+Combat VSL. **`Oteryn: sol post-vsl expansion`.**
- `OTV2_SOL_WORLD_CONTENT_PREP.md` - post-VSL World/Content read-only preparation; no mutation before exact later allocation. **`Oteryn: sol world content prep`.**
- `OTV2_SOL_NPC_AI_PREP.md` - post-VSL NPC/AI read-only preparation; no mutation before exact later allocation. **`Oteryn: sol npc ai prep`.**
- `OTV2_SOL_SYSTEMS_ECONOMY_PREP.md` - post-VSL Systems/Economy read-only preparation; no mutation before exact later allocation. **`Oteryn: sol systems economy prep`.**
- `OTV2_SOL_TOOLING_OPS_PREP.md` - post-VSL Tooling/Ops read-only preparation; no mutation before exact later allocation. **`Oteryn: sol tooling ops prep`.**

The Terra profile is additive and does **not** silently supersede `Oteryn: work coordinator` or `OTV2_IMPLEMENTATION_COORDINATOR`. When a programme has durably selected Terra as its unique active control plane, Terra may apply only deterministic GitHub/DAG/ownership/merge predicates; technical findings route to the owning Sol lead, material cross-lane decisions route to `Oteryn: sol supervising architect`, and owner-only scope/authority decisions return `OWNER_DECISION_REQUIRED`.

Canonical launch/promotion rules for this profile live in `../programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md`. Alias existence grants no write authority. Every mutating Sol lead must resolve a current exact merged allocation and exact owned paths before writing.
The four future-wave preparation aliases are deliberately non-mutating: after terminal VSL they may prepare exact allocation proposals, but they cannot create branches/commits, claim leases, integrate PRs or become implementation leads until a later merged exact allocation/prompt lifecycle grants that authority.

### Independent Work delivery audit

- `OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md` — read-only, high-effort forensic audit of the live Work coordinator lifecycle. **Short invocation: `Oteryn: work auditor`.**

The Work auditor independently reconstructs coordinator execution from live GitHub Issue/task/branch/PR/exact-head check/review/merge evidence, treats Work summaries as claims rather than proof, and verifies programme resolution, allocation timing, path/lease isolation, DAG order, architecture escalation, worker integration, QA truthfulness and closeout. It may recommend `PASS_CONTINUE`, lane/coordinator pause, reconciliation or architecture escalation, but it has no repository mutation, implementation, merge/close, production or cross-repository write authority. It does not supersede the broader `OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT`.

### Next-wave parallel preparation

- `OTV2_NEXT_WAVE_PARALLEL_PREPARATION.md` — preparation-wave launcher/matrix. **Short invocation: `Oteryn: next-wave prep swarm`.**
- `OTV2_PREP_WAVE2_RESOURCE_LIMITS.md` — Issue #93 resource-limit decision preparation. `Oteryn: prep resource limits`.
- `OTV2_PREP_DURABILITY_TOPOLOGY.md` — Issue #94 Durability topology preparation. `Oteryn: prep durability topology`.
- `OTV2_CONTENT_FORMAT_SPIKE.md` — Issue #95 evidence-only Content format spike. `Oteryn: content format spike`.
- `OTV2_PREP_SERVER_SEAM.md` — Issue #96 production gameplay server-seam preparation. `Oteryn: prep server seam`.
- `OTV2_PREP_PROGRAMME_STATUS.md` — Issue #97 maintained programme-status reconciliation. `Oteryn: prep programme status`.

These preparation prompts are deliberately isolated by domain. They may run concurrently only after each agent verifies live Issue/task ownership and disjoint paths. They do not grant implementation authority. Release Durability/Ability/Interaction/AI/Server Seam independently when their own master-plan gates close; do not wait for unrelated preparation work merely for symmetry.

### Next-wave blocker closure

- `OTV2_CLOSE_NEXT_WAVE_BLOCKERS.md` — owner-authorized blocker-closure coordinator for #93/#115/#116/#123. **Short invocation: `Oteryn: close next-wave blockers`.**

This coordinator may accept conservative evidence-backed first-slice hard maxima only inside the bounded owner authorization recorded by Issue #128, serializes registry canonicalization, and may carry the #115 Foundation verifier/consumer blocker through its separately allocated implementation lifecycle. It grants no Server Seam/gameplay implementation or production/Platform/external-repository authority.

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

A short invocation is only an alias for resolving the canonical prompt from live `main`; it is not permission to use a cached prompt body, bypass current repository instructions or activate a second control plane.
