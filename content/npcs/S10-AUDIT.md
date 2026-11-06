# SRV9 / S10 nonquest NPC audit

Base/fetched main: `b2cfa798e3def61002d39a6da8b9de5df5aa2fd8`.
Donors: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`, Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`. All 2,225 NPC files fetched successfully,
read-only. Paths below are relative to the donor repository at its exact pin.
TibiaWiki API requests returned HTTP 402: **wiki-pending**. No bypass attempted.

## Gaps reproduced and implemented

- **Ambient source population:** nine nonempty source voice sets were absent:
  Valindara, Tooth Fairy, Tired Tree, Rosemarie, Gunther, Grumpy Stone,
  Enpa Deia Pema, Allen, Maria. Exact file/line/SHA-256 and original say/yell,
  interval and chance are in `s10-nonquest.json`. Maria uses Crystal's
  `data-crystal/npc/maria.lua`; the other eight use `data-global/npc/`.
  Empty voice tables are silence, not missing lines. Explicitly unconfirmed
  Dragon Ancestor Spirit placeholders remain excluded.
- **Wandering caller:** the previous prepare-only NPC walk rules had no production
  consumer. The existing Channel NPC cadence now shares its 256-think budget
  between speech and walking. Canary `src/creatures/npcs/npc.cpp:1080-1103,1190-1228`
  supplies cadence, radius and shuffled cardinal candidates. The accepted native
  NPC-BEHAVIOUR0 contract additionally refuses damaging fields, triggers,
  floor changes, height, PZ transitions and occupied cells, and pauses for customers.
  Current Item facts are read under the existing scope transaction before the
  exact synchronous runtime position commit. No new messages or registry entries.
- **Wander source facts:** Wyrdin's Apprentice/Taya radius 2 and Fernlor radius 1
  lacked source fields/profiles. Captain Coohan's legacy 1500 ms/radius 2 becomes
  current Crystal `data-global/npc/captain_coohan.lua:10-11,24-25`: 2000 ms/radius 1,
  floorchange false. Engine default speed 55: Canary `npcs.hpp:43`.
  Omrabas/Flickering Soul retain the previously accepted stationary decisions
  (`tools/content-schema/npc-authoring/validate_promotion.py:108-120`).
- **Positive terrain height:** production bundle tiles have no legacy source-step
  metadata. `s10-walk-heights.json` projects actual protobuf flag-27 presence,
  following Canary `src/items/items.cpp:235`, instead of inventing zero height
  from missing XML attributes. Both donors agree on all shared IDs. The union
  contains 43,392 known identities; unknown identities refuse. Reproduce with
  `python3 content/npcs/s10_walk_facts.py <pinned-assets-directory>`; input hashes
  are checked before the complete semantic comparison. XML field damage is also
  projected; current dropped items additionally use the real Item owner.
- **Rashid:** all seven coordinates were already correct; trade incorrectly required
  the exact anchor despite donor radius 3 (`npc/rashid.lua:10-11` in both pins).
  Live weekday changes now relocate the existing actor and close conversations;
  occupied arrival cells retry. Accepted UTC interpretation remains unchanged
  (`docs/architecture/reviews/NX-4-NPC-SUMMER-2026.md:60`), including its difference
  from Crystal `data-global/scripts/globalevents/spawn/rashid.lua:3` host 00:01.
- **Bank population:** 24 registered `parseBank` NPCs plus Wentworth
  (`wentworth.lua:172,213,394`) lacked service references. All 25 now reuse BANK0 and the existing coin-exchange
  caller; individual source evidence is in `s10-nonquest.json`. Paulie and the
  separate hireling bank service were already populated. Plunderpurse is excluded:
  `plunderpurse.lua:360` explicitly places him in Dawnport. Wentworth's legacy
  level-8 / level-above-9 deposit caps (`wentworth.lua:157-168`) are an accepted
  difference: native BANK0 owns account rules, and the admission evidence explicitly
  excludes legacy junior/account rules (`docs/agents/evidence/OTV2-20261015-npc-bank-services/native-bank-admission.json:22`).
- **Travel population:** six ungated routes in `npc_catalogue/travel_routes.json`: Scrutinon x4, Rapanaio return and
  A Ghostly Sage. They reuse existing paid/free travel, current scope, conversation,
  price, premium and arrival checks. Scrutinon preserves the literal `ab'dendriel`
  (`scrutinon.lua:86`). The return uses current Crystal
  `rapanaio_isle_of_evil.lua:59-65` (32700,31990,15), rather than the older Canary
  destination (32699,31993,14); wiki-pending. Other source anchors:
  `scrutinon.lua:79-89` (all four free, nonpremium destinations) and
  `a_ghostly_sage.lua:53-55` (free help/yes escape to 32834,32275,9). Existing quest-gated routes stay held.
- **Promotion population:** Ishebad `ishebad.lua:50-53`, Enpa Rudra
  `enpa_rudra.lua:32-34`, Emperor Rehal `emperor_rehal.lua:128-130` lacked service
  references. Donor `parameters.monk=true` means monks are allowed, not required
  (`modules.lua:102`). Existing accepted promotion pricing/eligibility is retained.
- **Blessing population:** Norf, Humphrey, Edala, Pydar and 16 Twist-of-Fate priests
  lacked admitted service bindings. `s10-nonquest.json` records each source line.
  `StdModule.bless` (`modules.lua:163-188`) supplies duplicate, world-type,
  prerequisite and payment checks. Existing owner pricing ruling 2026-10-18 wins
  over donor prices. Nonpremium offers no longer require current Premium.
  Twist requires another blessing or a carried amulet of loss (3057), including nested bags.
  Kawill's free preparation is separate from Pydar's paid Spark and is consumed
  atomically with the paid receipt. A distinct immutable free-stage receipt keeps
  the existing strictly-positive fee invariant and participates in CharacterRevision
  recovery/replay. Only migrations **1082–1083** are used.
- **Free healing population:** 27 standard priests cure fire, then poison, then
  energy, otherwise heal health below 40 to 40 (Crystal `alia.lua:91-110`, Canary
  `alia.lua:94-105`). Cipfried, Dallheim/Zerbrus, Hector the Mentor and Enpa Deia
  Pema have separate source profiles. Full pinned source hashes/lines are in
  `s10-healing.json`. The caller uses the actual PlayerSpellState conditions and
  HP owner in one successor; no spell cast, side condition store or durable fee.

## Already present on the base

- Source voice preparation/delivery, say/yell and retry suppression:
  `npc_voice.rs`, exercised by the existing NX4 source-voice PostgreSQL case.
- Seven Rashid weekday anchors: `rashid_schedule.rs::destination_at`,
  `canary_week_rotates_all_seven_destinations`.
- Existing plain shop purchases/sales and framed shop LOOK/SELL_ALL:
  `postgres_tests/npc_shop.rs::npc_shop_postgres_real_wire_look_sell_all_replay_and_nothing_left`.
- Banker gold/platinum/crystal exchange:
  `npc_command/coin_exchange.rs`; population now reaches that same caller.

## Base holds (historical; SRV9B disposition below)

The supplied base held these two rows. The SRV9B section below supersedes their current disposition:

1. **Fluid/subtype trades.** `npc_catalogue/service.rs:120-122` explicitly holds
   `ChargedOrFluidPending` for NPC-TRADE-3. `trade_variant_repair.py:1-6` states
   donor fluid enums have no accepted native subtype binding; `count` is native
   transaction quantity. Current source holds: 75 enum/quantity, 32 water variant,
   nine explicit-subtype tuples. Activating them requires a new native mapping
   and instance-payload/mint/trade preservation rule, rather than repurposing count.
   Fifty-seven same-tuple variant holds likewise remain source evidence, not a
   fabricated choice between distinct variants (authoring schema D15, line 71).
2. **Marriage.** Both donors register Lynda's marriage/celebration callbacks
   (`lynda.lua:296-304`) and use legacy marriage/spouse globals (`:163-164`).
   Neither pinned donor schema defines the corresponding persistence. Current
   Tibia semantics and the native cross-Character marriage authority contract are
   unestablished; wiki checks are pending. Do not misclassify this as proven OT-only,
   or invent a cross-Character value/appearance transition under an unrelated receipt.

## OUT-OF-SCOPE-QUEST / other lanes

Quest missions/dialogue/callbacks are untouched. Quest/faction shop access holds
(284 tuples across Alesar, Haroun, Nah'Bob, Rashid and Yaman), Cael/Gnomux/Grizzly
quest offers, Peter's factory route, Rapanaio's outbound evil-isle passage
(`rapanaio_boat.lua:70-85`) and Dalbrect's passage (`dalbrect.lua:112-134`), Hairycles healing access and Chester Kahs's
quest-reward healing belong to questpc. Existing admitted quest access is not
removed. Spell teaching/casting belongs to S17. Oressa/Dawnport are excluded.
No sound files, protocol goldens, capability/command/dispatch registries or
`connection.rs` are changed. No new capability bits, command IDs or wire messages.

## Production evidence index

Paths in this table are relative to the repository root. Data population and runtime
consumption are distinct gaps; an existing engine does not prove its NPCs were populated.

| Sub-feature | Base gap / implemented caller | Regression evidence |
| --- | --- | --- |
| Ambient population | `content/npcs/s10-nonquest.json:6`; materializer `crates/gameplay/content/src/content/npc_catalogue/s10_nonquest.rs:115`; existing cadence `crates/gameplay/dispatch/src/gameplay_transport/npc_voice.rs:61` | `s10_repository_ambient_profiles_reach_the_production_catalogue`; `s10_valindara_postgres_source_voice_reaches_chat` |
| Wander population and real movement | `content/npcs/s10-nonquest.json:557`; `crates/gameplay/dispatch/src/gameplay_transport/npc_actor/current_map.rs:87` | `s10_crystal_only_wander_profiles_reach_the_production_book`; `s10_wandering_postgres_moves_within_source_radius_and_pauses_for_customers` |
| Rashid trade after wandering | `crates/gameplay/entities/src/rashid_schedule.rs:60` | `weekday_trade_remains_available_after_wandering`; `s10_rashid_wandering_postgres_shop_payment_and_replay` |
| Rashid live weekday change | `crates/gameplay/entities/src/carrier/npc_actor.rs:151`; cadence above | `rashid_weekday_rotation_preserves_actor_and_does_not_recentre_wandering`; `s10_weekday_rollover_moves_live_rashid_on_postgres` |
| Bank population | `content/npcs/s10-nonquest.json:305`; `crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs:612` | `s10_source_registered_bankers_reach_the_production_talk_book`; `s10_eva_postgres_bank_deposit_and_withdraw` |
| Ungated travel population | `crates/gameplay/content/src/content/npc_catalogue/travel_routes.json:3`; existing paid/free travel caller | `s10_ungated_source_travel_reaches_the_production_talk_book`; `s10_travel_postgres_source_keywords_and_free_arrival` |
| Promotion / blessings population | `content/npcs/s10-nonquest.json:649,852`; `crates/gameplay/durability/src/durability/blessing_purchase.rs:8`; `crates/gameplay/durability/src/durability/npc_purchase/phoenix.rs:4` | `s10_nonpremium_blessing_givers_reach_production_talk_book`; `s10_blessings_postgres_nonpremium_and_phoenix_stages`; `s10_blessings_enforced_pvp_postgres_refuses_purchase`; `s10_twist_postgres_accepts_amulet_carried_in_backpack` |
| Free healing population / actual conditions | `content/npcs/s10-healing.json`; `crates/gameplay/dispatch/src/gameplay_transport/npc_command/healing.rs:6` | `s10_healers_reach_production_talk_book`; `s10_healing_postgres_restores_only_emergency_health` |

ALREADY-DONE evidence on base `b2cfa798`:

- Ambient delivery engine: `crates/gameplay/dispatch/src/gameplay_transport/npc_voice.rs:54`
  on the base, `emit_npc_voices`; existing NX4 source-voice scenario also supplies
  the production fixture used by the new Valindara case:
  `nx4_source_npc_voice_reaches_chat_once_per_channel_think_on_postgres`.
- Seven weekday destinations: `crates/gameplay/entities/src/rashid_schedule.rs:29`,
  `canary_week_rotates_all_seven_destinations`.
- Plain shop dispatch and receipts: `crates/gameplay/dispatch/src/gameplay_transport/npc_trade_command.rs:118`,
  `npc_shop_postgres_real_wire_look_sell_all_replay_and_nothing_left`; the S10
  Rashid case executes the same framed scenario after moving the merchant.
- Coin exchange: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/coin_exchange.rs:212`,
  existing BANK0 authority,
  `coin_exchange_postgres_dispatch_rates_refusals_replay_and_custody`; this task
  supplies the missing source NPC bindings.

Generated bundle, content, topology and model-input digest changes are output of
repository regeneration tools. Prey files change only the input digest; there is
no new Prey behavior. Creature spell profile/manifest changes are also checksum
fields only, with no S17 behavior change. The bundle-gate test inventory adds the
NPC walking call-site and the two combat-only S10 fixtures; it does not add or
alter a dispatch/routing registry.

## Regression sequence

Actual semantic RED runs (compile/fixture failures are not counted as RED):

- `s10_` NPC population: four failures in 48.01 s, including absent voice and
  source-wander profiles. The isolated ambient test also failed in 40.17 s.
- `weekday_trade_remains_available_after_wandering`: rejected a legal radius-three
  position before the trade predicate fix.
- `source_height_is_positive_knowledge_and_unknown_is_not_zero`: the XML-only
  projection incorrectly classified real appearance height before protobuf repair.
- `s10_healing_postgres_restores_only_emergency_health`: with the production
  healing caller disabled, failed the canonical fire-condition assertion in 84.21 s.
- `s10_twist_postgres_accepts_amulet_carried_in_backpack`: the worn-only prerequisite
  rejected a source-legal carried amulet, failing paid receipt count in 83.13 s.
- `s10_ungated_source_travel_reaches_the_production_talk_book`: the exact source
  `ab'dendriel` keyword failed in 36.96 s before correction.
- `s10_travel_postgres_source_keywords_and_free_arrival`: caught unconditional
  quest-owned Rapanaio travel in 83.36 s before removing the gated route.

Number allocation: migrations **1082, 1083**; no capability bit, command ID,
protocol message, or golden allocation. Independent exact-head review is the
coordinator's responsibility under `oteryn-review`; this worker does not claim
that the candidate is merge-ready or that the whole S10 row is DONE.

Dawnport exclusion RED: `s10_source_registered_bankers_reach_the_production_talk_book`
failed the explicit no-Plunderpurse-binding assertion in **36.69 s** before
removing the out-of-scope data. The positive set remains the 25 in-scope bankers.

## Exact validation commands

All Cargo/PG commands source `/workspace/oteryn-env/env.sh`; builds use
`CARGO_BUILD_JOBS=1` after observed memory pressure. No tools were installed.

```sh
cargo fmt --all --check
rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/s10_*.rs
cargo clippy --locked -p oteryn-gameplay-content -p oteryn-gameplay-entities -p oteryn-gameplay-npc -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch --all-targets --all-features -- -D warnings
cargo test --locked -p oteryn-gameplay-dispatch --all-features s10_ -- --nocapture
```

The unit runs unset `OTERYN_TEST_POSTGRES_ADMIN_URL`; only the separately filtered
S10 command above executes PostgreSQL cases. Dispatcher unit filters are exactly
`npc_actor`, `npc_voice`, `bundle_gates_tests`, each with `--lib -- --skip postgres`.
NPC and entities run `--all-features --lib`; durability adds `-- --skip postgres`;
content runs `--all-features --lib npc_catalogue`.

```sh
cargo run --locked -p oteryn-architecture-check -- workspace .
cargo test --locked -p oteryn-game-server --test content_npc_catalogue --test content_world_project_repository
python3 tools/content-migration/validate_world_project_v2_to_tree.py
python3 tools/content-migration/test_world_project_v2_to_tree.py
python3 content/npcs/s10_walk_facts.py /tmp/s10-donors
oteryn-world-bundle-compiler pin-check . /tmp/s10-final-pin-check
```

Regeneration uses `python3 tools/content-migration/regenerate_content.py --resolve
--no-stage --skip-checks`; it invokes the repository materializer, content
extractors and pin-refresh. Nothing is hand-repinned. The full dispatcher unit
run attempted earlier was interrupted after finding the missing gate inventories;
those exact guards are rerun above. No full nextest or full PostgreSQL suite runs.

The existing production listener starts the owner cadence at
`crates/gameplay/dispatch/src/gameplay_transport/serving.rs:153`; that cadence
calls speech and wandering at `source_item_cycle.rs:107`, even with no pending
speech deadline. It lives for the Channel listener, across player reconnects.

Additional source parity: Crystal `data-global/npc/kawill.lua:49-54` checks only
whether the free preparation is active. After Pydar consumes it
(`modules.lua:187-188`), Kawill may prepare a fresh future visit even while Spark
is held. Both the runtime and deferred SQL guard follow that condition; replay
of the original consumed grant still cannot reactivate it. An actual PG RED
reproduced the former extra restriction (false/one receipt instead of true/two).
The same **83.40 s** two-case RED run disabled the actual wandering commit and
failed with "production NPC cadence never committed a step". The caller was
restored byte-for-byte before validation; no diagnostic toggle is delivered.

The three new promotion NPCs also have a dedicated PG case,
`s10_promotions_postgres_registered_sources_charge_and_replay`, using the existing
promotion fixture once per source NPC. Eremo's original full scenario remains
unchanged; new scenarios verify the NPC/service binding, permanent effect,
20,000-gold fee and replay, with per-NPC completion markers and durations.

## S10 original delivery validation (before SRV9B)

**S10 remains PARTIAL / STOPPED for the two native-contract holds above.** The
nonquest voice, wandering, Rashid schedule/trade and admitted service-population
slice is implemented and validated. Quest-only gaps remain OUT-OF-SCOPE-QUEST.
No new capability bits, command IDs or protocol messages; migrations 1082–1083 only.

Final focused PostgreSQL run: **11 tests passed, zero failed, 331.41 s**, with
13 fixture scenarios (the promotion test executes all three new source NPCs).
Every test emitted its completion marker; each promotion NPC also emitted its
own measured subcase marker:

```text
POSTGRES_CASE_COMPLETED S10 blessings_enforced_pvp duration=84.344896898s
POSTGRES_CASE_COMPLETED S10 twist_carried_amulet duration=84.715098692s
POSTGRES_CASE_COMPLETED S10 blessings duration=85.299375321s
POSTGRES_CASE_COMPLETED S10 eva_bank duration=85.750913666s
POSTGRES_CASE_COMPLETED S10 weekday_rollover duration=7.695084193s
POSTGRES_CASE_COMPLETED S10 healing duration=84.698483895s
POSTGRES_CASE_COMPLETED S10 rashid_wandering_shop duration=83.810397917s
POSTGRES_SUBCASE_COMPLETED S10 promotion npc=ishebad duration=84.430629878s
POSTGRES_CASE_COMPLETED S10 source_travel duration=85.919997958s
POSTGRES_CASE_COMPLETED S10 valindara_voice duration=41.135772752s
POSTGRES_SUBCASE_COMPLETED S10 promotion npc=enpa_rudra duration=81.822981016s
POSTGRES_CASE_COMPLETED S10 wandering duration=81.988432083s
POSTGRES_SUBCASE_COMPLETED S10 promotion npc=emperor_rehal duration=80.412510903s
POSTGRES_CASE_COMPLETED S10 promotions_source duration=246.690655942s
```

Final crate checks: NPC **30/30** (145.77 s), entities **252/252** (5.65 s),
durability **595/595** with PostgreSQL disabled and 93 filtered (66.49 s),
content `npc_catalogue` **7/7** (36.13 s), dispatcher `npc_actor` **11/11**,
`npc_voice` **8/8**, `bundle_gates_tests` **6/6** (10.65 s for the gate inventory).
Both game-server integration targets passed: **4/4** (36.42 s) and **5/5**
(52.59 s). Total: 909 crate unit checks and nine integration checks.

Fmt passed. All-target/all-feature Clippy of the five changed gameplay crates
passed with `-D warnings` (3m53s); the changed server test targets also passed
Clippy (1m49s):

```sh
cargo clippy --locked -p oteryn-game-server --all-features --test content_npc_catalogue --test content_world_project_repository -- -D warnings
```

Architecture `workspace-boundaries: PASS`; final Python tree validation and
regression checks passed, including the positive bank admission and all 12
closed negatives. The donor fact reproducer verified 43,392 identities and
36 damaging fields. Bundle pin-check passed for 31,101,773 bytes with digest
`feadb394d036441fdbdc52761b3fbd712a95f4dd2a4b538222ad4e1486c1be3d`.

The integration inventory now expects 2,517 NPC-related declarations and both
exact Bank rows. Paulie's sealed packet remains exact; the standard Bank row
admits exactly the 25 source consumers and excludes Plunderpurse. All production,
content and S10 PG test bytes stayed identical after the eleven-case PG pass;
only these separately tested integration inventory assertions changed afterward.
The final source snapshot confirmed that boundary. No ignored or skipped test
was added, and no protocol golden was changed.


## SRV9B native contracts (2026-11-06)

User allocation: branch `claude/happy-davinci-ijz3wy-srv9`, base
`e70054802f4097d45e2860c73aaa7bc5e2b0335b`. No quest missions, callbacks or dialogue
are added. No new capability, command, wire message or golden. Only migrations
**1084–1085** are used; 1086 remains unused.

### Source qualification

The original exact pins remain authoritative. Source paths here are accurate
for each donor (Canary uses `data-otservbr-global`, Crystal `data-global`).

- Fluid enums are identical: Canary `src/utils/utils_definitions.hpp:412-433`,
  Crystal `src/utils/utils_definitions.hpp:439-460`: none0,water1,wine2,beer3,mud4,
  blood5,slime6,oil7,urine8,milk9,mana10,life11,lemonade12,rum13,fruitjuice14,
  coconutmilk15,mead16,tea17,ink18,candy19,chocolate20. This is the existing I10a
  Item namespace, not a new NPC enum or a reinterpretation of native quantity.
- Source shop parsing: Canary `data/scripts/lib/register_npc_type.lua:156-174`,
  Crystal `data/scripts/lib/register_npc_type.lua:147-165`. The selected amount
  and subtype remain separate through the sales callback: Canary
  `data-otservbr-global/npc/maria.lua:64-73`, Crystal
  `data-global/npc/maria.lua:62-71`. Mug2880: beer subtype3/2gp,
  lemonade12/2gp, water1/1gp, wine2/3gp.
- Instance state: Canary `src/items/item.cpp:881-884,913-916`, Crystal
  `src/items/item.cpp:504-507,536-539`. Native owner already exists at
  `apps/game-server/migrations/0560_item_fluid_states.sql:14` and
  `crates/gameplay/durability/src/durability/item_use/fluid.rs:1`.
- All 57 same-tuple holds are **19 merchants times three exercise wraps**, not
  unresolved competing variants. Crystal exact tuples:50293/subtype500/347222gp,
  50294/1800/1250000gp,50295/14400/10000000gp. Canary lacks these new wrap rows;
  its omission does not contradict Crystal. All 19 Crystal scripts were fetched
  at the same pin and inspected. In `data-global/npc/<name>.lua`, line triples
  below are durable/normal/lasting (Esrik is normal/durable/lasting):
  Baltim83/90/104, Brengus93/102/121, Cedrik190/199/214, Esrik125/131/137,
  Flint85/93/114, Gamel127/133/145, Habdel130/136/147, Hardek94/101/114,
  Memech91/98/111, Morpel89/96/109, Robert92/101/116, Romella80/87/98,
  Rowenna81/87/98, Sam188/195/208, Shanar91/97/108, Turvy92/99/112,
  Ulrik80/87/100, Uzgod174/180/191, Willard132/141/156.
  Native matching charge facts: `tools/content-schema/native-gameplay/hireling-item-profiles.json:226914`
  (500), `:227063` (1800), `:227212` (14400). The existing native default
  representation is a missing timed row, already used by Hireling purchases.
- Wiki checked through approved MediaWiki APIs on2026-11-06: Fandom Marriage,
  Lynda, Wedding_Ring and Fluids returned HTTP402; BR Casamento/Lynda returned
  HTTP403. No bypass. Retained Marriage metadata is
  `docs/reference/content-prep/outfits-content-1/sources.json:2227-2237`,
  page7110/revision1114179 (2025-07-26; captured2026-10-18), SHA256
  `51a13434ddb784995163fdd4c2d83a278434a4dff39d19ca2963a240236b7995`.
  Raw text is absent even from the original packet commit; metadata is **not**
  evidence of inspected marriage transaction semantics. Retained BR Lynda
  crosscheck (`tools/content-schema/npc-authoring/samples/tibiawiki-br-crosscheck-v1.json`,
  NPC657, page11100/revision407179) has no marriage dialogue.

### OWNER QUESTION — marriage remains HELD

**Exact question:** Should native Lynda adopt the pinned donors' proposal /
reciprocal-acceptance / celebration / proposal-removal / free-divorce state
machine? At celebration should Item mint **one engraved ring to the speaker**
(as both executable scripts do), or **two rings, one per spouse** (as both
speeches promise)? Should outfits328/329 be granted permanently at engagement
and retained after divorce, as the scripts implement, or have different grant,
expiry and divorce-removal rules? Please specify the target ring outputs and
outfit lifetime together with the adopted state machine.

This is a durable-value/product conflict, not a missing SQL-schema excuse:
Canary `data-otservbr-global/npc/lynda.lua:161-208`, Crystal
`data-global/npc/lynda.lua:158-205` consume ring+9586 per participant and grant
both outfits at reciprocal acceptance. Celebration code at Canary216-238 /
Crystal213-235 creates **one** ring, despite promising **two** in the preceding
speech. Proposal removal refunds ring+9586 (Canary246-262 / Crystal243-259).
Divorce clears both spouses for no fee (Canary270-288 / Crystal267-285), without
outfit expiry/removal. Donor file SHA256: Canary
`edda7bc3c496b98624fabe8e9318860133798e73a017d71b8667e16e0e9ded5e`, Crystal
`c02b3072ac121d52ccd9e4e6d911942633bddbd9489c9325f66e375e8f5207a9`.

Choosing the ring count or outfit permanence would invent the durable Item /
appearance effect the user explicitly reserves for an owner decision when
source semantics cannot settle it. Marriage is therefore not activated or
marked DONE. A subsequent implementation must lock both Character roots in
stable identity order, require independent participant consent under their
session generations, use immutable stage receipts/revision recovery, and
compose all ring/garment burns, refunds and engraved mints through Item's
existing ledger in the same transaction. Legacy spouse globals are not authority.
