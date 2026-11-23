# Wave 8 remaining review queue

From wave 7: 637 pending = 393 STATIC_AST_ONLY + 10 PARTIALLY_REVIEWED + 234 INVENTORY_ONLY. Exactly ten previously static-only files have been fully read, source-blob pinned, and executed in this batch; no previously reviewed paths repeated.

After wave 8: **627 pending = 383 STATIC_AST_ONLY + 10 PARTIALLY_REVIEWED + 234 INVENTORY_ONLY**; cumulative **68 distinct fully read** content-tools test paths. The **130 separate global review groups** remain open. 29 passing Python methods are not 29 verified gameplay journeys.

The calibration test needs a rewritten independent oracle; the elephant producer-ID negative control correctly fails on a missing identity. No retirement or modifications to current test sources.
