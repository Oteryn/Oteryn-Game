# OTV2-20261007 Shards I1 current-source identity

```yaml
task_id: OTV2-20261007-shards-i1-current-source-identity
title: Admit the independently identified current-source Lit Torch identity
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/shards-i1-current-source-identity-20261007
pr: null
base_sha: d0b091b6b5ad4b9527d354df0043c6e55d5861cc
owner: chatgpt-shards-reconstruction-lead
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/item_identity.rs
  - apps/game-server/tests/content_item_identity.rs
  - docs/agents/tasks/OTV2-20261007-shards-i1-current-source-identity.md
public_contracts:
  - A12-ITEM-IDENTITY-TIBIA-ID-V1
depends_on: []
cross_repository_coordination_id: 1622
```

## Outcome and authority

A separate current-source cohort admits the minimal identity of i54610 before historical
key rewriting/reference closure. The original appearance-only route still refuses it.
This does not rewrite a historical alias, tombstone, held donor id or admission packet.
Both cohorts independently require membership in the pinned admitted current appearance
manifest, absence from historical allocated source ids, canonical family/key/revision,
ClientSafe, materializable=false, Unknown stack and all-Unknown gameplay semantics.

Human option A allocated SHARDS-Q1/I1/E1/R1 and necessary existing-contract adapters,
recorded in #1622 comment 6046563239. The source proof and minimal core candidates were
frozen separately in evidence-only #1852 at 70a2f9ab.

FACT: Crystal Summer 00ce02a57ca5a12e48f32a3476e37471167e4c3f items.xml Git blob
5530bb76d896d31fb75d6cad23969e1cc463b1c5 / byte SHA256
13a8773e34085daad1a716465c0510060d1f2255c4bc69995fd160c8b4afcece identifies
54610 as Lit Torch, distinct from 34017. The actual client binary and XML row were
independently checked by the evidence packet's build_i1_core_candidate.py. Identity-only
admission does not promote light, equipment, decay, transform or use fields.

## Qualification

Native content_item_identity integration target: 14 tests PASS on Linux/Rust 1.94.0.
New cases qualify source-held reference closure, rejection by the old appearance-only
route, 34017/54609/padded-id substitution, materialization, duplicate allocation, an
already-bound source id, wrong family/revision/projection, stack and semantic substitution.
The positive record equals the minimal all-Unknown fixture after admission.

Authority/recovery mutation model: NOT_APPLICABLE; this is an offline declarative identity
switch, not a current session/lease consumer, production mutation or durable writer.
Identity/provenance negative cases remain applicable and fail closed. No new wire field,
store or QuestState write. Independent exact-head admission review is required.

## Integration boundary and rollback

The materializer does not call the new route yet: its shared paths remain with #1924.
No served content is generated, and i54610 is not yet materializable or usable. I1 still
requires minimal core admission and current-generation bindings after that lease releases.
This slice does not activate a playable quest. The accepted appearance-only cohort and
all historical refusal behavior remain unchanged. Rollback removes the unused new route
before admitting dependent content; no persistent migration.
