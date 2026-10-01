# OTV2-20261001-imbuement-authoring

```yaml
task_id: OTV2-20261001-imbuement-authoring
title: Current imbuement data, schema and qualified public research
mode: AUDIT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/imbuement-authoring-draft-20261001
pr: 1438
head_sha: null
final_head_sha: null
owner: Codex root imbuement authoring lane
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/imbuement-authoring/**
  - docs/agents/tasks/archive/OTV2-20261001-imbuement-authoring.md
public_contracts:
  - IMBUE-FORGE-0
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

Delivered an authoring-only catalogue and schema with 24 families, 72 tier recipes,
72 material bindings, 72 scroll bindings, 629 current typed equipment profiles,
627 canonical bindings and two validated, unregistered Item proposals. The engine
matrix answers all 11 behavioral groups for Canary and both Crystal branches.
The public research addendum separates 23 bounded facts from genuine unknown
fields and runtime contracts. Full Global parity remains unproven.

## Architecture and source of truth

- Authority: IMBUE-FORGE-0 ([#1415](https://github.com/Oteryn/Oteryn-Game/pull/1415))
  and owner answers **I1a/I2a**, as explicitly routed by the control plane in
  [5936572587](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5936572587).
  This record adopts no runtime contract or authority beyond that authoring scope.
- D280 permits this existing content draft during close-out. The single task-record
  path is explicitly requested by control-plane
  [5937496910](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5937496910).
- Owner selects current Global research as of 2026-10-01, rather than July's snapshot.
  Historical captures remain provenance; Canary remains a research reference.
- Pinned Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`, Crystal imbuements
  `15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1`, Crystal summer-update
  `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.
- PROVEN: delivered identities, parsed source facts and offline validation results.
  DERIVED: explicitly bounded public-source interpretations. UNKNOWN: unresolved
  consumption, equipped-target, Premium-purchase, numeric transfer and fine Life
  fields. CONFLICT: qualified PZ/armor behavior and 101 equipment-source disputes.

## High-risk authority/recovery qualification

NOT_APPLICABLE: this draft does not perform production mutations, install runtime
controllers, authorize gameplay transactions or interpret recovery state.

## Acceptance criteria

- [x] Populate and validate the selected authoring catalogue and schema.
- [x] Preserve exact source identities, literal quotes, dates and access methods.
- [x] Compare pinned engines while distinguishing their behavior from Global proof.
- [x] Retain explicit unknowns/conflicts and assign runtime contracts through #162.
- [x] Provide this control-plane-requested task record in the draft's final authoring commit.

## Excluded scope

Runtime activation, canonical Item/Quest registration, architecture adoption,
persistence, combat execution, timers, protocol/UI and protected integration.
Remote Desktop was used exclusively to read public pages through Chrome/CDP.

## Implementation / findings

The original blanket requirement for gameplay recordings was too restrictive for
data research. Literal official/community statements now qualify their stated
fields. The 27 unperformed capture scenarios remain supplemental alternatives.
No consumed units or equipped-target permission were fabricated from OTS code.
See the authoring package's `global-research-closure.md` and `completion-handoff.md`.

## Validation

Before this single-record successor, head `1bef0e1b28566d9b4c1569a979ee69bae902c5f2`
passed 211 offline tests, nine replay/schema commands and all three XML reparses.
Both Item proposals had zero errors and warnings. Final successor validation,
source review and CI are rebound to its exact FREEZE_SHA in #162 and PR evidence;
this record does not inherit candidate-specific readiness from its predecessor.
Gameplay E2E is NOT_APPLICABLE to this data/schema/research draft.

## Self-review

Root reviewed the bounded authored delta, source qualifications, remaining nulls
and the sanctioned task-record path. Full Global parity is not claimed.

## Independent review

Required for final frozen head. Read-only independent review is dispatched;
the final verdict belongs to the exact-head FREEZE packet after publication.

## PR and closeout

- Canonical draft: [#1438](https://github.com/Oteryn/Oteryn-Game/pull/1438).
- Final head: recorded by FREEZE_SHA after this commit exists, avoiding self-reference.
- Merge commit/result: squash merge of #1438, only if subsequently authorized and integrated.
- Protected queue/review and ownership release remain with the control plane.
- This archive placement follows `archive/README.md`; it reaches main only if the PR merges.
