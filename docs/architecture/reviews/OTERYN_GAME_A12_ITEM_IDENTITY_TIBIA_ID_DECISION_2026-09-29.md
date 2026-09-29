# A12 Item identity equals the Tibia id

- Decision: `A12-ITEM-IDENTITY-TIBIA-ID-V1`
- Status: **CANDIDATE with owner decisions D146-D148 (§2)**. Acceptance requires exact-head
  validation, independent identity review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Answers: #162 comment 5892865958 (item-authoring lane, `ARCHITECTURE_ESCALATION_REQUIRED`)
- Ruling posted: #162 comment 5892983885
- Supersedes: `OTERYN_GAME_A8_DONOR_ITEM_IDENTITY_EPOCH_DECISION_2026-09-28.md` (D96, D97) and the
  sequential key allocation of CW2-B1 (`apps/game-server/src/content/cw2_b1_import.rs`)
- Amends:
  - `docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md`, for CipSoft ids only;
  - `OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md` D93.
- Runtime, migration, registry and production authority: **NONE**. The migration lane (§5) changes
  keys under its own allocation, with independent identity review.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

The Item registry has 38,496 opaque sequential keys (`oteryn:item.registry.iNNNNNNNN`). Only 20 of
them carry the item's Tibia id as their number. Should the canonical key follow the Tibia id
instead? This must be settled before WO-2 mints about 21,000 WorldObject and Terrain family keys on
the D93 same-number rule, because those keys inherit whichever numbering the Item registry uses.

## 2. Owner decisions

The owner confirmed these directly in this session on 2026-09-29, on top of the coordinator's relay
in 5892865958.

| # | Decision | Owner choice |
|---|---|---|
| D146 | The canonical key of every Tibia item is its Tibia id: `oteryn:item.tibia.i<id>`. This applies to all ids, whether or not Oteryn knows what the item is. It happens now, before WO-2. It supersedes A8 (D96, D97) and the CW2-B1 sequential allocation. | "Tak, teraz przed WO-2" |
| D147 | The 64 semantic named keys (for example `oteryn:item.currency.gold_coin`) stop being keys. Each Tibia item has exactly one key, and the name lives on as a code constant that points at it. | "Numer Tibii + stała w kodzie" |
| D148 | Items that exist only in Oteryn use a separate namespace, `oteryn:item.oteryn.<slug>`, never a numeric range. | "Osobna przestrzeń nazw" |

Decision number note: D140-D144 were already allocated to D4 (#1218) when A11 recorded its owner
decision as D140. The A11 decision is renumbered D145 in this change (§6).

## 3. Facts

**PROVEN** (item-authoring evidence in 5892865958, B3 client 15.30 `2dfa943b` against the Canary,
Crystal and donor `.dat` files)

- Between about 42,100 and 43,400 ids are common to the compared sources. None of them was renamed,
  and only 2 to 4 changed flags. CipSoft does not renumber items.
- Every new id is 52,977 or higher. About 70% of them fill gaps inside the recent high range, so
  CipSoft reserves blocks rather than only appending.
- One id, 53161, was removed.
- The registry holds 38,496 sequential keys. Content and apps reference 38,100 distinct keys.

**PROVEN** (main `b90f85c9`)

- G4 separates canonical identity from source numeric ids and says a source numeric id "can never
  mint or replace the canonical Oteryn definition identity". It was written for OT servers and
  donors, which renumber, collide and reuse ids.
- Durable Item rows store the key in `definition_production_key` (`0010`, `0011`, `0014`). Receipt
  tables are immutable.
- WO-0 D93 gives each routed family key the same number as its Item key.

## 4. Decision

### 4.1 Key format (D146)

- **Format.** The key is `oteryn:item.tibia.i<id>`, and its revision is `definition-r1`.
  - `<id>` is the item's decimal Tibia id, with no padding and no leading zero, so each id has
    exactly one key string.
  - Authorship is `OTERYN_TIBIA_ID_KEY_RULE_V1`. There is no allocator and no epoch: a key is the
    rule applied to the id.
- **Which id.** `<id>` is a CipSoft appearance object id in the **admitted CipSoft id set**. The
  set is the union of the appearance object ids in every CipSoft appearance file Oteryn has admitted
  as evidence:
  - client 15.30 `2dfa943b` (the B3 pin);
  - the CipSoft appearance files pinned by the Canary, Crystal and donor corpora compared in
    5892865958.

  The set only grows. A later admitted client adds ids and never changes what an existing key
  means.
- **Current and retired ids.** A key is *current* while its id is in the newest admitted client.
  Otherwise it is *retired*.
  - For example, 53161 is in an admitted older CipSoft file and absent from 15.30, so
    `oteryn:item.tibia.i53161` exists as a retired key.
  - A retired key resolves for history and existing rows, but no new Item can be minted with it.
  - If CipSoft ever reuses an id for a different item, the change stops with
    `ARCHITECTURE_ESCALATION_REQUIRED`; no key is re-meant silently.
- **Ids outside the set.** An OT server or donor id that is not a CipSoft appearance object id in
  the admitted set gets no Tibia key. It stays `UNKNOWN`, and loading or minting it fails closed
  until it is bound to a Tibia id (`ACCEPTED_ALIAS`) or authored in the D148 namespace.

### 4.2 G4 amendment

- The Reference for Oteryn is Global Tibia (CipSoft). CipSoft's item id therefore counts as
  canonical identity, not as a source numeric id. This is the only exception to G4's "never from a
  source numeric id" rule.
- Canary, Crystal, donors, TibiaWiki and every other source keep G4 unchanged. Their ids are
  bindings (`ots/item_server_id` and the like) that target the Tibia key.
  - `EXACT` needs G4 identity evidence that both records are the same item, for example the
    source row's appearance reference resolving to that CipSoft appearance record, with name and
    flags corroborating it.
  - Equal numbers only corroborate that evidence and never prove it (G4 rule 3).
  - A source row without that evidence is not bound `EXACT`, and it stays in the G4
    ambiguous/conflict path.
- The four-layer separation stays in force: canonical key, source id, presentation identity and
  runtime or wire id. For CipSoft items the canonical number and the wire number coincide by rule,
  not by merging the two layers.

### 4.3 Semantic names (D147)

- Each named key (such as `gold_coin`) becomes a code constant, for example
  `GOLD_COIN: ItemKey = "oteryn:item.tibia.i3031"`, in the module that owns the name today.
- The named key string is retired together with the registry keys (§4.5).
  - After the migration, no authored content, code or new write may name it as a key.
  - Historical durable rows and receipts that already hold it keep it, and they resolve through the
    alias table (§4.5).
- The R7-P04 re-key and the "2921 never reused" rule become history. The gold coin's canonical key is
  its Tibia key.

### 4.4 Oteryn-only items (D148)

- The key is `oteryn:item.oteryn.<slug>`, where `<slug>` is lowercase ASCII snake case and is
  unique within the namespace.
- The namespace carries no numbers, so a future CipSoft id can never collide with it.
- An Oteryn-only item that CipSoft later adds as a real item keeps its Oteryn key. It gets a
  binding to the Tibia key, and the owner decides whether to merge the two, with the Oteryn key
  retired as an alias.

### 4.5 Retirement and alias table

- **Retired keys.** Every `oteryn:item.registry.iNNNNNNNN` key (epochs 1 and 2) and every named
  Item key is retired. None is ever reassigned or re-meant.
- **Alias table.** An append-only alias table holds exactly one entry for every retired key. It lives
  with the Item registry, and its digest is recorded.
- **Successor entries.** An entry is either an alias or a terminal disposition:
  - An alias names the Tibia key derived from that retired key's own proven Tibia id. The proof is
    the §4.2 identity evidence for the source row that produced the old key, and each entry records
    it. For a named key, the proof is the Tibia id of the item the name stood for.
  - `retired_without_successor` applies when the old key has no proven CipSoft id in the admitted
    set (§4.1).
- **Referenced keys without proof.** A retired key without a proven Tibia id that authored content
  or code still references blocks ITEM-ID-1 (fail-closed). It is never guessed.
- **Durable rows.** Durable rows are not rewritten:
  - A stored row or receipt that holds a retired key keeps it, because it was true when written.
  - Readers resolve it through the alias table.
  - Writers accept only canonical keys, so a write naming a retired key is rejected.
  - This leaves receipt immutability intact.

### 4.6 WorldObject and Terrain (D93 follows)

- The family keys become `oteryn:world-object.tibia.i<id>` and `oteryn:terrain.tibia.i<id>`, with
  the same number as the Item key.
- The Item entry stays a non-materializable pointer.
- WO-2 mints only on this numbering. It starts after the migration lane (§5) merges.

## 5. Delivery

| Child | Scope | Depends on |
|---|---|---|
| ITEM-ID-1 migration | One mechanical change. Replace the CW2-B1 and epoch-2 allocation with the §4.1 rule, rewrite every key reference in content and apps (38,100 distinct), write the alias table, turn the 64 semantic keys into constants, and re-target the G4 bindings. The protected input evidence digests of CW2-B1 and epoch 2 stay as history. This needs the shared Rust lease and independent identity review. | this decision |
| WO-2 | Family keys on the Tibia numbering | ITEM-ID-1 |
| B4 / epoch 3 | Dropped; the §4.1 rule covers every future client id. | none |

ITEM-ID-1 must prove all of the following:

- Every canonical key follows §4.1.
- The alias table is total: every retired key has exactly one entry, either an alias or
  `retired_without_successor`.
- Every alias target exists in the canonical set and is derived from that retired key's own recorded
  Tibia-id evidence, never from a positional or bulk mapping.
- No two retired keys that meant different items map to the same key.
- Every key reference in authored content and code is rewritten, and none names a retired key.
- Every durable row with a retired key resolves through the alias table.
- The G4 bindings target the new keys, with no binding lost.
- Semantic constants resolve to existing Tibia keys.
- No Tibia key coincides with an Oteryn-namespace key.

## 6. Correction to A11

- In `OTERYN_GAME_STANCE0_CHARACTER_STANCE_PERSISTENCE_DECISION_2026-09-29.md`, D140 is renumbered
  D145. D140 in D4 is unchanged.
- The baseline migration number in that document is now stale, because `0015` was taken by corpse
  decay. STANCE-1 takes the next free number when it is allocated.

## 7. Rejected options

- **Keep opaque sequential keys and hold the Tibia id only in bindings.** This keeps the allocator,
  the epochs and a second numbering. It makes WO-2 mint about 21,000 keys that differ from the ids
  everyone else uses.
- **Migrate after WO-2.** The same migration would then also cover about 21,000 family keys.
- **Zero-padded numbers (`i00003031`).** This adds a second spelling for the same id and has no
  reader benefit.
- **A reserved numeric range for Oteryn-only items.** CipSoft reserves blocks in the high range, so
  a range could collide.

## 8. Decision test

- **Must decide now:** YES. WO-2 is blocked, and every day on the old numbering adds keys to migrate.
- **Minimum sufficient:** a deterministic key rule, one mechanical rename, an alias table, and no
  rewrite of durable rows.
- **Superseding evidence:** CipSoft renumbering or reusing an id, which triggers escalation (§4.1).
- **Deliberately not decided:** item facts, Presentation, wire encoding beyond §4.2, and which
  Oteryn-only items exist.

## 9. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#162 5892865958 (item-authoring, ARCHITECTURE_ESCALATION_REQUIRED)"
owner_decisions: [D146, D147, D148]
renumbered: {A11_stance: "D140 -> D145"}
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_A12_ITEM_IDENTITY_TIBIA_ID_DECISION_2026-09-29.md
supersedes: [A8 D96, A8 D97, CW2-B1 sequential allocation]
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # ITEM-ID-1 may be allocated; WO-2 waits for ITEM-ID-1
required_fresh_allocation: true
required_independent_review: "exact-head identity review of this decision; ITEM-ID-1 identity and migration review"
implementation_lanes: [ITEM-ID-1, WO-2]
remaining_unknowns:
  - which OT or donor ids lie outside the admitted CipSoft id set (they get no Tibia key)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates ITEM-ID-1."
```
