# Oteryn v2 Terra + Sol Execution Scheduler

## Purpose

Provide one canonical launch sheet for the execution model defined by `2026-08-27-oteryn-game-terra-sol-parallel-execution-design.md`.

The scheduler is interpreted by the programme's uniquely active control-plane profile. When Terra is selected, `Oteryn: terra game coordinator` applies it as a deterministic dependency/ownership map, not technical decision authority. Every invocation resolves live GitHub; the examples below are not cached state authority.

Owner-facing placement, model/effort selection and `DONE / ACTIVE / BLOCKED / READY_NEXT / DO_NOT_LAUNCH` reporting are standardized by `OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md`. That runbook is subordinate to this scheduler and live GitHub; it never activates a lane or transfers control-plane authority.

## Roles

| Alias | Requested profile | Default mode | Purpose |
| --- | --- | --- | --- |
| `Oteryn: terra game coordinator` | Work / Terra High | CONTROL_PLANE when durably selected, otherwise RECOVERY_READ_ONLY | GitHub state, DAG, ownership, leases, deterministic integration |
| `Oteryn: work coordinator` | ChatGPT Work / Terra High when selected for deterministic execution | CONTROL_PLANE when durably selected, otherwise RECOVERY_READ_ONLY | legacy/reusable Work delivery control plane |
| `Oteryn: sol supervising architect` | GPT-5.6 Sol Extra High | ON_DEMAND | material architecture/cross-lane decisions |
| `Oteryn: work auditor` | GPT-5.6 Sol highest available, independent | AUDIT_READ + EVIDENCE_WRITE | forensic audit plus bounded exact-target GitHub audit note |
| `Oteryn: owner execution guide` | GPT-5.6 Sol Extra High | READ_ONLY | owner-facing live launch/model/effort/status guidance; never a control plane |
| `Oteryn: sol durability lead` | GPT-5.6 Sol Extra High | MUTATING when allocated | current Durability critical lane |
| `Oteryn: sol server seam lead` | GPT-5.6 Sol Extra High | READ_ONLY until Durability terminal | production server/client-entry seam |
| `Oteryn: sol client qa lead` | GPT-5.6 Sol Extra High | READ_ONLY until Server Seam terminal | native client + Tier 1/Tier 2 evidence |
| `Oteryn: sol movement lead` | GPT-5.6 Sol Extra High | READ_ONLY until Client/QA + current Movement resource/dependency gate terminal | authoritative Movement |
| `Oteryn: sol combat lead` | GPT-5.6 Sol Extra High | READ_ONLY until Movement terminal | authoritative Combat/death/loot/XP/pickup |
| `Oteryn: sol post-vsl expansion` | GPT-5.6 Sol Extra High | READ_ONLY planning | decompose all remaining accepted Game work after VSL |

`Oteryn: owner execution guide` may inspect this scheduler, live tasks and exact PR evidence to tell the owner what to launch. It cannot allocate workers, trigger Codex, request Work Auditor evidence, mutate coordinator state, grant leases, merge or close anything.

## Control-plane activation

Exactly one mutating control-plane profile may own one programme lifecycle.

- If the current coordinator Issue/task contains `active_control_plane_profile`, use it exactly.
- For a legacy lifecycle without that field, the profile already named as canonical coordinator prompt/owner remains active; any other reusable control-plane profile is `RECOVERY_READ_ONLY`.
- A profile switch requires a durable docs/governance transition merged to protected `main` that updates the current coordinator Issue/task and releases the previous profile before the new one mutates.
- Alias invocation, model selection, chat instruction, `reusable` status or tool availability never transfers control-plane authority.
- If exactly one active profile cannot be proven, classify `POLICY_CONFLICT`; neither profile may allocate, grant shared leases, integrate/merge, mutate coordinator status or close out the programme.

Historical coordinator Issue numbers in this document are provenance only. Every invocation must resolve the current live coordinator lifecycle from GitHub before using a profile as mutating control plane.

## Global concurrency

- Exactly one control-plane profile may mutate the programme; the inactive Work/Terra profile is read-only recovery.
- The Work auditor's bounded PR/Issue COMMENT evidence writes do not consume implementation writer slots and do not make the auditor a mutating control plane.
- The owner execution guide is read-only and does not consume an implementation writer slot.
- Up to five Sol chats may be active when their responsibilities are distinct.
- Normally no more than two Sol leads may mutate the repository concurrently.
- A third mutating lead requires `PROVEN` disjoint primary paths, no shared-surface collision, and a recorded concrete throughput reason.
- Read-only preparation may continue while a dependency is blocked.
- Never launch a writer merely to occupy capacity.

Any canonical Oteryn Game agent or the owner may request `Oteryn: work auditor` against a uniquely identifiable PR/Issue/task/head. The auditor freezes exact target/head evidence, performs the audit independently, and persists exactly one non-dispositive GitHub audit note. If the target is ambiguous it returns `INSUFFICIENT_EVIDENCE`; if the head moves, the prior note remains historical and cannot qualify the new head.

## Shared serialization

The following remain one-writer-at-a-time:

- root/app Cargo manifests and `Cargo.lock`;
- workspace/architecture-check policy;
- server/client composition roots;
- stable protocol/event/resource registries and numeric IDs;
- shared ADRs/contracts consumed by concurrent lanes;
- workflows/protection/governance surfaces.

`SHARED_LEASE_REQUIRED` does not authorize the worker to edit the path. The active control plane executes only a pre-authorized deterministic shared turn; ambiguity escalates.

The Work auditor's evidence note is not a shared-surface lease and never authorizes tracked-file mutation.

## Current critical DAG

```text
Durability
  |
  v
Server Seam
  |
  v
Client + QA / Tier 1 + Tier 2
  |
  v
current Movement resource/dependency gate
  |
  v
Movement
  |
  v
Combat
  |
  v
VSL terminal closeout
  |
  v
Post-VSL expansion
```

The diagram is the dependency shape, not proof of current lane state. Recompute actual `DONE / ACTIVE / BLOCKED / READY_NEXT / DO_NOT_LAUNCH` from current protected-main/task/Issue/PR evidence before every launch.

## Wave V0 — current transition

### Mutating candidate

`Oteryn: sol durability lead`

Release mutation only when the live Durability allocation/branch/PR remains valid. If an existing branch or draft PR exists, continue its history; do not restart due to upstream main movement alone.

### Parallel non-implementation work

- `Oteryn: sol server seam lead` — read-only preparation;
- `Oteryn: sol client qa lead` — read-only preparation;
- `Oteryn: work auditor` — audit read + bounded GitHub evidence-write when requested.

Optional Movement read-only work should start only when it can materially prepare the current Movement resource/dependency gate and contracts without inventing numbers.

### Promotion

```text
Durability terminal merge + fresh protected-main readback
  -> Server Seam may become READY_TO_IMPLEMENT only after exact merged allocation
```

## Wave V1 — Server Seam

Primary mutating lane:

```text
Oteryn: sol server seam lead
```

Parallel:

- Client/QA read-only preparation;
- Movement read-only resource/dependency inventory if useful;
- independent audit/review preparation and bounded audit evidence note when requested.

Promotion:

```text
Server Seam terminal merge
+ applicable physical Tier 1 evidence state known
  -> Client/QA may become READY_TO_IMPLEMENT after exact allocation
```

## Wave V2 — Client/QA

Primary mutating lane:

```text
Oteryn: sol client qa lead
```

Required outcome includes truthful native-client Tier 2 evidence for supported journeys. Synthetic/direct-domain success never substitutes for physical evidence.

Parallel:

- Movement Lead freezes exact child slice and maps exercised resource rows;
- Combat Lead may perform read-only dependency/test preparation only;
- Work auditor may audit an exact requested target and persist its non-dispositive evidence note.

Promotion:

```text
compatible Client/QA terminal
+ required physical QA readiness PROVEN
+ exact Movement child slice frozen
  -> close the current Movement resource/dependency gate through its canonical decision/registry lifecycle
```

## Wave V3 — Movement

Mutation may begin only when:

```text
current Movement resource/dependency gate terminal for every exercised row
AND Interaction/current prerequisites terminal
AND compatible Client/QA terminal
AND exact Movement allocation merged
```

Primary mutating lane:

```text
Oteryn: sol movement lead
```

Parallel:

- Combat read-only preparation;
- auditor/reviewer; auditor evidence writes remain comment-only and non-dispositive.

Promotion:

```text
Movement terminal merge + physical evidence
  -> recompute Combat readiness from fresh main
```

## Wave V4 — Combat

Mutation may begin only when current live state proves:

- merged Movement;
- Ability ready for the exact slice;
- Interaction ready;
- Durability ready;
- Client/QA ready;
- current resource/value/item semantics sufficient;
- exact Combat allocation merged.

Primary mutating lane:

```text
Oteryn: sol combat lead
```

Any material durable loot/value/item/resource gap is architecture escalation, not an implementation shortcut.

## Autonomous Codex review gate

For every allocated mutating lane PR, the owning Sol lane lead applies protected-main `docs/agents/CODEX_REVIEW_POLICY.json` before `READY_FOR_INTEGRATION`.

When `CODEX_REQUIRED`, the owning lane lead performs the covered `freeze exact head -> @codex review -> repair within allocation -> exact-head revalidation -> fresh review after head movement` loop. The owner is not the default prompt relay. Work/Terra mechanically verify classification/evidence and return stale/failed review state to the lane as `REVIEW_RECONCILIATION_REQUIRED`; they do not trigger the lane's Codex review or adjudicate technical findings.

Every mutating Sol return packet should include the standardized `codex_review` block defined by `OTERYN_GAME_AGENT_OPERATOR_RUNBOOK.md` and the lane prompt.

## VSL closeout

The active control plane may classify VSL terminal only after:

- Server Seam + compatible Client terminal;
- applicable Tier 1/Tier 2 evidence truthful;
- Movement terminal;
- Combat terminal;
- required independent reviews terminal;
- all used tasks/PRs/branches/shared leases reconciled/released;
- no unresolved material architecture escalation;
- protected-main readback confirms claimed state.

A Work auditor note may satisfy an audit-evidence requirement only when the exact target/head, independence requirement and applicable repository policy are all proven. The note itself never performs closeout or merge.

This is not production/live deployment or full-game completion.

## Post-VSL expansion

After VSL terminal, launch:

```text
Oteryn: sol post-vsl expansion
```

It inventories all remaining accepted Game work and proposes exact next-wave lanes. Expected decomposition families may include World/Content, NPC/AI, Player Systems/Economy, Native Client/Renderer and Tooling/Operations, but current accepted architecture determines the actual split.

After VSL terminal the explicit future-wave preparation profiles may run read-only in parallel when useful:

```text
Oteryn: sol world content prep
Oteryn: sol npc ai prep
Oteryn: sol systems economy prep
Oteryn: sol tooling ops prep
```

Each may only return a `READY_FOR_ALLOCATION_PROPOSAL` packet; none may mutate, allocate itself, claim a lease, integrate or close out a lane before a later exact merged allocation.

Terra does not create future technical lanes itself. When Terra is the active control plane, it consumes the accepted expansion result and applies the same allocation/ownership/concurrency state machine recursively until all accepted Game programme lanes are terminal.

## Decision routing table

| Situation | Active control-plane action |
| --- | --- |
| active control-plane profile ambiguous | `POLICY_CONFLICT` |
| exact prerequisite missing | `WAITING_DEPENDENCY` |
| allocation missing | `WAITING_ALLOCATION` |
| bounded path-local technical judgment needed | `LANE_DECISION_REQUIRED` -> owning Sol lead |
| independent verification requested for exact target | `Oteryn: work auditor` -> persisted exact-target audit note |
| unowned shared path required | `SHARED_LEASE_REQUIRED` |
| public API/schema/persistence/trust/resource/cross-lane decision | `ARCHITECTURE_ESCALATION_REQUIRED` -> Sol Supervising Architect |
| product priority/scope/authority decision | `OWNER_DECISION_REQUIRED` |
| canonical rules conflict | `POLICY_CONFLICT` |
| external condition unchanged | `WAITING_EXTERNAL` |
| every integration predicate proven | mechanical expected-head integration permitted |

## Owner launch sheet

`Oteryn: owner execution guide` may synthesize this sheet read-only from fresh GitHub. The active control plane should also return the compact execution state at every material transition with exactly one next action:

```text
CURRENT_MAIN: <sha>
CONTROL_PLANE_PROFILE: <exact active profile>
ACTIVE_MUTATORS: <aliases>
READ_ONLY_PREPARATION: <aliases>
CODEX_REVIEW: <lane/pr/head/route/evidence or none>
AUDIT_EVIDENCE: <auditor target/note or none>
WAITING: <alias -> exact missing predicate>
SHARED_LEASE: <path/owner or none>
NEXT_UNLOCK: <terminal event -> alias>
ARCHITECTURE_ESCALATION: <issue/ref or none>
OWNER_DECISION: <precise question or none>
NEXT_ACTION: <exactly one deterministic action>
```

Do not make the owner reconstruct the DAG from chat history and do not give a control plane a menu of technical choices.
