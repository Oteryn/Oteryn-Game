# Game server audit — checkpoint 1

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Status: **IN PROGRESS**.
Scope: `apps/game-server`, 873 tracked files. Reviewed: 5 full files / 368 text lines. Exact remaining 868 files: `coverage-ledger.json`.

This lane checkpoint is a subset of cumulative coverage, which also retains independent and identical-blob historical evidence.

The reviewed files are the applicable `AGENTS.md`, `Cargo.toml` and the three owning bosstiary migrations (0445, 0532 and 0786). The manifest's production/test dependency boundaries were checked; this does not establish completion of any server entrypoint. The SQL scope covers the initial per-character boss counter, immutable occurrence receipts and slot state/receipt constraints. The kill counter and receipt migrations do not mutate CharacterRevision. Slot mutation receipt constraints and their actual callers are under further integration review; no additional confirmed finding or partial fee loss is claimed.

Server callers in other files have only selectively recorded cross-reference ranges. Those cross-references are not counted as full files in this checkpoint. All later migrations, runtime startup/listeners, content ingestion, test fixtures and remaining server code require substantive review. Large JSON fixtures require separate semantic validation; listing or parsing them alone is not full review.

All 5 entries match the immutable Git blob and worktree SHA-256 and exact full line ranges. No PostgreSQL migration/fixture, server process, native client or workspace build was run; no production/source changes or remote writes were made by this worker. This scope is not REVIEWED and not runtime VERIFIED.
