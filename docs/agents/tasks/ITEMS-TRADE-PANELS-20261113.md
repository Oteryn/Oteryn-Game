# ITEMS-TRADE-PANELS-20261113

```yaml
task_id: ITEMS-TRADE-PANELS-20261113
title: Publish bounded initial NPC trade panel evidence
mode: AUDIT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/items-trade-panels-evidence-cloud-20261113
pr: null
base_sha: 5efd67da164ac9dc33c8c066434b228cebb86c2b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: /root/mantra_resume
created_at: 2026-11-13T05:08:31Z
updated_at: 2026-11-13T05:08:31Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/items-trade-panels-20261113/**
  - docs/agents/tasks/ITEMS-TRADE-PANELS-20261113.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

PARTIAL: publish compact literal initial trade views for Decoration462 and Food169.
All631 physical ordinals and1,262 initial panels are covered; complete NPC lists
remain PARTIAL pending77 panel-specific retakes (43 Decoration,34 Food).

## Architecture and source of truth

PROVEN custody: private source unions Decoration9438aeca088547b3d56b1004e0bada19e6eefebdbb9dddce7b496a32c80c160e
and Food20b4a5e7258a5b98da670aedc9791e133d2dd596df7b4f1c84d6c06082454545.
Native visual observations use GLOBAL_BLACKBOX, client Tibia Test Server15.33.dd0955.
DERIVED: whitelist projection, explicit-currency parsing and coverage census.
UNKNOWN: offscreen offers, clipped residences and six bare-number quote currencies.
No name/icon/ordinal/appearance relationship binds a native/server ID.

Allocation: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6844725352
(read back live). Parent resolved META3.1.0 at1bfb5ff98c8aa156e73669a14e083a1d464c29fb.
Companion Basic drafts3155/3156 are separate and remain untouched.

## High-risk authority/recovery qualification

NOT_APPLICABLE: read-only reference observations and documentation export only;
no runtime authority, persistence, controller or session mutation.

## Acceptance criteria

- [x] Distinct631 ordinal records,1,262 panel states,637 visible quote entries.
- [x] Preserve source literals, UNKNOWN/missing states and same-name rows separately.
- [x] Explicit immutable-input Yasir310 and Ramina21 correction overlays.
- [x] Whitelist export excludes PNG/raw private JSON/narratives/configuration.
- [x]77 panel-specific retake pairs retained; Decoration Sell To310/382 preserved.
- [ ] Complete overflow retakes and qualify full NPC lists.
- [ ] Exact-head CI and independent review.

## Reference parity

NOT_APPLICABLE to gameplay: no mechanic or source item IDs changed.
Initial observations are not executable trade offers or global NPC absence.

## Excluded scope

No GUI input, NPC transactions/conversations, gameplay/content changes, ID
binding, screenshot redistribution, private narratives, frozen PR changes,
paid review triggers, merge or deploy.

## Implementation / findings

AUTHORING draft. Explicit schema adapter copies literal strings and order only;
currency is known only when a gp or x currency suffix was actually displayed.
Observed empty panels never establish global offer absence. Original private
inputs remain unchanged; source receipts pin both unions and correction sidecars.

## Validation

- python3 docs/agents/evidence/items-trade-panels-20261113/validate.py: PASS631cards,
  1,262panels,77retakes; repository-only run explicitly does not recheck private files.
- Same validator on Ubuntu with private-root: PASS631fresh Basic proof hashes,
  both private union hashes, both overlay hashes and exact whitelist projection;
  actual terminal exit0, no private records printed.
- python3 -m unittest discover -s docs/agents/evidence/items-trade-panels-20261113 -p test_validate.py:
  PASS5tests/0fail.
- python3 tools/agents/validate_governance.py: PASS22documents/9lanes.
- python3 -m unittest discover -s tools/agents/tests: PASS129tests/0fail. Initial read-only sandbox run had35temp-directory errors; same unchanged tests passed with writable fixtures.
- git diff --check: PASS; staged exact owned delta rechecked before commit.
- Runtime/integration/E2E: NOT_APPLICABLE, evidence-only docs/Python validator.
- Exact-head CI: PENDING after authorized draft publication.

## Self-review and independent review

TASK diff/provenance/privacy inspection completed over all8owned files; strict whitelist and private projection checks pass. This is not formal
independent review. REVIEW_PENDING; coordinator owns required review dispatch.
No paid trigger or readiness claim by this Codex lane.

## PR and closeout

PARTIAL, waiting on overflow retakes/CI/review. Keep active record; do not archive.
No merge/deploy authorized. Draft remains AUTHORING until coordinator qualification.
