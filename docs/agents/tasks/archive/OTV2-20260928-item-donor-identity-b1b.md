# OTV2-20260928-item-donor-identity-b1b

```yaml
task_id: OTV2-20260928-item-donor-identity-b1b
title: "B1b donor Item identity epoch 2 (alias gate, function, bindings)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/item-donor-identity-b1b
issue: 162
pr: 1179
base_sha: be242b8
head_sha: dd4e6e85bffae776fd2f63d647a718195e3969c7
final_head_sha: dd4e6e85bffae776fd2f63d647a718195e3969c7
final_head_frozen_at: 2026-09-28
owner: Content/World lane single writer
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/cw2_b1_import.rs   # additive only
  - imports/crystalserver/bindings/items.json
  - tools/content-census/g4_item_crystal_binding_generator.py
  - tools/content-census/g4_item_crystal_binding_generator_self_test.py
  - docs/agents/evidence/OTV2-20260928-item-donor-identity-b1b-alias-crosswalk.json
  - docs/agents/tasks/active/OTV2-20260928-item-donor-identity-b1b.md
  - docs/agents/tasks/archive/OTV2-20260928-a8-donor-item-identity-epoch-decision.md
public_contracts: []
depends_on:
  - "docs/architecture/reviews/OTERYN_GAME_A8_DONOR_ITEM_IDENTITY_EPOCH_DECISION_2026-09-28.md (#1163, 9f98b067)"
blocks:
  - "B2 and B3 donor wiki evidence and facts"
  - "WO-2 family keys for the 138 routed donor ids"
cross_repository_coordination_id: null
external_repositories:
  - "zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f (pinned facts, read-only)"
```

## Outcome

Decision `A8-DONOR-ITEM-IDENTITY-EPOCH-V1` (owner decisions D96 and D97) is implemented as an additive epoch 2.

- **Alias gate.** Every one of the 412 donor ids has a crosswalk state in the pinned evidence
  `docs/agents/evidence/OTV2-20260928-item-donor-identity-b1b-alias-crosswalk.json`. The gate is `A8-ALIAS-GATE-V1`:
  - Names only discover candidates (G4 rule 9).
  - Identity signals are non-presentation only: article/plural and the complete `items.xml` attribute set. A sprite or
    appearance change never remints an Item identity (G4 decision, identity layers), so the appearance sprite signature
    is presentation: it can corroborate an alias, and a difference never proves a distinct identity.
  - A candidate agreeing on both identity signals is a counterpart. A unique counterpart with the same sprite is
    `ACCEPTED_ALIAS`; with a different or absent sprite it is a held `PROBABLE_MATCH`; several counterparts are
    `AMBIGUOUS`. Held ids get no key and no binding until non-presentation evidence resolves them.
  - Only no same-name item, or every same-name item contradicted by non-presentation facts, leaves `NO_MATCH`.
- **Result.** 16 donor ids share a name with an existing base Item. 8 of them agree on article/plural and the full
  attribute set and differ at most in sprite, so they are held: `PROBABLE_MATCH` 35500 (against 35502) and 54610
  (against 34017); `AMBIGUOUS` 53380, 54609, 54613, 54614, 54615 and 54616 (several same-fact base items). The other 8
  (cookbook, two stone stairs, five pedestals) are contradicted by attributes and stay `NO_MATCH`. No id outside the 16
  is affected.

  | State | Ids |
  | --- | --- |
  | `NO_MATCH` (minted) | 404 |
  | `ACCEPTED_ALIAS` and `EXACT` | 0 |
  | Held: `PROBABLE_MATCH` 2, `AMBIGUOUS` 6, `CONFLICT` 0 | 8 |
  | Epoch-1 bindings, unchanged | 38,157 |

- **Allocation.** The decision derives the range (rank among the minted ids after 38,093), so 404 `NO_MATCH` ids get
  `oteryn:item.registry.i00038094` to `i00038497` in ascending donor source id. Their allocation digest is
  `c9bd33992d40c0ac36405b3a0b429485905add8f54460cccec9e1e7790569d3c`, built as the frozen import builds its own.
- **Function.** `protected_cw2_b1_donor_identity_epoch_2_import` sits beside the frozen import, with its own
  `CW2_B1_DONOR_EPOCH2_*` constants. It returns 404 Item pointers with `materializable: false`, stack class `Unknown` and
  authorship `OTERYN_OPAQUE_REGISTRY_ALLOCATION_EPOCH_2`, plus an import batch. The frozen function, its constants,
  its evidence pin and its allocation digest are untouched (no deleted or modified line).
- **Bindings.** `imports/crystalserver/bindings/items.json` is generated. Epoch 1 is unchanged, and its canonical
  bytes are a strict prefix of the file. The 404 epoch-2 `EXACT` bindings (`00ce02a5…`, `ots/item_server_id`,
  `definition-r1`) follow in ascending source id, for 38,561 in all.
- **Generator.** Epoch 1 is regenerated as before and pinned by digest. Epoch 2 is derived from the pinned census and the
  pinned crosswalk. `--build-alias-crosswalk` and `--verify-alias-crosswalk` recompute the gate from local checkouts
  of the two pinned Crystal revisions; the default and `--check` paths are offline.

## Architecture and source of truth

- `PROVEN`: the decision, the frozen CW2-B1 allocator, the donor census bytes and the two pinned Crystal
  revisions (`items.xml` and `appearances.dat` digests verified before use).
- `NOT DECIDED HERE`: the alias-gate signal set. The decision requires "multi-signal evidence" without naming the
  signals, so `A8-ALIAS-GATE-V1` is B1b's own evidence and the independent identity review must accept it. A held id
  that later resolves as distinct enters a later epoch, and one that proves an alias binds `ACCEPTED_ALIAS`.
- Deliberately not done: no `content/**` Item records (SHARED_LEASE_REQUIRED: `content/**` regeneration), no
  `.github/**` or `tools/repository/**` change, no facts, promotion, Presentation, runtime or wire ids.

## High-risk authority/recovery qualification

Identity minting: independent exact-head identity review is required before integration (decision §8). The
authority is the merged decision #1163. No production, credential or cross-repository authority is used.

## Acceptance criteria

- [ ] Every donor id has a crosswalk state; only `NO_MATCH` ids mint.
- [ ] Keys are 38,094 to 38,497 in ascending source id, with no source id or key collision.
- [ ] The epoch-1 import, its 38,157 keys and its digest are byte-identical; 2,921 stays retired.
- [ ] Each minted id has exactly one `EXACT` binding at `00ce02a5`; base bindings are unchanged.
- [ ] The validation below passes on the frozen final head of the PR.
- [ ] Independent exact-head identity review, then protected Merge Queue integration.

## Excluded scope

- `content/**` Item definitions for the epoch-2 keys, which need the shared lease.
- WO-1 `routed_to` Item schema, WO-2 family keys, B2 and B3 evidence.
- Re-running CW2-B1 over a larger corpus, or any renumbering.

## Validation

All results are for the authoring tree. Candidate-specific evidence is bound to the frozen final head of the PR, never a
SHA recorded here.

- Generator self-test and `--check`: PASS. `--verify-alias-crosswalk` against the two pinned checkouts: PASS.
- `cargo +1.94.0 fmt --all --check`, `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`, the CW2 and
  content tests and both repository validators: see the PR body for the exact-head results.

## Context checkpoint

```yaml
last_progress: PR #1179 merged via Merge Queue as ff498b5; protected-main readback matched dd4e6e8; record archived in the 2026-09-29 batch
status: completed
branch: claude/item-donor-identity-b1b
pr: 1179
owner_action_required: null
blocker: null
next_action: none for this task
jira_sync: pending (no mapped Story resolved in this session)
```

## Closeout

- merge commit/result: `ff498b5` on protected `main` (#1179); the changed files are byte-identical to `dd4e6e8`
- ownership release: all leases released at merge
- archived in the batch archive of 2026-09-29
