# OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence-504

```yaml
task_id: OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence-504
title: CW2-B4 Ability Effect Formula evidence catalogue
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-b4-ability-effect-formula-evidence-504
issue: 162
pr: null
base_sha: 03a821edd828e24ccff6e2cb7fc819a776cbd238
head_sha: null
owner: Oteryn: content world import
created_at: 2026-09-19T14:00:00Z
updated_at: 2026-09-19T14:00:00Z
owned_paths:
  - tools/reference-world-corridor-census/ability_effect_formula_evidence_catalog.py
  - tools/reference-world-corridor-census/ability_effect_formula_evidence_catalog_self_test.py
  - docs/agents/evidence/OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence.json
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw2-b4-ability-effect-formula-evidence-504.md
public_contracts: []
production_authority: NONE
closure: CANDIDATE_ONLY
```

## Outcome

Produce the smallest deterministic CW2-B4 candidate evidence batch for the
existing Light Healing and Ice Strike Reference source candidates. The mapper
binds each source Ability candidate to an already-protected qualitative Effect
family and records the exact quantitative Formula as unresolved.

This batch does not promote target evidence, provenance, legal review, parity,
native identity, implementation state or executable behavior.

## Protected inputs

The mapper reads objects from protected
`main@03a821edd828e24ccff6e2cb7fc819a776cbd238` and fails closed on any blob
mismatch:

- Reference evidence manifest blob `f5828732038ac2fd3ae03f3d1793505d48a61122`;
- first Ability evidence fixture package blob
  `5c9961a99616839a40b3ca933ac4371f93ffca48`;
- official spell-library research blob
  `6ada6c52a20beae37abfecb1e2792a36f5dba8ca`;
- continuity/provenance research blob
  `704e57840d0d3a1c84284e650c7c023d171d97dc`;
- Light Healing formula research blob
  `4a96e9859f6c425d3c03be09bc7712fc4cb6cf83`;
- current protected typed family surface blob
  `ec5fa303fa8a09f055c8043a560f9a6120dee6cb`.

The exact bytes, sizes and SHA-256 digests are retained in the generated
catalogue. Material is parsed as evidence data only.

## Binding disposition

The batch carries exactly two source Ability candidates and four already
registered Reference manifest cases:

- Light Healing / candidate incantation `exura` -> candidate `Heal` /
  `SELF_HEAL` shape -> quantitative formula `UNKNOWN`;
- Ice Strike / candidate incantation `exori frigo` -> candidate `Damage` /
  `TARGETED_ICE_DAMAGE` shape -> quantitative formula `UNKNOWN`.

For both candidates:

- target evidence remains `UNKNOWN`;
- source provenance and legal review remain `PENDING`;
- parity remains `PARITY_PENDING_EVIDENCE`;
- Oteryn implementation remains `NOT_STARTED`;
- native Ability identity remains `UNRESOLVED` with no `ContentKey`;
- exact quantitative function, coefficients, RNG and rounding remain absent;
- executable promotion is `BLOCKED`.

The protected Rust surface proves only that `Ability`, `Effect`, `Formula`,
`Damage` and `Heal` families already exist. It is classification input, not an
allocation to mutate the shared model and not evidence for target behavior.

## Deterministic product

- abilities: **2**;
- registered Reference cases: **4**;
- candidate qualitative effect bindings: **2**;
- resolved native Ability identities: **0**;
- resolved exact quantitative formulas: **0**;
- executable promotions: **0**;
- silently dropped records: **0**.

Mapper revision: `OTERYN_CW2_ABILITY_EFFECT_FORMULA_EVIDENCE_MAPPER/v1`.
Product digest: `e19bdf8eb3538685a4d806bdedd8a90e17127666fe22e8ea1785d5c194028e5e`.
Canonical evidence SHA-256:
`d86f5d5eb35343b834db987969624b1bdd18216cd5b12b8bfe3c5b4fc9f157bd`.

## Acceptance criteria

- [x] Every protected input is read at the exact admitted commit and pinned blob.
- [x] Blob and SHA-256 mismatches fail closed.
- [x] Repeat generation and manifest-case input permutation are byte-identical.
- [x] All four manifest cases retain `UNKNOWN / PENDING / PENDING` and parity pending.
- [x] No native Ability key is minted.
- [x] No quantitative formula, coefficient, RNG or rounding rule is invented.
- [x] Existing Effect families are referenced only as candidate classification.
- [x] Product closure is candidate-only with no runtime or production authority.
- [ ] Published immutable head passes focused and reused extractor self-tests.
- [ ] Published immutable head passes governance, diff and exact-path checks.
- [ ] Exact-head hosted repository CI and aggregate game-gate pass.

## Validation

Planned on the API-published immutable head:

- `python -B tools/reference-world-corridor-census/ability_effect_formula_evidence_catalog_self_test.py`;
- repeat and input-order determinism within that focused self-test;
- protected blob/digest fail-closed negatives within that focused self-test;
- relevant existing corridor census extractor self-tests;
- `python -B tools/agents/validate_governance.py`;
- `git diff --check` and exact four-path readback;
- hosted exact-head CI/game-gate.

E2E is `NOT_APPLICABLE`: this batch has no executable gameplay/runtime path.

## Excluded scope

No writes to shared Content/Reference models, GAME-ABILITY/SIM runtime,
protocol/schema/stable-ID/resource registries, Cargo/workspace/lock,
persistence, workflows, external repositories or production. No Reference
parity claim, native identity promotion, executable formula or fixture PASS.

## Self-review

Whole-product review confirms that the output contains no `oteryn:ability.*`
key, no non-null quantitative formula and no promoted target/provenance/parity
state. Open material findings before published-head qualification: **0**.

## Independent review

Required: `NO` for this bounded candidate-only evidence mapper. It changes no
shared model, protocol, durable data, production authority or native identity.
The worker does not trigger paid review; the control plane owns any later
review decision.

## Handoff

Parent control plane is the sole review and integration owner. This worker does
not merge, enqueue or trigger external review.
