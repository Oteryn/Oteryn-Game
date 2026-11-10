# OTERYN FULL AUDIT — STATUS

**This is a substantive partial audit, not completion of the full codebase audit.** Inventory is complete; most behavioral scope remains open. Canonical live register: [#3088](https://github.com/Oteryn/Oteryn-Game/issues/3088).

- **Base SHA:** `e2e5ced306040113dc9c395251a802f50eb5776a`.
- **Baseline read:** 2026-11-10 09:11:53 UTC.
- **Latest main observed:** `4f0b6c62a789489fca6c37dd36d5917d096f328e` at 09:50:51 UTC. Scoped checks preserve current findings; the 470-file successor delta is not counted as audited. [Freshness reconciliation](latest-main-reconciliation.md).
- **Modules total:** 67 Cargo packages plus 11 supplemental tracked-file domains.
- **Modules analyzed:** 3 REVIEWED in full at source level; 25 other packages IN PROGRESS; 39 NOT STARTED. Two supplemental domains partially analyzed.
- **Modules VERIFIED:** 0.
- **Substantive evidence:** 196 unique files, 96 full-file and 100 selected-range reviews; exact deduplicated ranges in the ledger.
- **P0 confirmed:** 0.
- **P1 confirmed:** 3 — DUR-003, SEC-001, GP-001.
- **P2 confirmed:** 3 — SEC-003, DUR-001, SEC-002.
- **P3 confirmed:** 0.
- **Findings awaiting verification:** 2 — DUR-002 and RNG-001, both LIKELY/P2 and excluded from confirmed counts.
- **False positives:** 4 formally recorded; SEC-002's original production P1 premise was separately rejected while its isolated P2 defect remains.
- **Blockers/limits:** session/protocol worker authentication failure; full Rust/SQL/TLS/production validation not run; most package files unreviewed; no authenticated private GitHub advisory route exposed.

## Evidence basis

FACT: all reported source locations refer to the exact baseline above. All P1 findings received a separate auditor's source review, including real caller, preconditions and counterguards. CONFIRMED here means the stated code-path defect is established; it does not claim an observed production incident. Tests read as source are never reported as passed. The small listener reproduction uses the actual poll closure with boundary stubs; it does not reproduce production scheduling.

## Confirmed high-priority findings

| ID | Severity / status | Verified consequence | Evidence |
|---|---|---|---|
| DUR-003 | P1 / CONFIRMED static, independent | A supported nested inventory can prevent completion of committed death item work, blocking fresh admission and reconnect/respawn. No irreversible contents loss established. | [Independent source proof](verification-dur-003.md) |
| SEC-001 | P1 / CONFIRMED static, independent | Conditional admission availability defect when the relevant optional service is enabled. Deployment state unknown. | Restricted evidence delivered privately to owner |
| GP-001 | P1 / CONFIRMED static, independent | Legitimate loyalty/proficiency magic bonuses cause early rejection on affected self/native/conjure paths. Ordinary combat is outside the finding. | [Gameplay report](gameplay-findings.md), [independent verification](verification-gp-001.md) |

## Separate P2/P3 backlog

| ID | Status | Bounded result |
|---|---|---|
| SEC-003 | P2 / CONFIRMED static | V2 guild/death bodies permitted by their operation caps can fail the shared 10,240-byte request-envelope cap; public views remain stale. [Details](public-projection-finding.md). |
| DUR-001 | P2 / CONFIRMED static | Inventory capacity invariant defect. Security-sensitive source/reproduction detail retained privately. No duplicate quantity or item loss established. |
| SEC-002 | P2 / CONFIRMED isolated model | Listener wake defect reproduced in a boundary-stub harness. Production periodic wake sources refute the asserted indefinite P1 outage. |
| DUR-002 | P2 / LIKELY | Transaction concurrency candidate requires controlled PostgreSQL scheduling. Not a confirmed deadlock. |
| RNG-001 | P2 / LIKELY | Exact deployed seed availability and native effect remain unresolved. No confirmed gameplay prediction exploit. |

No P3 finding met the evidence threshold in this pass. Zero findings in a category or module do not establish safety.

## Reports

- [Coverage matrix](coverage.md), [exact source ranges](coverage-ledger.json), [all module reports and cross-module flows](module-reports.md).
- [Inventory, targets and direct dependency map](inventory.json).
- [Structured finding register and false positives](findings.json).
- [Quest/reward/reconnect trace](quest-reward-recovery.md).
- [Executed checks and unexecuted validation](validation.md).
- [Concurrent work](coordination.md), [latest-main finding reconciliation](latest-main-reconciliation.md).

## Repair order — recommendation only

1. **DUR-003:** restore legal nested-death completion in Rust and SQL together, including existing pending receipts. Character availability is directly affected and ordinary reconnect repeats the refusal.
2. **SEC-001:** review the private availability evidence and correct the admission-resource lifetime before enabling/expanding the affected service. If that service is currently enabled, this may outrank DUR-003; deployment state was not inspected.
3. **GP-001:** unify effective magic consistency projections across all affected sibling gates, preserving raw-state binding.
4. **SEC-003:** align operation body and request-envelope limits and run native codec-to-transport boundary regressions.
5. **DUR-001:** repair full-tree capacity accounting in the affected custody transition; preserve identity and conservation guards.
6. **SEC-002:** make the listener's own wake/repoll behavior complete without relying on outer periodic work.
7. **DUR-002 / RNG-001:** resolve the precise missing concurrency/seed evidence before approving a repair scope or raising severity.

No game source, game tests, migrations, Cargo configuration, workflows or production state were changed. This branch contains audit documentation only; no PR or merge was created. The programme remains open because the full-audit completion criteria have not been met.
