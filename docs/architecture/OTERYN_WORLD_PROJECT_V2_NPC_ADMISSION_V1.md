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
| Dialogue records admitted | 0 in the first wave; 701 later (§7 slices 4b and 4c) |
| NPCs removed from Tibia Global | 5 held later (§7 slice 4d) |
| Offer prices from both wikis | 21 later (§7 slice 4d) |

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
| Presentation | `oteryn:presentation.npc.<slug>` |
| Behavior | `oteryn:behavior.npc.<slug>` |
| Trade service | `oteryn:service.trade.<slug>` |
| Travel service | `oteryn:service.travel.<slug>` |
| Dialogue | `oteryn:dialogue.npc.<slug>` |

`<slug>` is derived once from the registered NPC name and frozen at admission; a later rename keeps
the key. The NPC declaration lists its services in `services`. Its `dialogue` names the NPC's Dialogue
declaration once one is admitted (§7 slice 4b).

**Item references.** Offers and non-gold currencies resolve the source item id through the protected
Item identity map (`export_reference_item_identity_map`, map SHA-256 `83ba3c26…`, the digest the Item
classification crosswalk records). Canary and Crystal share the item id space, as the creature
admission established. 10,017 of 10,055 Canary shop rows resolve; the 38 that do not name Items the
registry does not have yet and wait until the Item domain registers them, like the Items the monster
admission waits for. Items stay identity
records; no Item field is added (`sold_by`/`bought_by` stay forbidden).

**Provenance.** Each NPC gets source identity bindings to `oteryn:source.canary` (added by the creature
admission) and/or `oteryn:source.crystalserver` (added by the Item family; the same Crystal revision)
with namespace `canary/npc-file` or `crystalserver/npc-file` and the source file stem as external id,
disposition `EXACT`. Where the wiki decided a fact or confirmed a single-source NPC (D6), a
`mediawiki/page_id` binding records the TibiaWiki page under the source `oteryn:source.tibiawiki`,
revision `tibiawiki-npc-<first 16 hex of the snapshot SHA-256>`, with its own import batch. The
candidate report records the snapshot and Item map digests.

## 4. Records

| v2 record | Carries in wave A |
|---|---|
| `NPC` declaration | identity, `presentation`, `behavior`, `services`, and candidate `fields` (namespaced paths such as `oteryn:source.npc.profession`): profession and speech bubble |
| `Presentation` (Generic Reference record, `ClientSafe`) + Presentation profile | the outfit, in the monster format: `asset_binding` `canary.appearance:outfit/<lookType>` (or `canary.appearance:object/<lookTypeEx>` for the 31 item-look NPCs), palette slots Head/Body/Legs/Feet `canary.appearance:palette/<color>`, Addon attachments `…/outfit/<lookType>/addon-<bit>`, a Mount attachment with its Mount palette slots; `light_level` 0 |
| `Behavior` (Generic Reference record, `ServerOnly`) + Behavior profile | `movement.can_walk` (false for the 105 NPCs with walk interval 0), `movement.wander` `{interval_ms, radius_tiles}` from the source walk interval and radius; targeting not hostile and not targeting; no attacks, voices (D5) or summons |
| `Service` (trade) | `offers`: Item, direction (`SellToPlayer` for the source `buy` price, `BuyFromPlayer` for `sell`), unit price, currency (absent = gold), and the new count and sub type (§6) |
| `Service` (travel) | the new typed routes (§6) |

The NPC reuses the creature admission's Presentation and Behavior profiles instead of candidate fields.
The only extension is `ProjectV2Movement.wander` (slice 2b): an optional `{interval_ms, radius_tiles}`
that requires `can_walk` and a positive interval. `floorchange` is false for all 984 NPCs and is not
carried. The Generic Presentation and Behavior records are executable Reference records, as for
monsters; the NPC declaration itself stays declarative.

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

Implemented in slice 2 (`apps/game-server/src/content/project/v2.rs`): `ProjectV2ServiceOffer.count`
(positive) and `.sub_type`; `Service.routes` of `ProjectV2TravelRoute { key, destination:
ProjectV2TravelDestination { coordinate_frame, x, y, floor }, price, premium, min_level }`. Offers are
sorted and unique, with one price per Item row (Item, direction, currency, count, sub type); route keys
are sorted, unique lowercase slugs (`[a-z0-9]+` joined by `_`, at most 64 bytes); destinations have a
non-empty coordinate frame, x and y in 0..=65535 and floor 0..=15. Tests:
`apps/game-server/tests/content_world_project_v2_npc_admission.rs`.

## 7. Slices

1. This decision (with the promotion candidates and the Item join in the same change).
2. Rust: §6 extension, `canonicalize` and validation, focused positive and negative tests. No content change.
   Done in `OTV2-20260927-npc-admission-v2-services`.
2b. Rust: `ProjectV2Movement.wander` and the NPC Presentation/Behavior binding of §4, with tests.
   Done in `OTV2-20260927-npc-admission-presentation-behavior`.
3. Writer and pilot: about 20 NPCs through v2 load and validation, including a travel NPC, a shop with a
   non-gold currency, count and sub-type offers, and a single-source NPC decided by the wiki.
   Done in `OTV2-20260927-npc-admission-pilot`: `tools/content-migration/npc_admission_stage.py` stages the
   candidates (`--pilot` for the 20 NPCs); `materialize_content_world_project_v2` pins the staged file.
   Travel destinations use the project coordinate frame. Travel discounts (the postman) stay in the
   candidates. The one NPC whose sources declare no walk configuration is deferred rather than given the
   engine default.
4. Wave A in bulk (984 NPCs) and regeneration of the content tree with the NPC and Service families.
   Done in `OTV2-20260927-npc-admission-wave-a`: 983 NPCs, 289 trade and 53 travel Services, 2,035 source
   bindings; the successor tree gains `content/npcs/definitions` and `content/services/{trade,travel}`.
   Dragon Ancestor Spirit is deferred (`MOVEMENT_UNDECLARED`): neither source declares its walk configuration.
   D8 (`OTV2-20260927-npc-admission-wiki-completion`) then admits 110 more NPCs, all confirmed by the wiki:

   - 82 NPCs that neither source places but whose wiki page has a position;
   - 17 Day/Night or stage variants confirmed by their base name's page;
   - 2 NPCs confirmed by the infobox `actualname`;
   - 2 Canary misspellings confirmed by a single-edit match;
   - 2 seasonal NPCs without a fixed position (Santa Claus, Messenger of Santa);
   - 5 NPCs whose conflicting source placements the wiki position decides.

   A sandbox fix (one symbol per name, neutral `configManager` values and unset global storage) loads
   Towncryer, Captain Dreadnought, Enpa-Deia Pema and Enpa Rudra. The first two are admitted. The two Enpa
   NPCs, which had passed only on Crystal while Canary failed to load, are now held for their conflicting
   outfit colours.

   Wave A is then 1,093 NPCs, with 308 trade and 55 travel Services and 2,281 bindings.

   Still held:
   - 7 unplaced NPCs without a wiki page;
   - 9 NPCs unknown to the wiki;
   - 2 rejected server-only NPCs;
   - 9 definition conflicts;
   - 1 placement conflict;
   - 3 naming issues;
   - 3 NPCs that do not load.
4b. Dialogue: `OTV2-20260928-npc-dialogue-wave-a` (D9). `tools/content-migration/npc_dialogue_stage.py`
   builds the typed Dialogue declarations from the Canary and Crystal NPC texts, and
   `npc_admission_stage.py --dialogues` links them to their NPCs. Only static content is admitted:
   - greet, farewell, walk-away and send-trade messages, as lists of parts;
   - ambient voices with their cadence and say/yell mode;
   - `say` keyword replies with no condition and no effect, in source sibling order, with their
     conversation flags and fallback nodes.

   Conditional, scripted and action keywords (5,927 nodes) are left out, and so is every reply that an earlier
   sibling left out here can shadow (815 nodes; the engine answers with the first matching sibling, the rule
   `convert.py` applies to services). A voice profile
   without an interval or chance is left out (21 NPCs). A dialogue is admitted when both sources agree, or
   when only one source has the NPC; a source bundle named by a candidate must exist. Each text bundle is
   authenticated against the committed census: the reference-only conversion must reproduce the census
   `bundle_digest`, and the text bundle without its text must equal its reference bundle. 129 NPCs whose
   sources disagree are held with `DIALOGUE_CONFLICT`; Captain Dreadnought, whose script registers keywords
   in Lua `pairs()` order and so converts differently per run, is held with `TEXT_BUNDLE_UNVERIFIED`. 617 Dialogue declarations (4,220 keyword nodes) are admitted;
   the deferred Dragon Ancestor Spirit keeps none. The materializer verifies each admitted Dialogue against
   the pinned dialogue evidence. The successor tree
   gains `content/dialogues/definitions`.
4c. Dialogue conflicts (D10): `OTV2-20260928-npc-dialogue-transcripts`. `npc_dialogue_stage.py --transcripts`
   breaks Canary/Crystal dialogue conflicts with the Tibia Global in-game transcripts of `s2ward/tibia`
   (commit `8824eb38`): the source whose differing texts match more transcript lines is admitted whole, and
   the decision, its scores and the transcript used are recorded (`resolved`, `source.transcripts_digest`).
   84 conflicts are resolved (75 Canary, 9 Crystal). 45 stay held: 44 whose transcript matches neither
   side better and one without a transcript. 701 Dialogue declarations (6,313 keyword nodes) are admitted.
4d. Removed NPCs (D11): `OTV2-20260928-npc-removed-from-game`. The five Duelling Arena supervisors that both
   TibiaWiki BR and Fandom record as removed in 13.12 are held `REMOVED_FROM_GAME`, with Victor's trade
   Service. Under D12, 21 offers of 13 NPCs take the price TibiaWiki Fandom and TibiaWiki BR both state
   (`WIKI_PRICE`), which adds their wiki page bindings. Wave A is then 1,088 NPCs, with 307 trade and 55
   travel Services and 2,282 bindings; the 701 Dialogues are unchanged (none of the five had one).
5. Later:
   - placements after World admission;
   - conditional dialogue and dialogue conflicts;
   - held NPCs, offers and routes as their blockers close.

Each slice runs the repository gates. A slice that changes `content/world/**` also gets one independent
exact-head review before the Merge Queue (standing authorization in `OWNER_FUNDED_AI_POLICY.md`).

## 8. Boundaries

Admission adds no runtime behaviour: no conversation, trade settlement, currency movement, travel
execution, spawn or placement. It admits no Lua, no gated offer or route, and changes no
Item semantics. Everything stays candidate-only under the v2 rule.
