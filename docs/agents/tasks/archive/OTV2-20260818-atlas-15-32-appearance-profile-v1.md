# OTV2-20260818-atlas-15-32-appearance-profile-v1 — COMPLETED

```yaml
task_id: OTV2-20260818-atlas-15-32-appearance-profile-v1
status: completed
repository: blakinio/Oteryn-v2
pr: 333
final_head_sha: e5ca4a58d4ea1797f3cbcbf378fc188a8e2ae733
merge_sha: 2e793c0954e0502441e6ff275463053b1f41787e
closed_at: 2026-08-18T10:32:00+02:00
public_contracts:
  - oteryn-atlas-15-32-appearance-spatial-v1
```

## Terminal result

PR #333 merged the bounded Game-owned appearance presentation profile for the exact latest Atlas source `15.32.zip`.

Exact source identity:

```text
Drive file id = 1Dlo3bS4K1nS3mw4BhPZdlHT7lX5zRAvv
Drive object name = 15.32.zip
Drive size = 246811594 bytes
ZIP sha256 = 1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f
catalog sha256 = 35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85
appearance sha256 = dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075
```

Direct deterministic decoding records 43,514 object appearances, 5,084 sprite sheets and 79,269 unique object sprite IDs. The exact Drive probe and metadata-only durable evidence are recorded in `Oteryn/Oteryn-Atlas@0b56d9a95279f1ec02fddd0dfcf8bd6ffd16b539`.

The accepted contract defines the 32-unit presentation scale, source visual anchor/coverage semantics, explicit shift/height displacement conversion and producer-owned concrete frame/phase/pattern/layer/sprite resolution. Pixel-derived visual coverage remains explicitly non-authoritative for gameplay occupancy/collision.

The separate accepted 15.25 profile remains bounded to its own distinct archive and was not rewritten or generalized.

## Exact-head validation

Final PR #333 head `e5ca4a58d4ea1797f3cbcbf378fc188a8e2ae733` passed:

- Agent Governance #1628 — SUCCESS;
- Architecture Semantic Audit #215 — SUCCESS;
- Merge Authority Audit #434 — SUCCESS;
- Merge Gate #478, including CodeQL actions/python and final validate — SUCCESS;
- review threads: 0;
- review submissions: 0.

PR #333 was squash-merged as `2e793c0954e0502441e6ff275463053b1f41787e`.

## Remaining DYN-ATLAS-001 dependencies

- bounded deterministic Game-owned Thais semantic export fixture/artifact under the accepted export/spatial/importer/15.32 appearance profiles;
- explicit project rights/authorization for public source-pixel publication from the exact 15.32 ZIP digest before a public Atlas pixel publication may claim that authority.

No runtime/exporter/browser or production deployment change was part of this contract task.
