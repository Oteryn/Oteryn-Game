# A8 donor Item identity epoch decision

- Decision: `A8-DONOR-ITEM-IDENTITY-EPOCH-V1`
- Status: **CANDIDATE with owner decisions D96-D97 (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Routed as architecture package item A8 (#162 comment 5877360232, B1b stopped with
  `DESIGN_DECISION_REQUIRED` under allocation 5877308300)
- Owner decisions posted: #162 comment 5877519880
- Extends: `docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md` (identity
  layers and binding rules unchanged)
- Admission baseline: `main@e19b19d6`
- Identity minting, runtime, registry and production authority: **NONE**. B1b mints under its own
  allocation with independent identity review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

The donor census (`tools/content-schema/item-authoring/samples/donor-census-crystal-summer-update-00ce02a5.json`)
finds 412 ids present in the donor `items.xml` (Crystal `summer-update` at `00ce02a5`) and absent
from the pinned base (`ff7ede59`). The frozen CW2-B1 allocator cannot mint them. How do they get
Oteryn Item keys without moving any existing key?

## 2. Owner decisions

| # | Decision | Owner choice (2026-09-28) |
|---|---|---|
| D96 | Additive donor-identity epoch (option B). The donor ids continue the opaque sequence after its highest allocated number. The frozen CW2-B1 import stays byte-identical. A later donor generation adds a further epoch by the same rule. | "Tak, dopisać na końcu" |
| D97 | All 412 donor ids get an Item key, including the 138 routed to WorldObject or Terrain. Those are non-materializable Item pointers, and their family key follows the WO-0 D93 rule. | "Wszystkie 412, jak WO-0" |

## 3. Facts

**PROVEN** (main `e19b19d6`)

- **CW2-B1** (`apps/game-server/src/content/cw2_b1_import.rs`):
  - `protected_cw2_b1_full_item_family_import` admits exactly 38,157 rows under the protected
    evidence digest.
  - It walks them in ascending `source_item_id` and gives each non-semantic row
    `opaque_item_key(opaque_sequence)` = `oteryn:item.registry.i{sequence:08}`.
  - It closes on `CW2_B1_OPAQUE_ITEM_COUNT` = 38,093 and 64 protected semantic bindings.
  - The allocation is positional over a closed corpus, so re-running it over a larger corpus would
    shift keys (#162 5877360232: 168 of the 412 fall between existing ids).
- **Bindings** (`imports/crystalserver/bindings/items.json`, `OTERYN_SOURCE_IDENTITY_BINDINGS/v1`):
  - 38,157 G4 source bindings (`oteryn:source.crystalserver` at `ff7ede59`,
    `ots/item_server_id`, `EXACT`).
  - 38,092 of them target opaque keys, with a highest sequence of 38,093.
  - Sequence 2,921 is retired: it was the gold coin's key until R7-P04 re-keyed it to
    `oteryn:item.currency.gold_coin` (`R7_P04_GOLD_COIN_OLD_KEY`), so 65 entries in all target
    named keys.
- **Donor census:**
  - 412 new ids: 261 resolved, 13 unresolved, and 138 routed non-Item. The routed ids are
    WorldObject corpse 80, immovable 24 and primarytype 4, and Terrain ground or border 16 and
    primarytype 14.
  - The ids are disjoint from the base.
  - The census mints and consults no identity.
- **G4 identity decision:** canonical Oteryn identity never comes from a source numeric id. A source
  revision change preserves the old binding and adds a new one without reminting (rules 5 and 8).
- **WO-0** (D93): a routed id's family key reuses its Item sequence number, and the Item record
  stays as a non-materializable pointer.

## 4. Decision

### 4.1 Epoch rule (D96)

- **Sequence.** Epoch 2 assigns the 412 donor ids, in ascending donor source id, the sequence
  numbers 38,094 to 38,505: the highest sequence allocated by any earlier epoch (38,093) plus the
  rank. The key is `oteryn:item.registry.iNNNNNNNN`, revision `definition-r1`, and the authorship is
  `OTERYN_OPAQUE_REGISTRY_ALLOCATION_EPOCH_2`.
- **Frozen input.** The epoch is frozen by the donor census bytes (digest recorded by B1b), the
  donor commit `00ce02a57ca5a12e48f32a3476e37471167e4c3f` and the `items.xml` digest in the
  census. Its allocation digest is recorded the same way as CW2-B1's.
- **Epoch 1 untouched.** `protected_cw2_b1_full_item_family_import`, its constants, its 38,157
  keys and its evidence digest do not change. Epoch 2 is a separate function and constants next to
  it. Its closure checks are:
  - exactly 412 rows;
  - no source id shared with epoch 1;
  - no key shared with any bound key.
- **Never reused.** A sequence number is allocated once. Numbers of retired keys (such as 2,921)
  are never reassigned.
- **Later epochs.** Each further donor generation is epoch N, starting after the highest sequence
  of all earlier epochs. Its input is only the ids absent from every earlier epoch's corpus.

### 4.2 Bindings and records

- **Bindings.** Each donor id gets one G4 binding with these fields:
  - `source_key` `oteryn:source.crystalserver`;
  - `source_revision` `00ce02a5…`;
  - `identity_namespace` `ots/item_server_id`;
  - `external_id` verbatim;
  - `disposition` `EXACT`;
  - `target` the epoch-2 key.

  Base bindings are unchanged.
- **Item records.** Each donor id gets one Item record: `materializable: false` and stack class
  `Unknown`, as for epoch-1 opaque keys. Promotion, facts and Presentation belong to B2 and B3.
  The 13 unresolved ids get keys too, because identity does not depend on classification.
- **Routed ids (D97).**
  - The 138 routed ids are Item pointers with `routed_to`, once the WO-1 Item schema amendment
    exists.
  - WO-2 gives them `oteryn:world-object.registry.iNNNNNNNN` or
    `oteryn:terrain.registry.iNNNNNNNN` with the same number, by the D93 rule.

### 4.3 Re-pinning the base

When B3 or a later re-pin moves the base to a revision that already contains the donor ids, the
following holds:

- CW2-B1 is not re-run over the new corpus.
- The existing epoch-2 keys stay.
- A binding for the new source revision is added (G4 rule 8).
- An id present in a new base and in no epoch's corpus goes to a new epoch.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| B1b | Epoch-2 function, constants and tests beside the frozen import; the 412 bindings and Item records; independent identity review. The Rust change needs the shared lease. | this decision |
| B2, B3 | Wiki evidence and facts for the donor ids | B1b |
| WO-2 | Family keys for the 138 routed donor ids | WO-1, B1b |

## 6. Rejected options

- **A. Re-freeze one epoch over 38,568 items.** It needs a renumbering policy that proves 38,157
  keys byte-identical and an allocator rewrite, and it gives no player-visible gain.
- **C. Defer.** It blocks items that exist in the Reference (Global 2026-09-27), and with them B2,
  B3 and part of WO-2.
- **A separate namespace per epoch.** It would make keys carry their origin, which the G4 decision
  keeps in bindings instead, and it would break the WO-0 D93 number reuse.

## 7. Decision test

- **Must decide now:** YES. B1b is parked, and B2, B3 and the WO-2 donor part wait on it.
- **Minimum sufficient:** one additive function with constants, 412 bindings, and no change to the
  frozen import.
- **Superseding evidence:** a source that renumbers existing ids (G4 rule 8 covers it); a donor id
  that later proves to be an alias of an existing item (`ACCEPTED_ALIAS`, no remint).
- **Deliberately not decided:** facts, promotion, Presentation binding, runtime or wire ids.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#162 5877360232 (A8, B1b DESIGN_DECISION_REQUIRED)"
owner_decisions: [D96, D97]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_A8_DONOR_ITEM_IDENTITY_EPOCH_DECISION_2026-09-28.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # B1b may be re-allocated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (epoch rule, frozen import untouched)"
implementation_lanes: [B1b]
required_revalidation:
  - "B1b: the 412 donor ids get 38,094..38,505 in ascending source id; the epoch-1 import, its 38,157 keys and digest are byte-identical; no source id or key collides; the allocation is reproducible from the frozen census; 2,921 stays retired"
  - "B1b: every donor id has exactly one EXACT binding at 00ce02a5; base bindings unchanged"
remaining_unknowns:
  - which donor ids later prove to be aliases of existing items
next_action: "#162 validates this exact head, routes the independent review, integrates it, then re-allocates B1b."
```
