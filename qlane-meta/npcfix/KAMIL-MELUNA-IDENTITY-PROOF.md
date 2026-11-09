# Ferryman Kamil Meluna display-name PostgreSQL evidence

Code: 2f3aeb84e611c3d947d3e100da2e61f685aa4d09.

The isolated physical actor PostgreSQL regression failed before the repair on Ferryman Kamil Meluna versus the exact Crystal internalNpcName Ferryman Kamil in 75.20 seconds.

After the exact-before repair, the same real PostgreSQL case passed: one test, zero failures, 92.27 seconds. It emitted:

```
POSTGRES_CASE_COMPLETED NPCFIX physical oteryn:npc.ferryman_kamil_meluna duration=92.268854916s success=true
```

All 67 earlier repairs and all other 1289 NPC declarations are preserved. Native content, quest packets and final bundle pins were regenerated with repository tooling. Completed: native content-tree validation, quest-authoring checks, all 105 quest unit modules (1131 tests), and focused all-target/all-feature cargo check for dispatch and game-server. Strict all-target/all-feature Clippy for dispatch and game-server passed after recovering from disk exhaustion. The remaining repository tests, integration and formatting results are still pending completion. This proves the displayed identity at the retained Meluna anchor, not held travel or mission semantics. No node boot, main merge or qmech merge was performed for this repair.
