# OTV2-20260813-game-ability-reference-catalogue-parity-fixture-contract

```yaml
task_id: OTV2-20260813-game-ability-reference-catalogue-parity-fixture-contract
title: Define representative Reference Mechanic Catalogue and parity-fixture entry contract
mode: CONTRACT
status: ready
repository: blakinio/Oteryn-v2
base_branch: main
branch: null
head_sha: null
pr: null
claim_state: UNCLAIMED
handoff_delivery_pr: 248
owner: unassigned architecture continuation agent
implementation_status: NOT_STARTED
runtime_client_authority: NONE
postgresql_ddl_migration_authority: NONE
platform_write_authority: NONE
production_authority: NONE
```

## Durable handover

Verified `main` before handoff delivery: `199bb6a7559d610be9e01508dd1e5d192b5db820` (PR #247). PR #248 only delivers this checkpoint; the next task is intentionally unclaimed and must use a fresh branch after drift verification.

Accepted and lifecycle-closed `GAME-ABILITY-01` baselines:
- `GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md`
- `GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md`
- `GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md`
- `GAME-ABILITY-01_COOLDOWN_CHARGE_CONDITION_OWNER_BASELINE.md`
- `GAME-ABILITY-01_EFFECT_COMPOSITION_DAMAGE_HEAL_OWNER_BASELINE.md`
- `GAME-ABILITY-01_EFFECT_FAMILIES_REFERENCE_CATALOGUE_OWNER_BASELINE.md`

`GAME-ABILITY-01` remains `REQUIRED_FOR_ALPHA / OPEN`; runtime is unauthorized. Preserve typed-effect/domain ownership boundaries, `GAME-ITEM`/`DUR-03` value conservation, no generic state patch/event bus, no giant per-mechanic core enum, no hidden global transaction model, catalogue-local identity only, and no parity promotion from catalogue presence. `UNKNOWN`/`CONFLICT` remains fail-closed.

Before work, read live governance plus all six baselines, `REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md`, `ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md`, and applicable `GAME-ITEM`, `DUR-03`, `DUR-04`, `SIM-DETERMINISM-01`. PR #191 was separate stale-base GAME-CHAR provenance work at last verification and must not be silently absorbed.

After drift verification, the bounded programme is one representative Reference Mechanic Catalogue entry + parity-fixture binding contract: catalogue-local key, evidence references, behavior-affecting revision bindings, parity/unknown state without promotion-by-presence, target/cast/cost/cooldown/formula/effect/condition/RNG revision references, explicit missing/conflicting evidence, one non-factual example shape, preserved domain ownership, paper-only scope.

Governance: exact-head `Merge gate / validate`; no reuse of old-head CI; resolve all review threads; no owner-funded Codex/OpenAI invocation without explicit authorization.

## Context checkpoint

```yaml
status: ready
branch: null
head_sha: null
pr: null
claim_state: UNCLAIMED
handoff_delivery_pr: 248
owned_paths:
  - docs/agents/tasks/active/OTV2-20260813-game-ability-reference-catalogue-parity-fixture-contract.md
public_contracts:
  - six accepted GAME-ABILITY-01 owner baselines
  - docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md
last_progress: Sixth partial baseline delivered by PR #245 and lifecycle-closed by PR #247; handoff delivery is PR #248.
validation_state: PR #248 requires fresh exact-head validation after review repair.
audit_state: Repaired stale claimed-branch metadata, unsupported status vocabulary, and multi-action next_action; no architecture semantic change.
e2e_state: NOT_APPLICABLE
ci_generation: fresh PR #248 exact-head generation required
run_ids: []
counters: {}
blocker: null
next_action: Verify live main, open PRs and active tasks for drift against this checkpoint.
```
