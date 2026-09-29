# Item identity mirrors Tibia item id numbering

- Decision: `ITEM-ID-TIBIA-MIRROR-V1` (task ITEM-ID-M0)
- Status: **CANDIDATE with a final owner decision (§2)**. Acceptance requires exact-head validation, independent
  identity review and protected integration.
- Owner decision: #162 comments 5892865958 (proposal) and 5892949642 (accepted, 2026-09-29)
- Amends:
  - `docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md`, for the Item family and its routed
    WorldObject/Terrain keys only;
  - `reviews/OTERYN_GAME_A8_DONOR_ITEM_IDENTITY_EPOCH_DECISION_2026-09-28.md` §4.1 (sequence rule), which this
    decision supersedes.
- Admission baseline: `main@b90f85c9`
- Identity minting, runtime, registry and production authority: **NONE**. The migration (ITEM-ID-M1) mints under its
  own allocation with independent identity review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

Item keys are an Oteryn-allocated opaque sequence (`oteryn:item.registry.iNNNNNNNN`), positional over each closed
corpus and appended by epochs (CW2-B1 epoch 1, A8 epoch 2). Only 20 of 38,496 numeric keys equal their Tibia id. Ids
that no pinned engine `items.xml` defines (9,310 in the 15.30 client) have no key, and each client update would need
a new epoch decision. Should the Item key instead mirror the Tibia item id?

## 2. Owner decision

| # | Decision | Owner words (2026-09-29) |
|---|---|---|
| ID1 | The Item key number is the Tibia item id, for every id, whether or not Oteryn knows the item yet. This rule is permanent. | "powinniśmy byli odwzorować numerację z pliku dat bez znaczenia czy my ten item znamy czy nie"; "wybierzmy tę opcję i już będziemy się tego trzymać" |

Rationale given by the owner: history stays 1:1 with the client, and future client updates map without
re-allocation.

## 3. Facts

**PROVEN** (owner's 15.30 client `appearances.dat` sha256 `2dfa943b…`, B3 #1197; pinned engine `.dat` files)

- **One numbering.** Crystal, Canary and the 15.30 client use one id space: the client appearance object id is the
  `items.xml` id and the `.otbm` map id. There is no separate OTB server id in these sources.
- **Stability.** Against the Canary and Crystal `.dat` (about 42.1k common ids) and the donor `.dat` (43,392 common
  ids), 15.30 renames no common id and changes flags on 2–4.
- **Additions.** 15.30 adds 1,409 ids against Canary/Crystal and 124 against the donor, all ≥ 52,977. About 70% fill
  gaps inside the recent high range (CipSoft reserves blocks), so new ids are not only appended at the end.
- **Removals are rare and not reused:** one id (53161) is present in Crystal's `.dat` and absent from 15.30.
- **Client file:** 43,516 object ids in 100–55,117 with 11,502 gaps.
- **Bindings** (`imports/crystalserver/bindings/items.json`): 38,561 rows, namespace `ots/item_server_id`, 65 of them
  targeting semantic keys (for example `oteryn:item.currency.gold_coin`). 4,590 bound ids have no client
  appearance (for example 1–99). The union of bound and client ids is 48,106, max 55,117.
- **References:** 38,100 distinct `oteryn:item.*` keys are referenced in `content/**` and `apps/**`.

## 4. Decision

### 4.1 Key rule (ID1)

- **Item key:** `oteryn:item.tibia.iNNNNNNNN`, where `NNNNNNNN` is the Tibia item id zero-padded to 8 digits.
  Revision `definition-r1` as today.
- **Namespace.** The Tibia item id is the shared id of the client appearance objects and modern OTS `items.xml`
  (identity namespace `tibia/item_id`). An `ots/item_server_id` binding from Crystal, Canary or the donor at a pinned
  revision binds to the key with the same number.
- **Coverage.** Every id in a pinned client `appearances.dat` or a pinned engine `items.xml` has a key, known or not.
  Family and facts stay UNKNOWN until evidence or an owner decision sets them. Identity never waits on family.
- **Routed families (WO-0 D93 unchanged):** `oteryn:terrain.tibia.iN` and `oteryn:world-object.tibia.iN` reuse the
  same number, and the Item record is the non-materializable `routed_to` pointer.
- **A new string, not a new meaning.** Old keys `oteryn:item.registry.iN` are retired and never re-meant. Reusing
  them would silently point every reference at a different item.

### 4.2 Stability and updates

- A key never changes and its number is never reused.
- **New client or engine revision:** each new id gets its key directly. No epoch, allocation order or decision is
  needed. The update task pins the new file and records the diff (new, changed flags, removed).
- **Removed id:** the key stays, and its record is marked absent from that client revision (exact field set by M1).
- **Renumbering or reuse by CipSoft** (never observed): stop with `CONFLICT` and escalate. Do not remint silently.

### 4.3 Oteryn-own items

Items Oteryn authors without a Tibia id use the semantic G4 form `oteryn:item.<category>.<name>`, never a numeric
Tibia-form key. Future CipSoft ids can therefore never collide with them.

### 4.4 Open questions for the independent review

- **Q1. The 65 semantic keys** (gold coin and others) that bind Tibia ids. Recommendation: keep them unchanged,
  because they are already G4-conformant and runtime code references them. The binding for their Tibia id targets
  the semantic key, and the rule "semantic key if bound, else `item.tibia.iN`" is documented with that exact list.
  Alternative: move them to the numeric form too, at the cost of a runtime change.
- **Q2.** The exact "absent from client revision" field for removed ids (M1).

## 5. G4 amendment

- G4 keeps its invariant for every other family and source.
- For the Item family (and routed Terrain/WorldObject keys), the canonical key's number is the Tibia item id by owner
  decision ID1. This is a deliberate exception to "external numbering never participates in minting this key".
- G4 rules 1–12 still apply to every binding: exact source, revision and namespace, and crosswalk states.
- Rule 3 still holds for other numbering schemes, such as TibiaWiki page ids and legacy OTB ids: equal numbers mean
  nothing.

## 6. Migration (ITEM-ID-M1, separate allocation)

- **CW2-B1 importer and A8 epoch-2 generator** (`apps/game-server/src/content/cw2_b1_import.rs` and the binding
  generator): key from the source id instead of the sequence. The protected evidence digests change. Owner:
  Content/World, with independent identity review.
- **Bindings:** retarget every numeric row to its `item.tibia` key, and commit an old→new alias table for history.
- **References:** mechanical rewrite of the 38,100 referenced keys in `content/**` and `apps/**`, then run the
  materialized-tree validator and the Rust suite.
- **New ids:** the 9,310 client-only ids and the 8 A8 held ids get keys by §4.1. The held ids still pass the alias
  gate first.
- **WO-2 is held** until M1 lands, so ~21k family keys are minted once, on the final numbering.

## 7. Validation of this candidate

Docs only: the governance and repository policy validators, and `git diff --check`.
