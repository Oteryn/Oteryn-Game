# Oteryn v2 Terra + Sol Execution Scheduler

## Purpose

Provide one canonical launch sheet for the execution model defined by `2026-08-27-oteryn-game-terra-sol-parallel-execution-design.md`.

The scheduler is interpreted by `Oteryn: terra game coordinator`. It is a deterministic dependency/ownership map, not technical decision authority. Every invocation resolves live GitHub; the examples below are not cached state authority.

## Roles

| Alias | Requested profile | Default mode | Purpose |
| --- | --- | --- | --- |
| `Oteryn: terra game coordinator` | Work / Terra High | CONTROL_PLANE | GitHub state, DAG, ownership, leases, deterministic integration |
| `Oteryn: sol supervising architect` | GPT-5.6 Sol Extra High | ON_DEMAND | material architecture/cross-lane decisions |
| `Oteryn: work auditor` | independent high-effort read-only | READ_ONLY | forensic control/audit |
| `Oteryn: sol durability lead` | GPT-5.6 Sol Extra High | MUTATING when allocated | current Durability critical lane |
| `Oteryn: sol server seam lead` | GPT-5.6 Sol Extra High | READ_ONLY until Durability terminal | production server/client-entry seam |
| `Oteryn: sol client qa lead` | GPT-5.6 Sol Extra High | READ_ONLY until Server Seam terminal | native client + Tier 1/Tier 2 evidence |
| `Oteryn: sol movement lead` | GPT-5.6 Sol Extra High | READ_ONLY until Client/QA + #139 terminal | authoritative Movement |
| `Oteryn: sol combat lead` | GPT-5.6 Sol Extra High | READ_ONLY until Movement terminal | authoritative Combat/death/loot/XP/pickup |
| `Oteryn: sol post-vsl expansion` | GPT-5.6 Sol Extra High | READ_ONLY planning | decompose all remaining accepted Game work after VSL |

## Global concurrency

- Terra coordinator and read-only auditor do not consume implementation writer slots.
- Up to five Sol chats may be active when their responsibilities are distinct.
- Normally no more than two Sol leads may mutate the repository concurrently.
- A third mutating lead requires `PROVEN` disjoint primary paths, no shared-surface collision, and a recorded concrete throughput reason.
- Read-only preparation may continue while a dependency is blocked.
- Never launch a writer merely to occupy capacity.

## Shared serialization

The following remain one-writer-at-a-time:

- root/app Cargo manifests and `Cargo.lock`;
- workspace/architecture-check policy;
- server/client composition roots;
- stable protocol/event/resource registries and numeric IDs;
- shared ADRs/contracts consumed by concurrent lanes;
- workflows/protection/governance surfaces.

`SHARED_LEASE_REQUIRED` does not authorize the worker to edit the path. Terra executes only a pre-authorized deterministic shared turn; ambiguity escalates.

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
Movement resource gate #139
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

## Wave V0 — current transition

### Mutating candidate

`Oteryn: sol durability lead`

Release mutation only when the live Durability allocation/branch/PR remains valid. If an existing branch or draft PR exists, continue its history; do not restart due to upstream main movement alone.

### Parallel read-only preparation

- `Oteryn: sol server seam lead`
- `Oteryn: sol client qa lead`
- `Oteryn: work auditor`

Optional Movement read-only work should start only when it can materially prepare #139/current contracts without inventing numbers.

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
- independent audit/review preparation.

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

- Movement Lead freezes exact child slice and maps exercised #139 rows;
- Combat Lead may perform read-only dependency/test preparation only.

Promotion:

```text
compatible Client/QA terminal
+ required physical QA readiness PROVEN
+ exact Movement child slice frozen
  -> activate/close #139 through its decision/registry lifecycle
```

## Wave V3 — Movement

Mutation may begin only when:

```text
#139 terminal for every exercised Movement row
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
- auditor/reviewer.

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

## VSL closeout

Terra may classify VSL terminal only after:

- Server Seam + compatible Client terminal;
- applicable Tier 1/Tier 2 evidence truthful;
- Movement terminal;
- Combat terminal;
- required independent reviews terminal;
- all used tasks/PRs/branches/shared leases reconciled/released;
- no unresolved material architecture escalation;
- protected-main readback confirms claimed state.

This is not production/live deployment or full-game completion.

## Post-VSL expansion

After VSL terminal, launch:

```text
Oteryn: sol post-vsl expansion
```

It inventories all remaining accepted Game work and proposes exact next-wave lanes. Expected decomposition families may include World/Content, NPC/AI, Player Systems/Economy, Native Client/Renderer and Tooling/Operations, but current accepted architecture determines the actual split.

Terra does not create future technical lanes itself. It consumes the accepted expansion result and applies the same allocation/ownership/concurrency state machine recursively until all accepted Game programme lanes are terminal.

## Decision routing table

| Situation | Terra action |
| --- | --- |
| exact prerequisite missing | `WAITING_DEPENDENCY` |
| allocation missing | `WAITING_ALLOCATION` |
| bounded path-local technical judgment needed | `LANE_DECISION_REQUIRED` -> owning Sol lead |
| unowned shared path required | `SHARED_LEASE_REQUIRED` |
| public API/schema/persistence/trust/resource/cross-lane decision | `ARCHITECTURE_ESCALATION_REQUIRED` -> Sol Supervising Architect |
| product priority/scope/authority decision | `OWNER_DECISION_REQUIRED` |
| canonical rules conflict | `POLICY_CONFLICT` |
| external condition unchanged | `WAITING_EXTERNAL` |
| every integration predicate proven | mechanical expected-head integration permitted |

## Owner launch sheet

At every material transition, Terra should return only a compact sheet:

```text
CURRENT_MAIN: <sha>
ACTIVE_MUTATORS: <aliases>
READ_ONLY_PREPARATION: <aliases>
WAITING: <alias -> exact missing predicate>
SHARED_LEASE: <path/owner or none>
NEXT_UNLOCK: <terminal event -> alias>
ARCHITECTURE_ESCALATION: <issue/ref or none>
OWNER_DECISION: <precise question or none>
```

Do not make the owner reconstruct the DAG from chat history.