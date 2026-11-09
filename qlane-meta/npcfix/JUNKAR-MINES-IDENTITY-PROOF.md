# Junkar Mines display-name PostgreSQL evidence

Code: 6da9cb57f98c0841676bd499c5da7f473f44e2d0.

The isolated physical actor PostgreSQL regression failed before the repair on Junkar Mines versus the exact Crystal internalNpcName Junkar in 98.80 seconds.

After the exact-before repair, the same real PostgreSQL case passed: one test, zero failures, 100.45 seconds. It emitted:

```
POSTGRES_CASE_COMPLETED NPCFIX physical oteryn:npc.junkar_mines duration=100.445166284s success=true
```

All 68 earlier repairs and all other 1289 NPC declarations are preserved. Native content, quest packets and final world pins were regenerated with repository tooling. Quest-authoring checks and all 105 quest unit modules (1131 tests) passed. Native content-tree validation, focused all-target/all-feature cargo check, and strict Clippy for dispatch and game-server passed. Full content regeneration completed, including all five world repository tests in 79.81 seconds. All 123 content integration tests passed in 4.62 seconds; final formatting and diff checks passed. The complete required validation for this display-name correction is green. This proves the displayed identity at the retained audited mines anchor, not travel or mission semantics. No node boot, main merge or qmech merge was performed for this repair.
