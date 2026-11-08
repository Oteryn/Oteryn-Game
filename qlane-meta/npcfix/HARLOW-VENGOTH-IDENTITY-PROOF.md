# Harlow Vengoth display-name qualification

Code: 42b8c224c1486c6d6e9ed63679c4e025545965cc.

The exact Crystal internalNpcName is Harlow. The isolated physical actor test failed on the prior display name Harlow Vengoth in 74.40 seconds. After the exact-before repair, the same PostgreSQL case passed in 74.89 seconds and emitted:

```
POSTGRES_CASE_COMPLETED NPCFIX physical oteryn:npc.harlow_vengoth duration=74.889965134s success=true
```

Validation completed: focused cargo check; strict all-target/all-feature Clippy for dispatch and game-server; formatting; 123 content integration tests; five world-project repository tests; content-tree validation; quest-authoring checks and all 105 quest unit modules. The first post-repair PG compilation exhausted disk; unrelated incremental caches were removed and the real PG retry passed.

This proves the displayed identity at the retained audited Vengoth anchor. It does not qualify held travel or mission semantics. Node boot was not run, as explicitly instructed for this environment. No main or qmech merge was performed in this continuation.
