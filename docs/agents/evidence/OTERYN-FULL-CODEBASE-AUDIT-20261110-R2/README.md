# OTERYN FULL AUDIT — continuation checkpoint 2

Input base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`.
Audit is ongoing; all remaining packages and supplementary domains remain in scope.

## Verified evidence at this checkpoint

- 67 workspace packages; 11 supplementary domains; 27900 tracked input files.
- 36 packages REVIEWED, 21 IN PROGRESS, 10 NOT STARTED; 0 VERIFIED.
- 1312 distinct files with substantive recorded analysis: 1150 full files and 162 selected-range files; 128846 deduplicated analyzed line positions.
- 192 earlier analyzed files have identical Git blobs and retain their exact previous ranges. Four changed files receive no automatic carryover credit (`carryover.json`); any fresh analysis is separately attributed in the ledger.
- Source review is distinct from tests executed. Coverage includes individually reasoned source/test review and explicitly scoped semantic data validation. Mechanically checking a hash/range does not substitute for either.

## Findings register

The [first published checkpoint](https://github.com/Oteryn/Oteryn-Game/tree/1c39ebc3347319fe08eaf58299a8a9e9339b0367/docs/agents/evidence/OTERYN-FULL-CODEBASE-AUDIT-20261110) retains three confirmed P1 and three confirmed P2 findings, two LIKELY findings and four rejected hypotheses. Its scoped reconciliation already checked this continuation input SHA.

Four newly confirmed P2 findings have independent lead verification:

| ID | Confirmed effect | Evidence |
|---|---|---|
| GR-SOUL-001 | Valid promoted Premium soul above the free cap is clamped on fresh restoration; raising the later cap does not restore lost points | `verify-soul-cap.md` |
| CL-001 | Valid UseOn sources in open nested carried containers fail client freshness revalidation | `verify-client-item-actions.md` |
| CL-002 | Plain custom Use item hotkeys are rejected because their command has no world selection | `verify-client-item-actions.md` |
| PS-R2-001 | A remembered-device login integration path rejects a valid response | Independent source evidence retained privately under `SECURITY.md` |

Cumulative confirmed counts: **P0=0, P1=3, P2=7, P3=0**. Existing LIKELY=2 and rejected hypotheses=4 remain. Static confirmation does not claim a deployed incident or native end-to-end reproduction. Findings are deferred to separate implementation work; this is an audit-only branch.

## Navigation

- `coverage.md`, `coverage-ledger.json`: package/domain matrix, exact ranges, hashes, provenance and remaining files.
- `module-reports.json`: substantive package reports and report references; client-support module results are inline.
- `admin-review.md`, `admission-recovery-review.md`, `determinism-ops-review.md`: lead package reports.
- `protocol-session.md`, `platform-client.md`, `session.md`: completed session/platform packages.
- `characters.md`, `node-files.md`, `weather.md`, `source-critical-baseline.md`, `housing.md`, `economy.md`, `combat-rules.md`, `encounter-rules.md`: gameplay package reports.
- `inventory.json`, `carryover.json`, `coordination.json`, `snapshot-checks.json`, `validation.md`: exact inventory, transfer, allocation and checks.

## Continuation

Three read-only workers and the lead continue. Large client UI/app, protocol, gameplay owners, server, dispatch, durability, content and tool scopes retain explicit remaining files. The [central issue](https://github.com/Oteryn/Oteryn-Game/issues/3088) records the published exact head and allocation. No game source, main branch, PR, merge or production environment is changed.
