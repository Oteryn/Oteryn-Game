# P07 Gem Atelier parity evidence — 2026-11-01

## Identity and evidence classification

- Task: `OTV2-20261101-gem-atelier`; branch: `chatgpt/gem-atelier`.
- Inspected Oteryn base: `38641fecd4558a4a14e6b15b3874110ffa2dff19`.
- Primary reference: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`.
- Secondary reference: `opentibiabr/canary@52550d5e3c0ffe565f4bd1014f4e5575abf1059b`; its Wheel files now live under `src/creatures/players/components/wheel/`.
- Reference evidence class: `OTS_HYPOTHESIS_ONLY`. The task explicitly selects Crystal behavior; this report does not claim independent Tibia Global verification.
- `PROVEN` means source inspected at those commits. Runtime validation is a separate result; no command result is inferred from source inspection.
- Status cells use only `DONE`, `PARTIAL` or `MISSING`. `DONE` identifies source parity for the stated bounded rule or a named native adaptation; it does not assert that every named test ran on this candidate. `PARTIAL` identifies an outstanding contract or scope difference. `MISSING` identifies an absent required surface. Runtime results are recorded separately in Validation.
- Validation records measured results from the isolated user-computer checkout. Changed-file anchors below refer to the formatted implementation; unchanged-file and explicitly marked Baseline anchors refer to the inspected base above. The immutable published head and its final readback/qualification are recorded in the draft PR closeout comment, avoiding a self-referential commit hash in this file. This evidence does not claim the whole mechanic is complete while Q1-Q7 remain unanswered.

The re-audit ledger is intentionally not edited by this mechanic PR. Rules are added to the existing Character Atelier, item custody, fee and revision owners.

## Source key

All donor paths in this section refer to the primary Crystal commit.

| Key | Full path |
| --- | --- |
| PW | `src/creatures/players/wheel/player_wheel.cpp` |
| P | `src/creatures/players/player.cpp` |
| G | `src/game/game.cpp` |
| WG | `src/creatures/players/wheel/wheel_gems.cpp` |
| C | `src/items/containers/container.cpp` |
| Config | `src/config/configmanager.cpp` |
| Tools | `src/utils/tools.cpp` |

Oteryn paths are relative to repository root. Atelier implementation paths share these prefixes:

| Key | Full path |
| --- | --- |
| Rules | `crates/gameplay/progression/src/atelier.rs` |
| Durable | `crates/gameplay/durability/src/durability/character_atelier.rs` |
| Actions | `crates/gameplay/durability/src/durability/character_atelier/actions.rs` |
| Custody | `crates/gameplay/durability/src/durability/character_atelier/custody.rs` |
| Materials | `crates/gameplay/durability/src/durability/character_atelier/materials.rs` |
| Outputs | `crates/gameplay/durability/src/durability/character_atelier/outputs.rs` |
| Fee | `crates/gameplay/durability/src/durability/item_fee_burn.rs` |
| AtelierFee | `crates/gameplay/durability/src/durability/item_fee_burn/atelier.rs` |
| AtelierAudit | `crates/gameplay/durability/src/durability/item_fee_burn_audit/atelier_cause.rs` |
| Effects | `crates/gameplay/progression/src/wheel_effects.rs` |
| GemEffects | `crates/gameplay/progression/src/wheel_effects/gems.rs` |
| Dispatch | `crates/gameplay/dispatch/src/gameplay_transport/atelier.rs` |
| Wire | `crates/protocol-oteryn/src/atelier.rs` |
| WheelRules | `crates/gameplay/progression/src/wheel.rs` |
| WheelWriter | `crates/gameplay/durability/src/durability/character_wheel.rs` |
| GemData | `rulesets/progression/wheel-of-destiny/gems.json` |
| AtelierContract | `docs/contracts/protocol-oteryn/v1/atelier-v1.md` |
| MaterialsSQL | `apps/game-server/migrations/0744_character_atelier_carried_stash.sql` |
| CoinsSQL | `apps/game-server/migrations/0745_character_atelier_carried_coins.sql` |
| OutputsSQL | `apps/game-server/migrations/0746_character_atelier_carried_outputs.sql` |
| RuleTests | `crates/gameplay/progression/src/atelier/parity_tests.rs` |
| PG | `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/` |
| Decision | `docs/architecture/reviews/OTERYN_GAME_WHEEL_GEM0_GEM_ATELIER_DECISION_2026-09-30.md` |

## Parity table

The table retains all41 rules:32 DONE,8 PARTIAL,1 MISSING. Explicit Baseline references describe the old implementation at the inspected base; the other implementation anchors identify the formatted code or unchanged base files. The complete27-case PostgreSQL run qualifies recursive reveal, carried payment, stash grades and ordinary dismantle outputs. Dismantle output remains PARTIAL because managed routing is unresolved in Q5. Named native adaptations and the separate equipment-policy row limit the scope of DONE; this is not a claim of complete literal Crystal parity. Validation distinguishes executed checks from unchanged source evidence.

| Element | Rule | Donor file:line | Oteryn file:line | Test | Status |
| --- | --- | --- | --- | --- | --- |
| Reveal family | A quality selects that vocation's unrevealed gem item. | PW:1218-1223 | Actions:206-217; Rules:151-156 | RuleTests `reveal_quality_prices_shapes_and_resource_refusals_are_atomic` | DONE |
| Reveal carried scope | Equipped inventory and recursively carried containers qualify; stash does not. | PW:1224,1233; P:5304-5324,5821-5844 | Baseline Custody:53-70 was direct-backpack only. Materials:25-91 selects live carried roots and recursive descendants; the grade-only stash fallback is at Materials:92-102. MaterialsSQL:38-67 proves current Character/World custody. See native equipment constraint below. The complete PostgreSQL qualification passed. | PG `atelier_wire_nested_gem_burns_and_replays_once`; `atelier_wire_deeply_nested_gem_burns_and_replays_once`; `atelier_wire_last_nested_gem_retires_and_removes_only_its_leaf`; `atelier_wire_reveal_does_not_consume_a_stashed_gem` | DONE |
| Recursive order | Carried slot order, then depth-first pre-order within each container. | P:5821-5844; C:1019-1087 | Baseline Custody:69 had only direct descending ordinal. Materials:23-58 maps donor slot order and sorts full ordinal paths descending. The complete PostgreSQL qualification passed. | PG `atelier_wire_reveal_follows_carried_container_display_order` | DONE |
| Reveal fees | Lesser 125,000; Regular 1,000,000; Greater 6,000,000 gold. | PW:1200-1215; Config:435-437 | Actions:206; Rules:200-206; GemData:10287-10292 | RuleTests `reveal_quality_prices_shapes_and_resource_refusals_are_atomic` | DONE |
| Payment resources | Carried money including nested containers first; bank pays residual. | G:2872-2928,2943-2969 | Baseline Fee:406-435,780-864 read only direct backpack. Fee:422-481 selects AtelierFee:29-70 carried inputs through CoinsSQL:27-66; other fee consumers retain their original owner. The bank residual remains in the same transaction. The complete PostgreSQL qualification passed. | PG `atelier_wire_nested_coins_pay_before_bank_and_make_change_once`; `atelier_wire_last_nested_coin_stack_retires_and_makes_change_once`; `atelier_wire_stash_and_bank_upgrade_without_a_backpack` | DONE |
| Coin selection | Lower unit denominations first, then lower total stack worth; ceiling division on the final stack creates exact change. | G:2930-2957 | AtelierFee:72-98 sorts by denomination and quantity, preserving native traversal for donor-unspecified equal ties. AtelierAudit:111-133 validates selected source metadata; CoinsSQL:127-178 verifies the selected prefix and unchanged unselected sources. The planner units and complete PostgreSQL qualification passed. | AtelierFee `atelier_coins_use_denomination_then_stack_worth_and_physical_capacity`; `atelier_full_coin_grade_can_burn_more_than_twenty_stacks`; PG `atelier_wire_domain_switch_reads_seventy_five_nested_coin_stacks`; `atelier_wire_large_grade_audit_burns_seventy_five_nested_coin_stacks` | DONE |
| Reveal shape | Lesser one basic; Regular two distinct basics; Greater additionally one vocation supreme. | PW:1237-1250,4124-4129 | Actions:174-204; Rules:182-199 | RuleTests `reveal_rejects_duplicate_identity_wrong_shape_and_incompatible_modifiers` | DONE |
| Reveal random domain | Uniform choice among four affinities; server chooses modifiers. | PW:1241-1250 | Actions:167-204 uses a protected root and deterministic occurrence-bound draws as the native randomness adaptation. | Existing `PG/paid.rs`, `PG/wire_paid.rs` replay cases. | DONE |
| Basic slot2 pool | Crystal permits Mitigation id30 and excludes Death Resistance id2. | PW:64-95 | GemData:44-75,138-169,232-263,326-357,420-451 instead selects IDs 0..29. Decision:99-100 records the historical pending amendment, not a new decision for this PR. The current pool is retained pending Q1. | A source-priority decision is required before changing paid draws. | PARTIAL |
| Reveal cap | Crystal has no explicit cap in this function; native contract caps225. | PW:1218-1263 | Rules:169-174; GemData:10380-10381 and AtelierContract:91-92 enforce 225. Decision:149-150 records the historical 250/pending 225 proposal. The current native bound is retained pending Q4. | RuleTests `revealed_gem_limit_and_vocation_validation_refuse_without_spending` | PARTIAL |
| Initial grant | Once per character: Lesser+Regular for each domain, eight gems; no Greater. | PW:2087-2113 | Durable:217-221,269-278,281-356 | Existing Atelier initialization PG cases. | DONE |
| Initial draws | Fixed domains, compatible random basic slots, no fee/item debit. | PW:2091-2109 | Durable:299-352; the second basic slot retains the same unresolved Q1 pool difference. | Existing initialization replay; same slot2 decision applies. | PARTIAL |
| Dismantle yields | Uniform inclusive Lesser1..5 and Regular2..10 basic, Greater1..5 supreme; no gold fee. | PW:1304-1338 | Rules:23-29,293-307; Actions:135-164 | `crystal_revealed_fragment_bounds_are_inclusive_and_refusals_are_atomic` | DONE |
| Dismantle locks | Missing/locked gems cannot be destroyed. | PW:1293-1302 | Rules:274-278,423-432 | RuleTests `native_vessel_and_lock_guards_remain_atomic` | DONE |
| Dismantle vessel/last-domain | Crystal does not guard either; accepted native contract requires both. | PW:1293-1342 | Rules:279-291; Decision:79-80,392. Both native guards are retained pending Q2. | RuleTests `native_vessel_and_lock_guards_remain_atomic` | PARTIAL |
| Dismantle output | Managed obtain containers, then normal carried add; direct map-drop fallback disabled. Ordinary destination search is breadth first and can top up a partial stack before placing the remainder. | PW:1319-1337; G:2782-2805; P:4932-5047 | Baseline Custody:122-125 was direct main backpack only. Outputs:18-66,67-178 and OutputsSQL:39-96 implement ordinary carried destinations and partial-stack splits, with current capacity/weight and atomic refusal. Complete PostgreSQL qualification passed; managed obtain preferences remain Q5. | PG `atelier_wire_dismantle_uses_nested_capacity_when_main_bag_is_full`; `atelier_wire_dismantle_splits_one_yield_between_partial_stacks`; `atelier_wire_full_carried_containers_refuse_dismantle_without_losing_gem` | PARTIAL |
| Rotation cycle | Green→Red→Purple→Blue→Green. | PW:1369-1371; Tools:2110-2125 | Rules:251-262 | RuleTests `rotation_prices_and_clockwise_domains_match_crystal` | DONE |
| Rotation fee | Lesser 125,000; Regular 250,000; Greater 500,000 gold. | Config:438-440 | Actions:110-115; Rules:263-269; GemData:10293-10297 | RuleTests `rotation_prices_and_clockwise_domains_match_crystal` | DONE |
| Rotation protection | Crystal rejects locked gem; native additionally rejects occupied vessel by accepted rulingR2. | PW:1359-1362 | Rules:243-250; Decision:407-408. The occupied-vessel refusal is retained pending Q3. | RuleTests `native_vessel_and_lock_guards_remain_atomic` | PARTIAL |
| Lock | Toggle protection in donor; explicit desired bool in native wire. | PW:1374-1381 | Rules:271-273; AtelierContract:3-12 describes native command 309. | RuleTests `native_vessel_and_lock_guards_remain_atomic`; existing wire lock case. | DONE |
| Grade ladder | 0→1→2→3; Basic2M/5M/30M, Supreme5M/12M/75M gold,5/15/30 matching fragments. | PW:1514-1597 | Rules:309-350; GemData:10237-10271 | RuleTests `shared_modifier_grade_costs_cap_and_promotion_points_follow_crystal` | DONE |
| Grade sources | Carried fragments recursively first, Supply Stash remainder; stash-only needs no backpack. | PW:1547,1557; P:5304-5356 | Baseline Custody:53-97 required a direct main backpack. Materials:72-102 selects up to 30 matching carried leaves; Materials:105-117 debits only the missing stash quantity through its existing balance/ledger. MaterialsSQL:69-100 proves the exact stash and equipment successors within the same transaction. The complete PostgreSQL qualification passed. | PG `atelier_wire_nested_fragments_upgrade_and_replay_once`; `atelier_wire_stash_fragments_upgrade_and_replay_once`; `atelier_wire_supreme_stash_fragments_upgrade_and_replay_once`; `atelier_wire_grade_uses_carried_fragments_before_stash_remainder`; `atelier_wire_stash_and_bank_upgrade_without_a_backpack`; `atelier_wire_grade_three_consumes_thirty_nested_single_fragments` | DONE |
| Grade refusal | Max grade and insufficient fragments/gold refuse. Native also validates vocation/mod identities and ruleset. | PW:1537-1559 | Rules:309-348 | RuleTests `modifier_grade_refusals_preserve_gold_fragments_revision_and_gems` | DONE |
| Shared grade | Character modifier grade applies to every gem with that modifier. | PW:1130-1135,1562-1565 | Rules:75-79,325-350; Effects:341-354 | RuleTests `shared_modifier_grade_costs_cap_and_promotion_points_follow_crystal` | DONE |
| GradeIV point | Each modifier reaching grade3 grants one promotion point. | PW:1568 | Rules:357-361; Effects:303-314 | RuleTests grade ladder; existing wheel projection tests. | DONE |
| Four vessels | Matching domain; one active gem per domain, owned gem lookup. | PW:1383-1398,1763-1772 | WheelWriter:405-430,959-960,1152-1180 validates and persists the four-domain selection; Durable:359-372 derives the Atelier vessel projection. | Existing `PG/paid.rs` vessel cases. | DONE |
| Resonance | Basic1 at1; Basic2 at2 for Regular/Greater; supreme at3 for Greater. | PW:2726-2757 | Effects:320-354 | Existing wheel socket projection tests. | DONE |
| Independent grades | Each active modifier uses its own stored grade, without preceding-mod cap. | PW:2735-2752 | Effects:341-354 explicitly follows Crystal, despite historical catalogue description. | Existing wheel effects tests. | DONE |
| Gem values | Multipliers1/1.1/1.2/1.5; effect-specific integer truncation before aggregation. | WG:40-47,235-245 and individual assignments | GemEffects:35-62,71-129 | GemEffects `spell_integer_rounding_occurs_per_gem_and_global_crit_keeps_hundredths` | DONE |
| Revelation Mastery | Direct and queued additions both execute:300/330/360/450. | WG:261-263,271-320,633-643; PW:2757 | GemEffects:117-121 double-add projection | GemEffects `mastery_preserves_both_source_additions_and_momentum_is_global` | DONE |
| Shared UI interval | Mutations share ExAction interval; config delay minus10ms. | G:11360-11396 | AtelierContract:111-129 records the 490 ms interval and the existing coordinator-approved read-only query exemption. | PG `wheel_atelier_wire_share_fresh_exhaustion_but_keep_replay_and_queries_open` | DONE |
| Replay/fence | Native immutable occurrence, request binding and current session generation; complete resource/state transition is atomic. | Native durability requirement; donor is in-memory execution reference. | Durable:125-258; Actions:227-297; the existing Character revision sequencer owns the atomic commit. The carried/stash extensions retain this owner and passed the complete PostgreSQL qualification. | Existing PG replay/stale-session cases plus new stash/nested cases. | DONE |
| Wire/client | Existing capability148 and commands308/309/424 plus Wheel161; recursive/stash selection is automatic. This row covers mutations and existing projections, not the missing aggregate resource balances below. | Donor action adapter G:11371-11395 | AtelierContract:3-12,83-107 and the existing 0148 Atelier registrations. The new custody paths passed PostgreSQL qualification; the negotiated wire shape is unchanged. | Existing codec/client handler tests; new wire PG cases | DONE |
| Eligibility | Donor panel opening requires vocation, promotion, Premium and level strictly greater than 50. Native fresh mutations recheck the current facts; read-only access to existing state is a named native adaptation. | PW:1600-1613,2116-2135 | WheelRules:439-470; Durable:183-214 reads current Premium and progression under the fenced writer. Dispatch:219-250 refuses an absent admission state; queries of existing state remain available. | WheelRules `allocation_eligibility_requires_premium_permission`; existing admission/Atelier mutation and replay tests | DONE |
| Live resource balances | Panel opening publishes bank, carried gold, three carried unrevealed-gem counts, and two carried-plus-stash fragment totals. | PW:1634-1643 | Wire:74-82 and Dispatch:282-302 contain no resource-balance fields. No aggregate balance projection is added to the strict existing V1 decoder; this surface remains absent pending Q7. | No positive balance-display test exists; Q7 requests a negotiated surface and compatibility qualification. | MISSING |
| Live price quotes | Server-owned quality prices and all three grades of each fragment kind are displayed without accepting client-supplied prices. | PW:1182-1215,1574-1597; Config:435-440 | Dispatch:251-260,435-477 emits three quality and six grade quotes from the writer's catalogue. Mutation receipts omit live quotes; the client queries the current view as the native projection adaptation. | Existing codec/client quote tests and RuleTests price/grade matrix | DONE |
| Basic slot1 and supreme pools | Each vocation has 20 first-slot basic choices and 23 compatible supreme choices. The second basic slot is the separate Q1 conflict. | PW:41-62,150-292; `src/creatures/players/wheel/player_wheel.hpp:57-110,112-212` | GemData:22-100,116-194,210-288,304-382,398-476; Actions:174-204; Durable:299-352. All five first-slot and supreme sets match Crystal; sequence byte equality is not claimed. | Read-only enumeration/set comparison for five vocations; RuleTests incompatible modifier refusal | DONE |
| Stable identity and display | Donor generates gem UUIDs, sends revealed-vector indexes and persists UUID/metadata; load reorders numeric UUIDs numerically or other UUIDs lexicographically. | PW:1237-1259,1266-1290,1461-1487,1819-1844,4136-4176 | Rules:55-79 uses stable u128 IDs in a BTreeMap; paid reveal Actions:212-217 derives identity from the occurrence; Durable:299-352 owns starter IDs; Dispatch:261-281 emits sorted stable identities. This native identity/order adaptation does not reproduce donor vector indexes. | Existing replay/relog cases and RuleTests duplicate-identity refusal | DONE |
| Grade persistence | Basic and supreme modifier grades survive reload and drive every matching revealed modifier. | PW:1929-1977 | Rules:75-79,395-421 serializes the grade map and rejects duplicate keys or grades outside 0..3; Durable:240-258 commits state and immutable receipt together; Durable:265-266,359-372 reload it. Carried/stash replay and relog passed PostgreSQL qualification. | PG `atelier_wire_grade_up_burns_fragments_and_gold_once` and new carried/stash replay/relog cases | DONE |
| Native equipment constraint | Donor enumeration admits resources in equipped slots. Native current item definitions do not allow coin/gem/fragment equipment, while durable selection still proves any admitted equipped source and advances one shared equipment epoch. | P:5821-5844; G:2872-2895 | `tools/content-schema/native-gameplay/coin-item-profiles.json:83,226,369`; `tools/content-schema/native-gameplay/wheel-gem-item-profiles.json:1078,1240,1402,2050,2212` have equipment NOT_APPLICABLE; `crates/gameplay/content/src/content/native_gameplay.rs:3432-3434` refuses non-Known claims. Materials:85-89, Custody:162-165 and AtelierFee:35-43,152-158 share the original equipment snapshot; Dispatch:183-217 refreshes current projections. This is an observed content constraint, not a new owner-approved donor exception. | PG native-policy assertions refuse every equipment slot claim; no forged positive equipped-resource fixture is counted. | PARTIAL |
| Coin change destination | Donor addMoney tries managed collection, normal carried insertion and ground fallback. Native Atelier preserves exact main-backpack change output and refuses atomically when it cannot fit. | G:2930-2969,2972-3025 | AtelierFee:90-98 counts only main-bag slots freed by burns; Fee:902-921 retains the existing change-mint shape, and CoinsSQL:186-197 proves its fresh main-backpack outputs. No debit survives ChangeDoesNotFit. The native delivery boundary remains pending Q6. | AtelierFee `atelier_nested_partial_coin_requires_a_real_change_slot`; PG nested partial/whole coin cases | PARTIAL |

## Secondary-source differences

The following Canary differences are observed at the exact secondary SHA above. Crystal remains the task's primary reference.

| Rule | Crystal | Canary | Native selection |
| --- | --- | --- | --- |
| Dismantle distribution | Uniform draw, PW:1309-1315 | `normal_random`, component PW:1228-1234 | Uniform occurrence-bound draw. |
| Destroy active gem | Deletes revealed vector entry, PW:1341-1342; active data is copied elsewhere. | Clears active gem and persisted reference, component PW:1260-1275 | Existing explicit in-vessel refusal; owner decision required before change. |
| Reveal notification | Refresh Wheel, PW:1263 | Additional revealed-gem message, component PW:1178-1180 | Native mutation result plus live query. |
| Basic mitigation |2000 hundredths × grade, WG:162-164 |500 hundredths × grade, component WG:156-158 | Native catalogue20/22/24/30 percent. |
| Monk Spirit Mend |6 × grade, WG:575-577 | Constant6, component WG:586-588 | Crystal scaling. |
| Flurry of Blows | Enum damage case writes critical; enum critical case writes damage, WG:595-601 | Names and fields align, component WG:627-640 | Explicit Crystal field correction in GemEffects. |
| Sweeping Takedown |8 × grade, WG:611-613 |5 × grade, component WG:660-662 | Crystal8 override. |
| Focus Serenity |150 seconds, WG:619-622 |300 seconds, component WG:676-680 | Native catalogue150. |

## Implemented extension and durable proof

The production extension uses the existing Atelier receipt and Character transaction. `Materials` selects recursive carried resources and, for grades only, the exact remaining Supply Stash quantity. `AtelierFee` extends only the three Atelier fee causes; every other fee owner retains its earlier direct-backpack selection and limits. `Outputs` uses ordinary carried container capacity, compatible partial stacks and the current weight owner. Each refusal rolls back every material, coin, stash, state and revision change.

The additive migrations are `0744_character_atelier_carried_stash.sql`, `0745_character_atelier_carried_coins.sql`, `0746_character_atelier_carried_outputs.sql` and `0747_character_atelier_audit_bounds.sql`. They retain immutable same-transaction provenance, current Character/World ancestry, full source before-state, exact quantity successors, current occurrence/receipt binding, finite audit retention and PUBLIC execution revocation. Their conditional arms extend the existing custody guards rather than authorizing unrelated sources. Shared equipped material/coin work reuses the original equipment snapshot and advances its epoch once.

The native carried reader's existing bound is 500 tree rows, including the main bag, plus at most nine separately equipped roots (`crates/gameplay/durability/src/durability/item_use/carried_inventory.rs:169-256`). The new traversal counts descendants of equipped containers in the tree budget, detects duplicate/over-depth rows and uses a 510-row overflow sentinel. It does not silently truncate matching items to the earlier 20 fee stacks. The derived coin leaf bound is 508, material burns need at most 30 leaves, and a dismantle yield needs at most 10 outputs.

`AtelierAudit` and its `materials.rs` child validate source location, canonical item/parent identities, definition/revision, before/count/after quantities, source uniqueness and the optional stash ledger transition. Strict material fields apply when the new `coin_inputs` marker is present; historical proof shapes remain accepted. The registered Atelier maxima are 540 asset lines, 1,620 effect work units, two value lines and 262,144 envelope bytes, retaining the existing global 196,608 payload ceiling. Actual encoded size is checked before publication; maximum count does not authorize unbounded text fields. Migration 0747 allows that larger outbox envelope only for fresh, matching Atelier evidence; other shapes retain their existing deferred byte checks.

The audited delivery path in the inspected repository ends at the registered immutable SQL outbox and shape decoder. This report does not claim qualification of an external broker or projector. The global envelope/payload resource rows already allow 262,144/196,608 bytes. The existing proto comment and authored `GAME_EVENT_FOUNDATION_REGISTRY.json` note now describe the Atelier exception without changing tags, schema/retention tuples or other fees' 20-input and 25,398/25,712-byte limits.

## Left out — binding PR questions

All seven questions below are posted on the same [draft PR 2795](https://github.com/Oteryn/Oteryn-Game/pull/2795). At the latest read for this evidence update there were seven `QUESTION:` comments and no `ANSWER:` comments. The root publisher rereads that channel before its next step and final report; a later answer supersedes this snapshot.

| Question | Unsettled choice | Current implementation |
| --- | --- | --- |
| [Q1](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6559500372) | Crystal slot2 includes Mitigation id30 and excludes Death Resistance id2; the accepted native planner selects the opposite. | Existing native pool retained for paid and starter draws; no silent catalogue change. |
| [Q2](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6559500641) | Whether to remove the accepted occupied-vessel and last-domain dismantle refusals absent in Crystal. | Both native guards retained. |
| [Q3](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6559500902) | Whether to permit rotating a gem currently in a vessel, including the treatment of copied active effects. | Existing occupied-vessel refusal retained. |
| [Q4](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6559501111) | Whether to expand the published native limit of 225 revealed gems when Crystal has no explicit reveal cap. | Existing 225-gem wire/resource bound retained. |
| [Q5](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6559501317) | Whether this scope also integrates managed obtain-container preferences for dismantle. | Ordinary carried selection and partial-stack splits implemented; managed preferences remain outside this change. |
| [Q6](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6559553217) | Whether coin change gains managed/generic carried/ground delivery. | Existing main-backpack change and atomic no-spill refusal retained. |
| [Q7](https://github.com/Oteryn/Oteryn-Game/pull/2795#issuecomment-6559861003) | Whether to add a separately negotiated read-only projection for seven live panel resource balances. | Aggregate balance display is MISSING. Existing strict V1 replies remain unchanged pending protocol scope/compatibility approval. |

The observed native equipment policy is a separate remaining literal-parity boundary: the current coin, gem and fragment definitions mark equipment as NOT_APPLICABLE. The carried owner validates admitted equipped sources, but the current content/normal equipment path cannot equip these resources. This PR preserves that content policy and tests the refusal; it does not claim a positive equipped-resource wire transition or an owner-approved Crystal exception.

## Added rule test matrix

`RuleTests` covers reveal prices and shape, promoted vocation and cap, invalid identity/modifier input, every grade cost, gradeIV points, resource/max/ruleset refusals, the full domain cycle and prices, locking, vessel guards, missing gems and stale revision. Every refusal asserts the entire state remains unchanged.

The executable recursive/stash regression is the real-dispatch PostgreSQL suite owned by this PR's durability/test lanes. Pure rules do not model custody source selection.

The new mounted cases cover deep and last-leaf reveal, donor material order, Basic/Supreme stash-only grades, carried-before-stash remainder, final stash exhaustion, a grade paid by stash and bank with no backpack, 30 separate fragment sources, insufficient combined resources, insufficient gold, stashed-gem reveal refusal, nested partial/whole coin burns with exact change, more than 20 coin burns, ordinary nested dismantle capacity, partial-stack output splits and complete-capacity refusal. Replay and relog compare the durable resource/state footprint. The same mounted suite checks migration reapplication, 26 immutable guard definitions and retained history. PUBLIC revocation, fixed search paths and runtime grants are also reviewed directly in the migration source; ACL equality is not an executed reapplication assertion. Authorship of those cases is separate from their final runtime result.

## Validation

Qualification ran in an isolated checkout on the owner-authorized computer, using PostgreSQL18.6. The authoring scratch environment has no Rust toolchain. Package tests below ran without `OTERYN_TEST_POSTGRES_ADMIN_URL`; the mounted Atelier command ran separately with that variable configured for the isolated database. Its27 completion markers and zero filtered-case failures confirm actual database execution.

### Completed implementation checks

| Exact command | Result | Scope |
| --- | --- | --- |
| `cargo test --locked -p oteryn-gameplay-progression --quiet` | PASS,132 tests | Complete changed progression package. |
| `cargo test --locked -p oteryn-gameplay-durability --quiet` | PASS,604 tests plus4 doctests | Includes the preserved legacy byte-limit assertion and the added decoder-classification regression. |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --quiet` | PASS,1907 tests;16 existing ignored tests | Complete changed dispatch package; no new ignored test was added. PG-dependent cases use their normal absent-environment path in this command. |
| `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib postgres_tests::atelier:: --quiet -- --nocapture --test-threads=2` | PASS,27 tests,0 failed,0 ignored;452.59s test runtime | Real PostgreSQL18.6, full mounted Atelier suite, including immutable replay, insufficient-resource rollback, larger audits and ordered migration reapplication. |
| `cargo clippy --locked -p oteryn-gameplay-progression -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --all-targets --all-features --quiet -- -D warnings` | PASS | All three changed Rust packages and test-support paths. The final publication qualification reruns after the last test-oracle correction. |
| `cargo fmt --all -- --check` | PASS | Complete workspace formatter check. |
| `python3 tools/contracts/registry_fragments.py` | PASS | Four registered Atelier resource-limit leaves. |
| `python3 tools/agents/validate_governance.py` | PASS |22 required policy documents and9 project lanes; rerun with final task/evidence text before publication. |
| `python3 -m unittest discover -s tools/agents/tests` | PASS,129 tests | Governance tooling tests. |
| `git diff --check` | PASS | Final candidate additionally checks the complete staged/unstaged diff against HEAD. |
| `git fetch origin main` followed by `git merge origin/main`, `cargo fmt --all`, `bash tools/merge-driver/regen.sh` | PASS | Main remained38641fe; merge reported already up to date. Regeneration completed in742.97s and retained the same world pin. The complete changed-path audit found only the30 owned changed files. |

The last reviewed wire-test correction restores an independent literal2..10 dismantle yield and verifies every output key. It was synchronized before the dispatch package and real PostgreSQL suite compiled/ran. The final PR qualification records the exact published-head readback and repeats the required gates on that unchanged head; worktree checkpoints are not mislabeled as earlier runs on a commit that did not yet exist.

### Baseline and repair evidence

Both primary regressions were first run against base38641fe plus the initial task record, with the new tests only. They reached the old implementation and failed on its ordinary refusal:

```text
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib atelier_wire_nested_gem_burns_and_replays_once --quiet -- --nocapture
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib atelier_wire_stash_fragments_upgrade_and_replay_once --quiet -- --nocapture
```

Those baseline results were RED,1 failure each (19.27s and24.29s). Their implemented-path checkpoints passed individually and they now pass within the complete27-case suite. The earlier wider run found four fixture/permission errors (22/26 passed), which were corrected without weakening the SQLSTATE or production assertions. The first full durability run found one legacy oversized-envelope classification regression (602/603 passed); the production fix retained the existing failing assertion and added a focused classification test, and the full rerun passed604 tests plus4 doctests. Migration0745's initial inline-CASE parser error was corrected and all four additive leaves were also applied twice in a real PostgreSQL rollback transaction before the final mounted reapplication check.

No new capability or client registration was added, so the new-capability architecture/client/E2E matrix is not applicable. Existing client wire shapes remain unchanged. External broker/projector delivery and coordinator-dispatched frozen-head review are not part of the executed local checks. Open questions Q1-Q7 and the resource-balance projection remain explicitly unresolved.

## Numbers

No new capability, command, mechanic ordinal or client option is required by recursive selection or automatic stash payment. The authored migration numbers are 0744-0747; resource-limit entries are 1684-1687 (`ATELIER-RL-ASSET-LINES`, `ATELIER-RL-EFFECT-WORK`, `ATELIER-RL-VALUE-LINES`, `ATELIER-RL-AUDIT-BYTES`). Capability 148 and commands 308/309/424 with Wheel161 are reused. Reserved capability272-273, command484-487 and mechanic/offer624-627 remain unused. Q7 would require a separately accepted negotiated surface.

The number audit used current main `38641fecd4558a4a14e6b15b3874110ffa2dff19`. `python3 .claude/skills/oteryn-dev/scripts/next-free-numbers.py --no-prs` passed; the default command reached its PR-query stage but could not run because `gh` is absent. The publisher therefore checked the complete changed-file lists of all15 other open PRs through the GitHub API:2798,2797,2796,2793,2792,2785,2784,2783,2782,2781,2773,2770,2766,2763,1942. The later changed heads of2793 and2796 were rechecked. Neither those PRs nor an explicit current-main tree scan occupied migrations0744-0747 or resource-limit1684-1687. This is the API fallback result, not a claim that the default script completed.

## Integration risks and review

PRs2783,2784 and2785 also modify the shared fee writer and audit-cause files. The combined integration head must retain every closed cause arm and each unrelated fee's existing limits. Migrations0744 and0746 replace specific existing Atelier/stash guard bodies; drift in their exact anchors deliberately aborts migration. Later guard replacements must preserve0745's coin delegation and nested/equipment proofs,0746's nested-output arm and0747's audit-size arm. The mounted reapplication case rebuilds0314/0624 followed by0744-0747, compares26 function definitions and retains committed state/receipt history, including a split-output history. That case must be rerun on a later combined integration head.

Source review verified11 new SQL functions with PUBLIC access revoked; nine runtime query/proof helpers receive EXECUTE and the two trigger wrappers receive no new EXECUTE grant. The new coin-proof table grants runtime SELECT/INSERT only. Every new SECURITY DEFINER helper has a fixed `pg_catalog,<schema>,pg_temp` search path. The main/application schemas and immutable history are forward additions; future fixes must preserve committed receipts and ledger rows.

The production, test and documentation paths were reviewed for existing-owner use, bounded reads, lock ordering, source/World/session identity, refusal rollback, receipt replay and unchanged native wire shapes. The full package run caught the oversized legacy-audit error-category regression; the production correction preserves the original assertion and adds a focused regression case. A peer test review restored the independent literal2..10 dismantle-yield oracle alongside live-quote checking and required every output key to be the basic-fragment key. No test is deleted or newly ignored. Coordinator-dispatched frozen-head review is not claimed or requested by this draft.
