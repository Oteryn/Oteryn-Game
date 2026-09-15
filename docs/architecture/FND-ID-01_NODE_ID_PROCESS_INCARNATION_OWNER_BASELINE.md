# FND-ID-01 NodeId Process-Incarnation Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: GameNode identity, runtime registration, channel assignment, recovery, fencing, observability and later runtime/operations contracts

## Purpose

Record the project owner's accepted lifecycle and representation semantics for canonical `NodeId` before the complete `FND-ID-01` identifier catalogue is finalized.

This baseline refines the existing ADR-0001/ADR-0009 GameNode model and UUIDv7 durable-identity direction. It does not implement GameNode runtime, registration, orchestration, protocol, persistence or production behavior.

## Owner-accepted decision

Canonical `NodeId` identifies exactly one concrete **GameNode process incarnation**.

The target representation is:

```text
NodeId = strongly typed UUIDv7, full 128 bits
```

Every new GameNode process incarnation receives a new `NodeId`.

Therefore:

```text
process starts        -> NodeId A
same process runs     -> NodeId A
process terminates    -> NodeId A never represents a later process
process restarts      -> NodeId B
replacement process   -> NodeId C
```

A restart that happens on the same host, VM, container image, deployment target or orchestrator node still creates a new process incarnation and therefore a new `NodeId`.

## Canonical terminology

ADR-0001 uses `NodeId` as the logical identity of one GameNode process runtime. This remains the canonical project term.

Some earlier candidate material uses `GameNodeId`. For the current architecture:

- `NodeId` is the canonical semantic type name;
- `GameNodeId` must not silently become a second identity for the same concept;
- a future explicit terminology migration may rename the type, but must preserve the same semantics and migration rules.

This avoids parallel aliases that could be confused in protocol, persistence or observability contracts.

## NodeId is not infrastructure identity

`NodeId` must not identify or be derived from:

- physical host;
- VM;
- Kubernetes node;
- pod;
- Docker container ID;
- systemd unit instance;
- replica ordinal;
- deployment name;
- autoscaling-group member;
- stable placement slot;
- availability-zone/rack identity.

These are infrastructure or placement concepts with different lifecycles.

If Oteryn later requires a stable identity for a deployment slot, logical placement target or host-like resource, it must use a **separate strongly typed identifier**. Candidate names such as `PlacementId`, `DeploymentTargetId` or `NodeSlotId` are descriptive only and are not fixed by this baseline.

The exact semantic type, owner, issuer and lifecycle for any such stable placement identity remain future operations/topology contract work.

## Relationship to WorldId and ChannelId

`NodeId` is execution identity, not world/channel topology identity.

Canonical channel identity remains:

```text
WorldId + ChannelId
```

A channel may move from one GameNode process incarnation to another without changing its semantic identity:

```text
WorldId W + ChannelId C + NodeId A
    -> failure/recovery/relocation
WorldId W + ChannelId C + NodeId B
```

The change from `NodeId A` to `NodeId B` says that execution moved to a new process incarnation. It does not mint a new world or channel.

A single NodeId may host zero or more ChannelRuntimes during its own process lifetime according to ADR-0009 capacity and placement policy.

## Relationship to ownership generation and fencing

`NodeId` and ownership fencing solve different problems.

`NodeId` answers:

> Which GameNode process incarnation is this?

An ownership generation/fence answers:

> Which incarnation is currently authorized to mutate a particular authoritative scope?

Therefore:

- a new `NodeId` does not automatically acquire authority over any channel;
- knowing a valid `NodeId` is not proof of current ownership;
- a ChannelRuntime assignment must still include the applicable world/channel identity, revision constraints and current generation/fence;
- a stale process with an old `NodeId` must be rejectable even if it once legitimately owned the channel;
- a replacement process with a new `NodeId` must not mutate the channel until current assignment/fencing authority is established;
- recovery may preserve `WorldId + ChannelId` while changing both `NodeId` and the applicable ownership generation.

This preserves the single-authoritative-writer model without overloading process identity into a fencing token.

## Lifecycle semantics

Accepted rules:

- `NodeId` is immutable for one process incarnation;
- a process restart always creates a new `NodeId`;
- a crashed process's `NodeId` is never reused for the replacement process;
- orderly restart also creates a new `NodeId`;
- rolling deploy creates new NodeIds as new process incarnations start;
- scaling out creates one distinct NodeId per GameNode process incarnation;
- scaling in retires those process incarnations; their NodeIds remain historical identities and are not reassigned;
- nil/zero UUID is invalid;
- absence is explicit;
- duplicate/collision handling fails closed and never aliases two process incarnations;
- UUIDv7 timestamp ordering is not used as authority, freshness, heartbeat precedence, fencing or causal ordering.

A restart does not need to preserve the previous process identity because the purpose of `NodeId` is precisely to distinguish incarnations.

## Representation

`NodeId` uses strongly typed UUIDv7 and preserves all 128 bits at canonical boundaries.

Conceptually:

```text
NodeId(UUIDv7)
```

It must not be interchangeable with:

```text
WorldId
ChannelId
InstanceId
CharacterId
SessionId
host id
pod id
container id
placement id
```

Frequent internal runtime structures may use local handles where appropriate, but logs, registration state, assignment state, audit evidence and durable cross-boundary references that need process-incarnation identity retain canonical `NodeId` semantics.

Exact wire byte order/text rendering remains `FND-02` where transmitted over gameplay/control-plane protocol boundaries. Exact storage/index choices remain durability/operations work.

## Generation and registration boundary

This owner decision fixes the identity semantics and lifecycle but **does not yet freeze the exact generator/registration handshake**.

The complete contracts must later determine:

- which logical component generates the UUID bytes;
- when generation occurs during process bootstrap;
- whether generation is local to the GameNode process or supplied by a registration authority;
- how Platform/runtime/orchestrator registries authenticate and register the incarnation;
- how duplicate/stale registration is rejected;
- how heartbeat and liveness state are associated with `NodeId`;
- how process identity is attested to channel assignments;
- failure behavior when registration authority is unavailable.

Mandatory constraint: regardless of physical generation location, a fresh GameNode process incarnation must receive a fresh canonical `NodeId`, and registration does not make infrastructure identity equivalent to `NodeId`.

This deliberate deferral avoids treating implementation placement of UUID generation as already accepted architectural authority.

## Observability and recovery consequences

The process-incarnation model gives operations and recovery unambiguous evidence.

A diagnostic record can distinguish:

```text
NodeId A hosted Channel W/C under generation 41
NodeId A stopped or lost liveness
NodeId B later hosted Channel W/C under generation 42
```

This improves:

- stale heartbeat detection;
- split-brain investigation;
- channel recovery timelines;
- assignment history;
- audit correlation;
- crash-loop analysis;
- rolling-deploy visibility;
- capacity attribution to one process incarnation.

It also prevents metrics/logs from accidentally merging a restarted process into the historical identity of its predecessor merely because they share a host or deployment label.

## Security and authority

`NodeId` is an identifier, not a credential or capability.

Possession or knowledge of a NodeId grants no right to:

- register a GameNode;
- receive channel assignments;
- mutate channel state;
- publish readiness;
- issue sessions;
- write persistence;
- impersonate an old/new GameNode process.

Later control-plane contracts must authenticate the process/service identity and bind allowed operations to current assignment/fencing state independently from the NodeId value.

## Rejected alternatives

### Stable NodeId across process restarts

Rejected because it would conflate a durable deployment/placement concept with a concrete process incarnation. A restarted process could then be confused with its predecessor in heartbeat, recovery and fencing evidence.

### Use host/VM/pod/container ID as NodeId

Rejected because infrastructure identity has a different lifecycle, technology dependency and ownership model. GameNode architecture must remain portable across deployment technologies.

### Make NodeId part of ChannelId identity

Rejected because channels survive GameNode replacement/relocation. Runtime placement must not redefine topology identity.

### Use ownership generation instead of NodeId

Rejected because the values answer different questions. Generation establishes current authority for a scoped resource; NodeId identifies a process incarnation across assignments, diagnostics and lifecycle evidence.

### Reuse NodeId after clean shutdown

Rejected because the later process is still a distinct process incarnation. Reuse would lose the exact identity boundary the type exists to provide.

## Required application to later contracts

This baseline is mandatory input to:

- complete `FND-ID-01` identifier catalogue and owner/issuer/lifecycle matrix;
- `FND-03` GameNode bootstrap, lifecycle, assignment, recovery and fencing;
- `FND-04` where admission/session state refers to current game-server placement or assignment evidence;
- durability/audit contracts where process-incarnation provenance matters;
- operations/orchestrator contracts for registration, heartbeat, capacity, rolling deploy and recovery;
- E2E/fault tests proving restart generates a new NodeId while channel identity remains stable;
- observability schemas that correlate one GameNode process incarnation without leaking authority.

## Programme effect

- canonical project term: `NodeId`;
- representation: strongly typed UUIDv7, full 128 bits;
- semantic lifecycle: one `NodeId` per GameNode process incarnation;
- every process restart/replacement receives a new `NodeId`;
- stable host/VM/pod/deployment-slot identity is a separate concept and cannot be overloaded into `NodeId`;
- `NodeId` is not part of semantic `WorldId + ChannelId` identity;
- process identity is not mutation authority; current assignment/generation/fencing remains required;
- exact UUID generator/registration handshake and stable placement-ID contract remain deliberately unresolved;
- no runtime/protocol/orchestrator/persistence implementation is authorized by this baseline.
