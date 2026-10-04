# OTV2-20261004-crystal-map-source-candidates

```yaml
task_id: OTV2-20261004-crystal-map-source-candidates
title: "Stage reproducible Crystal summer-update map source candidates"
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/crystal-map-source-candidates-20261004
pr: 1791
base_sha: 69f171fcbe3bf8f320dd4ea69b0e3bc4501a6f39
head_sha: null
final_head_sha: null
owner: Codex owner-authorized map integrator
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/map-content-linking/fill_links.py
  - tools/content-schema/map-content-linking/test_fill_links.py
  - tools/content-schema/map-content-linking/README.md
  - imports/crystalserver/summer-update/README.md
  - imports/crystalserver/summer-update/source-tree.json
  - imports/crystalserver/summer-update/raw/
  - imports/crystalserver/summer-update/map-content-linking/
  - docs/agents/tasks/archive/OTV2-20261004-crystal-map-source-candidates.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Outcome

PROVEN: reproducible source proposals link the current map palette to the
existing client asset inventory and project complete supported primary Crystal
monster groups into an import-only Spawn.Source core candidate. All unsupported
groups are retained without deleting points. The producer never writes content.

## Architecture and source of truth

- Crystal `summer-update` revision
  `00ce02a57ca5a12e48f32a3476e37471167e4c3f`, OtsHypothesisOnly.
- Full source tree and primary XML Git blobs, plus original LICENSE.
- Existing owner-confirmed client 15.30 manifest and 6,248 physical files.
- Existing native Creature and Presentation keys; accepted Spawn.Source core
  shape and current placement palette, including provisional donor identities.
- World Bundle v3 still has its admitted Canary profile. These proposals do
  not replace that profile, extend a contract or activate a population.

## Acceptance criteria

- Exact input/output manifest, closed directory inventory and `--check`.
- 25,982 appearance links, 5,084 sprite sheets, two explicit Source NULL slots.
- 54,713 raw groups / 88,261 points = 54,680 core candidate groups / 87,815
  points + 33 held groups / 446 points.
- All canonical content, runtime code, public contracts and source admission
  remain unchanged.

## Excluded scope

The 5,952 palette-target admissions, direct Terrain authoring amendments,
additional kinds and speed 1200, MAP-KIND-CLASS-1/Common (#1786), replacing
canonical spawns, weighted-choice runtime semantics, native realization,
startup/cutover/overlay, map wire and client rendering belong to owning lanes.
The broader prior local prototype and its qualification are preserved locally;
they are not relabelled as validation of this source-only PR.

## Validation

- `python3 tools/content-schema/map-content-linking/fill_links.py --check`:
  PASS, exact input/output reproduction; final SHA bound in the PR.
- `python3 -m unittest discover -s tools/content-schema/map-content-linking -v`:
  PASS, 6 boundary tests.
- `python3 -m ruff check tools/content-schema/map-content-linking`: PASS.
- `python3 -m ruff format --check tools/content-schema/map-content-linking`: PASS.
- `python tools/agents/validate_governance.py`: PASS;
  remote CI and review results remain pending.
- `python -m unittest discover -s tools/agents/tests`: PASS, 54 tests.
- `python3 tools/content-census/item_key_references.py`: FAIL, exactly two
  pre-existing main references to Item 40450. The current palette is preserved;
  this PR introduces no additional dangling Item references. Control plane
  D581 routes the baseline defect to ITEM-KEY-I40450-1. No checker exceptions.
- E2E: NOT_APPLICABLE, import proposals with no runtime consumer change.
- High-risk authority/recovery: NOT_APPLICABLE, no authority/persistence change.

## Self-review and publication

All changed files are source evidence, import proposals, a read-only producer,
tests and documentation. Provisional identities remain explicit, Asset keys
remain proposals and Presentation joins are Source-ID-only. Source XML is
pinned and DTD/entities are refused. Missing references and unsupported group
semantics retain the complete source group. No official client binaries are
added. Independent external review defaults to none for this bounded
source-only candidate; unchanged source logic has prior local review evidence,
which is historical and grants no remote candidate qualification.

The owner requested an ordinary non-draft PR. Exact candidate SHA, remote
readback and CI are recorded in that PR rather than a self-referential commit.
No merge or automatic merge is authorized by this task.
