# Owner decision batch D174-D235

- Decision: `OWNER-DECISION-BATCH-D174-D235-V1`
- Status: decisions recorded; docs only. Acceptance requires validation and protected integration.
- Origin: decisions given on #162 on 2026-09-30 by the owner (verbatim answers), by the Sol Supervising Architect (rulings) or by the control plane (derived or delegated). The previous batch is `OTERYN_GAME_OWNER_DECISION_BATCH_D165_D173_2026-09-30.md`.
- Numbering: continues from D173. All 62 numbers D174-D235 were found on #162.
- Runtime, schema, registry, Platform and production authority: **NONE**. Each decision names its own limits. Nothing here is a merge, release or production grant, and no contract text is changed by this record.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

"Decided by" is `owner` (the owner's own words or answer), `owner via Sol` (given to the Sol architect), `owner via lane` (given directly to a lane worker and numbered by the control plane) or `control plane` (derived from evidence, or delegated by the owner). Source ids are comment ids on Oteryn/Oteryn-Game#162 unless another place is named.

## 1. Decisions

| # | Decision | Decided by | Source |
|---|---|---|---|
| D174-D178 | Recorded in `OTERYN_GAME_CHARACTER_GOLD_FEE_BOUNDARY_DECISION_2026-09-30.md` (gold fee boundary, Q35-Q39; D178 amended to at most 2 change outputs by the control plane, 5905710497). Not repeated here | owner, control plane | 5905498654, 5905710497 |
| D179 | Workflow change authorized in #1311 at head `e9b8337b`: `achievement-authoring-schema.yml` gains three trigger paths and the step `build_catalogue.py --check`. That exact change only (Q40a) | owner | 5906329668 |
| D180 | CHAR-NAME-1 (1a): a name is 2-29 ASCII letters, as words joined by single spaces; no digits, punctuation or other scripts | owner via lane | 5906729949 |
| D181 | CHAR-NAME-1 (2c): the comparison key is the name in ASCII lower case with every space removed; it is versioned and a later revision may only make it finer | owner via lane | 5906729949 |
| D182 | CHAR-NAME-1 (3a): one global name namespace across all Worlds | owner via lane | 5906729949 |
| D183 | CHAR-NAME-1 (4b): a reservation is held while its Character holds the name; after a rename or deletion the name stays blocked for everyone for 30 days | owner via lane | 5906729949 |
| D184 | CHAR-NAME-1 (5a): the name travels as the required `requested_name` in bootstrap intent v2; Platform and Game change in lockstep and Game refuses v1 | owner via lane | 5906729949 |
| D185 | New `.github/workflows/charm-authoring-schema.yml` authorized in #1293 at head `d89f30f3` (read-only permissions, pinned actions). That exact file only (Q41a) | owner | 5906738265 |
| D186 | Refines 16a (Q42c): on an auto-attack hit on a target other than the main one, Low Blow, Savage Blow and the leech charms all apply; proc and charm damage still triggers no charm (12b) | owner | 5906844443 |
| D187 | In #1316 only the pinned Platform commit may change, and only to the merge commit of Oteryn-Platform#1428, in the listed pin files (Q43a). Q44 was resolved by the control plane from evidence: no backfill migration is needed now | owner, control plane | 5907026698 |
| D188 | ADR-0021 (1a): the full base map loads eagerly into a compact server model shared read-only by the channels of a World | owner via Sol | 5907138288 |
| D189 | ADR-0021 (2a): B3 stays the source; the server reads only a compiled server World Bundle | owner via Sol | 5907138288 |
| D190 | ADR-0021 (3a): the project frame is the source frame; the compiler maps it to native positions, floor = -z | owner via Sol | 5907138288 |
| D191 | ADR-0021 (4a): Tibia parity. Each channel gets a volatile overlay; DUR-03 Ground survives a crash and is retired at the planned reset; picking up a map item is a MINT | owner via Sol | 5907138288 |
| D192 | ADR-0021 (5a): unknown keys fail the build; the 5 provisional keys are skipped outside production and block release | owner via Sol | 5907138288 |
| D193 | ADR-0021 (6a): the new families are the only source; drafts are non-production only and block release; no dual loading | owner via Sol | 5907138288 |
| D194 | ADR-0021 (7a): a new base activates only at a planned reset | owner via Sol | 5907138288 |
| D195 | ADR-0021 (8a): the MAP01 budgets are the initial gates | owner via Sol | 5907138288 |
| D196 | ADR-0021 (house tiles, option a): until a house custody family exists, Ground items on house tiles are retired at the reset | owner via Sol | 5907138288 |
| D197 | Fist weapons use the Standard proficiency thresholds (Q46a) | owner | 5907607834 |
| D198 | Arbalest, Chain Bolter, The Ironworker, The Devileye and Thorn Spitter use the Crossbow threshold table; the class is keyed per binding (ammo type and weapon type), not per profile. Delegated by the owner ("check several wikis") and settled by evidence (Q45) | control plane | 5907741722 |
| D199 | The perk enum mapping from TibiaWiki `Weapon_Proficiency_Tables` (revid 1206177, 3,671/3,671 perks) and Canary `weapon_proficiency.hpp` is accepted (Q47a) | owner | 5907763153 |
| D200 | The Knight threshold table applies only to knight-restricted sword, axe and club weapons, following the Canary rule; every other melee weapon uses Standard; bolt weapons use Crossbow (D198) (Q49b) | owner | 5907763153 |
| D201 | Stage-A change to `.github/workflows/merge-authority-audit.yml` updating exactly `EXPECTED_MERGE_GATE_BLOB` and `EXPECTED_MERGE_GROUP_GATE_BLOB` to the blobs at #1316's final re-reviewed head (Q48a). Nothing else in the audit changes | owner | 5907807489 |
| D202 | HOUSES (1a): prepare the House catalogue owner contract | owner via lane | 5908348569 |
| D203 | HOUSES (2a): populate `content/houses/` right after the contract is accepted | owner via lane | 5908348569 |
| D204 | HOUSES (3b): no committed TibiaWiki BR facts file; the README summary and the capture workflow stay | owner via lane | 5908348569 |
| D205 | HOUSES (shops 2a): shops (`kind: shop`) are ordinary physical houses with an owner CharacterId, the personal physical-house slot, and the same auction, rent and ACL rules; "(Shop)" is only a name marker | owner via lane | 5908348569 |
| D206 | Confirms the workflow change merged with #1324 (`9ca2bfd2`) in `item-authoring-schema.yml`: 3 trigger paths and the "TibiaWiki Item stat snapshot self-test" step. Verbatim: "tak zostaje" (Q50). That exact change only | owner | 5908625173 |
| D207 | Item weight unit is hundredths of an ounce ("41.00 oz = 4100"), given by the owner to the ITEM-SEM-2b writer. The owner's verbatim words are not on #162; the control plane asked the writer to post them | owner via lane | 5909477417 |
| D208 | NPC buying, selling and travel are admitted as value sources (the D178 amendment path), verbatim "tak a" (Q1a). Also recorded in `OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md` | owner | 5909366267, 5909477417 |
| D209 | Serene is non-durable and re-evaluated at actor initialization (OQ-SERENE, accepted) | owner | 5909451973, 5909477417 |
| D210 | Chained Penance keeps 5 targets (`max_targets` 4) provisionally, pending the owner's in-game test (OQ-CP-TARGETS) | owner | 5909451973, 5909477417 |
| D211 | One control-plane change to `item-authoring-schema.yml`: run `test_item_weapon_proficiency.py` and `item_weapon_proficiency.py --check`, and trigger on `imports/crystalserver/facts/**` and the proficiency generator and artifact paths. Verbatim "zgoda" (Q51). That exact change only | owner | 5909486484 |
| D212 | ITEM-MOVE-WIRE-0, verbatim "1a 2 a". 1a: the item view and move wire is accepted behind capability 4 `ITEM_VIEW_MOVE_V1` (state domains 9-11, per-connection item handles, command type 9 `ITEM_MOVE_INTENT`, corpse opening as a USE target). 2a: the first slice is pick-up and loot into the main backpack only. Clarification: domain 10 is narrower than "closes MAP-WIRE-1", which stays open (5911192977) | owner via Sol | 5909624320, 5910151683, 5911192977 |
| D213 | Train Party includes fist fighting, verbatim "fist tez" (Q52, option b, the Fandom reading), against BR and Canary; the disagreement is recorded as known parity evidence | owner | 5910208482 |
| D214 | WO-2 (1a): the workflow edits in #1319 to `item-authoring-schema.yml` (catalogue drift step against pinned Crystal, two catalogue triggers) and `g4-canonical-worldproject-package-seed.yml` (package comparison removes the shards). Those exact edits only | owner, numbered by control plane | 5910372054 |
| D215 | WO-2 (2a): contested routes that match no rule are recorded as UNKNOWN (65 Terrain records) | owner, numbered by control plane | 5910372054 |
| D216 | WO-2c, verbatim "kontynuuj", answers 1a-5a: roof-named tile with no ground, border or wall flag is `roof`; `trashholder` keeps its kind with `behavior: trash_holder`; unbanked magic field routes to Terrain `field`; `teleport` keeps its kind with `behavior: teleport`; fixed carpet routes to WorldObject `decoration` | owner, numbered by control plane | 5910372054 |
| D217 | `house-authoring-schema.yml` gains a `content/houses/**` trigger and a `build_catalogue.py --check` step, in #1331. That exact change only | owner, numbered by control plane | 5910716161 |
| D218 | `item-authoring-schema.yml` gains 3 triggers and a `test_lower_wiki_stats_packet.py` step, in #1336 (owner "1a" from the ITEM-SEM-2b task record). That exact change only | owner, numbered by control plane | 5910716161 |
| D219 | New `.github/workflows/proficiency-authoring-schema.yml` at head `b3fbbe04` in #1327 ("1a" relayed in #1327 5909208421). That exact file only | owner, numbered by control plane | 5910716161 |
| D220 | `item-authoring-schema.yml` gains a `content/proficiencies/**` trigger in #1342 (may ride the #1342 repair push). That exact change only | owner, numbered by control plane | 5910716161 |
| D221 | Workflow edits in #1353 at head `1c71bdef`: new `area-authoring-schema.yml` (SHA-pinned actions, `contents: read`) and its G4 seed edit. Verbatim "55 a" (Q55). Those exact edits only | owner | 5911763257 |
| D222 | Amends D87: when the view list hits its 256-entry ceiling, actors (players and creatures) rank before items and within each group the rule stays nearest first. Verbatim "54 a" (Q54) | owner | 5911768800 |
| D223 | Achievement display-1 (secret): verbatim "2b"; a secret achievement is shown only once the account has earned it | owner via lane | 5912071646, 5911933242 |
| D224 | Achievement display-2 (points): verbatim "jedna liczba 3"; one number, the account total, no per-grade breakdown | owner via lane | 5912071646, 5911933242 |
| D225 | Achievement display-3 (delivery): verbatim "4b"; fetched on request when the panel opens, no push | owner via lane | 5912071646, 5911933242 |
| D226 | Achievement display-4 (world scope, supersedes the D49 display rule): verbatim "5a", confirmed by "jedna liczba 3, 5a"; all earned facts shown and all count toward points, read from the Oteryn catalogue record of each key | owner via lane | 5912071646, 5911933242 |
| D227 | Achievement display-5 (rows): verbatim "1a, 2b, 3a" resolved to 2b; the panel lists earned achievements only | owner via lane | 5912071646, 5911933242 |
| D228 | Achievement display-6 (paging): verbatim "1a, 2b, 3a" resolved to 1a; the reply is paged (64 entries). Command type 10 `ACCOUNT_ACHIEVEMENTS_QUERY` is reserved | owner via lane | 5912071646, 5911933242 |
| D229 | BANK-0 Q1, verbatim "1b": the bank balance is shared by every character of the Account within one World (architect reading; Worlds are separate economies, ADR-0010 §6) | owner via Sol | 5912865259, 5912593702 |
| D230 | DEPOT-0 Q2, verbatim "2b": one depot per Character, reachable from any town, as in current Tibia | owner via Sol | 5912865259, 5912593702 |
| D231 | Branch practice Q3, verbatim "3a": the architect may create a separate `claude/arch-...` branch per decision so all can be reviewed at once | owner via Sol | 5912865259, 5912593702 |
| D232 | HOUSES-6 close-out (1a): the engine entry tile is the entrance; the 46 houses where it is not in front of an outer door go to the base-map walkability check | owner via lane | 5913271293, 5913073586 |
| D233 | HOUSES-6 close-out (2a): the official bed count is kept; the 84 engine-map bed divergences go to the base-map owner | owner via lane | 5913271293, 5913073586 |
| D234 | HOUSES-6 close-out (3a): the closing PR goes ahead (branch `claude/tender-mendel-06tjg2`). The owner wrote "1tak, 2 tak, 3 trak" | owner via lane | 5913271293, 5913073586 |
| D235 | Charm release gate (3a): after CHARM-6 ships, charms may be released while some effects still fail closed; the client shows each as "not yet active" | owner | 5913271293, 5913227118 |

## 2. Notes

- Not found on #162: none. D174-D235 all appear; D174-D178 and D208 are also recorded in merged decision files.
- Verbatim gaps: D207 has no verbatim owner text on #162. D214 to D220 were numbered by the control plane from owner answers that live in task records or other comments; the exact wording is in the sources.
- Related answer given the same day without a D number: "tak z konta" (5913348961) admits the Market fee, house auction price and rent as gold sinks paid from the bank account (D178 gate for MARKET-0 Q1 and HOUSE-OWN-0 H1).
- Every workflow authorization (D167 earlier, D179, D185, D187, D201, D206, D211, D214, D217-D221) covers only the exact change named and nothing else.
- D198 and D187 Q44 are control-plane rulings from evidence, not owner words. The owner may override.

## 3. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
owner_decisions: [D174-D235]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D174_D235_2026-09-30.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
```
