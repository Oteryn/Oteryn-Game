# Independent coverage consolidation validation

Base: `e2e5ced306040113dc9c395251a802f50eb5776a`. Validator: `durability_economy_audit`.

## Verdict

**CONFIRMED:** `audit-publication/docs/agents/evidence/OTERYN-FULL-CODEBASE-AUDIT-20261110/coverage-ledger.json` accurately consolidates the three current raw ledgers and agrees with `inventory.json`. No path, range, deduplication, aggregate-count, module-assignment, review-provenance, or package-status discrepancy was found in those JSON artifacts. Publication files were not edited.

**Initial adjacent publication discrepancy — now RESOLVED (see fresh-read closure below):** at the first inspection, `coverage.md:9-75` still listed every package as `NOT STARTED` with zero analyzed files; `coverage.md:81-91` also listed every supplemental domain as `NOT STARTED`. For example, game-server at line 10 and foundation at line 20 contradict their current JSON entries. The initial recommendation was to regenerate this Markdown table from the validated current ledger before publication; the fresh read below confirms that regeneration is complete. This is an underreported/stale table, not inflated JSON coverage.

## Independent checks and counts

The validation used sets of individual line positions rather than the consolidation script's interval-merging implementation. It read each reviewed file directly from the exact Git base blob, checked the working-copy bytes, checked every raw and published range boundary and file length, compared the exact published review attribution against the raw entries, and reconstructed all package/domain assignments. The consolidation script was read but was not invoked.

| Check | Independently confirmed result |
|---|---|
| Raw records | Root 58 + gameplay 69 + durability/economy 80 = 207 |
| Deduplicated paths | 196; 11 repeated path records removed from the aggregate |
| Full files / selected-range files | 96 / 100 |
| Union of analyzed line positions | 40,516 |
| Analyzed Rust / SQL files | 176 / 14 |
| Cargo package statuses | 3 REVIEWED; 25 IN PROGRESS; 39 NOT STARTED |
| Supplemental domains | 2 IN PROGRESS; 9 NOT STARTED |
| Inventory | 27,776 tracked files; 7,159 Rust; 424 SQL; 1,398 Python; 67 workspace packages; 11 supplemental domains |

All tracked files in the three `REVIEWED` packages have complete range coverage: `oteryn-foundation` 3/3; `oteryn-ntp-clock` 2/2; `oteryn-task-supervisor` 2/2. All other touched packages remain `IN PROGRESS`. No package is marked `VERIFIED`; global scope is `PARTIAL`. The ledger explicitly distinguishes full-file review from completed package scope or runtime validation. Additional structured slices and independent finding-verification reports do not inflate these file/range totals.

Package names and workspace member paths were checked against base manifests. The full tracked-file inventory and its package/supplemental partition were recomputed from the base Git tree. Published local dependency lists equal the inventory's lists. Cargo metadata was not rerun; the 268 target count was checked as the sum of inventory target entries. This validation confirms consistency and counting semantics of recorded analysis; it does not independently reperform the substantive review of all 196 files.

Machine-readable results: `/workspace/scratch/9868b8ed6bb7/audit-work/coverage-independent-results.json`.

## Actual pure-harness validation

Inspected the complete actual 50-line harness `audit-work/durability_pure_checks.rs`. It is an ordinary std-only binary using assertions, not a Cargo or PostgreSQL suite. Rebuilt it from the current source and executed the resulting separate binary. Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`.

Exact build command, exit 0:

```sh
/home/agent/.cargo/bin/rustc --edition=2024 -Awarnings /workspace/scratch/9868b8ed6bb7/audit-work/durability_pure_checks.rs -o /workspace/scratch/9868b8ed6bb7/audit-work/durability_pure_checks_independent
```

Exact execution command, exit 0:

```sh
/workspace/scratch/9868b8ed6bb7/audit-work/durability_pure_checks_independent
```

Actual output:

```text
PASS: 300 bid escrow scenarios (funding, conservation, release, winner burn); notice bounds; max-price auction tie; quick-loot selection/exclusivity cap; ammo underflow.
```

The 300 cases are the 5 × 4 × 5 × 3 balance/escrow/maximum/rent combinations at harness lines 12-34, with three winner-price cases for each funded bid. Lines 35-48 contain the additional notice, tie, loot, and ammunition assertions. This is bounded example coverage, not exhaustive arithmetic or transaction testing. It does not reproduce any DUR finding.

The following three scratch copies were compared byte-for-byte both with the immutable checkout and with `git show BASE:path` source blobs:

| Base source | Scratch module copy |
|---|---|
| `crates/gameplay/housing/src/house_auction.rs` | `audit-work/isolated/house_auction/mod.rs` |
| `crates/gameplay/housing/src/house_auction/eligibility.rs` | `audit-work/isolated/house_auction/eligibility.rs` |
| `crates/gameplay/housing/src/house_auction/value.rs` | `audit-work/isolated/house_auction/value.rs` |

All three comparisons passed. The two directly included source files, `crates/gameplay/housing/src/house_move_out.rs` and `crates/gameplay/items/src/quick_loot.rs`, also byte-match their base blobs. SHA-256 evidence for the copied modules is retained in `audit-work/coverage-harness-source-checks.json`. Compiling the eligibility module does not mean the harness exercised its eligibility behavior. No production source or publication file was modified.

## Validated artifact identities

- `coverage-ledger.json` SHA-256: `8dadd2088b79abda033760f83f70e2100d96a1857be8b6c12f3bcbf1a28a68e5`.
- `inventory.json` SHA-256: `e873015e13501616c2ea5a0171a8092febd5803c04564c4ec304983001ff8b30`.

## Fresh-read closure: stale Markdown table resolved

**RESOLVED.** After regeneration, a fresh read of the current `coverage.md` was compared row-for-row against the already independently validated `coverage-ledger.json`. All **78 rows** (67 packages and 11 supplemental domains), including each row's tracked/Rust/SQL/analyzed/full-file counts and status, match exactly. The aggregate summary now correctly reports **196 unique files, 96 full files, 100 selected-range files, and 40,516 union lines**, with **3 REVIEWED / 25 IN PROGRESS / 39 NOT STARTED / 0 VERIFIED** packages and **2 IN PROGRESS / 9 NOT STARTED** supplemental domains.

The regenerated prose clearly labels the audit partial, says selected ranges do not complete a file/package, and distinguishes inspected test source from executed checks. The coverage-ledger SHA-256 is unchanged from the independent validation above. There is **no remaining discrepancy** from this validation. No harness or other tests were rerun for this closure, and publication files were not edited by this validator.

Fresh-read `coverage.md` SHA-256: `53233ebb8bda9f7c578e7727868b0a253835b880905a28ed0eb7baf1e79b4b94`.
