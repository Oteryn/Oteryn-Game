# OTERYN FULL AUDIT — continuation checkpoint 9

Input base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. The audit continues across all remaining packages and supplementary domains.

## Verified evidence at this checkpoint

- 67 workspace packages; 11 supplementary domains; 27900 tracked input files.
- **52 packages REVIEWED / 14 IN PROGRESS / 1 NOT STARTED / 0 VERIFIED**.
- **4,542 distinct substantively analyzed files**: 4,342 full and 200 selected-range files; 635,142 deduplicated text line positions.
- Four binary fixtures were semantically decoded in full, totaling 475 bytes. Their exact ranges use bytes and are excluded from line counts.
- 192 historical files retain their exact ranges only because their Git blobs are identical. Four changed historical files receive no automatic carryover credit; fresh review is attributed separately.
- REVIEWED requires every tracked package file and a substantive report. File discovery, text search, hash checks and test-source inspection are not runtime execution.

## Findings register

The [first published checkpoint](https://github.com/Oteryn/Oteryn-Game/tree/1c39ebc3347319fe08eaf58299a8a9e9339b0367/docs/agents/evidence/OTERYN-FULL-CODEBASE-AUDIT-20261110) retains three confirmed P1, three confirmed P2, two LIKELY findings and four rejected hypotheses.

Eleven additional findings have independent lead verification:

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
| GR-ENT-003 | The first ordinary home-spawn deadline omits the installed selected-creature DailyBoost reduction | `verify-spawn-modifiers.md` |

Cumulative confirmed counts: **P0=0 / P1=4 / P2=13 / P3=0**. Existing LIKELY=2 and earlier rejected hypotheses=4 remain. New unconfirmed hardening and integration candidates are excluded. Static confirmation does not claim a deployed incident, permanent reconnect failure or native end-to-end reproduction.

## Current main and scope

Fresh main is `af33fd3baeac15d9991bd24c91bcacc8733492e2`. Its complete diff from the input is four client startup-diagnostic files. All other input blobs are identical. All four changed files were separately reviewed in full; the lead verified their exact latest-main hashes and complete ranges. The primary package counts remain bound to the immutable input. See `latest-main-reconciliation.json` and `latest-main-delta.md`; native Windows/installer execution was not performed.

## Reports

- `coverage.md`, `coverage-ledger.json`: package/domain status, exact ranges, hashes, provenance and explicit remaining files.
- `module-reports.json`: completed package reports; client-support reports are inline.
- Lead: `admin-review.md`, `admission-recovery-review.md`, `determinism-ops-review.md`, `world-bundle-review.md`, `dev-client-bots-review.md`, `architecture-review.md`, `world-compiler-review.md`, `domain-review.md`, `combat-review.md`, `entities-review.md`, `progression-review.md`.
- Native client application: `apps-client.md`; 1,180 full files, with unresolved candidates separated from confirmed findings.
- Protocol/session: `protocol-session.md`, `platform-client.md`, `session.md`, `protocol.md`; partial `durability-progress.md` and `game-server-progress.md`.
- Gameplay: `characters.md`, `node-files.md`, `weather.md`, `source-critical-baseline.md`, `housing.md`, `economy.md`, `combat-rules.md`, `encounter-rules.md`, `social-rules.md`, `spell-rules.md`, `world-change.md`, `npc.md`.
- `findings-summary.json`, `inventory.json`, `carryover.json`, `coordination.json`, `snapshot-checks.json`, `validation.md`: findings, scope, allocation and verification.

## Continuation

Protocol retains full REVIEWED status: 596 files, 81,639 text lines and two binary fixtures totaling 302 bytes. The protocol lane continues durability; the released client worker now reviews game-server and migrations. New worker output enters coverage only after exact hash/range validation and integration.

The client application is now **REVIEWED: 1,180/1,180 full files and 143,488 lines**. Its delegated 422-file batch is complete and parent-validated. All 1,334 client-ui files retain full review. The client lane now continues gameplay foundation and content-promotion.

The progression package is now **REVIEWED: 43/43 files and 11,169 lines**. Its report covers achievement/classifier boundaries, training and loyalty, ranking, daily rewards, regeneration, proficiency, Wheel/Atelier and Charm effects. Exact Git/worktree/full-range validation passed. Its test sources were read, not executed.

Entities retains **86/86 full files and 37,547 lines**. Eight existing tests in its unchanged spawn scheduler passed the earlier direct rustc run. GR-ENT-003 subsequently passed independent actual-caller and counterguard review; no native reproduction is claimed. Two separate periodic-processing candidates remain unconfirmed.

Frozen durability cumulative coverage is **163/721 complete files**, 194 files with substantive ranges and **70,133 lines**. Frozen game-server cumulative coverage is **134/873 complete files**, 148 files with substantive ranges and **41,122 lines**. These totals include attributed earlier cross-references. The server worker's 121-file capture was independently validated before parent integration; later live progress is not credited to this frozen checkpoint.

The lead continues remaining gameplay and supplementary scope. Existing restricted findings, duplicate corroboration and new candidates remain separately tracked. No source fixes or deployed-system reproduction are claimed.

The [central issue](https://github.com/Oteryn/Oteryn-Game/issues/3088) records the actual published head and lifecycle. Repairs remain separate work; this branch changes audit evidence only.
