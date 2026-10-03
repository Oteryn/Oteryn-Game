# OTV2-20261001-charm-exact-source-json

```yaml
task_id: OTV2-20261001-charm-exact-source-json
title: Exact closed-shape Charm source JSON prerequisite
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-exact-source-json-20261001
pr: 1508
issue: 162
base_sha: e225b3f76e152d195f75cb757b3d639a3ff5577a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; Content source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/Cargo.toml
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/charm_source_json.rs
  - apps/game-server/src/content/charm_source_json_tests.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-exact-source-json.md
public_contracts: []
depends_on: [CONTENT-0, CHARM-3]
blocks: []
external_repositories: []
```

## Bounded result

Uses upstream serde_json RawValue to preserve source numerical literals and convert accepted
nonnegative decimals to exact u32 hundredths. Rejects overflow, exponent/string/object forms,
extra precision and precision otherwise rounded away through f64. Duplicate decoded member
names reject before overwrite, including Unicode-escaped aliases. Typed consumption plus
ensure_empty supports closed effect shapes with missing/wrong/unknown fields rejected.

196 lines of production/tests plus a targeted module declaration and existing dependency's
raw_value feature. No dependency version or lock change. This prerequisite is separate from
the approximately500-line canonical25 decoder. It adds no source ceilings, runtime authority,
generation identity, protocol, storage or activation. The owning decoder must bound source
bytes with existing Content evidence limits before invoking these crate-private adapters.
A documented module dead-code allowance lasts until that allocated consumer is supplied.

## Validation and remaining work

Three focused tests PASS: exact decimals/u32 boundaries, duplicate decoded keys and typed
closed-shape consumption, with escaped metadata unable to substitute numerical fields.
Full library1282PASS,2existing ignored; workspace fmt, diff check and governance/lifecycle13PASS.
Initial strict Clippy diagnosed five test expectations; two tests now return Result and use ?
with every oracle retained and no lint allowance. Final focused3PASS. Strict workspace all-target
Clippy PASS on the final source; exact frozen identity is recorded in the external packet.

The canonical Charm decoder and accepted qualified-generation attachment remain unfinished;
source evidence does not prove current Content authority. The original full nine-row parent
remains IMPLEMENTING. Root publishes guarded actual Git identity. Exact frozen head, independent
review and CI belong to the external packet. This archive reaches main only if PR1508 merges;
a commit cannot contain its own final SHA.
