# CONTENT-QUEST-01 r8 — Annihilator reward item binding witness

Status: **OTS_HYPOTHESIS_ONLY / ITEM_BINDING_GAP / PROPOSED_NONCANONICAL**
Date: 2026-09-21
Tracking: #707 / PR #709

## Purpose

Bind the four source reward payload identities discovered for the selected Crystal Annihilator to the existing Oteryn Content/World item-catalog evidence, and determine whether they are currently executable native Item targets.

## Exact source-generation compatibility

The existing protected CW2-B1 item source catalogue is pinned to `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`. The selected Annihilator scripts are pinned to `zimbadev/crystalserver@ac447fef0935e6df52dc6b6376ae4c1534ecd73f`.

Direct blob readback proves that the item catalogue and its interpreter/schema inputs are byte-identical across those revisions:

| Path | Exact shared Git blob |
|---|---|
| `data/items/items.xml` | `0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f` |
| `src/items/functions/item/item_parse.cpp` | `d95e2c83f44a81e7da7dc0d066c3b6f3a3302426` |
| `src/items/functions/item/item_parse.hpp` | `e22ede24f58aac0be6f85cd8d8167a6aa4341974` |
| `src/items/items_definitions.hpp` | `c09a62b585766909045e0fff5cf770e5550d333c` |

Therefore the existing deterministic CW2-B1 source-node evidence is reusable for these exact item bytes. This does **not** make unrelated scripts/runtime behavior interchangeable across the repository revisions.

Existing catalogue evidence path:
`docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json`

Catalogue Git blob on the Game base used by this task: `2f0121f3ea6586477b4535840b9a1f1bc28c677c`.

CW2-B1 product digest: `d773076b576599b6ced7eb53e262cdc6363515d842da3e8518b610e2106db0fc`.

## Current native-resolution state

The broad CW2-B1 catalogue contains 38,157 expanded source item identities and records:

```text
native_RESOLVED   = 0
native_UNRESOLVED = 38157
native_AMBIGUOUS  = 0
native_CONFLICT   = 0
```

The separately protected runtime importer currently carries an explicit source binding for item `2876` only. Current-tree searches for explicit source identities `3213`, `3288`, `3319`, and `3388` found no corresponding admitted source-to-native binding.

Disposition for every Annihilator reward payload below is therefore **UNRESOLVED**. A display name, legacy numeric ID, item statistics or source-node digest must not mint an Oteryn `ContentKey`.

## Exact source reward item records

| Source item ID | Source role in Annihilator | Source-node digest | Field-profile ID | Native disposition |
|---:|---|---|---|---|
| `3213` | Present payload / secondary label Annihilation Bear | `b9dc0488ef9aae6d8ad842fd1cab2b15f69370d5458fb5c0e06bf31e3cb86878` | `4f644e9be432a4e94d1bccebb1c3e4ee16850a78a22170597f910f2a0b6c4248` | `UNRESOLVED` |
| `3288` | Magic Sword selector payload | `9f367b04924ad0778d4180d6226103d19c11d93d589e10149b6cc88e82e34bd9` | `73f000129a1cc9400b4947bb8fb4e74f9ab5d1b98654122b99b013e11680a5a4` | `UNRESOLVED` |
| `3319` | Stonecutter Axe selector payload | `468b27e0ea460b5ddff7c225842a88177b2546933001a91d7ede9baa9b8e6f71` | `73f000129a1cc9400b4947bb8fb4e74f9ab5d1b98654122b99b013e11680a5a4` | `UNRESOLVED` |
| `3388` | Demon Armor selector payload | `e1f02874d354d6b15293dabd56b86338b22be2bc0f34ce3cef22898598825f7b` | `f59e1b9f71ca5347cca5d5aa9c18ca05c1e31d6ecc8c2197e1a596a212613a72` | `UNRESOLVED` |

These source-role names are provenance/discovery labels. They are not canonical native identity.

## Typed candidate observations retained by existing CW2-B1

### Source item 3213

- weight candidate: `4300`;
- profile also contains provenance-only description/primary-type fields.

### Source item 3288

- attack `48`;
- defense `35`;
- extra defense `3`;
- weapon type `sword`;
- weight `4200`;
- imbuement-slot source observation `2`, with the source-listed tier/family children retained by the catalogue.

### Source item 3319

- attack `50`;
- defense `30`;
- extra defense `3`;
- weapon type `axe`;
- weight `9900`;
- imbuement-slot source observation `1`, with the source-listed tier/family children retained by the catalogue.

### Source item 3388

- armor `16`;
- weight `8000`;
- imbuement-slot source observation `2`, with the source-listed protection/leech family children retained by the catalogue.

All of these remain **typed source candidates under GAME-ITEM**, not accepted Reference item values or native Oteryn item definitions. In particular, imbuement slot count/type participates in item power/balance only after the owning GAME-ITEM/native-content decision; Quest must not reinterpret or rebalance it.

## Executable reward lowering rule

An authored ONE_OF_N reward alternative may reference a source candidate during migration/evidence work, but executable content requires an admitted native Item binding for each alternative that can be selected.

For the observed four-way Annihilator source shape:

```text
4 source alternatives
+ 0 admitted native bindings for source IDs 3213/3288/3319/3388
= reward definition NOT EXECUTABLE
```

The importer/compiler must **not**:

- auto-mint four Oteryn item keys from names or numeric IDs;
- silently drop unresolved alternatives and expose a smaller choice set;
- replace an unresolved reward with a similar native item;
- use source stats to guess a native equivalent;
- create items directly from Quest Runtime.

Safe states are:

1. retain the reward alternatives as source/evidence candidates and fail executable linking; or
2. after separate GAME-ITEM/Content acceptance, bind every executable alternative to explicit native `ContentKey@Revision` identities with provenance.

If product/Reference evidence intentionally changes the reward set, that is a declared content difference/owner decision, not an importer repair.

## Claim/reward transaction consequence

r2 shared-entitlement semantics remain unchanged. Native resolution happens **before** a reward claim can form the bounded immutable value intent sent to GAME-ITEM/DUR-03.

An unresolved alternative can never reach DUR-03 as a legacy item ID. Once a native alternative is resolved, the accepted claim operation freezes its native identity/revision together with the alternative selection so retry/lost ACK cannot re-resolve to a different item revision.

## New required tests

- **T76 — unresolved source reward fail-closed:** source item ID/name/node digest without explicit native binding cannot form an executable reward value intent.
- **T77 — item-source revision compatibility:** source-node evidence may be reused across repository revisions only when all admitted item catalogue/interpreter/schema blobs required by the evidence are exact matches; unrelated behavior remains revision-qualified.
- **T78 — complete ONE_OF_N resolution:** an executable four-alternative reward requires every selectable alternative to resolve; partial resolution cannot silently shrink the choice set or substitute another item.
- **T79 — source item fields are candidate evidence:** attack/defense/armor/weight/imbuement observations from OTS source cannot become Reference/native values without the owning GAME-ITEM/Content acceptance path.

Required CONTENT-QUEST-01 corpus is now **79 cases**.

## Evidence classification

```yaml
annihilator_reward_item_binding:
  source_classification: OTS_HYPOTHESIS_ONLY
  source_item_bytes_compatible_with_cw2_b1: PROVEN_BY_EXACT_GIT_BLOBS
  source_node_records: PROVEN_IN_EXISTING_CW2_B1_EVIDENCE
  source_ids:
    3213: UNRESOLVED
    3288: UNRESOLVED
    3319: UNRESOLVED
    3388: UNRESOLVED
  native_reward_execution: FAIL_CLOSED_UNTIL_EXPLICIT_BINDINGS
  reference_reward_values: EVIDENCE_REQUIRED
```

No owner boundary changes are introduced. Quest still owns eligibility/orchestration only; native item identity/semantics remain Content/GAME-ITEM and materialization/anti-duplication remain DUR-03.
