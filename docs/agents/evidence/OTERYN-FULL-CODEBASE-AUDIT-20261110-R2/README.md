# OTERYN FULL AUDIT — continuation checkpoint 6

Input base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. The audit continues across all remaining packages and supplementary domains.

## Verified evidence at this checkpoint

- 67 workspace packages; 11 supplementary domains; 27900 tracked input files.
- **49 packages REVIEWED / 17 IN PROGRESS / 1 NOT STARTED / 0 VERIFIED**.
- **3,663 distinct substantively analyzed files**: 3,437 full and 226 selected-range files; 424,323 deduplicated text line positions.
- Four binary fixtures were semantically decoded in full, totaling 475 bytes. Their exact ranges use bytes and are excluded from line counts.
- 192 historical files retain their exact ranges only because their Git blobs are identical. Four changed historical files receive no automatic carryover credit; fresh review is attributed separately.
- REVIEWED requires every tracked package file and a substantive report. File discovery, text search, hash checks and test-source inspection are not runtime execution.

## Findings register

The [first published checkpoint](https://github.com/Oteryn/Oteryn-Game/tree/1c39ebc3347319fe08eaf58299a8a9e9339b0367/docs/agents/evidence/OTERYN-FULL-CODEBASE-AUDIT-20261110) retains three confirmed P1, three confirmed P2, two LIKELY findings and four rejected hypotheses.

Ten additional findings have independent lead verification:

| ID | Confirmed bounded effect | Evidence |
|---|---|---|
| GR-SOUL-001 | Fresh restoration clamps valid promoted Premium soul above the free cap before the current cap is applied | `verify-soul-cap.md` |
| CL-001 | Valid UseOn sources in open nested carried containers fail client freshness revalidation | `verify-client-item-actions.md` |
| CL-002 | Plain custom Use item hotkeys are rejected because their command has no world selection | `verify-client-item-actions.md` |
| CL-003 | Three Polish movement-spell controls send translated words that do not match the canonical spells | `verify-localized-movement.md` |
| PS-R2-001 | A remembered-device login integration path rejects a valid response | Restricted independent evidence retained privately under SECURITY.md |
| PS-R2-003 | Accepted House entry can make the native Session unusable because the producer sends ambient state before the required destination snapshot | `verify-house-handoff.md` |
| GR-ECON-001 | Economy invariant defect with a verified gain under its stated service and capacity preconditions; no deployed incident or unbounded throughput claim | Restricted independent evidence retained privately under SECURITY.md |
| CL-004 | Retrying a frozen Store request can display confirmation terms from a replacement catalogue | `verify-client-store-retry.md` |
| CL-005 | The native VIP presence window sets its open flag before its opener and consequently refuses to create the window | `verify-client-windows.md` |
| CL-006 | A locally entered guild name above the encoder bound ends the native client Session loop | `verify-guild-input.md` |

Cumulative confirmed counts: **P0=0 / P1=4 / P2=12 / P3=0**. Existing LIKELY=2 and earlier rejected hypotheses=4 remain. New unconfirmed hardening and integration candidates are excluded. Static confirmation does not claim a deployed incident, permanent reconnect failure or native end-to-end reproduction.

## Current main and scope

Fresh main is `af33fd3baeac15d9991bd24c91bcacc8733492e2`. Its complete diff from the input is four client startup-diagnostic files. All other input blobs are identical. All four changed files were separately reviewed in full; the lead verified their exact latest-main hashes and complete ranges. The primary package counts remain bound to the immutable input. See `latest-main-reconciliation.json` and `latest-main-delta.md`; native Windows/installer execution was not performed.

## Reports

- `coverage.md`, `coverage-ledger.json`: package/domain status, exact ranges, hashes, provenance and explicit remaining files.
- `module-reports.json`: completed package reports; client-support reports are inline.
- Lead: `admin-review.md`, `admission-recovery-review.md`, `determinism-ops-review.md`, `world-bundle-review.md`, `dev-client-bots-review.md`, `architecture-review.md`, `world-compiler-review.md`, `domain-review.md`, `combat-review.md`; partial `entities-progress.md`.
- Protocol/session: `protocol-session.md`, `platform-client.md`, `session.md`, `protocol.md`; partial `durability-progress.md` and `game-server-progress.md`.
- Gameplay: `characters.md`, `node-files.md`, `weather.md`, `source-critical-baseline.md`, `housing.md`, `economy.md`, `combat-rules.md`, `encounter-rules.md`, `social-rules.md`, `spell-rules.md`, `world-change.md`, `npc.md`.
- `findings-summary.json`, `inventory.json`, `carryover.json`, `coordination.json`, `snapshot-checks.json`, `validation.md`: findings, scope, allocation and verification.

## Continuation

Protocol is now REVIEWED in full: 596 files, 81,639 text lines and two binary fixtures totaling 302 bytes. Its worker continues full durability and game-server scope. The client lane and delegated worker continue the application; frozen coverage and remaining paths are in the ledger. Gameplay domain, combat and all 1,334 client-ui files retain completed source review. The lead is reviewing actor and authority owners, remaining gameplay packages and supplementary scope after taking over the prior gameplay worker's unfinished allocation.

Two earlier combat cross-module hypotheses were refuted by actual caller and durable-owner inspection. The confirmed finding counts are unchanged; new candidates await independent verification. The [central issue](https://github.com/Oteryn/Oteryn-Game/issues/3088) records the actual published head and lifecycle. Repairs remain separate work; this branch changes audit evidence only.
