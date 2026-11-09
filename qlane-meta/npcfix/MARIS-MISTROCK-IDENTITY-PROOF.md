# Maris Mistrock display-name PostgreSQL evidence

Code: eb4644bb1090e526e70b08e3e06cc954e2fff4a8.

The isolated physical actor test reproduced the incorrect display name Maris Mistrock instead of the exact Crystal internalNpcName Maris in 74.82 seconds before the repair.

The post-repair real PostgreSQL case passed: one test, zero failures, 76.71 seconds. It emitted:

```
POSTGRES_CASE_COMPLETED NPCFIX physical oteryn:npc.maris_mistrock duration=76.703971965s success=true
```

All other 1289 NPC declarations remain unchanged. Repository tooling regenerated native content, quest packets and bundle pins. Completed: focused all-target/all-feature cargo check and strict Clippy for dispatch and game-server, quest-authoring checks, and the full 105-module quest-authoring unit runner. Disk exhaustion interrupted an earlier attempt; obsolete incremental variants were removed and these retries passed. The five world repository tests passed in 60.10 seconds, all 123 content integration tests passed in 4.10 seconds, and the final formatting check passed. Content-tree validation passed before the interrupted repository-test build; all failed checks from disk exhaustion have now been retried successfully. This evidence qualifies the displayed identity at the retained audited Mistrock anchor, not travel or mission semantics. No node boot was run; no main or qmech merge was performed in this continuation.
