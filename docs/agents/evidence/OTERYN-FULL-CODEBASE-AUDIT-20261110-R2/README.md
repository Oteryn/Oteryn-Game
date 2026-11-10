# OTERYN FULL AUDIT — continuation checkpoint 27

Input base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. The audit continues across all remaining packages and supplementary domains.

## Verified evidence at this checkpoint

- 67 workspace packages; 11 supplementary domains; 27900 tracked input files.
- **67 packages REVIEWED / 0 IN PROGRESS / 0 NOT STARTED / 0 VERIFIED**.
- **8,438 distinct substantively analyzed files**: 8,409 full and 29 selected-range files; 2,087,237 deduplicated text line positions.
- Seven binary fixtures were semantically decoded in full, totaling 3,178 bytes. Their exact ranges use bytes and are excluded from line counts.
- 192 historical files retain their exact ranges only because their Git blobs are identical. Four changed historical files receive no automatic carryover credit; fresh review is attributed separately.
- REVIEWED requires every tracked package file and a substantive report. File discovery, text search, hash checks and test-source inspection are not runtime execution.

## Findings register

The [first published checkpoint](https://github.com/Oteryn/Oteryn-Game/tree/1c39ebc3347319fe08eaf58299a8a9e9339b0367/docs/agents/evidence/OTERYN-FULL-CODEBASE-AUDIT-20261110) retains three confirmed P1, three confirmed P2, two LIKELY findings and four rejected hypotheses.

Seventeen additional findings have independent lead verification:

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
| GR-KILL-001 | A delayed-effect completion can omit required reward settlement after a post-application cleanup failure; conditional integration defect, without a deployed-incident or durable-corruption claim | Restricted independent evidence retained privately under SECURITY.md |
| GR-EQUIP-001 | A legitimately upgraded weapon retains its durable tier but its supported equipment placement omits the expected primary auto-attack contribution; conditional source-level defect | Restricted independent evidence retained privately under SECURITY.md |
| R2-SUP-TOOLS-001 | Empty derived staging selection can include unrelated local worktree changes in the Git index; no automatic commit/push or production incident claimed | Restricted independent source and bounded Git-fixture evidence retained privately |

Cumulative confirmed counts: **P0=0 / P1=4 / P2=19 / P3=0**. Existing LIKELY=2 and earlier rejected hypotheses=4 remain. New unconfirmed hardening and integration candidates are excluded. Static confirmation does not claim a deployed incident, permanent reconnect failure or native end-to-end reproduction.

## Current main and scope

Fresh main reconfirmed at21:00:15 UTC during checkpoint27 preparation remains `c255c6b88dff9467deae5dfc7cd0ff1a0c8fb75f` (#3096, LOG-17), sole parent `43f48d1f3bf932bfaef6ca47334aa72117db6c81`. Its complete14-path successor delta is now **REVIEWED and independently root-qualified**:12 complete target files and2 complete deltas, totaling3,320 target lines plus27 cross-reference lines. The two delta files are not claimed as full target rereads. Return/panic preservation, held blocking permits, signature-before-decode, acknowledgement ordering and lexical-check limitations were examined. No new confirmed finding or Rust/native/SQL execution follows from this review. See `latest-main-log17-review.md` and its exact validation/check/root packets.

The prior main `43f48d1f3bf932bfaef6ca47334aa72117db6c81` retains its separately attributed117 successor-path reviews and historical reconciliation `latest-main-reconciliation-43f48.json`:64 final-commit full targets/19,875 lines, with another27 full targets and26 complete deltas/9,414 lines at that cutoff. Overlapping successor versions are not blindly summed as unique files. Immutable input counts and findings remain bound to `4f0b6c62a789489fca6c37dd36d5917d096f328e`.

The new report is `latest-main-protocol-delta.md`, with `latest-main-protocol-validation.json`, `latest-main-protocol-checks.json` and independent `latest-main-protocol-root-validation.json`. It covers optional connection-trace negotiation, terminal error handling, structured-log producers, task supervision and CI failure capture/pins. **Two original unchanged redactor tests passed**, and shell syntax plus bounded schema/pin/dependency checks passed. One stale diagnostic line label is an observation, not a failure of the actual count-based logger policy. No Rust/native/SQL/Actions execution is inferred.

Prior client/modal and Task Board reports and their root validations remain included, with **three original unchanged modal codec tests passed**. All successor reviews establish no additional confirmed finding and do not automatically close existing findings. Historical inventory remains marked as such; current completion is in `latest-main-reconciliation.json`. The prior HUD/diagnostic reviews retain their exact version attribution. Primary counts and findings remain tied to `4f0b6c62a789489fca6c37dd36d5917d096f328e`; source review is distinct from current deployment or runtime qualification.

The delayed-effect finding has a public bounded summary in `delayed-settlement-review-summary.md`; its detailed source/producer proof remains private.

The Forge/equipment finding has a bounded public summary in `forge-equipment-review-summary.md`; its maintained producer and complete integration proof remain private.

## Reports

- `coverage.md`, `coverage-ledger.json`: package/domain status, exact ranges, hashes, provenance and explicit remaining files.
- `module-reports.json`: completed package reports; client-support reports are inline.
- Lead: `admin-review.md`, `admission-recovery-review.md`, `determinism-ops-review.md`, `world-bundle-review.md`, `dev-client-bots-review.md`, `architecture-review.md`, `world-compiler-review.md`, `domain-review.md`, `combat-review.md`, `entities-review.md`, `progression-review.md`, `monster-ai-review.md`.
- Gameplay foundation, content promotion and native admission source: `foundation.md`, `content-promotion.md`, `native-admission-source.md`.
- Complete spell source: `spell-review.md`, `spell-coverage-validation.json`, `spell-embedded-source-validation.json`; complete content: `content.md`, `content-completion-validation.json`; complete items: `items-review.md`, `items-coverage-validation.json`, `items-imbuement-unchanged-tests.json`.
- Complete encounters: `encounters-review.md`, `encounters-coverage-validation.json`, `encounters-entry-unchanged-tests.json`; complete dispatch: `dispatch-progress.md`; complete quests: `quests-review.md`, `quests-coverage-validation.json`, `quests-calendar-unchanged-tests.json`.
- Complete social: `social-review.md`, `social-coverage-validation.json`, `social-rate-window-unchanged-tests.json`; Task Board durable-owner closure: `task-board-owner-links.md`, `task-board-owner-links.json`, `task-board-owner-validation.json`.
- Complete World: `world-review.md`, `world-root-validation.json`, three `world-*-unchanged-tests.json` records, floor validation and immovable method map/semantic validation; delegated capture qualification: `capture-validation-27.json` and seven `game-server-*-method-map.md` files.
- Native client application: `apps-client.md`; 1,180 full files, with unresolved candidates separated from confirmed findings.
- Protocol/session: `protocol-session.md`, `platform-client.md`, `session.md`, `protocol.md`; complete `durability-review.md` and `durability-completion-validation.json`; complete `game-server-review.md` and `game-server-completion-root-validation.json`; complete `harness-review.md` and `harness-coverage-validation.json`.
- Gameplay: `characters.md`, `node-files.md`, `weather.md`, `source-critical-baseline.md`, `housing.md`, `economy.md`, `combat-rules.md`, `encounter-rules.md`, `social-rules.md`, `spell-rules.md`, `world-change.md`, `npc.md`.
- Supplementary root/deploy/tests: `supplementary-core-review.md`; complete repository automation: `github-review.md`; partial tools/configuration: `supplementary-tools-review.md`.
- `findings-summary.json`, `inventory.json`, `carryover.json`, `coordination.json`, `snapshot-checks.json`, `validation.md`: findings, scope, allocation and verification.

## Continuation

Protocol retains full REVIEWED status:596 files,81,639 text lines and two binary fixtures totaling302bytes. Its durability, server and original social/world and items/economy shares are complete; both separate99-file Dispatch quest-test assignments are now complete and root-qualified. The resumed agents continue separate supplementary rulesets/content and tools/configuration assignments.

The client application is now **REVIEWED: 1,180/1,180 full files and 143,488 lines**. Its delegated 422-file batch is complete and parent-validated. All 1,334 client-ui files retain full review. Gameplay foundation is now **REVIEWED: 55/55 files and 38,117 lines**; content-promotion is **REVIEWED: 36/36 files and 13,600 lines**. The promotion packet checks passed 106 read-only assertions, including all 34 embedded byte pins. Native-admission-source is now **REVIEWED: 81/81 files and 21,163 lines**; 267 complete local Store fixture/schema checks passed. The client lane completed content, its original304-file Dispatch reservation and all44 repository-root/deploy/tests supplementary files; it now reviews the separate imports/vendor domains.

The progression package is now **REVIEWED: 43/43 files and 11,169 lines**. Its report covers achievement/classifier boundaries, training and loyalty, ranking, daily rewards, regeneration, proficiency, Wheel/Atelier and Charm effects. Exact Git/worktree/full-range validation passed. Its test sources were read, not executed.

Entities retains **86/86 full files and 37,547 lines**. Eight existing tests in its unchanged spawn scheduler passed the earlier direct rustc run. GR-ENT-003 subsequently passed independent actual-caller and counterguard review; no native reproduction is claimed. Two separate periodic-processing candidates remain unconfirmed.

Monster AI is now **REVIEWED: 64/64 files and 17,496 lines**. Seven existing pure AI tests passed through an unchanged path-only rustc harness; the native runtime and host benchmark were not run. A catalog/caller compatibility question remains private and unconfirmed.

Spell is now **REVIEWED: 83/83 files and 41,011 lines**, including every source and test file. Two embedded JSON literal lines were completely semantically analyzed against selected native closures: **817 read-only checks PASS**. This is data/source verification, not Cargo/native/SQL execution. GR-SOUL-001 has an additional independently confirmed live level-up path under the same existing P2 ID; the report and exact source evidence are updated.

Items is now **REVIEWED: 59/59 files and 15,160 lines**. Review includes the complete Item planners, custody boundary contracts, loot/reward/container rules, equipment, imbuement, forge, cosmetics, mail and associated tests. Four existing imbuement tests passed through an unchanged-source path-only rustc harness. Other test sources were read, not executed. Actual boss-reward materialization counterguards refute the local ordinal hypothesis; remaining producer/policy questions stay unconfirmed.

Content is now **REVIEWED: 139/139 files and 71,505 line-accounting units**. Its complete Rust, manifest, JSON and binary assets were reviewed with source/reference closure and real consumers. Read-only validation includes all 5,095 market filter records, 233 travel records with 3,695 checks, both native room assets with 1,184 checks and the full land mask. CL-018 received independent root confirmation of the complete actual marriage-to-appearance path and maintained content mismatch; its bounded public proof is included.

Encounters is now **REVIEWED: 93/93 files and 22,624 lines**. The complete source, manifest and test bodies were read, and exact Git/worktree/full-range checks passed. Review traces pure rules, native physical state, map-dependent callbacks, timers, child lifecycle and actual boss entry/party prerequisites. Six original endgame-entry tests passed through an unchanged-source path-only rustc harness. Other native/source test bodies were inspected, not executed. No additional confirmed finding was established.

Frozen durability remains **721/721 full files / 216,548 lines — REVIEWED**. Game-server is now **REVIEWED: 873/873 files / 489,700 text lines / 2,703 binary bytes**, comprising **862 manual and 11 mixed semantic reviews**. All migrations, source, tests/support, examples and package root files are covered. The final substantive report, independent root qualification and earlier method maps distinguish source/data review from execution.

Frozen Dispatch coverage is **1,668/1,668 full files and517,542 deduplicated lines — REVIEWED**, with no remaining package files. Final quest-test captures18 and15 each contain99 full files; all original reservations and the root158 share are complete. Root read all198 quest notes across checkpoints,39 supporting notes and final reports, then qualified exact assignment/history/source/attribution. See `capture-validation-27.json`, `dispatch-progress.md` and the root implementation report. No runtime execution or second complete manual reread of delegated sources is claimed.

Supplementary repository-root16/16, deploy16/16 and tests12/12 are **REVIEWED**, totaling44 files and9,938 lines. Every file has a full semantic note; root checked the complete packet. Bounded workspace/lock/fixture/staging consistency checks and unexecuted runtime limits are stated in `supplementary-core-review.md`. Historical NAS format/pin drift remains qualified by the explicit retirement instruction; active host impact is unknown.

The synthetic client harness is now **REVIEWED: 40/40 files / 10,454 lines**, all manually analyzed and independently checked for exact Git/worktree/full-range closure. Entry/CLI/controller/model and every native/qualification test source are included. Scripted TLS, duplex and mocked Stepper peers are explicitly distinguished from real gameplay owners, database transactions and actual test execution. The replacement-chat producer question remains unconfirmed. No harness test was executed.

World remains **REVIEWED: 73/73 files / 28,966 lines**, with 72 manual and one documented mixed semantic review, 29 original unchanged-source Rust tests and the original ladder check passed. Quests remains **103/103 files / 30,058 lines** with three original calendar tests passed; Social remains **56/56 / 11,871 lines** with one original rate-window test passed. Their reports and exact version attribution remain retained.

All **67 Cargo packages are REVIEWED**. Supplementary domains are **4 REVIEWED /5 IN PROGRESS /2 NOT STARTED** at this exact cutoff; all11 and all27,900 input files remain mandatory. Completed domains are repository-root, deploy, tests and.github. Root read all75/75 .github files (12,301lines). Qualified supplementary tools/configuration capture06 contains130full files/25,330lines with129manual source reviews and one whole-corpus semantic map. Root read all130 primary notes/15supporting notes, reports and method maps, then verified source/assignment/capture identities; this is not a second complete manual reread of all delegated sources. Imports, vendor, rulesets and content work continues and later output is excluded until qualified.

Confirmed counts are now **P0=0 /P1=4 /P2=19 /P3=0**. New R2-SUP-TOOLS-001 is conditional local index behavior with an independently reproduced empty-selection route and two counterguard cases. Production regeneration and an accidental user commit are not claimed. Restricted proof stays private. No SQL, full Cargo, native Windows, live server or deployment execution is claimed.

The [central issue](https://github.com/Oteryn/Oteryn-Game/issues/3088) records the actual published head and lifecycle. Repairs remain separate work; this branch changes audit evidence only.
