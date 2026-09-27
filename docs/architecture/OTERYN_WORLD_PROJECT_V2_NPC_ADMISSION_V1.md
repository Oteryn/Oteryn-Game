# Oteryn WorldProject/v2 NPC admission v1

- Date: 2026-09-27
- Status: CANDIDATE / admission route for the NPC promotion candidates; implementation follows in the slices of §7
- Task: `OTV2-20260927-npc-promotion-candidates`
- Programme: KAN-16 / #162
- Companions: `OTERYN_NPC_AUTHORING_SCHEMA_V1.md` (authoring format, decisions D1–D7, promotion candidates §8),
  `OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1.md` (the same route for monsters),
  `OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md` (v2 declarative NPC/Service rules),
  `OTERYN_FIRST_REFERENCE_NPC_SERVICE_BOUNDARY_2026-09-09.md` (runtime NPC service boundary)

## 1. Decision

NPCs enter the game through the protected WorldProject/v2 project, the only content the server reads,
as the monsters do. The generated content tree (`content/npcs/**`, `content/services/**`) is then
regenerated from v2; it is never written by hand.

v2 already declares the `NPC`, `Dialogue` and `Service` families as **declarative, candidate-only**
records (`ProjectV2Declaration`). No runtime path interprets them, and admission does not change that:
NPC conversation, trade settlement and travel execution stay with the `GAME-NPC-SERVICE` runtime
boundary, which is not implemented.

Owner decision **D7** (2026-09-27): v2 is extended first with typed travel routes and offer
quantities, so that the NPC services are admitted as typed data rather than untyped candidate
fields. This mirrors the owner's "druga droga" choice for monsters.

## 2. Scope of the first admission (wave A)

Input is `tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json`: Canary `47dfd51f`
and Crystal `ff7ede59` merged under D4–D6, with the TibiaWiki (Fandom) snapshot as tie-breaker and the
protected Item identity map for Item references.

| Group | Count |
|---|---:|
| NPC declarations (wave A) | 984 |
| Trade services | 289, with 10,723 offer rows (507 with a count, 66 with a sub type) |
| Travel services | 53, with 189 ungated routes |
| Placements admitted | 0 (§5) |
| Dialogue records admitted | 0 (D5: Oteryn-authored text first) |

Held back and reported in the candidates: 145 NPCs (91 unplaced in both sources, 32 single-source
NPCs the wiki does not know, 7 outfit/movement conflicts, 6 placement conflicts the wiki cannot
decide, 2 key collisions, 1 name without a slug, 6 not loadable in either source), 218 offers
(122 gated by player storage, 52 one-sided and unconfirmed, 38 Items newer than the Oteryn Item
registry, 6 conflicts) and 65 routes (46 gated by Lua predicates, 15 conflicts the wiki cannot decide, 4 one-sided routes the
wiki does not confirm).

## 3. Identities

Keys are production keys, revision `definition-r1` (D4). The `canary:` / `crystal:` keys stay provenance.

| Record | Key |
|---|---|
| NPC | `oteryn:npc.<slug>` |
| Trade service | `oteryn:service.trade.<slug>` |
| Travel service | `oteryn:service.travel.<slug>` |

`<slug>` is derived once from the registered NPC name and frozen at admission; a later rename keeps
the key. The NPC declaration lists its services in `services`; `dialogue` stays empty until
authored text exists.

**Item references.** Offers and non-gold currencies resolve the source item id through the protected
Item identity map (`export_reference_item_identity_map`, map SHA-256 `83ba3c26…`, the digest the Item
classification crosswalk records). Canary and Crystal share the item id space, as the creature
admission established. 10,017 of 10,055 Canary shop rows resolve; the 38 that do not name Items the
registry does not have yet and wait until the Item domain registers them, like the Items the monster
admission waits for. Items stay identity
records; no Item field is added (`sold_by`/`bought_by` stay forbidden).

**Provenance.** Each NPC gets source identity bindings to `oteryn:source.canary` (added by the creature
admission) and/or `oteryn:source.crystalserver` with namespace `<source>/npc-type` and the source file
stem as external id, disposition `EXACT`. Where the wiki decided a fact (D6), a binding records the
TibiaWiki page id and revision. The candidate report records the snapshot and Item map digests.

## 4. Records

| v2 record | Carries in wave A |
|---|---|
| `NPC` declaration | identity, `services`, and candidate `fields`: profession, speech bubble, outfit (look type, head/body/legs/feet, addons, mount or item look), walk interval and radius, floor change |
| `Service` (trade) | `offers`: Item, direction (`SellToPlayer` for the source `buy` price, `BuyFromPlayer` for `sell`), unit price, currency (absent = gold), and the new count and sub type (§6) |
| `Service` (travel) | the new typed routes (§6) |

Outfit and movement ride as candidate fields until the NPC presentation and behaviour are bound to the
Presentation and Behavior profiles the creature admission introduces; that binding is a later slice.

## 5. Placements

v2 has `ProjectV2Placement` (world, map revision, coordinate frame, x, y, floor), but the project holds
zero worlds today. NPC placements are therefore not admitted in wave A; they stay in the promotion
candidates until a World and its map revision are admitted, and then enter as placements of the NPC
definitions in one slice.

## 6. v2 extension (D7)

- `ProjectV2ServiceOffer` gains optional `count` (units per trade row) and `sub_type` (fluid or charge
  subtype). Both default to absent, so existing documents stay valid.
- `Service` gains `routes: Vec<ProjectV2TravelRoute>`: `key` (destination keyword slug), `destination`
  (coordinate frame, x, y, floor), `price`, `premium`, optional `min_level`. Routes are sorted by key
  and unique; a Service may carry offers, recipes or routes.
- Validation covers ranges, sorting and uniqueness and exact Item references; everything stays
  candidate-only. Gated routes and offers are never admitted: a condition belongs to the runtime owner.

## 7. Slices

1. This decision (with the promotion candidates and the Item join in the same change).
2. Rust: §6 extension, `canonicalize` and validation, focused positive and negative tests. No content change.
3. Writer and pilot: about 20 NPCs through v2 load and validation, including a travel NPC, a shop with a
   non-gold currency, count and sub-type offers, and a single-source NPC decided by the wiki.
4. Wave A in bulk (984 NPCs) and regeneration of the content tree with the NPC and Service families.
5. Later: placements after World admission; dialogue after Oteryn-authored text; held NPCs, offers and
   routes as their blockers close.

Each slice runs the repository gates. A slice that changes `content/world/**` also gets one independent
exact-head review before the Merge Queue (standing authorization in `OWNER_FUNDED_AI_POLICY.md`).

## 8. Boundaries

Admission adds no runtime behaviour: no conversation, trade settlement, currency movement, travel
execution, spawn or placement. It admits no Tibia text, no Lua, no gated offer or route, and changes no
Item semantics. Everything stays candidate-only under the v2 rule.
