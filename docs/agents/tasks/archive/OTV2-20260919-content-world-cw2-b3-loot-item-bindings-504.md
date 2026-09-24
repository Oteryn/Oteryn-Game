> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #676 merged as `c22388da53ea721b0ac5fb74ad1b1f95e3b74814`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260919-content-world-cw2-b3-loot-item-bindings-504

```yaml
task_id: OTV2-20260919-content-world-cw2-b3-loot-item-bindings-504
title: CW2-B3 loot to native ItemKey binding source batch
mode: MIGRATE
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-b3-loot-item-bindings-504
issue: 162
pr: 676
base_sha: c7688069bc22ac3cde46e48e6b05d8051418fed1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world import
created_at: 2026-09-19T16:56:00+02:00
updated_at: 2026-09-19T16:56:00+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/loot_item_binding_catalog.py
  - tools/reference-world-corridor-census/loot_item_binding_catalog_self_test.py
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw2-b3-loot-item-bindings-504.md
  - docs/agents/evidence/OTV2-20260919-content-world-cw2-b3-loot-item-bindings.json
public_contracts: []
depends_on:
  - protected PR #670 / CW2-B1
  - protected PR #674 / CW2-B2
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
```

## Outcome

Build the smallest deterministic candidate-only CW2-B3 product that binds every
protected B2 admitted loot row first to a protected B1 source-item identity when
the pinned source semantics prove that join, then carries the protected B1
native ItemType/ContentKey disposition without inventing native identity.

The batch does not create ItemType definitions, ItemInstance state, executable
loot RNG, corpse ownership, settlement, transfer/value transactions or
production behavior.

## Architecture and source of truth

- **PROVEN** — protected admission Game SHA:
  `c7688069bc22ac3cde46e48e6b05d8051418fed1`.
- **PROVEN** — protected B1 producer PR/head/merge:
  `#670 / 4fea32f7d7efc604eeef8a0dc7ed79341eb7d972 /
  715a22f26f6ec5472597f63cf5d6b939d7583cc1`.
- **PROVEN** — protected B1 product digest:
  `d773076b576599b6ced7eb53e262cdc6363515d842da3e8518b610e2106db0fc`.
- **PROVEN** — protected B2 producer PR/head/merge:
  `#674 / 0086702ebb2477ade22c82a65b9ad48f80fceee1 /
  c7688069bc22ac3cde46e48e6b05d8051418fed1`.
- **PROVEN** — protected B2 product digest:
  `f69a941a4953c9f3b2532e90ab6589c7a89126b1e8123b0ff2e8d88777f7853f`.
- **PROVEN** — pinned migration source:
  `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.
- **PROVEN** — B3 directly re-reads 1,800 admitted monster-definition blobs plus
  seven pinned Otheryn item/loot semantic files; every blob, size and SHA-256 is
  recorded in the evidence JSON.
- **DERIVED** — aggregate directly consumed source-byte digest:
  `8e44a538c70119f6e68bfcd0be2285b64170e5ccfe44bd8e87b90d095f4f9c8e`.
- **PROVEN** — source classification remains
  `OTERYN_LEGACY / MIGRATION_EVIDENCE`; production authority is `NONE`.
- **PROVEN** — selected closure remains `CANDIDATE_ONLY`.

## Source-item identity semantics

Pinned Otheryn source proves the runtime selector behavior used by this batch:

- `register_monster_type.lua` selects `loot.name` before `loot.id`;
- `loot_functions.cpp` resolves a name through
  `Item::items.nameToItems.equal_range(...)` and writes a server
  `LootBlock::id`;
- `items.cpp` seeds runtime names from `appearances.dat` and applies
  `items.xml` name overrides;
- the exact `appearances.proto` object/id/flags/name fields and the
  `appearances.dat` load path are pinned and verified.

B3 intentionally does **not** reproduce the source runtime's first-entry choice
for duplicate names. Multiple runtime name candidates are import evidence
`AMBIGUOUS`, because insertion order is not native/source-binding authority.

A source join is `RESOLVED` only when the source runtime selector identifies
one server item and the applied pinned Otheryn `items.xml` canonical source
node is byte-semantically identical, under the protected B1 canonicalizer, to
the protected B1 source node for that same member. Numeric equality alone is
insufficient.

`clientId` is provenance only and has no admitted server-item crosswalk.

## Native identity boundary

The second join consumes only protected B1 native disposition/detail.

- B1 `UNRESOLVED` stays native `UNRESOLVED`.
- B1 `AMBIGUOUS` / `CONFLICT` remain those classes only after a single B1
  source identity has been proven.
- Native `RESOLVED` requires an explicit protected B1 `content_key`.
- No display name, numeric ID, client ID, Atlas ID, path/order/hash, appearance
  ID, creature name/hash or synthesized value can mint `oteryn:*`.
- Synthetic `oteryn:vsl.*` targets are rejected.

## Deterministic product

Observed exact protected B2 row set:

- admitted B2 loot rows: **17,086**;
- loot -> B1 source item:
  - `RESOLVED=14,770`;
  - `UNRESOLVED=2,315`;
  - `AMBIGUOUS=1`;
  - `CONFLICT=0`;
- B1 source item -> native:
  - `RESOLVED=0`;
  - `UNRESOLVED=17,086`;
  - `AMBIGUOUS=0`;
  - `CONFLICT=0`.

Source-join reason totals:

- `EXACT_CANONICAL_SOURCE_NODE_CROSSWALK=14,770`;
- `EXACT_SOURCE_NODE_MISMATCH=2,032`;
- `NO_OTHERYN_XML_SOURCE_IDENTITY=255`;
- `NO_B1_SOURCE_ITEM_IDENTITY=25`;
- `SERVER_ITEM_UNAVAILABLE=3`;
- `SOURCE_NAME_AMBIGUOUS=1`.

The one ambiguous row is the source label `transcendence potion`, whose pinned
runtime name registry exposes server item IDs `49271` and `51302`; B3 does
not choose between them by source insertion order.

Collision/accounting evidence:

- 1,661 repeated loot display-name groups;
- 3,245 runtime item-name collision groups;
- 0 duplicate source-row digest groups;
- 0 duplicate B3 loot-row identities;
- 0 admitted `clientId` rows in the current protected corpus;
- 120 B2 source-profile issue records retained explicitly with their existing
  supported/partial/unknown reason codes.

Runtime registry census:

- 42,107 appearance objects with flags;
- 9,439 final runtime name keys;
- 32,957 XML item members applied;
- 4,569 XML members skipped by the pinned missing-appearance rule;
- 0 reversed XML ranges in the pinned Otheryn source.

Mapper canonical SHA-256:
`0155337520b5af64f45bc2685e71d02f20f0c2d6468253f57f56b6856b2d56a8`.
Mapper fingerprint semantics are repository text bytes with CRLF canonicalized
to LF; lone-CR checkout line endings fail closed.

B3 product digest:
`5251d211b531481f340a4093a2e70af473ea721f6b51c7109ee7e516b33b1460`.

Canonical tracked evidence bytes: **17,151,735**.
Canonical evidence file SHA-256:
`be5d523b642caf10163f6e520aeec77517bbf3d59cedbb0e373d5da87dd792d1`.

Two clean complete generations over identical protected/pinned inputs produced
the same product digest, identical byte size and identical evidence-file
SHA-256.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: candidate-only offline source/evidence batch; no protocol, persistence, production authority, runtime loot execution, native identity minting, PREPARE/COMMIT or recovery semantics
```

## Acceptance criteria

- [x] Exact protected B1/B2 evidence blobs and product digests fail closed on
  mismatch.
- [x] Exact pinned Otheryn repository/revision and every directly consumed blob
  fail closed on provenance mismatch.
- [x] Every one of 17,086 protected B2 loot rows produces exactly one source
  resolution and exactly one native resolution.
- [x] Existing protected gameplay extraction semantics are replayed and must
  exactly match B2 before a raw source row is admitted.
- [x] Source name/id selector semantics follow the pinned runtime implementation.
- [x] Duplicate display/runtime names cannot silently become identity.
- [x] Numeric equality without exact canonical source-node equivalence does not
  resolve the source join.
- [x] `clientId` cannot become server/native item identity.
- [x] B1 native `UNRESOLVED` cannot be promoted to native `RESOLVED`.
- [x] Fake/synthetic native keys are rejected.
- [x] Loot `chance_ppm`, `min_count` and `max_count` are copied from the
  protected B2 extractor result without reinterpretation.
- [x] Explicit source-profile issue records and collision accounting are retained.
- [x] Input-order perturbation leaves semantic B3 output byte-identical.
- [x] Two clean complete B3 generations are byte-identical.
- [ ] Exact-head repository CI completes on the final PR head.

## Excluded scope

No writes to B1/B2 producers, Game-owned Atlas exporters, shared
Content/Reference models, CW4/world runtime, GAME-AI/combat/loot settlement,
ItemInstance/value/custody/economy, NPC/shop/service B5, Cargo/workspace/lock,
protocol/schema/stable-ID registries, persistence/migrations,
workflows/governance/resource limits, external repositories or production.

## Validation

Focused and component checks passed on the local candidate:

- `python -B tools/reference-world-corridor-census/loot_item_binding_catalog_self_test.py`
  — PASS.
- `python -B tools/reference-world-corridor-census/item_identity_catalog_self_test.py`
  — PASS.
- `python -B tools/reference-world-corridor-census/creature_spawn_binding_catalog_self_test.py`
  — PASS.
- `python -B tools/game-atlas-creature-gameplay/self_test.py` — PASS.
- `python -B tools/reference-world-corridor-census/content_source_batch_self_test.py`
  — PASS.
- `python -B tools/game-atlas-creatures/self_test.py` — PASS.
- Full B3 input-order perturbation — PASS; semantic binding digest
  `1e20e261ab7a97701da0ecc125d130032509b106ded2f5ee0198ba262de51a33`.
- Two clean full B3 generations — PASS / byte-identical.
- `git diff --check` — PASS.
- `python -B tools/agents/validate_governance.py` — PASS.
- Final staged changed-path readback — PASS, exactly four allocated custody paths.
- Exact-head CI — pending PR publication.

### E2E

- scenario: `NOT_APPLICABLE`;
- reason: offline candidate/source identity evidence only; no executable runtime
  loot or user-visible behavior is authorized;
- result: `NOT_APPLICABLE`.

### Exact-head CI

- PR: `#676`;
- final repaired head: pending commit/push;
- trigger source: existing PR `#676` after repaired head publication;
- workflow/run/job: pending repaired-head generation;
- result: pending repaired-head generation.

## Self-review

- exact candidate: final repair head recorded externally after commit;
- method/reviewer: implementing agent, whole-diff adversarial review;
- pre-freeze material finding: PR #676 head `e2b883ce8ad10bacf886f282eef281aa1fc1732f` committed mapper SHA-256 `88fd5b44e1f399694efa17a95ceae93b18ece691a6230652780b27ba528e5308` while its tracked evidence fingerprint referenced unpublished mapper bytes; repair head `9092205c40dd54812d9564f210dd4cd9ac8328cc` refreshed evidence/task data but still left mapper fingerprint dependent on checkout line endings;
- repair: canonicalize mapper fingerprint input to repository-LF bytes, fail closed on lone CR, add focused checkout-line-ending coverage, and regenerate evidence. Current mapper SHA-256 is `0155337520b5af64f45bc2685e71d02f20f0c2d6468253f57f56b6856b2d56a8`, evidence SHA-256 is `be5d523b642caf10163f6e520aeec77517bbf3d59cedbb0e373d5da87dd792d1`, and all 17,086 binding records/resolution counts remain unchanged;
- open material findings after repair: 0;
- verdict after repair: `PASS_ZERO_MATERIAL_FINDINGS`;
- evidence checks: exact mapper/evidence SHA binding, product-digest recomputation, 17,086-row source/native partitions, no native content key in the protected current product, exact custody.

## Independent review

- required: `NO`;
- reason: bounded candidate-only importer/evidence tooling; no shared model,
  protocol, durable-data, production authority or native identity mutation.

## PR and closeout

- changed-file review: PASS — exactly four allocated custody paths;
- unresolved review threads: pending PR;
- protected auto-merge/direct merge: forbidden for worker;
- merge authority: `OTV2_WORK_DELIVERY_COORDINATOR_ONLY`;
- ownership release: pending handoff.

## Context checkpoint

```yaml
last_progress: checkout-line-ending-invariant mapper fingerprint repair and regenerated evidence locally qualified
status: validating
branch: agent/content-world-cw2-b3-loot-item-bindings-504
head_sha: null
pr: 676
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
owner_action_required: null
blocker: null
next_action: commit and non-force push the four-path EOL-invariant repair, then qualify exact-head CI
```
