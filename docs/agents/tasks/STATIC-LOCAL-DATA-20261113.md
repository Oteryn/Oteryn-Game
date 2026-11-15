# STATIC-LOCAL-DATA-20261113

```yaml
task_id: STATIC-LOCAL-DATA-20261113
title: Publish five bounded offline static evidence whitelists
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/static-local-data-cloud-20261113
pr: null
base_sha: 3e168ce6afe374485da20ee0ac13d3749558e2f8
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Cloud /root/mantra_resume
created_at: 2026-11-13T13:00:17Z
updated_at: 2026-11-13T13:00:17Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/static-local-data-20261113/**
  - docs/agents/tasks/STATIC-LOCAL-DATA-20261113.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

PARTIAL offline evidence publication: five approved static whitelists,550926B.
Achievements378; Creatures845/classes21/bosses453; Quests104; Houses995; Maps473.
Overall full static audit stays active and incomplete; existing Items PRs remain
unchanged. This is not runtime admission or full Cyclopedia UI parity.

## Architecture and source of truth

DERIVED prepared projections, pinned by private manifest
fb1cd4089e581e0aa3a80917228e86185af7b4d01613558d4b3f4b60d6d9a816.
PROVEN exact imported byte hashes and bounded structure checks; UNKNOWN semantic
fields remain explicit. House provenance follows the existing owner contract,
not engine entrance or runtime World House identity.
META3.1.0 pin1bfb5ff98c8aa156e73669a14e083a1d464c29fb resolved by lead.
Allocation readback: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6853408185

## High-risk authority/recovery qualification

NOT_APPLICABLE: auxiliary evidence/docs and standalone validator only; no runtime,
authority, session, persistence, production, controller or GUI mutation.

## Acceptance criteria

- [x] Five exact approved payloads, no raw assets/narratives/private paths.
- [x] Standalone hash/schema/privacy validator with meaningful negative checks.
- [x] Preserve source namespaces, known zeros, candidate/UNKNOWN/admission holds.
- [x] Required governance checks and final owned-delta review.
- [ ] Publish authorized draft, freeze exact returned remote SHA outside this record.
- [ ] Exact-head CI and formal independent review remain separately pending.

## Reference parity

NOT_APPLICABLE to gameplay: no executable mechanic or content admission.
House source provenance joins are explicit; other name matches stay candidates.
Raw layout55015 change and quest new client IDs192/195 are not gameplay fixes.

## Excluded scope

No Items3170/3173 or other branch changes, proficiency duplication, GUI/input/
anti-logout/login/transactions, game builds, assets/screenshots/title/Description/
restriction literals, runtime bindings, paid review, merge or deployment.

## Implementation / findings

AUTHORING only on allocated new branch. Five payload originals unchanged.
House static metadata995 equal; layout994equal/1different. Six zero bed counts
preserved. Old raw quest102 equal plus2 new. Creature raw-field semantics and
achievement field4 remain held; map provenance463equal/2changed/8new distinct.

## Validation

- python3 docs/agents/evidence/static-local-data-20261113/validate.py: PASS exit0.
- python3 -m unittest discover -s docs/agents/evidence/static-local-data-20261113 -p test_validate.py: PASS20tests/0fail exit0.
- python3 tools/agents/validate_governance.py: PASS22documents/9lanes exit0.
- python3 -m unittest discover -s tools/agents/tests: PASS129tests/0fail exit0.
- git diff --check and exact owned-file scope: PASS before grouped commit.
- Component/integration/E2E: NOT_APPLICABLE, evidence-only, no game code/build.
- Exact-head CI: pending authorized push; no old-head green claim.

## Self-review

Author task-diff/hash/privacy inspection, not formal independent PR review.
Final ten-file owned delta checked; all five approved payload bytes unchanged.

## Independent review

REVIEW_PENDING. Required before merge; coordinator owns dispatch, no worker trigger.

## PR and closeout

Active waiting/PARTIAL record. First commit cannot name its own SHA; exact returned
remote head and freeze belong in live PR/handoff without a metadata-only commit.
No ready, complete, merge or deploy claim.
