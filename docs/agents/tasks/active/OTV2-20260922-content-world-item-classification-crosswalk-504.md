# OTV2-20260922-content-world-item-classification-crosswalk-504

```yaml
task_id: OTV2-20260922-content-world-item-classification-crosswalk-504
title: D6-M1 Item Classification Crosswalk
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-classification-crosswalk-504
pr: 763
base_sha: 9ddd6e020837bd43ec66ce7d78d2af7742afeb39
head_sha: pending
owner: "Oteryn: item classification lead"
owned_paths:
  - apps/game-server/examples/export_reference_item_identity_map.rs
  - tools/reference-world-corridor-census/item_classification_crosswalk.py
  - tools/reference-world-corridor-census/item_classification_crosswalk_self_test.py
  - docs/agents/evidence/OTV2-20260922-content-world-item-classification-crosswalk.json
  - docs/agents/tasks/active/OTV2-20260922-content-world-item-classification-crosswalk-504.md
```

## Scope

Build a deterministic, bounded crosswalk over the protected 38,157-Item native identity closure. Reuse the protected B1 catalogue, 64 binding map and family-scale registry without importing source XML or allocating a second identity family. Classify source observations separately from accepted Reference capabilities and preserve absence as `UNKNOWN`, with current-source status `NOT_EVALUATED` unless a pinned current source is explicitly supplied.

The crosswalk outcomes are `EXACT_ONE`, `ZERO_MATCH`, `AMBIGUOUS` and `CONFLICT`. Display names and source labels never resolve identity. This task publishes evidence only: it does not promote OTS gameplay values or change runtime, client, registry, protocol or persistence behavior.

The canonical identity map is exported from `protected_cw2_b1_full_item_family_import()` through the owned example path; the exporter serializes existing bindings only and contains no allocation or key construction.

Source-policy direction is inherited from Oteryn/Oteryn-Game#504 comments `5771546143`, `5771561550`, `5773340420`; no history rediscovery is performed.

## Validation

- Python self-test covers deterministic output, duplicate/missing/remap rejection, crosswalk partition states, orthogonal capability states and absent current-source handling.
- Full protected 38,157-record compile reproduces the family allocation digest, emits every native identity exactly once, preserves all 64 semantic bindings and produces byte-identical repeated output.
- Committed evidence is a compact manifest of canonical full-record output digests and counts; it does not hand-author or duplicate the 38,157-row corpus.
- Repository CI is required before qualification. No exact-head review, freeze, paid review or Merge Queue action is part of this implementation handoff.

### Completed locally

- Canonical Rust exporter: PASS twice, byte-identical 5,789,755-byte maps, SHA-256 `83ba3c26d10af8834191bf5491280882b6453bca0911b86d180c07a15cec679a`.
- Python self-test: PASS, including all four crosswalk states plus missing, duplicate, native-key collision, remap and schema-coverage rejection.
- Full census: PASS twice, 38,157 records, 64 preserved semantic bindings, 38,093 opaque identities, 17,665 source profiles and allocation digest `ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966`.
- Full output: byte-identical 136,223,781-byte canonical JSON, SHA-256 `004948eeda07afb20d5560ec583eaa2a32397f19f891a7d8749962bc32fa0f8d`; bulk output remains scratch-only.
- Compact evidence manifest: 6,349 bytes, SHA-256 `46e925de6e624e6e8c4027b797b7240d1389c0784da387ab74e7a011599ea5f8`.
- Rust formatting: PASS on the published exporter generation.

### Limitations

- No current-source catalogue was supplied or searched. Current-source state is `NOT_EVALUATED/UNKNOWN`, never a no-match claim.
- Pinned OTS field presence is hypothesis evidence only. Absence is not false or zero, and accepted typed Reference capability state remains `UNKNOWN`.
- Existing retained core semantics and all 38,157 native identities are consumed unchanged. The Python compiler performs no identity allocation.
- The source-signal census covers all 24 orthogonal classification capabilities. Eleven have no direct pinned OTS signal; all 24 remain accepted Reference `UNKNOWN`, so Item classification completion remains a later evidence-backed task.
- Repository CI remains required; this handoff is not an exact-head qualification or release decision.

## Status

Implementation and local validation are complete. Draft PR #763 was opened before validation; coordinator qualification, repository CI and any later review/freeze remain pending.
