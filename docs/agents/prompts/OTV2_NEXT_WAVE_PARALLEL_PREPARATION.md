# OTV2 Next-Wave Parallel Preparation Launcher

Short invocation:

```text
Oteryn: next-wave prep swarm
```

## Purpose

Launch or coordinate the preparation wave defined by `docs/superpowers/plans/2026-08-24-oteryn-game-next-wave-master-plan.md` using isolated agents for Issues #93, #94, #95, #96 and #97.

This launcher is an orchestration aid only. It grants no write authority, creates no implementation allocation and does not allow one worker to edit another worker's owned paths.

## Mandatory startup

Before dispatch, read live `main`, root and nearest `AGENTS.md`, the master plan, `OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`, open PRs/tasks and Issues #93-#97. GitHub state outranks this prompt if anything has advanced.

Do not dispatch a lane that is already completed, superseded, allocated elsewhere or overlapping another active task. Use one Issue/task/branch/PR per substantial lane.

## Parallel preparation lanes

Dispatch one isolated agent per independent domain:

| Issue | Alias | Output | Implementation unlocked only after |
| --- | --- | --- | --- |
| #93 | `Oteryn: prep resource limits` | classified hard-max decision packet; accepted registry updates only through separately serialized coordinator mutation | applicable Ability/Interaction/AI limits accepted or excluded fail-closed; Movement dimensions must also close before Movement allocation |
| #94 | `Oteryn: prep durability topology` | exact first Durability topology, DB/migration technology decision and allocation proposal | topology accepted, exercised DUR-03 bounds closed and exact implementation allocation merged |
| #95 | `Oteryn: content format spike` | bounded representation evidence/dossier only | no implementation lane is automatically unlocked; permanent format remains owner-gated |
| #96 | `Oteryn: prep server seam` | exact production gameplay server-seam decision/allocation packet | accepted packet + exact implementation allocation; then `Oteryn: impl server seam` may run |
| #97 | `Oteryn: prep programme status` | verified status reconciliation only | does not gate unrelated path-disjoint implementation once current truth is preserved |

## Concurrency rules

The five preparation workers may run concurrently only when their exact owned paths are disjoint. Architecture contracts, resource registries, stable IDs, Cargo/workspace files, runtime composition, workflows and other shared policy paths remain serialized coordinator mutations.

A preparation worker must not consume unmerged sibling output. If one lane discovers a dependency on another lane, record the finding and let the coordinator order the merge/handoff; do not edit the other lane's files.

## Recommended dispatch

Start all preparation lanes that are still open and unowned in the same dispatch window. Treat each prompt as self-contained context; do not assume agents inherit this conversation.

```text
Agent 1 -> Oteryn: prep resource limits
Agent 2 -> Oteryn: prep durability topology
Agent 3 -> Oteryn: content format spike
Agent 4 -> Oteryn: prep server seam
Agent 5 -> Oteryn: prep programme status
```

## Handoff into implementation

Release implementation lanes independently as their own readiness gates close; do not wait for the whole preparation wave merely for symmetry.

- `Oteryn: impl ability`, `Oteryn: impl interaction`, `Oteryn: impl ai` may release after their applicable #93 bounds and exact allocations are merged.
- `Oteryn: impl durability` may release after #94 topology, applicable DUR-03 bound closure and exact allocation are merged.
- `Oteryn: impl server seam` may release after #96 decision/allocation is merged and its child Superpowers plan is created.
- Client remains blocked until Server Seam is merged and verified.
- Movement remains blocked until Interaction, Client, real QA readiness and all applicable Movement hard maxima are merged/accepted.
- Combat remains blocked until merged Movement plus Ability, Interaction, Durability, Client and QA prerequisites are integration-ready.

AI may continue in parallel and is not a hard dependency of the first Movement/Combat slice unless a reviewed concrete mechanic adds it.

## Canonical Codex review routing

Before any Codex/OpenAI/API review action, resolve protected-main `docs/agents/CODEX_REVIEW_POLICY.json` and `docs/agents/OWNER_FUNDED_AI_POLICY.md`.

- Review operations explicitly covered by `CODEX_REVIEW_POLICY.json` are standing-authorized. `owner_confirmation_per_covered_run: false` means this role MUST NOT ask the owner to approve each covered review invocation or use the owner as a prompt relay.
- Any owner-funded Codex/OpenAI/API use outside the exact covered review contract still requires explicit owner authorization for that invocation.
- Standing authorization grants no candidate ownership, write authority, control-plane authority, merge authority or production/live-state authority. Trigger Codex only when the live role/allocation is the canonical candidate/review-request owner under current policy; otherwise verify or route durable evidence to that owner.
- While this prompt is operating in read-only/preparation mode, it is not a candidate/review-request owner and must not trigger Codex. If later implementation is allocated, the canonical mutating owner/prompt for that candidate applies the review loop.
- A qualifying review requires successful exact-head evidence, zero unresolved P0/P1 findings, zero unresolved required review threads and no material head change after review. Green CI alone is not review.
- Codex remains strict read-only/non-mutating under the canonical policy. It may not implement fixes, mutate tracked/Git/persistent/external/live state, commit, push, merge, alter protections, access secrets or expand scope.

## Completion

Preparation-wave orchestration is complete when every still-required prep lane has either merged its bounded output and released ownership or is truthfully blocked on an explicit owner decision. Do not convert `UNKNOWN`, `CONFLICT` or evidence candidates into product policy merely to make the wave appear complete.

## Remote Desktop execution routing

Before any Remote Desktop/Desktop Commander use, resolve the current Game `AGENTS.md` and the canonical META execution-routing policy at `Oteryn/Oteryn@e002fc7532188e73a0f495da3e20710541ed50e0`. Out-of-band local connector/tool registration and argument-schema inspection is capability discovery; every direct `Remote_Desktop_Commander.*` invocation is exception-only and requires a fresh valid host-exception context plus a positive per-action decision for the exact semantic host action and exact connector tool immediately before the call.

`list_devices`, `who_am_i`, `ping`, `get_config`, filesystem/search/process/session/terminal/history operations and other direct connector calls are not capability-discovery exemptions. Unknown or undeclared tools fail closed, and a prior ALLOW never authorizes a different action or tool. This prompt cannot broaden META exception reasons or use Remote Desktop as a routine fallback for repository tests, Git inspection, CI/log polling or convenience. A Remote Desktop DENY is not automatically a blocker: continue through GitHub, GitHub Actions, repository-native connectors or an isolated workspace when they can perform useful authorized work.
