# Workspace inventory and coverage

Base: `e2e5ced306040113dc9c395251a802f50eb5776a`. Coordination: [#3088](https://github.com/Oteryn/Oteryn-Game/issues/3088).

This is a partial behavioral audit. The inventory covers the entire tracked tree; behavioral analysis covers only the exact source ranges in [coverage-ledger.json](coverage-ledger.json). No arbitrary coverage percentage is used.

**67 Cargo packages**, plus **11 supplemental domains**, account for all **27,776 tracked files**. **196 unique files** have substantive analysis: **96 full files** and **100 selected-range files**. The deduplicated line union is **40,516**; this is an evidence index, not a semantic coverage percentage.

Package status: **3 REVIEWED**, **25 IN PROGRESS**, **39 NOT STARTED**, **0 VERIFIED**. Supplemental domains: **2 IN PROGRESS**, **9 NOT STARTED**. REVIEWED here means completed source review for the small package, not a native integration or production qualification. The interrupted session/protocol worker has a BLOCKED lane; its unfinished work is not silently counted.

| Module | Files | Rust | SQL | Analyzed | Full files | Status |
|---|---:|---:|---:|---:|---:|---|
| oteryn-client | 1164 | 1041 | 0 | 0 | 0 | NOT STARTED |
| oteryn-game-server | 867 | 427 | 414 | 22 | 12 | IN PROGRESS |
| oteryn-admin-agent | 12 | 11 | 0 | 0 | 0 | NOT STARTED |
| oteryn-admin-contracts | 7 | 6 | 0 | 0 | 0 | NOT STARTED |
| oteryn-client-assets | 19 | 18 | 0 | 1 | 1 | IN PROGRESS |
| oteryn-client-domain | 4 | 3 | 0 | 0 | 0 | NOT STARTED |
| oteryn-client-runtime | 91 | 90 | 0 | 2 | 2 | IN PROGRESS |
| oteryn-client-simulation | 7 | 6 | 0 | 0 | 0 | NOT STARTED |
| oteryn-client-ui | 1297 | 740 | 0 | 0 | 0 | NOT STARTED |
| oteryn-diagnostics | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-error-codes | 12 | 11 | 0 | 0 | 0 | NOT STARTED |
| oteryn-foundation | 3 | 2 | 0 | 3 | 3 | REVIEWED |
| oteryn-gameplay-admission-evidence | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-character-recovery-fence | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-characters | 10 | 9 | 0 | 5 | 4 | IN PROGRESS |
| oteryn-gameplay-combat | 29 | 28 | 0 | 3 | 0 | IN PROGRESS |
| oteryn-gameplay-combat-rules | 29 | 28 | 0 | 4 | 2 | IN PROGRESS |
| oteryn-gameplay-content | 138 | 132 | 0 | 2 | 0 | IN PROGRESS |
| oteryn-gameplay-content-promotion | 36 | 35 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-dispatch | 1647 | 1535 | 0 | 45 | 9 | IN PROGRESS |
| oteryn-gameplay-domain | 66 | 65 | 0 | 1 | 0 | IN PROGRESS |
| oteryn-gameplay-durability | 718 | 714 | 0 | 42 | 26 | IN PROGRESS |
| oteryn-gameplay-economy | 28 | 27 | 0 | 4 | 4 | IN PROGRESS |
| oteryn-gameplay-encounter-rules | 21 | 20 | 0 | 4 | 2 | IN PROGRESS |
| oteryn-gameplay-encounters | 93 | 92 | 0 | 2 | 2 | IN PROGRESS |
| oteryn-gameplay-entities | 86 | 85 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-foundation | 55 | 54 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-housing | 24 | 23 | 0 | 6 | 6 | IN PROGRESS |
| oteryn-gameplay-items | 59 | 58 | 0 | 4 | 1 | IN PROGRESS |
| oteryn-gameplay-monster-ai | 64 | 63 | 0 | 3 | 2 | IN PROGRESS |
| oteryn-gameplay-native-admission-source | 81 | 36 | 0 | 4 | 0 | IN PROGRESS |
| oteryn-gameplay-node-files | 4 | 3 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-npc | 17 | 16 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-progression | 43 | 42 | 0 | 5 | 5 | IN PROGRESS |
| oteryn-gameplay-quests | 102 | 99 | 0 | 4 | 1 | IN PROGRESS |
| oteryn-gameplay-social | 56 | 55 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-social-rules | 22 | 21 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-source-critical-baseline | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-gameplay-spell | 83 | 82 | 0 | 7 | 2 | IN PROGRESS |
| oteryn-gameplay-spell-rules | 24 | 23 | 0 | 2 | 1 | IN PROGRESS |
| oteryn-gameplay-weather | 3 | 2 | 0 | 1 | 1 | IN PROGRESS |
| oteryn-gameplay-world | 73 | 72 | 0 | 10 | 6 | IN PROGRESS |
| oteryn-gameplay-world-change | 14 | 13 | 0 | 0 | 0 | NOT STARTED |
| oteryn-identity | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-input-actions | 7 | 6 | 0 | 0 | 0 | NOT STARTED |
| oteryn-input-platform | 7 | 6 | 0 | 0 | 0 | NOT STARTED |
| oteryn-ntp-clock | 2 | 1 | 0 | 2 | 2 | REVIEWED |
| oteryn-platform-client | 21 | 20 | 0 | 0 | 0 | NOT STARTED |
| oteryn-platform-contracts | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-protocol-oteryn | 580 | 314 | 0 | 2 | 0 | IN PROGRESS |
| oteryn-renderer | 18 | 17 | 0 | 0 | 0 | NOT STARTED |
| oteryn-session | 115 | 114 | 0 | 0 | 0 | NOT STARTED |
| oteryn-session-tcp | 4 | 3 | 0 | 0 | 0 | NOT STARTED |
| oteryn-simulation-determinism | 3 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-synthetic-assets | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-task-supervisor | 2 | 1 | 0 | 2 | 2 | REVIEWED |
| oteryn-test-support | 6 | 5 | 0 | 0 | 0 | NOT STARTED |
| oteryn-world-bundle | 11 | 10 | 0 | 1 | 0 | IN PROGRESS |
| oteryn-pre-native-client-tests | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-security-auth-tests | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-architecture-check | 29 | 15 | 0 | 0 | 0 | NOT STARTED |
| oteryn-dev-client | 11 | 10 | 0 | 0 | 0 | NOT STARTED |
| oteryn-game-ops-top | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-player-bots | 3 | 2 | 0 | 0 | 0 | NOT STARTED |
| oteryn-synthetic-asset-compiler | 2 | 1 | 0 | 0 | 0 | NOT STARTED |
| oteryn-synthetic-client-harness | 40 | 39 | 0 | 0 | 0 | NOT STARTED |
| oteryn-world-bundle-compiler | 31 | 24 | 0 | 0 | 0 | NOT STARTED |
| .github | 75 | 0 | 0 | 1 | 0 | IN PROGRESS |
| agent-configuration-and-retained-evidence | 113 | 0 | 0 | 0 | 0 | NOT STARTED |
| content | 8030 | 0 | 0 | 0 | 0 | NOT STARTED |
| deploy | 16 | 0 | 0 | 0 | 0 | NOT STARTED |
| docs | 7379 | 2 | 9 | 2 | 0 | IN PROGRESS |
| imports | 565 | 0 | 0 | 0 | 0 | NOT STARTED |
| repository-root | 16 | 0 | 0 | 0 | 0 | NOT STARTED |
| rulesets | 76 | 0 | 0 | 0 | 0 | NOT STARTED |
| tests | 12 | 0 | 0 | 0 | 0 | NOT STARTED |
| tools | 2681 | 3 | 1 | 0 | 0 | NOT STARTED |
| vendor | 793 | 768 | 0 | 0 | 0 | NOT STARTED |

## How to read the matrix

- A selected function range counts as one partially analyzed file, never as a fully reviewed file or package.
- Repeated independent reads are deduplicated. Searches, filenames, Cargo metadata and files merely opened are not behavioral reviews.
- Tests that were read are source evidence, not passing test results. Executed checks are separately recorded in validation.md.
- The public ledger retains paths/ranges; detailed security scenarios remain private. Independent security review files are separate and do not inflate the consolidated count.
- Direct local dependency edges, target kinds and platform conditions are in inventory.json. Listing a dependency does not mean it was audited.
- Every inventoried package/domain has a report entry in module-reports.md, including explicit NOT STARTED entries. Those entries do not mean its audit is complete.
