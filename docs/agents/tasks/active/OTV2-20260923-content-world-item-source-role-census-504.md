# OTV2-20260923-content-world-item-source-role-census-504

```yaml
task_id: OTV2-20260923-content-world-item-source-role-census-504
title: Item source-role denominator census
mode: AUDIT
status: validating
repository: Oteryn/Oteryn-Game
branch: agent/content-world-item-source-role-census-504
issue: 162
pr: 777
admission_main_sha: 19c15ebf8377608db5b72cf7082dc856c4fd76be
owner: Oteryn: content world import
```

## Summary

One-time denominator audit before large-scale Item import. No Item schema, runtime, client, identity-registry or semantic-promotion mutation.

## Result

Protected source: `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`, `data/items/items.xml`, blob `0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f`.

- XML nodes: 17,669
- reversed ranges excluded: 4
- admitted definitions: 17,665
- direct definitions: 14,689
- range definitions: 2,976
- expanded source IDs: 38,157
- range-expansion extra IDs over definition denominator: 20,492

Role census:
- PLAYER_CATALOG_STRONG: 4,935 definitions / 5,452 IDs
- WORLD_OBJECT_INTERACTION: 5,163 / 13,001
- TECHNICAL_PLACEHOLDER: 652 / 5,440
- STATE_VARIANT_DECAY: 3,414 / 3,422
- STATE_VARIANT_TRANSFORM: 287 / 320
- RANGE_VARIANT_NO_FIELDS: 1,024 / 7,835
- DIRECT_SIMPLE_NO_FIELDS: 866 / 866
- SIMPLE_PRESENTATION_CANDIDATE: 1,139 / 1,601
- RESIDUAL_OTHER: 185 / 220

Strong player lower bound = 4,935 definitions. Strong player plus unresolved direct/simple band = 7,125 definitions / 8,139 IDs. This is a scaling estimate, not Reference truth.

## Validation

Synthetic self-test plus exact fresh-source reproduction; generated compact evidence must match the tracked manifest byte-for-byte.

## Boundary

Names/source taxonomy are discovery/classification signals only. No name-only identity proof, gameplay promotion or identity remap.
