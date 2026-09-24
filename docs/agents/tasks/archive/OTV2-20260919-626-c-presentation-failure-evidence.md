> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #672 merged as `eb122df0a94e1b461c882ae66a394058c595711c`, and the canonical task branch is deleted. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-626-c-presentation-failure-evidence

```yaml
task_id: OTV2-20260919-626-c-presentation-failure-evidence
title: Repair Tier2/Tier3 ClientPresentation failure evidence
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/626-c-presentation-failure-evidence
issue: 626
pr: null
base_sha: 715a22f26f6ec5472597f63cf5d6b939d7583cc1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: impl qa"
created_at: 2026-09-19T14:49:00+02:00
updated_at: 2026-09-19T14:49:00+02:00
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/tests/support/evidence.rs
  - apps/game-server/tests/evidence_shell.rs
  - docs/agents/tasks/active/OTV2-20260919-626-c-presentation-failure-evidence.md
public_contracts: []
depends_on:
  - "#162 comment 5741382269"
  - "#626 Finding C"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allow complete Tier2/Tier3 attempts whose first product divergence is
`ClientPresentation` to remain valid failure evidence, without weakening
successful-attempt, Tier1 headless, cleanup, phase-order, identity, or boundary checks.

## Authority and source of truth

- **FACT** — #162 comment `5741382269` is the protected allocation authority.
- **FACT** — admission branch/head is
  `agent/626-c-presentation-failure-evidence@715a22f26f6ec5472597f63cf5d6b939d7583cc1`.
- **FACT** — Finding C in #626 proves `validate_phases()` rejected
  Tier2/Tier3 `ClientPresentation = Failed` before existing divergence/outcome validation.
- **FACT** — current protected `main` advanced to
  `ebc860d7cd12bb855228a48759c4cc37b828da63` only through path-disjoint CW3-B1.
- **FACT** — no open PR changed after the allocation preflight, and PR #671 changed
  only Content/World paths, so the three-file custody remains collision-free.

## Acceptance criteria

- [x] RED proves a Tier2/Tier3 product failure at `ClientPresentation` is rejected on admission code.
- [x] Tier2 product failure first diverging at `ClientPresentation` validates.
- [x] Tier3 product failure first diverging at `ClientPresentation` validates.
- [x] Passed Tier2/Tier3 attempts with failed presentation remain invalid.
- [x] Tier2/Tier3 `NotApplicable` presentation remains invalid.
- [x] Tier1 keeps non-empty `NotApplicable` presentation semantics.
- [x] Existing first-divergence and cleanup negatives remain green.

## Excluded scope

No production runtime mutation, workflow/CI routing change, evidence-tier weakening,
Cargo/workspace change, protocol/schema/registry change, external repository write,
protected-environment mutation, direct merge, or expansion beyond the three owned paths.

## Implementation

The repair changes only the Tier2/Tier3 presentation precondition in
`validate_phases()`:

- `NotApplicable` remains rejected;
- `AttemptOutcome::Passed` still requires `ClientPresentation = Passed`;
- `ClientPresentation = Failed` is permitted to reach the existing earliest-failure,
  `first_divergence`, failure-class, and outcome-consistency checks.

No new evidence status, error class, phase, or authority model is introduced.

## Validation

### RED

Command:

`cargo +1.94.0 test --locked -p oteryn-game-server --test evidence_shell`

Result before the implementation repair: **FAILED**, 19 passed / 1 failed.
The new positive regression failed with
`Tier2NativeClient: Err(EvidenceIncomplete)`, reproducing Finding C exactly.

### GREEN / repository gates

- `cargo +1.94.0 test --locked -p oteryn-game-server --test evidence_shell` — **PASS**, 20 passed / 0 failed.
- `cargo +1.94.0 fmt --all --check` — **PASS**.
- `cargo +1.94.0 clippy --locked -p oteryn-game-server --all-targets -- -D warnings` — **PASS**.
- `python tools/agents/validate_governance.py` — **PASS**, 26 required policy documents and 9 project lanes.
- `git diff --check` — **PASS**.
- Applicable focused-support census — **PASS**: `evidence_shell.rs` is the only test consumer of this helper.

Exact staged changed-path readback is **PASS**: only the three allocated custody paths are present. Whole-diff adversarial self-review is `PASS_ZERO_MATERIAL_FINDINGS`. Hosted exact-head CI remains pending until the immutable PR head exists.

## Review and integration

- implementing-worker self-review: `PASS_ZERO_MATERIAL_FINDINGS` on the complete staged three-path diff; final immutable-head readback remains external to avoid tracked self-reference;
- independent second reviewer: not required by the allocation unless live policy/risk
  classification or a material finding requires one;
- protected integration authority: none for this worker;
- direct merge: forbidden;
- final handoff target: `READY_FOR_INTEGRATION` only after exact-head qualification.

## Context checkpoint

```yaml
last_progress: RED 19/1 reproduced; GREEN 20/0 plus fmt, strict Clippy, governance and diff checks pass
status: validating
branch: agent/626-c-presentation-failure-evidence
admission_head: 715a22f26f6ec5472597f63cf5d6b939d7583cc1
current_local_head: 715a22f26f6ec5472597f63cf5d6b939d7583cc1
pr: null
blocker: null
next_action: commit/push immutable candidate, open PR, verify exact-head hosted CI
```
