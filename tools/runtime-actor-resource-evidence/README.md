# Runtime actor resource evidence

This directory contains the reproducible **non-production** evidence harness for Issue #530 and the protected allocation `OTV2-RUNTIME-ACTOR-CARRIER-RESOURCE-EVIDENCE-530`.

The harness does not implement `ChannelRuntime`, does not select a production actor container/ECS, does not mutate the resource registry, and does not choose a production actor-count maximum. It models only the minimum accepted actor-reference semantics needed to classify the resource dimensions discovered by #508/#530.

## Protected admission

- protected admission main: `951f98746e2fb8b93be0a4f04518b74e266df188`
- #162 activation comment: `5619094154`
- protected allocation blob: `0b81a498f1553e61fca89eebd85a23df080e428b`
- #532 merge-group gate: `34478991801` — SUCCESS

Every modeled Channel reference binds distinct `WorldId` and `ChannelId`, current `ScopeOwnershipGeneration`, actor-local semantic identity and actor-local generation. A stale scope generation, stale actor generation, different World, different Channel or untyped AI/client/protocol-like handle rejects.

## Candidate shapes

Two deliberately synthetic shapes are compared so the evidence does not smuggle one implementation into architecture:

1. **fused fixed bucket** — actor record, exact lookup location and generation/retirement state occupy one fixed table;
2. **split record + fixed index** — the actor record and lookup index have separately accountable retained storage; generation history remains separately accountable if the physical design retains it independently.

The model uses opaque deterministic `u64`-like local identities. They are not vector indexes, protocol handles or production IDs.

## Logical byte accounting

The reported byte values are explicit fixed-width **logical accounting**, not Python object size, process RSS, Rust ABI size or production memory benchmarks:

- actor reference: 40 bytes (`WorldId`, `ChannelId`, scope generation, local identity, actor generation);
- fused bucket: 64 bytes;
- split actor record: 56 bytes;
- split index entry: 24 bytes;
- split generation-history entry: 16 bytes.

Synthetic tested active ceilings `2, 3, 8, 64, 256` are test points only. None is a production maximum.

## Run

From this directory:

```bash
python self_test.py
python generate.py --check ../../docs/agents/evidence/OTV2-20260910-runtime-actor-carrier-resource-evidence.json
```

Expected focused results:

```text
PASS 14 tests
PASS evidence JSON matches deterministic generator
```

## Key evidence result

The harness demonstrates that actor-local generation/history cannot automatically be classified as the same resource as **active actor count**. With opaque unique local identities, active count can return to zero while retired identity/generation state continues consuming the fixed candidate table. Repeated reuse of the *same* identity can keep retained storage fixed, but a policy for retirement/reuse/history retention is required before production can claim a finite same-resource bound.

Therefore the evidence packet classifies:

- `RUNTIME-ACTOR-RL-01 = PERF_REFERENCE_CELL_REQUIRED`;
- `RUNTIME-ACTOR-RL-02 = MEASURED_CANDIDATE_EVIDENCE_AVAILABLE`;
- `RUNTIME-ACTOR-RL-03 = ARCHITECTURE_ESCALATION_REQUIRED`;
- `RUNTIME-ACTOR-RL-04 = NOT_EXERCISED_BY_FIRST_CARRIER` for variable candidate materialization;
- `RUNTIME-ACTOR-RL-05 = NOT_EXERCISED_BY_FIRST_CARRIER`.

ADR-0009 remains authoritative for production capacity: representative `PERF-01` evidence on a named reference cell is required before a total Channel actor/player capacity can be accepted. The harness deliberately does not convert successful M/M+1 tests or logical-byte equations into that production claim.
