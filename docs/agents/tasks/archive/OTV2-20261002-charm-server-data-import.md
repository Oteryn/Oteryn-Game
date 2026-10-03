# OTV2-20261002-charm-server-data-import

```yaml
task_id: OTV2-20261002-charm-server-data-import
title: Import canonical Charm data at server boot
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/charm-canonical-catalogue-20261001
branch: codex/charm-server-data-import-20261002
pr: 1600
issue: 162
base_sha: 704dcfb1e6338a2e3b9a6837c9a84522f133b92c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher
created_at: 2026-10-02
updated_at: 2026-10-02
owned_paths:
  - apps/game-server/src/content/charm_source.rs
  - apps/game-server/src/content/charm_source_tests.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - docs/agents/tasks/archive/OTV2-20261002-charm-server-data-import.md
public_contracts: []
depends_on: [CHARM-1, CHARM-3, CHARM-4, 1520]
blocks: []
external_repositories: []
```

## Requested result

The owner explicitly narrowed the task to importing the prepared Charm data into the server,
acknowledging that some gameplay components remain unfinished. Embed the registered index and
single canonical shard in the server build and decode all 25 definitions, 75 costs and stage
values with the already reviewed decoder. Reuse existing Content resource ceilings for the
read-only evidence import; neither the named evidence profile nor its framed hash grants
generation authority or changes the admitted production artifact format.

The real boot path imports the catalogue alongside the spell book before binding sockets or
publishing readiness. Decode failure refuses readiness. A non-secret operator event reports
the imported row count, source byte count and evidence SHA. The complete immutable catalogue
is retained by the gameplay owners for the serve lifetime; the qualification composition uses
the same importer. The broad source-module unused-code allowance is removed. One precise
field allowance records that actual Charm gameplay consumers are separately composed.

Canonical source files, wire offers/dispatch, Character state, persistence, commercial facts,
mechanics and active Content identity are unchanged. Import does not filter definitions whose
gameplay consumers are missing or advertise effect availability. This is data import only,
not a claim that the original nine-row parent or all Charm gameplay is complete.

## Validation and publication

RED: the new actual embedded-import test fails to compile because no embedded importer exists.
GREEN: all 12 focused JSON/effect/catalogue tests PASS, including byte-for-byte retained source,
all typed domain/effect definitions and inclusion of Cleanse/Scavenge while consumers are absent.
Existing malformed-source/limit tests continue to pass through the unchanged decoder.
Full package validation PASS: 57 suites, 16299 passed, seven existing ignored. Component locked
all-target Clippy, workspace formatting/whitespace, governance validator and 36 governance tests
PASS. The PostgreSQL admin URL was absent; environment-guarded PG/Platform qualification is not
physical E2E evidence. This delta changes no persistence. Exact independent source review belongs
to the final freeze packet.

Root publishes the guarded actual Git candidate on its exclusive new branch. Required external
review, parent-first retarget/requalification and protected integration remain with the active
control plane. A commit cannot contain its own final frozen SHA; this archive reaches main only
if PR1600 merges. There is no live deployment or complete combat/Platform/PostgreSQL E2E claim.
