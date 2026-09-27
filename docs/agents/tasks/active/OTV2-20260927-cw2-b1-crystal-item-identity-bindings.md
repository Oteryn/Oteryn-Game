# OTV2-20260927-cw2-b1-crystal-item-identity-bindings

```yaml
task_id: OTV2-20260927-cw2-b1-crystal-item-identity-bindings
title: CW2-B1 Crystal Item identity bindings (explicit EXACT, 38,157 rows)
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: 996
base_sha: ccef9bbe8ecf75c4b1b99b07237c78ebf4642dce
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session
created_at: 2026-09-27T00:00:00Z
updated_at: 2026-09-27T15:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-census/g4_item_crystal_binding_generator.py
  - tools/content-census/g4_item_crystal_binding_generator_self_test.py
  - imports/crystalserver/bindings/items.json
  - docs/agents/tasks/active/OTV2-20260927-cw2-b1-crystal-item-identity-bindings.md
  - .github/workflows/g4-item-crystal-bindings.yml
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md"
  - "PR #670 protected-merged (CW2-B1 item identity catalogue + allocator)"
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
jira: KAN-16
```

## Outcome

The committed CW2-B1 allocator (`apps/game-server/src/content/cw2_b1_import.rs`)
already assigns every one of the 38,157 `content/items/definitions/*.json` Item
keys from Crystal `data/items/items.xml` `id` values, ascending, via the frozen
evidence catalogue. That mapping was only implicit allocator logic; per G4
doctrine (`docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md`
section 3) an external identifier must become an explicit typed
`(target, source, identity_namespace, external_id, disposition)` binding, not
an inference. This task makes those existing assignments explicit as
`imports/crystalserver/bindings/items.json`: 38,157 `EXACT` bindings, Crystal
only (Canary is out of scope). No canonical identity is minted; no
`content/items/definitions/*.json` record is touched.

## Architecture and source of truth

- `docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md` §3: PROVEN, gives the binding tuple shape and namespace examples (`ots/item_server_id`, `mediawiki/page_id`, ...).
- `apps/game-server/src/content/cw2_b1_import.rs`: PROVEN, `protected_cw2_b1_full_item_family_import` walks ascending `source_item_id` from `semantic_catalog.identity_records`; 64 `NATIVE_ITEM_BATCH` rows keep their semantic key, all others get `oteryn:item.registry.i%08d` via `opaque_sequence`.
- `docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json`: PROVEN, 16,877,870 bytes, sha256 `7836c78c...d96a7`, matches the allocator's pinned constants.
- `imports/tibiawiki/bindings/items.json`: PROVEN, existing sibling `OTERYN_SOURCE_IDENTITY_BINDINGS/v1` format this file matches field-for-field, ordered by ascending `canonical_bytes`.
- Crystal `items.xml`'s `id` attribute is the OTServer-family server-side item type id, i.e. doctrine's `ots/item_server_id` example, distinct from `clientid`/appearance identity (which would bind Presentation, never Item).

## Acceptance criteria

- [x] `tools/content-census/g4_item_crystal_binding_generator.py` independently re-derives the allocator's mapping (parsing `NATIVE_ITEM_BATCH` from the Rust source, never hand-copied) and fails closed on: wrong evidence digest/bytes, non-ascending/duplicate `source_item_id`, wrong opaque/native counts, duplicate native key, an allocated key absent from `content/items/definitions/*.json`, a definitions key with no allocation, or the Magic Sword golden cross-check (Crystal `3288` -> `oteryn:item.registry.i00003167`, matching TibiaWiki page `5810`) disagreeing.
- [x] Generated `imports/crystalserver/bindings/items.json` has exactly 38,157 `EXACT` bindings, `identity_namespace: ots/item_server_id`, `source_key: oteryn:source.crystalserver`, `source_revision: ff7ede593c69d4c658b382c97443e8155926924a`, every `target.key` present in `content/items/definitions/*.json`.
- [x] Protected identity promotions declared in the allocator source (`<PREFIX>_SOURCE_ITEM_ID`/`_OLD_KEY`/`_KEY`, currently R7 P04 Gold Coin: Crystal `3031` -> `oteryn:item.currency.gold_coin`) are applied after allocation, failing closed unless the allocator assigned exactly the old key.
- [x] `tools/content-census/g4_item_crystal_binding_generator_self_test.py` exercises the fail-closed paths above at small scale.
- [x] `python tools/agents/validate_governance.py`, the content-tree migration validators and ruff pass (see Validation).
- [x] `.github/workflows/g4-item-crystal-bindings.yml` re-runs the self-test and `--check` whenever the generator, its output, `content/items/definitions/**`, the Rust allocator, the evidence catalogue or the TibiaWiki bindings change, so drift cannot land silently.

## Excluded scope

- No Canary bindings (owner decision: Crystal only, this task).
- No embedding of `source_bindings` into `content/items/definitions/*.json` records: only 165/38,157 records currently carry any embedded `source_bindings` (the TibiaWiki-matched subset, mirroring `imports/tibiawiki/bindings/items.json` exactly); no validator or doctrine text requires the same for the standalone `imports/crystalserver/bindings/items.json`, and `.github/workflows/content-tree-migration.yml` triggers only on `imports/tibiawiki/bindings/**`, not `imports/crystalserver/**`. Embedding into all 38,157 records would be a much larger, unrequired change, so it was not done; flagged here for an explicit owner call if full parity is later wanted.
- No `content/items/definitions/*.json`, `content/world/**`, or `imports/tibiawiki/**` mutation.
- No gameplay-field promotion, no Presentation/Asset/runtime identity change.
- No consumer reads `imports/crystalserver/bindings/items.json` yet; the Item authoring work in PR #952 is its first intended consumer (canonical keys for real examples).

## Validation

### Focused

- command/run: `venv/bin/python tools/content-census/g4_item_crystal_binding_generator.py --check`
- result: PASS bindings=38157 bytes=10864254
- command/run: `venv/bin/python tools/content-census/g4_item_crystal_binding_generator_self_test.py`
- result: PASS
- command/run: `ruff check` and `ruff format --check` (0.16.1) on both scripts from the repo root
- result: PASS (both scripts are executable, so `EXE001` no longer applies)

### Component/integration

- command/run: `venv/bin/python tools/agents/validate_governance.py`
- result: PASS ("Governance validation passed for Oteryn/Oteryn-Game.")
- command/run: `venv/bin/python tools/content-migration/test_world_project_v2_to_tree.py && venv/bin/python tools/content-migration/validate_world_project_v2_to_tree.py`
- result: both PASS, unaffected (neither reads `imports/crystalserver/**`)

### E2E

- scenario: NOT_APPLICABLE (auxiliary import-provenance data; `imports/**` classifies as `auxiliary`/`unconsumed-auxiliary-inputs` per `tools/repository/classify_pr_test_lanes.py`, confirmed by `tools/repository/test_classify_content_routing.py` fixtures already listing `imports/crystalserver/*.json`; no Rust/game-gate build is triggered by this change)

### Exact-head CI

- head `de14e8a2` (before the drift-guard workflow): Merge gate (scope, governance,
  routing contract, dependency review, CodeQL, trusted-base risk lanes, validate) and
  `game-gate` success; classification `auxiliary` (Rust/Windows lanes skipped)
- head `60dac2d9`: all checks green, including the new drift guard
- head `d62d32c4` (owner merged `main`): the drift guard failed with
  `ALLOCATED_KEY_MISSING_FROM_DEFINITIONS:[i00002921]` because #987 promoted Gold Coin;
  #989 merged from the queue as `0c7098eb` before the repair could land; the repair
  (declared identity promotions) follows in PR #996
- final head: pending

## Self-review

- exact head: `de14e8a2` content plus the drift-guard workflow
- method/reviewer: implementing session
- material findings: no CI job re-ran `--check` (fixed by the new workflow); stale record
  and PR text (fixed)
- verdict: ready for independent review of the final head

## Independent review

- required: YES (identity/provenance change)
- exact head: `de14e8a2` (content unchanged by the successor; workflow reviewed as the
  uncommitted diff)
- method/auditor: read-only independent review agent
- material findings: mapping re-derived from `cw2_b1_import.rs` without the generator's
  code, 0 of 38,157 rows differ; the opaque counter skips native rows exactly as the
  allocator does; the pinned Crystal `items.xml` blob hashes to the catalogue's recorded
  `c847293e...` and expands to the same 38,157 ids (3288 magic sword -> `i00003167`,
  3388 demon armor -> `i00003256`); format and ordering match the TibiaWiki bindings;
  self-test is fail-closed. Blocking: no drift guard in CI (fixed by the workflow, whose
  trigger paths cover every file the generator reads). Nit: the
  `SOURCE_ITEM_ID_DUPLICATE` branch is unreachable after the strict-ascending check;
  kept as defensive code.
- verdict: APPROVE, conditional on the workflow landing in this PR (done)

## PR and closeout

- changed-file review: done (5 files)
- unresolved review threads: none
- related/superseded PRs: none
- protected auto-merge: NOT_AUTHORIZED (owner merges)
- merge commit/result: #989 merged as `0c7098eb`; follow-up repair #996 pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: drift guard caught the #987 Gold Coin promotion after the main merge; generator now applies declared identity promotions
status: validating
branch: claude/compassionate-albattani-s29syw
pr: 996
final_head_sha: null
owner_action_required: "merge after exact-head CI is green"
blocker: null
next_action: freeze the pushed head, confirm CI, mark ready for review
```
