# OTERYN FULL AUDIT — continuation checkpoint 20

Input base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. The audit continues across all remaining packages and supplementary domains.

## Verified evidence at this checkpoint

- 67 workspace packages; 11 supplementary domains; 27900 tracked input files.
- **64 packages REVIEWED / 3 IN PROGRESS / 0 NOT STARTED / 0 VERIFIED**.
- **7,030 distinct substantively analyzed files**: 6,928 full and 102 selected-range files; 1,547,542 deduplicated text line positions.
- Seven binary fixtures were semantically decoded in full, totaling 3,178 bytes. Their exact ranges use bytes and are excluded from line counts.
- 192 historical files retain their exact ranges only because their Git blobs are identical. Four changed historical files receive no automatic carryover credit; fresh review is attributed separately.
- REVIEWED requires every tracked package file and a substantive report. File discovery, text search, hash checks and test-source inspection are not runtime execution.

## Findings register

The [first published checkpoint](https://github.com/Oteryn/Oteryn-Game/tree/1c39ebc3347319fe08eaf58299a8a9e9339b0367/docs/agents/evidence/OTERYN-FULL-CODEBASE-AUDIT-20261110) retains three confirmed P1, three confirmed P2, two LIKELY findings and four rejected hypotheses.

Fourteen additional findings have independent lead verification:

| ID | Confirmed bounded effect | Evidence |
|---|---|---|
| GR-SOUL-001 | Fresh restoration, bed wake and a separately verified live level-up path clamp valid promoted/current-Premium Soul before the current cap is applied | `verify-soul-cap.md` |
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
| CL-016 | An ordinary periodic condition pass removes a permanent environmental appearance before the player exits the tile | `verify-permanent-appearance.md` |
| CL-018 | The actual sex-qualified appearance path omits the correctly earned permanent Newly Wed grant | `verify-marriage-outfit.md` |
| PS-R2-010 | A supported state transition prevents a subsequent feature-state refresh; bounded integration effect, without a permanent account-lock claim | Restricted independent evidence retained privately under SECURITY.md |

Cumulative confirmed counts: **P0=0 / P1=4 / P2=16 / P3=0**. Existing LIKELY=2 and earlier rejected hypotheses=4 remain. New unconfirmed hardening and integration candidates are excluded. Static confirmation does not claim a deployed incident, permanent reconnect failure or native end-to-end reproduction.

## Current main and scope

Fresh main observed at 17:40:42 UTC remains `43f48d1f3bf932bfaef6ca47334aa72117db6c81`. It contains **133 changed paths from the immutable audit input** and **117 successor paths since the separately reviewed b028 main**, across #3090 and #3091. **All 64 files of the final commit are now reviewed separately**: 54 client/modal files and 10 Task Board files, totaling **19,875 lines**. This comprises 57 manual source/test/contract reviews and seven explicitly documented localization data reviews. Exact Git bytes, complete ranges and the disjoint full-commit path partition passed root validation. The **53 preceding successor paths remain IN PROGRESS** with a separate delta ledger; they are not covered by inspecting the last commit alone.

See `latest-main-client-gaps-delta.md`, `latest-main-client-gaps-root-validation.json`, `latest-main-client-gaps-locales-validation.json`, `latest-main-task-board-review.md` and `latest-main-task-board-validation.json`. Three original unchanged server-modal codec tests passed in an isolated rustc harness. Native Windows, PostgreSQL and actual async transport were not executed. This review establishes no additional confirmed finding and does not close CL-001, CL-002, CL-003 or PS-R2-003. The initial `latest-main-43f48-inventory.json` remains historical inventory; current completion is in `latest-main-reconciliation.json`.

The prior 18-file HUD/manifest and four-file diagnostic reviews remain valid for their recorded versions. Their unchanged-source test and semantic locale checks are retained. No Windows launch, linker, installer, SQL or native cleanup execution was performed. Primary counts and findings remain tied to `4f0b6c62a789489fca6c37dd36d5917d096f328e` until successor effects are independently assessed.

## Reports

- `coverage.md`, `coverage-ledger.json`: package/domain status, exact ranges, hashes, provenance and explicit remaining files.
- `module-reports.json`: completed package reports; client-support reports are inline.
- Lead: `admin-review.md`, `admission-recovery-review.md`, `determinism-ops-review.md`, `world-bundle-review.md`, `dev-client-bots-review.md`, `architecture-review.md`, `world-compiler-review.md`, `domain-review.md`, `combat-review.md`, `entities-review.md`, `progression-review.md`, `monster-ai-review.md`.
- Gameplay foundation, content promotion and native admission source: `foundation.md`, `content-promotion.md`, `native-admission-source.md`.
- Complete spell source: `spell-review.md`, `spell-coverage-validation.json`, `spell-embedded-source-validation.json`; complete content: `content.md`, `content-completion-validation.json`; complete items: `items-review.md`, `items-coverage-validation.json`, `items-imbuement-unchanged-tests.json`.
- Complete encounters: `encounters-review.md`, `encounters-coverage-validation.json`, `encounters-entry-unchanged-tests.json`; partial dispatch: `dispatch-progress.md`; complete quests: `quests-review.md`, `quests-coverage-validation.json`, `quests-calendar-unchanged-tests.json`.
- Complete social: `social-review.md`, `social-coverage-validation.json`, `social-rate-window-unchanged-tests.json`; Task Board durable-owner closure: `task-board-owner-links.md`, `task-board-owner-links.json`, `task-board-owner-validation.json`.
- Complete World: `world-review.md`, `world-root-validation.json`, three `world-*-unchanged-tests.json` records, floor validation and immovable method map/semantic validation; delegated capture qualification: `capture-validation-20.json` and seven `game-server-*-method-map.md` files.
- Native client application: `apps-client.md`; 1,180 full files, with unresolved candidates separated from confirmed findings.
- Protocol/session: `protocol-session.md`, `platform-client.md`, `session.md`, `protocol.md`; complete `durability-review.md` and `durability-completion-validation.json`; partial `game-server-progress.md` and `harness-progress.md`.
- Gameplay: `characters.md`, `node-files.md`, `weather.md`, `source-critical-baseline.md`, `housing.md`, `economy.md`, `combat-rules.md`, `encounter-rules.md`, `social-rules.md`, `spell-rules.md`, `world-change.md`, `npc.md`.
- `findings-summary.json`, `inventory.json`, `carryover.json`, `coordination.json`, `snapshot-checks.json`, `validation.md`: findings, scope, allocation and verification.

## Continuation

Protocol retains full REVIEWED status: 596 files, 81,639 text lines and two binary fixtures totaling 302 bytes. The protocol lane has completed durability and continues the separate 53-path successor delta; its server worker continues the remaining tests and examples. New worker output enters coverage only after exact hash/range validation and integration.

The client application is now **REVIEWED: 1,180/1,180 full files and 143,488 lines**. Its delegated 422-file batch is complete and parent-validated. All 1,334 client-ui files retain full review. Gameplay foundation is now **REVIEWED: 55/55 files and 38,117 lines**; content-promotion is **REVIEWED: 36/36 files and 13,600 lines**. The promotion packet checks passed 106 read-only assertions, including all 34 embedded byte pins. Native-admission-source is now **REVIEWED: 81/81 files and 21,163 lines**; 267 complete local Store fixture/schema checks passed. The client lane has completed content and continues dispatch.

The progression package is now **REVIEWED: 43/43 files and 11,169 lines**. Its report covers achievement/classifier boundaries, training and loyalty, ranking, daily rewards, regeneration, proficiency, Wheel/Atelier and Charm effects. Exact Git/worktree/full-range validation passed. Its test sources were read, not executed.

Entities retains **86/86 full files and 37,547 lines**. Eight existing tests in its unchanged spawn scheduler passed the earlier direct rustc run. GR-ENT-003 subsequently passed independent actual-caller and counterguard review; no native reproduction is claimed. Two separate periodic-processing candidates remain unconfirmed.

Monster AI is now **REVIEWED: 64/64 files and 17,496 lines**. Seven existing pure AI tests passed through an unchanged path-only rustc harness; the native runtime and host benchmark were not run. A catalog/caller compatibility question remains private and unconfirmed.

Spell is now **REVIEWED: 83/83 files and 41,011 lines**, including every source and test file. Two embedded JSON literal lines were completely semantically analyzed against selected native closures: **817 read-only checks PASS**. This is data/source verification, not Cargo/native/SQL execution. GR-SOUL-001 has an additional independently confirmed live level-up path under the same existing P2 ID; the report and exact source evidence are updated.

Items is now **REVIEWED: 59/59 files and 15,160 lines**. Review includes the complete Item planners, custody boundary contracts, loot/reward/container rules, equipment, imbuement, forge, cosmetics, mail and associated tests. Four existing imbuement tests passed through an unchanged-source path-only rustc harness. Other test sources were read, not executed. Actual boss-reward materialization counterguards refute the local ordinal hypothesis; remaining producer/policy questions stay unconfirmed.

Content is now **REVIEWED: 139/139 files and 71,505 line-accounting units**. Its complete Rust, manifest, JSON and binary assets were reviewed with source/reference closure and real consumers. Read-only validation includes all 5,095 market filter records, 233 travel records with 3,695 checks, both native room assets with 1,184 checks and the full land mask. CL-018 received independent root confirmation of the complete actual marriage-to-appearance path and maintained content mismatch; its bounded public proof is included.

Encounters is now **REVIEWED: 93/93 files and 22,624 lines**. The complete source, manifest and test bodies were read, and exact Git/worktree/full-range checks passed. Review traces pure rules, native physical state, map-dependent callbacks, timers, child lifecycle and actual boss entry/party prerequisites. Six original endgame-entry tests passed through an unchanged-source path-only rustc harness. Other native/source test bodies were inspected, not executed. No additional confirmed finding was established.

Frozen durability coverage is **721/721 full files / 216,548 lines — REVIEWED**, with zero remaining package paths. Frozen game-server coverage is **771/873 full files**, 772 with ranges, **387,581 text lines** and **2,703 binary bytes**. Frozen dispatch coverage is **579/1668 full files**, 657 with ranges and **135,436 lines**. Stable captures contain durability 721, server 771 and dispatch 559 full files. Every captured full file is included; all attributed ranges are deduplicated. The synthetic client harness has **10/40 full files / 4,055 lines**; its complete entry/CLI/controller/model and main test source are reviewed, with no test execution.

The server capture separates **760 manual and 11 mixed semantic reviews**, with seven complete method maps and an additional manual range map for the full durability PostgreSQL suite. Five inline TLS credential arrays received complete structural and cryptographic review as part of one text file, with no extra file/byte credit or handshake execution. Root independently qualified all captures and read the method maps; it did not repeat every delegated manual review.

World is now **REVIEWED: 73/73 files / 28,966 lines**, comprising 72 manual reviews and one documented mixed semantic catalogue. Six historical movement-helper reviews retain original identical-blob attribution. Complete map, bootstrap, floor, collision, movement, speed, interest, path search, terrain, object runtime and reversion sources and tests are covered. Exact Git/worktree/full ranges passed. **29 original unchanged-source Rust tests passed**, plus the original ladder catalogue check. Full immovable membership/XML comparisons are documented; absent original appearance bytes leave external donor regeneration unknown.

Quests remains **REVIEWED: 103/103 files / 30,058 lines**, with three original calendar tests passed. Social remains **REVIEWED: 56/56 files / 11,871 lines**, with one original rate-window test passed. Their separately assigned main changes are not assumed reviewed at the successor version.

All 67 packages have substantive coverage; three remain incomplete: dispatch, game-server and synthetic client harness. Supplementary domains remain **5 IN PROGRESS / 6 NOT STARTED**; all eleven remain mandatory. Remaining dispatch work has four disjoint agent assignments, with server and successor reviewers moving to their reserved slices after their current packages. Confirmed counts stay **P0=0 / P1=4 / P2=16 / P3=0**. Restricted proof stays private. No SQL, full Cargo, TLS handshake, native Windows, live server or deployment execution is claimed.

The [central issue](https://github.com/Oteryn/Oteryn-Game/issues/3088) records the actual published head and lifecycle. Repairs remain separate work; this branch changes audit evidence only.
