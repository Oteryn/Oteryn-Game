> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #684 is merged on protected main as `c9778672da3c8060ec0f9a5821391aa30b4a6267`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

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
updated_at: 2026-09-19T14:45:00Z
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
Exact mapper blob: `e6d98aadd352ad36b466970e1f7182e1bf93643b`.
LF-canonicalized mapper SHA-256:
`bc68f0f63a5dd6ea5ee7a3c20b708d6ea78c6033a2f5dddb518c49d6f45f8666`.
Product digest: `56cef2d78442a37c00daa4cb737007e8a069e10ae3d4a298c0d38c38976f8289`.
Canonical evidence SHA-256:
`97fbfe027f93834bfaef365e4271dbb56b479ba29528e3f00a1b046aae0a7491`.

## Acceptance criteria

- [x] Every protected input is read at the exact admitted commit and pinned blob.
- [x] Blob and SHA-256 mismatches fail closed.
- [x] Repeat generation and manifest-case input permutation are byte-identical.
- [x] All four manifest cases retain `UNKNOWN / PENDING / PENDING` and parity pending.
- [x] No native Ability key is minted.
- [x] No quantitative formula, coefficient, RNG or rounding rule is invented.
- [x] Existing Effect families are referenced only as candidate classification.
- [x] Product closure is candidate-only with no runtime or production authority.
- [x] Published immutable head passes focused and reused extractor self-tests.
- [x] Published immutable head passes governance, diff and exact-path checks.
- [ ] Exact-head hosted repository CI and aggregate game-gate pass.

## Validation

Passed on API-published immutable head
`ce7a789710431da778bb7b5389357d317ac57d69`:

- `python -B tools/reference-world-corridor-census/ability_effect_formula_evidence_catalog_self_test.py`
  — PASS, including repeat/input permutation, protected blob/digest negatives,
  exact tracked evidence regeneration and exact mapper binding;
- `python -B tools/reference-world-corridor-census/self_test.py` — PASS;
- `python -B tools/reference-world-corridor-census/content_source_batch_self_test.py`
  — PASS;
- `python -B tools/agents/validate_governance.py` — PASS;
- `git diff --check` and exact four-path readback — PASS;
- hosted exact-head CI/game-gate — pending final PR head.

E2E is `NOT_APPLICABLE`: this batch has no executable gameplay/runtime path.

## Excluded scope

No writes to shared Content/Reference models, GAME-ABILITY/SIM runtime,
protocol/schema/stable-ID/resource registries, Cargo/workspace/lock,
persistence, workflows, external repositories or production. No Reference
parity claim, native identity promotion, executable formula or fixture PASS.

## Self-review

Whole-product review confirms that the mapper revision is bound to exact Git
blob and LF-canonicalized bytes without a self-SHA loop, and that the output contains no `oteryn:ability.*`
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
