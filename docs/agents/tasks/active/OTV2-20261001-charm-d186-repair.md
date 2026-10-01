# OTV2-20261001-charm-d186-repair

```yaml
task_id: OTV2-20261001-charm-d186-repair
title: Repair D186 Charm source eligibility and hand off owning integration work
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-d186-repair-20261001
pr: null
issue: 162
base_sha: aad17f99d86abea01bd50b9d559e69eaa5549d88
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher of this owner-directed bounded repair
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/combat/charm_effects.rs
  - apps/game-server/src/combat/charm_effects_tests.rs
  - docs/agents/evidence/OTV2-20261001-charm-d186-repair-integration-ownership.md
  - docs/agents/tasks/active/OTV2-20261001-charm-d186-repair.md
  - docs/agents/tasks/archive/OTV2-20261001-charm-d186-repair.md
public_contracts: []
depends_on: [CHARM-0, CHARM-4, D186]
blocks: []
external_repositories: []
cross_repository_coordination_id: null
```

## Outcome and authority

The owner explicitly directed this session to update #162 with coordinator/architect work and
finish the worker's Charm work. The root is the sole author on this new isolated branch;
six existing subagents provide read-only source/contract checks and independent review.
The live scope packet is #162 comment5936434510. This owner-directed repair does not lift
the programme-wide D252 allocation freeze, take over STATE or allocate other owners' children.

The repair supplies required source provenance on the critical calculation event and
effect-specific secondary-target eligibility for both leeches. Critical/leech stay dormant;
owning consumers deliver actual activation in their allocated children.

## Architecture and source of truth

- PROVEN: D186, accepted owner decision batch D174–D235 row24, covers all four critical/leech
  exceptions and forbids generated Charm damage chaining.
- PROVEN: architect5936017719 assigns16remaining effects to their owning lanes; critical/leech
  are one GAME-COMBAT formula child. D2795936039060 allows partial release after CHARM-6.
- PROVEN: the root's main audit and independent review found the source eligibility defect;
  five domain followups found no other confirmed small pure-evaluator defect.
- Bound META3.1.0: `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- Detailed source classification, ownership and acceptance are in the handoff evidence file.

## High-risk authority/recovery qualification

NOT_APPLICABLE: pure evaluator source eligibility and test/evidence changes. No production
mutation, current session/lease authorization, persisted recovery interpretation, PREPARE,
COMMIT, protocol, identity, migration or entitlement source is added. Explicit source facts
are required rather than silently reconstructed. Existing race/state validation stays intact.

## Acceptance criteria

- [ ] Both leeches evaluate on primary and secondary auto targets at every stage, lethal and
  nonlethal, and remain fail-closed until their owning runtime exists.
- [ ] Both critical bonuses require an explicit source and reject CharmDamage.
- [ ] Secondary targets exclude every non-exception hit/kill effect, including mixed assignments.
- [ ] Existing25shape/fail-closed matrix, caps, stage values, race isolation and RNG replay pass.
- [ ] Existing speed-condition inputs are consistent without activating an effect.
- [ ] Required narrow Rust and governance checks pass; coordinator receives exact-head review packet.

## Excluded scope

Production combat/client composition, condition store/timers, critical/leech formula activation,
loot/skinning/death/fleeing consumers, Character/Platform entitlement, migrations, wire/capability,
unassign/reset and protected integration remain their owning children. No speculative system,
formula or parallel writer is introduced. Draft preparation #1434 remains unchanged.

## Implementation and validation

- Focused RED: new secondary-leech regression is being executed against the pre-repair engine.
- GREEN/component: pending repair and required game-server fmt/clippy/test checks.
- Governance: pending validator,36tests and whitespace/owned-path review.
- DB/client/gameplay: integration consumers are outside this pure-evaluator repair; no execution
  claim from skipped PostgreSQL tests or catalogues.
- Self-review and independent exact-head review: pending final candidate.
- Publication: normal atomic `createCommitOnBranch(expectedHeadOid=...)` route; no local material
  commit selected, no ref reconstruction, no credentials printed or persisted.
- CI/review dispatch/MQ: the active programme control plane owns qualification and integration.

## Context checkpoint

```yaml
last_progress: posted coordinator/architect packet and started focused regression
status: implementing
branch: codex/charm-d186-repair-20261001
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
owner_action_required: null
blocker: null
next_action: confirm focused RED, repair source eligibility and run GREEN
```
