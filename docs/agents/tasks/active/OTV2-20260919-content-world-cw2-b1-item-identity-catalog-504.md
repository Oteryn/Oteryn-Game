# OTV2-20260919-content-world-cw2-b1-item-identity-catalog-504

\`\`\`yaml
task_id: OTV2-20260919-content-world-cw2-b1-item-identity-catalog-504
title: CW2-B1 item identity catalogue source batch
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-b1-item-identity-catalog-504
issue: 162
pr: null
base_sha: 8dac2e86a66f9743a99d8fd14ab66265432db969
head_sha: 626a3062fbbc29ab7eb63c420da73503e5e08dd1
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: content world import"
created_at: 2026-09-19T10:20:43+02:00
updated_at: 2026-09-19T10:20:43+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/item_identity_catalog.py
  - tools/reference-world-corridor-census/item_identity_catalog_self_test.py
  - docs/agents/tasks/active/OTV2-20260919-content-world-cw2-b1-item-identity-catalog-504.md
  - docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json
public_contracts: []
depends_on:
  - "#162 comment 5740367516"
  - "PR #668 protected-merged"
blocks:
  - CW3-B1 minimal typed item semantic delta
cross_repository_coordination_id: null
external_repositories:
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
\`\`\`

## Outcome

Produce the first broad CW2 item source catalogue as deterministic candidate evidence without creating a second Item/Content model and without minting canonical Oteryn identity from Crystal numeric IDs, names, paths, hashes or appearance IDs.

The final tracked evidence binds every emitted source identity to the exact pinned Crystal snapshot, classifies observed source fields against GAME-ITEM semantic families, records loss/unsupported/conflict conditions explicitly, and leaves native ItemType/ContentKey resolution fail-closed when no already-admitted Game-owned explicit binding exists.

## Architecture and source of truth

**PROVEN — allocation authority**

- #162 comment \`5740367516\`;
- admission protected \`main@8dac2e86a66f9743a99d8fd14ab66265432db969\`;
- write custody is exactly the four files listed above;
- no write authority exists for \`apps/game-server/src/**\`, CW4, CW3 shared Content model, registries, Cargo/workspace, persistence, workflows or external repositories.

**PROVEN — native identity authority**

- \`docs/architecture/GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md\`;
- \`docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md\`;
- legacy/source numeric IDs are mappings/provenance, not canonical ItemType/ContentKey identity;
- authoritative arbitrary attribute bags are forbidden;
- CW2 emits candidate/source IR only; CW3 remains the shared typed model/promoter.

**PROVEN — source snapshot**

Repository/revision:

\`zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a\`

Raw Git objects verified independently of Windows working-tree line-ending conversion:

| path | blob | raw bytes | SHA-256 |
|---|---|---:|---|
| \`data/items/items.xml\` | \`0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f\` | 3,819,874 | \`c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb\` |
| \`src/items/functions/item/item_parse.cpp\` | \`d95e2c83f44a81e7da7dc0d066c3b6f3a3302426\` | 59,234 | \`08687bc0e096ebfcce55daa6a523b78bca0191f25ce4134c133927d4a10dfce7\` |
| \`src/items/functions/item/item_parse.hpp\` | \`e22ede24f58aac0be6f85cd8d8167a6aa4341974\` | 20,529 | \`4493fe6ed56696402eb9eeb47d2685544130b8b0ed0074e2fae2f81bd193a796\` |
| \`src/items/items_definitions.hpp\` | \`c09a62b585766909045e0fff5cf770e5550d333c\` | 16,234 | \`0eff3929fca762fada5e31aa5ad069cf8f92713ee8c047c0578bb7ea8612c4cf\` |
| \`LICENSE\` | \`d159169d1050894d3ea3b98e1c965c4058208fe1\` | 18,092 | \`8177f97513213526df2cf6184d8ff986c675afb514d4e68a404010521b880643\` |

Source role/classification remains exactly:

\`CRYSTAL_OTS / OTS_HYPOTHESIS_ONLY\`

Closure remains:

\`CANDIDATE_ONLY\`

Production authority and Reference parity claim remain \`NONE\`.

## High-risk authority/recovery qualification

\`\`\`yaml
applicable: false
reason: source-catalogue evidence only; no runtime, persistence, authority fence, production mutation or protected integration is performed by this task
\`\`\`

## Acceptance criteria

- [x] Exact Crystal repository/revision/path/blob/size is fail-closed verified from raw Git objects.
- [x] SHA-256 is recorded for every admitted source input.
- [x] One deterministic source identity record is emitted for every valid deterministic source-ID/range expansion member.
- [x] Every emitted identity lands in exactly one native mapping class.
- [x] No native key is minted from numeric ID, display name, path, hash or appearance ID.
- [x] Field/source observations are explicitly classified as GAME-ITEM candidate, provenance-only, unsupported, excluded-by-policy, unknown or conflict.
- [x] Range exclusions, field conflicts, unsupported fields and native unresolved state are explicit records, not totals only.
- [x] Duplicate IDs/ranges, name collisions, unsupported attributes and binding ambiguity/conflict are covered by self-tests.
- [x] Two clean runs over the exact pinned input are byte-identical and have the same product digest.
- [x] Selected closure is \`CANDIDATE_ONLY\`.
- [ ] Normal repository exact-head CI on the final PR head.
- [ ] Independent review only if required by the repository risk policy after final exact-head qualification.

## Excluded scope

No mutation of:

- \`apps/game-server/src/**\` or the shared typed Content model/compiler;
- CW4 runtime paths;
- existing CW3 source/task paths;
- protocol/schema/stable-ID/resource registries;
- Cargo/workspace/lock;
- persistence/migrations;
- workflows/governance;
- external repositories;
- production/protected environments;
- raw third-party source payloads inside Oteryn Git.

No target-sensitive value is promoted from Crystal to Reference truth.

## Implementation / findings

### Identity census

Full pinned \`items.xml\` result:

- source XML item nodes: **17,669**;
- direct-ID nodes: **14,689**;
- range nodes: **2,980**;
- reversed ranges explicitly excluded: **4**;
- expanded/emitted source identity records: **38,157**;
- native \`RESOLVED\`: **0**;
- native \`UNRESOLVED\`: **38,157**;
- native \`AMBIGUOUS\`: **0**;
- native \`CONFLICT\`: **0**.

Fresh admission-tree search found no already-admitted explicit Crystal source-item-ID -> native ItemType/ContentKey binding. Therefore zero native identities are resolved in B1. This is intentional fail-closed behavior, not missing auto-mapping.

### Source-field disposition

- unknown expanded field observations: **0**;
- unsupported expanded field observations: **2,177**;
- excluded-by-policy script observations: **1,565**;
- explicit field conflict records: **5**;
- normalized-name collision groups: **3,174**;
- deduplicated field profiles: **821**.

The five field conflicts are source records where multiple distinct source observations map to one B1 native candidate field (\`weight\`). They are preserved as conflicts rather than resolved through source order/last-write behavior.

Four reversed ranges are preserved as explicit \`REVERSED_RANGE_EXCLUDED\` records. B1 does not silently repair or reinterpret them.

Name collisions never participate in native identity resolution.

### Mapper/evidence binding

Mapper profile:

\`OTERYN_CW2_ITEM_IDENTITY_CATALOG_MAPPER/v1\`

Mapper generation commit:

\`626a3062fbbc29ab7eb63c420da73503e5e08dd1\`

Mapper blob:

\`70556ebc58aec059f898960de4abf43163736eef\`

Mapper SHA-256:

\`fdd2bb82efdd4fe4371e5d39ffe921633c0f7ffebaaecbf86e632eafe6c2d25a\`

Product digest:

\`a28dd86ba0f045d9f863c2745df0647a6b39cdf377743beb40240bda96f65a50\`

Canonical evidence file SHA-256:

\`33828d3a9f372c019b0798f8ec11cc0f417d0c940a02528ad46122d46d0241bb\`

A tracked Git object cannot contain its own final commit SHA without self-reference. The evidence therefore binds the committed mapper/generation head above; the final PR head must be recorded by immutable live PR/check readback after the evidence/task commit.

## Validation

### Focused

- \`python -m py_compile tools/reference-world-corridor-census/item_identity_catalog.py tools/reference-world-corridor-census/item_identity_catalog_self_test.py\` — PASS.
- \`python tools/reference-world-corridor-census/item_identity_catalog_self_test.py\` — PASS.
- \`python tools/reference-world-corridor-census/content_source_batch_self_test.py\` — PASS.
- Full pinned Crystal run — PASS: 17,669 nodes / 38,157 identities / 38,157 unresolved / 0 unknown field observations.
- Clean repeat run — PASS: byte-identical evidence SHA-256 \`33828d3a9f372c019b0798f8ec11cc0f417d0c940a02528ad46122d46d0241bb\`.

The \`tools/game-atlas-appearances\` self-test is \`NOT_APPLICABLE\`: B1 consumes no qualified appearance/client crosswalk and therefore does not enter that boundary.

### Component/integration

Pending final whole-branch validation after evidence/task commit.

### E2E

\`NOT_APPLICABLE\` — this task is source-catalogue candidate evidence only and changes no runtime.

### Exact-head CI

- final head: pending external live PR readback;
- trigger source: pending PR;
- workflow/run/job: pending;
- result: pending.

## Self-review

- exact head: pending final commit;
- method/reviewer: implementing worker whole-diff adversarial review;
- material findings: pending;
- verdict: pending.

## Independent review

- required: pending final risk-policy classification;
- exact head: pending;
- method/auditor: pending;
- material findings: pending;
- verdict: pending.

## PR and closeout

- changed-file review: pending;
- unresolved review threads: pending;
- related/superseded PRs: none known;
- protected integration: not authorized to this worker;
- ownership release: after qualified PR/handoff.

## Context checkpoint

\`\`\`yaml
last_progress: deterministic full-source evidence generated twice byte-identically
status: validating
branch: agent/content-world-cw2-b1-item-identity-catalog-504
head_sha: 626a3062fbbc29ab7eb63c420da73503e5e08dd1
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: commit evidence/task packet, run final local validation, publish normally, open PR and qualify exact-head CI
\`\`\`
