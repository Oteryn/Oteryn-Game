# CONTENT-QUEST-01 r8 repository retention

Status: PROPOSED_NONCANONICAL
Date: 2026-09-21
Tracking: #707 / PR #709

## Current composition

Current CONTENT-QUEST-01 state is the composition of r1-r6 retained in Issue #707, tracked r6/r7 retention, the exact Annihilator map-binding witness, and the r8 reward-item binding witness.

## r8 result

The existing deterministic CW2-B1 Crystal item catalogue is byte-compatible with the selected Annihilator source revision for all four item-catalog/interpreter/schema inputs. Exact shared blobs were verified for `items.xml`, `item_parse.cpp`, `item_parse.hpp`, and `items_definitions.hpp`.

The four source reward payload identities are present in the existing catalogue but all remain `UNRESOLVED` as native Item identities:

- `3213` node `b9dc0488ef9aae6d8ad842fd1cab2b15f69370d5458fb5c0e06bf31e3cb86878`;
- `3288` node `9f367b04924ad0778d4180d6226103d19c11d93d589e10149b6cc88e82e34bd9`;
- `3319` node `468b27e0ea460b5ddff7c225842a88177b2546933001a91d7ede9baa9b8e6f71`;
- `3388` node `e1f02874d354d6b15293dabd56b86338b22be2bc0f34ce3cef22898598825f7b`.

Executable ONE_OF_4 lowering therefore remains fail-closed until every selectable alternative has an explicit Content/GAME-ITEM native binding. Quest cannot mint by legacy ID/name/stat similarity and cannot silently shrink the choice set.

Source item attack/defense/armor/weight/imbuement observations remain typed OTS candidate evidence, not Reference/native values.

## Validation horizon

Current required corpus: **79 cases**. r8 adds T76-T79 covering unresolved reward fail-closed behavior, exact item-source revision compatibility, complete ONE_OF_N resolution, and non-promotion of OTS item fields.

Architecture areas remain 48/48 traced; resource dimensions 26; adversarial classes 14.

## Current status

```text
architecture semantics: CLOSED_IN_CANDIDATE
tracked retention: PRESENT_IN_PR_709
selected Crystal physical map binding: PROVEN_FOR_PINNED_SOURCE_BYTES
Annihilator source reward identities: 4/4 SOURCE_IDENTIFIED
Annihilator native reward bindings: 0/4 RESOLVED
Reference target reward values: EVIDENCE_REQUIRED
runtime/DDL/production authority: NONE
protected architecture acceptance: PENDING
```

Next safe work is either explicit GAME-ITEM/Content binding work under its owning allocation or further official Reference evidence acquisition. Neither requires changing Quest/Encounter ownership.
