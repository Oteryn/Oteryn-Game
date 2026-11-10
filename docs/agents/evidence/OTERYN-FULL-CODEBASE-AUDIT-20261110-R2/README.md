# OTERYN FULL AUDIT — continuation checkpoint 3

Input base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. The audit continues across all remaining packages and supplementary domains.

## Verified evidence at this checkpoint

- 67 workspace packages; 11 supplementary domains; 27900 tracked input files.
- **44 packages REVIEWED / 21 IN PROGRESS / 2 NOT STARTED / 0 VERIFIED**.
- **2,182 distinct substantively analyzed files**: 1,957 full and 225 selected-range files; 200,286 deduplicated text line positions.
- Two binary fixtures were semantically decoded in full, totaling 302 bytes. Their exact ranges use bytes and are excluded from line counts.
- 192 historical files retain their exact ranges only because their Git blobs are identical. Four changed historical files receive no automatic carryover credit; fresh review is attributed separately.
- REVIEWED requires every tracked package file and a substantive report. File discovery, text search, hash checks and test-source inspection are not runtime execution.

## Findings register

The [first published checkpoint](https://github.com/Oteryn/Oteryn-Game/tree/1c39ebc3347319fe08eaf58299a8a9e9339b0367/docs/agents/evidence/OTERYN-FULL-CODEBASE-AUDIT-20261110) retains three confirmed P1, three confirmed P2, two LIKELY findings and four rejected hypotheses.

Six additional P2 findings have independent lead verification:

| ID | Confirmed bounded effect | Evidence |
|---|---|---|
| GR-SOUL-001 | Fresh restoration clamps valid promoted Premium soul above the free cap before the current cap is applied | `verify-soul-cap.md` |
| CL-001 | Valid UseOn sources in open nested carried containers fail client freshness revalidation | `verify-client-item-actions.md` |
| CL-002 | Plain custom Use item hotkeys are rejected because their command has no world selection | `verify-client-item-actions.md` |
| CL-003 | Three Polish movement-spell controls send translated words that do not match the canonical spells | `verify-localized-movement.md` |
| PS-R2-001 | A remembered-device login integration path rejects a valid response | Restricted independent evidence retained privately under SECURITY.md |
| PS-R2-003 | Accepted House entry can make the native Session unusable because the producer sends ambient state before the required destination snapshot | `verify-house-handoff.md` |

Cumulative confirmed counts: **P0=0 / P1=3 / P2=9 / P3=0**. Existing LIKELY=2 and earlier rejected hypotheses=4 remain. New unconfirmed hardening and integration candidates are excluded. Static confirmation does not claim a deployed incident, permanent reconnect failure or native end-to-end reproduction.

## Current main and scope

Fresh main is `af33fd3baeac15d9991bd24c91bcacc8733492e2`. Its complete diff from the input is four client startup-diagnostic files. All other input blobs are identical. The client lane owns a separate review of those changed files; no automatic credit or completed current-main claim is made. See `latest-main-reconciliation.json`.

## Reports

- `coverage.md`, `coverage-ledger.json`: package/domain status, exact ranges, hashes, provenance and explicit remaining files.
- `module-reports.json`: completed package reports; client-support reports are inline.
- Lead: `admin-review.md`, `admission-recovery-review.md`, `determinism-ops-review.md`, `world-bundle-review.md`, `dev-client-bots-review.md`, `architecture-review.md`.
- Protocol/session: `protocol-session.md`, `platform-client.md`, `session.md`.
- Gameplay: `characters.md`, `node-files.md`, `weather.md`, `source-critical-baseline.md`, `housing.md`, `economy.md`, `combat-rules.md`, `encounter-rules.md`, `social-rules.md`, `spell-rules.md`, `world-change.md`, `npc.md`.
- `findings-summary.json`, `inventory.json`, `carryover.json`, `coordination.json`, `snapshot-checks.json`, `validation.md`: findings, scope, allocation and verification.

## Continuation

Three read-only workers and the lead continue. Large client UI/app, protocol, gameplay owners, server, dispatch, durability, content and tool scopes retain explicit remaining paths. The [central issue](https://github.com/Oteryn/Oteryn-Game/issues/3088) records the actual published head and current lifecycle. Repairs are separate work; this report branch changes audit evidence only.
