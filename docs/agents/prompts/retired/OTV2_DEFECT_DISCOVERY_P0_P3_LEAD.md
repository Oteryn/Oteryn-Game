# OTV2 DEFECT DISCOVERY P0-P3 LEAD

Short invocation after canonical merge and an exact #162 allocation:

```text
Oteryn: defect discovery p0-p3
```

```yaml
prompt_id: OTV2_DEFECT_DISCOVERY_P0_P3_LEAD
prompt_version: "1.0"
prompt_mode: DEFECT_DISCOVERY_FIRST_PROOF_IMPLEMENTATION
repository: Oteryn/Oteryn-Game
short_invocation: "Oteryn: defect discovery p0-p3"
merge_authority: false
production_authority: false
cross_repository_write_authority: false
```

## Mission

Implement only P0-P3 from #162 comments `5684916133`, `5684920732`, and `5684926851`, using the canonical branch and exact writable paths granted by the live merged #162 allocation. Without that allocation, remain read-only and return `WAITING_ALLOCATION`.

Do not repair unrelated findings from product-owned paths; retain and hand them to their owning lane.

## Required outcomes

- P0: prove heavy discovery is isolated from ordinary PR/push/Merge Queue execution, supports manual exact-ref selection, and records exact tested SHA plus effective build/dependency identity.
- P1: qualify one small deterministic generative/state proof with Oteryn-specific invariants and repeatable retained evidence.
- P2: qualify one production parser/decoder exploration target with separate raw and structured input families while preserving normal validation semantics.
- P3: qualify one machine-readable finding/replay envelope that retains the original case, optional minimized case, tested identity, method/environment identity, expected invariant, observed divergence, result classification, artifacts and replay selector.

A seed is supporting metadata, not the sole retained reproducer. Missing required capability is explicit and never a silent PASS.

## Acceptance

The package must demonstrate a green bounded control, detection of a controlled test-only negative case, deterministic replay of the retained semantic case, and separation between product, harness, infrastructure, unsupported-capability and incomplete-campaign outcomes.

Preserve `game-gate`, Merge Queue, existing required tests and ADR-0007 authority. Broad real-PostgreSQL tooling, optional concurrency-tool adoption, mutation testing, native-client expansion and product fixes are later separately allocated work.

Return one exact-head qualification packet with P0/P1/P2/P3 disposition, remaining unknowns and exactly one next action.
