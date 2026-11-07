# Monster lab qualification — 2026-10-02

1660/1660 profiles passed the headless native component arena, including 48 isolated approximate variants. All original stats/loot and the 105-path backup from PR #1533 are preserved. This is source-bound local evidence, not live gameplay or remote exact-head CI. See `qualification.json` for limits, source pins and checks.

`monster-lab-run.tar.gz` contains the full final arena input/stage/result, inventory/catalog, all 48 training bundles and test/review receipts. Every archive member is verified in `archive-verification.json`. It contains no donor checkout, credentials, compiled artifacts or Python environment. Paths in receipts identify the execution workspace; configure paths for a fresh run through `tools/monster-lab/config.example.json`. Original immutable prepared bundles/index/Item map are in the parent draft PR #1533 archive.

The existing `native_entry_room.json` supplies the two adjacent walkable spawn slots. Per-monster authored HP, damage/retry/death/despawn run through native owner fixtures; death uses its separate position fixture. Actual spell effects, installed summons, loot drops, respawn and a client-connected server remain unqualified.
