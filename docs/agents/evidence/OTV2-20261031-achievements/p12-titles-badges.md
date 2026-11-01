# P12a/P12b source and durable-owner inventory

Evidence classification: **PROVEN** means read in the pinned source/current repository;
**CONFLICT** means the source's catalogue and runtime disagree; **UNKNOWN** means a rule or
owner is absent. This is retained implementation evidence, not a status authority.
Crystal evidence retains the source registry's **OTS_HYPOTHESIS_ONLY** classification.
**PROVEN** here means observed in the pinned code; it does not establish official Tibia
behavior. Accepted native interpretations and deviations are identified separately below.
The live question/answer channel is PR [#2782](https://github.com/Oteryn/Oteryn-Game/pull/2782).
Coordinator [answer 6547429543](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547429543)
accepts Q1=A (no invented tournament results), Q2=B (actual class/category completion) and
Q3=B (one canonical outfit identity, one count). These are accepted deviations/interpretations
of the donor; there are no open P12a/b questions after that answer.

- Base: `ae4188a9c8d461acb152044814ea23075de02ed4`.
- Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- Baseline title inventory: 43 admitted + 69 `NOT_ADMITTED_TITLES` = 112 Global catalogue entries.
- This change admits 41 more titles (44–84), leaving 28 excluded for the sources/owners below.
- Existing badges: 20 admitted IDs (1–14 and 22–27), with seven tournament badges excluded.
- No new capability or command. Titles use cap 97/cmd 113; badges use cap 199/cmd 387.
- Migration 0705 widens only the existing title/selection checks. It creates no parallel owner.

Coordinator [Q7=A](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6547727362)
keeps native client UI for caps 97/199 and activation beyond the bundle world in **Left out**
as a separate client migration. The server/protocol implementation and real command tests
described here do not assert that this client migration is complete.

## New title parity and production path

| Element | Exact rule | Pinned donor | Native owner / consumer | Verification |
| --- | --- | --- | --- | --- |
| Beastly (44) | Available Charm Points ≥ 2000, permanent | `src/creatures/players/cyclopedia/player_title.cpp:312–314`; `src/creatures/players/player.cpp:1779–1785` | Existing canonical Bestiary rewards and durable Charm unlock costs, interpreted by `derive_balance`; `durability/character_title/charm.rs` | `p12a_beastly_and_iks_use_the_donor_thresholds_and_remain_earned`; mounted `character_title_native_sources_real_dispatch_and_reload` |
| Challenger of the Iks (45) | Ahau race 2346 credited at least once, permanent; the source's attire comment is not an extra implemented check | `src/creatures/players/cyclopedia/player_title.cpp:297–299`; `data-global/monster/bosses/ahau.lua:16–18` | Existing Bestiary/Bosstiary progress for `oteryn:creature.ahau` | Same unit/PG case, wrong boss, stale fence and occurrence replay |
| Beaststrider Grades 1–5 (47–51) | 10/20/30/40/50 owned admitted mounts, permanent | `src/game/game.cpp:327–331`; `src/creatures/players/cyclopedia/player_title.cpp:195–203` | Existing receipt-backed account appearance grants through active `NativeCosmeticsView` mappings | `p12a_cosmetic_grades_and_full_outfits_follow_donor_thresholds`; mounted `character_title_cosmetics_real_dispatch_requires_compatible_earned_facts` |
| Admirer of the Crown (46) | Both looks 1457 and 1456, each with both addons | `src/creatures/players/cyclopedia/player_title.cpp:291–293` | Same native cosmetics owner; exact look identity and compatible base/addon grants | Same cosmetic unit/PG case |
| Big Spender (52) | Both looks 1211 and 1210, each with both addons | `src/creatures/players/cyclopedia/player_title.cpp:294–296` | Same native cosmetics owner | Same cosmetic unit/PG case |
| Royal Bounacean Advisor (53) | Both looks 1437 and 1436, each with both addons | `src/creatures/players/cyclopedia/player_title.cpp:300–303` | Same native cosmetics owner | Same cosmetic unit/PG case |
| Topmodel Grades 1–5 (54–58) | 10/20/30/40/50 earned canonical outfit identities; a male/female pair counts once, permanent | `src/game/game.cpp:333–337`; `src/creatures/players/cyclopedia/player_title.cpp:205–207`; **accepted Q3=B** | Existing appearance grant owner and active canonical outfit keys | `p12a_topmodel_counts_canonical_identities_at_every_grade`; `topmodel_counts_an_earned_addon_on_an_available_starter_once`; cosmetic PG refuses nine identities despite eighteen sex looks, then grants at ten through an earned starter addon |
| Class completion (59–79) and Executioner (80) | Every canonical member of the class, or of all Bestiary, reaches its existing final threshold; conditional | `src/game/game.cpp:361–382`; **accepted Q2=B** corrects `player_title.cpp:254–267` / `player.cpp:9761–9766`; lifetime from `player_title.hpp:52–54` | Existing validated Creature decoder exposes 823 races across 21 classes; existing Bestiary counters | `canonical_title_catalogue_retains_all_classes_and_boss_categories`, `completion_requires_every_exact_member_at_the_final_threshold`, all completion rule unit test and mounted `character_title_class_and_boss_completion_real_dispatch` |
| Category/all Boss completion (81–84) | Every canonical boss in the category, or all bosses, reaches the existing category mastery threshold; conditional | `src/game/game.cpp:384,387–388,391`; **accepted Q2=B** corrects the same donor enum-as-race bug | Same Creature decoder exposes 291 bosses; existing Bosstiary owner supplies counters and category thresholds | Same completion unit tests and mounted class/Boss PG scenario (one kill below final mastery, real final kill, replay, all four title grants) |

The existing mechanism registers command 113 and its capability. Dispatch passes the active
immutable cosmetic definition into the existing title owner. Qualification and grant commit in
one current-session-fenced transaction; a shared Character root lock serializes Charm/Bestiary
reads with their gameplay writers. Neither a client-supplied count nor a prior observed balance
can mint a title. The old trusted internal insertion seam remains for existing internal callers;
the live client path uses `qualify_and_unlock_character_title`.
The page requalification path also reads its native predicates inside that same fenced
transaction and root lock. It accepts no earlier facts snapshot, so an intervening kill/grant
cannot be revoked by a stale page read. The mounted completion scenario observes an actual
PostgreSQL lock wait before checking that earned conditional titles and selection survive.

Cosmetic reads are bounded by the at-most-4096 authored grant keys and use the existing
`read_account_appearance_unlocks` receipt join. Compatible quest, NPC and mount-taming grants
qualify; Store receipts, incompatible provenance and cosmetic-looking opaque keys do not.
Topmodel counts a compatible base grant or a compatible earned addon for an available starter
base. The latter follows `src/lua/functions/creatures/player/player_functions.cpp:3306–3311`
and `src/creatures/players/player.cpp:6840–6850`: `addOutfitAddon` calls `addOutfit`, which
creates the map entry counted at `player_title.cpp:205–207`. Starter content alone does not
create that earned fact, and an addon with no available base cannot qualify the native outfit.
The appearance owner's already accepted premium policy D61 is retained. Production cosmetic
content remains separately admitted (`content/cosmetics/outfits/index.json` is
`READY_UNPOPULATED`); the test's synthetic artifact is not a production content promotion.
No active definition means no new cosmetic grant, while an already earned permanent title stays.

IDs 44–58 add fifteen permanent awards; IDs 59–84 add twenty-six conditional completion
awards. A newly introduced canonical class/category member prevents a completed-set predicate
from holding until its final threshold is reached. Empty catalogues never count as complete.
The original accepted title rules (IDs 1–43) retain their semantics. The new Iks rule follows the
actual pinned Crystal check rather than inventing attire tracking. The available Charm balance
is used, not lifetime earned points. Completed Bestiary rewards can trigger automatic refresh.

## Complete original 69-title disposition

Each row names one of the exact 69 baseline exclusions. `Admitted` means a live production
consumer exists; cosmetic content still has the admission dependency described above.

| Title | Baseline exclusion | Disposition / remaining reason | Donor evidence |
| --- | --- | --- | --- |
| Admirer of the Crown | Appearance | **PROVEN — admitted 46**; existing native appearance grants plus active content; no guessed mapping. | `src/game/game.cpp:410` |
| Armor Breaker | Encounter | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Aspiring Huntsman | TaskPoints | **UNKNOWN — no hunting-task point owner.** Crystal checks current task points; no substitute from charm, prey or quest counts. | `src/game/game.cpp:401`; `src/creatures/players/cyclopedia/player_title.cpp:275–277` |
| Beastly | CharmPoints | **PROVEN — admitted 44**; existing native Charm balance. | `src/game/game.cpp:418` |
| Beaststrider (Grade 1) | Appearance | **PROVEN — admitted 47**; existing native appearance grants plus active content; no guessed mapping. | `src/game/game.cpp:327` |
| Beaststrider (Grade 2) | Appearance | **PROVEN — admitted 48**; existing native appearance grants plus active content; no guessed mapping. | `src/game/game.cpp:328` |
| Beaststrider (Grade 3) | Appearance | **PROVEN — admitted 49**; existing native appearance grants plus active content; no guessed mapping. | `src/game/game.cpp:329` |
| Beaststrider (Grade 4) | Appearance | **PROVEN — admitted 50**; existing native appearance grants plus active content; no guessed mapping. | `src/game/game.cpp:330` |
| Beaststrider (Grade 5) | Appearance | **PROVEN — admitted 51**; existing native appearance grants plus active content; no guessed mapping. | `src/game/game.cpp:331` |
| Big Boss | NoHighscore | **UNKNOWN — no published World boss-point highscore category.** Existing per-character boss points do not supply the World highscore winner. | `src/game/game.cpp:348`; `src/creatures/players/cyclopedia/player_title.cpp:213–251` |
| Big Spender | Appearance | **PROVEN — admitted 52**; existing native appearance grants plus active content; no guessed mapping. | `src/game/game.cpp:411` |
| Bipedantic | BestiaryClass | **PROVEN — admitted 59**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:361`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Blood Moon Hunter | BestiaryClass | **PROVEN — admitted 60**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:362`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Boss Annihilator | BosstiaryTotal | **PROVEN — admitted 81**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:384`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Boss Eraser | BosstiaryTotal | **PROVEN — admitted 82**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:387`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Boss Executioner | BosstiaryTotal | **PROVEN — admitted 83**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:388`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Boss Slayer | BosstiaryTotal | **PROVEN — admitted 84**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:391`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Bringer of Rain | Tibiadrome | **UNKNOWN — no Tibiadrome wave-100 owner.** Donor comment only; `checkOther` falls through to false. | `src/game/game.cpp:417`; `src/creatures/players/cyclopedia/player_title.cpp:310–311,337` |
| Challenger of the Iks | Encounter | **PROVEN — admitted 45**; existing native Ahau kill counter. | `src/game/game.cpp:412` |
| Chompmeister | Quest | **UNKNOWN — no reachable native completion owner for the named quest/event.** Donor has a comment, no implemented title predicate; generic quest tracks are not a guessed completion key. | `src/game/game.cpp:416`; `src/creatures/players/cyclopedia/player_title.cpp:308–309,337` |
| Cleansing Conqueror | Quest | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Coldblooded | BestiaryClass | **PROVEN — admitted 61**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:363`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Competent Beastslayer | TaskPoints | **UNKNOWN — no hunting-task point owner.** Crystal checks current task points; no substitute from charm, prey or quest counts. | `src/game/game.cpp:402`; `src/creatures/players/cyclopedia/player_title.cpp:275–277` |
| Death from Below | BestiaryClass | **PROVEN — admitted 62**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:364`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Dedicated Entrepreneur | MapPercent | **UNKNOWN — no overall explored-map percentage owner.** The native BasicArea is a single discovered boolean, not the donor global percentage. | `src/game/game.cpp:405`; `src/creatures/players/cyclopedia/player_title.cpp:279–283` |
| Demonator | BestiaryClass | **PROVEN — admitted 63**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:365`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Djinn Dominator | Encounter | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Dragonslayer | BestiaryClass | **PROVEN — admitted 64**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:366`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Eclipse Bringer | Encounter | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Elementalist | BestiaryClass | **PROVEN — admitted 65**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:367`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Executioner | BestiaryClass | **PROVEN — admitted 80**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:382`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Exterminator | BestiaryClass | **PROVEN — admitted 66**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:368`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Feared Bountyhunter | TaskPoints | **UNKNOWN — no hunting-task point owner.** Crystal checks current task points; no substitute from charm, prey or quest counts. | `src/game/game.cpp:403`; `src/creatures/players/cyclopedia/player_title.cpp:275–277` |
| Fey Swatter | BestiaryClass | **PROVEN — admitted 67**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:369`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Ghosthunter | BestiaryClass | **PROVEN — admitted 68**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:370`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Handyman | BestiaryClass | **PROVEN — admitted 69**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:371`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Hero of Bounac | Quest | **UNKNOWN — no reachable native completion owner for the named quest/event.** Donor has a comment, no implemented title predicate; generic quest tracks are not a guessed completion key. | `src/game/game.cpp:422`; `src/creatures/players/cyclopedia/player_title.cpp:325–326,337` |
| Huntsman | BestiaryClass | **PROVEN — admitted 70**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:372`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Ink Eraser | BestiaryClass | **PROVEN — admitted 79**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:381`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Interdimensional Destroyer | BestiaryClass | **PROVEN — admitted 71**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:373`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Jack of all Taints | NoHighscore | **UNKNOWN — no Goshnar/aspects highscore owner.** The donor itself returns false for this category. | `src/game/game.cpp:349`; `src/creatures/players/cyclopedia/player_title.cpp:229–231` |
| Lamp Breaker | Encounter | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Light Reaver | Encounter | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Manhunter | BestiaryClass | **PROVEN — admitted 72**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:374`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Master Enhancer | Proficiency | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Master of Illusion | BestiaryClass | **PROVEN — admitted 73**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:375`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Master of Souls | Proficiency | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Master of the Arsenal | Proficiency | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Novice Enhancer | Proficiency | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Ooze Blues | BestiaryClass | **PROVEN — admitted 74**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:376`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Pit Fighter | Encounter | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Planegazer | Quest | **UNKNOWN — no reachable native completion owner for the named quest/event.** Donor has a comment, no implemented title predicate; generic quest tracks are not a guessed completion key. | `src/game/game.cpp:424`; `src/creatures/players/cyclopedia/player_title.cpp:330–331,337` |
| Proconsul of Iksupan | Quest | **UNKNOWN — no reachable native completion owner for the named quest/event.** Donor has a comment, no implemented title predicate; generic quest tracks are not a guessed completion key. | `src/game/game.cpp:409`; `src/creatures/players/cyclopedia/player_title.cpp:289–290,337` |
| Robinson Crusoe | Quest | **UNKNOWN — no reachable native completion owner for the named quest/event.** Donor has a comment, no implemented title predicate; generic quest tracks are not a guessed completion key. | `src/game/game.cpp:415`; `src/creatures/players/cyclopedia/player_title.cpp:306–307,337` |
| Royal Bounacean Advisor | Appearance | **PROVEN — admitted 53**; existing native appearance grants plus active content; no guessed mapping. | `src/game/game.cpp:413` |
| Sea Bane | BestiaryClass | **PROVEN — admitted 75**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:377`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Seasoned Armorer | Proficiency | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Snake Charmer | BestiaryClass | **PROVEN — admitted 76**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:378`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Tibia's Topmodel (Grade 1) | Appearance | **PROVEN — admitted 54**, accepted Q3=B; one earned canonical outfit identity counts once. | `src/game/game.cpp:333`; `src/creatures/players/cyclopedia/player_title.cpp:205–207`; `src/creatures/players/player.cpp:6840–6850` |
| Tibia's Topmodel (Grade 2) | Appearance | **PROVEN — admitted 55**, accepted Q3=B; one earned canonical outfit identity counts once. | `src/game/game.cpp:334`; `src/creatures/players/cyclopedia/player_title.cpp:205–207`; `src/creatures/players/player.cpp:6840–6850` |
| Tibia's Topmodel (Grade 3) | Appearance | **PROVEN — admitted 56**, accepted Q3=B; one earned canonical outfit identity counts once. | `src/game/game.cpp:335`; `src/creatures/players/cyclopedia/player_title.cpp:205–207`; `src/creatures/players/player.cpp:6840–6850` |
| Tibia's Topmodel (Grade 4) | Appearance | **PROVEN — admitted 57**, accepted Q3=B; one earned canonical outfit identity counts once. | `src/game/game.cpp:336`; `src/creatures/players/cyclopedia/player_title.cpp:205–207`; `src/creatures/players/player.cpp:6840–6850` |
| Tibia's Topmodel (Grade 5) | Appearance | **PROVEN — admitted 58**, accepted Q3=B; one earned canonical outfit identity counts once. | `src/game/game.cpp:337`; `src/creatures/players/cyclopedia/player_title.cpp:205–207`; `src/creatures/players/player.cpp:6840–6850` |
| Time Traveller | Quest | **UNKNOWN — no reachable native completion owner for the named quest/event.** Donor has a comment, no implemented title predicate; generic quest tracks are not a guessed completion key. | `src/game/game.cpp:425`; `src/creatures/players/cyclopedia/player_title.cpp:332–333,337` |
| Tumbler | BestiaryClass | **PROVEN — admitted 77**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:379`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |
| Unbreakable Purifier | Encounter | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Usurper of the Demon Throne | Encounter | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Veteran Armorer | Proficiency | **UNKNOWN — no pinned Crystal title rule.** Existing encounter/proficiency/quest facts cannot establish an absent title threshold; no inferred Global rule. | `src/game/game.cpp:322–428` (no entry) |
| Weedkiller | BestiaryClass | **PROVEN — admitted 78**, accepted Q2=B; actual complete class/category/all predicate over existing durable counters. | `src/game/game.cpp:380`; `src/creatures/players/cyclopedia/player_title.cpp:254–267` |

### Owner checks behind the exclusions

- Bestiary has 823 admitted rows with final kill thresholds and creature authoring class metadata:
  `crates/gameplay/content/src/content/project/bestiary.rs` (`canonical_title_catalogue`).
  Durable completed entry facts exist. Q2=B resolves the donor contradiction in favor of actual class/all completion. The projection
  extends this existing decoder and creates no second Bestiary owner.
- Bosstiary has current counters and immutable category receipts in migrations 0445/0532;
  `crates/gameplay/durability/src/durability/bosstiary.rs` owns credited kills. Q2=B authorizes the complete category/all projection. Category mastery thresholds are read
  from the existing `domain::bosstiary::kill_thresholds` owner, not redefined by titles.
- Published highscore categories stop at 11 (the optional Charm source):
  `crates/gameplay/durability/src/durability/highscore_snapshot.rs:96–115` and
  `highscore_snapshot/reply.rs:1–6`. There is no boss or Goshnar rank projection to reuse.
- Native map discovery reports `u8::from(progress.discovered) * 100` for its single BasicArea:
  `crates/gameplay/domain/src/domain/cyclopedia/basic.rs:84–96`. It does not measure all
  donor map areas. Previously accepted Globetrotter behavior is not reopened by this change.
- Searches of production Rust and migrations found no hunting-task points owner, Tibiadrome
  rota/wave owner, or named completion owner for Iksupan, Galthen/Bounac, Hot Cuisine,
  Schrödinger's Island, Opticording Sphere or 25 Years of Tibia. Generic quest state alone
  cannot authorize a guessed key or threshold.
- Crystal's only gameplay `addTitle` Lua call is the already admitted Pensive Wanderer grant
  (`data-global/npc/enpa_deia_pema.lua:88`); the other calls are administrative talk actions.
- Reigning Drome Champion is already admitted as stable ID 4, but its rota result remains
  unavailable (`src/creatures/players/cyclopedia/player_title.cpp:226–228`). No fabricated
  expiry/result or tournament title is added. The prompt's seven tournament omissions are
  the seven badge IDs below; no separate tournament title catalogue exists in the donor.

## Badge audit

The existing badge production path is complete: cap 199/cmd 387, account-scoped native facts,
session-generation-fenced `refresh_account_badges`, permanent idempotent insertion, response
encoding and account sharing. P12b creates no new badge state or wire command. Migration 0573
already admits all 20 sourced native badges and rejects 15–21.

| IDs | Existing admitted rule | Native source | Crystal comparison / disposition |
| --- | --- | --- | --- |
| 1–5 | Account age ≥ 1/5/10/15/20 calendar years | Account UUIDv7 timestamp through `account_badge.rs` | `src/game/game.cpp:294–298` gives thresholds; Crystal `player_badge.cpp:120–122` substitutes loyalty/365. Preserve the already accepted native account-age rule. |
| 6–8 | Loyalty strictly greater than 100/1000/5000 | Existing Platform `game_account_loyalty_projections` | Catalogue `src/game/game.cpp:300–302`; donor `player_badge.cpp:124–126` uses ≥ and an 8-bit parameter. Preserve accepted native semantics and Platform ownership; no Game-side loyalty invention. |
| 9–11 | Sum of account character levels strictly greater than 500/1000/2000 | Existing Character roots/progression | Catalogue `src/game/game.cpp:304–306`; donor `player_badge.cpp:159–164` uses ≥ and an 8-bit parameter. Preserve accepted native semantics. |
| 12–14 | Druid, Knight, Paladin, Sorcerer each at level ≥ 100/250/500, promotions normalized | Existing character build/progression | Catalogue `src/game/game.cpp:308–310`; Crystal `player_badge.cpp:167–189` also requires Monk. The accepted native Global four-vocation rule remains unchanged. |
| 22–24 | All five base vocations each at level ≥ 100/250/500 | Same native build/progression owner | Already accepted Extended Master Class rules on base main; not in the pinned Crystal badge catalogue. |
| 25–27 | A Monk at level ≥ 100/250/500 | Same native build/progression owner | Already accepted Enlightened Student rules on base main; not in the pinned Crystal badge catalogue. |
| 15–17 | Tournament participation 1/5/10 | **No source/owner** | `src/game/game.cpp:312–314` lists names/amounts; `player_badge.cpp:86–88,192–195` never grants them. Q1=A on PR #2782; remains out. |
| 18–21 | Tournament points 1000/2500/5000/10000 | **No source/owner** | `src/game/game.cpp:316–319`; `player_badge.cpp:86–88,197–199` never grants them. Q1=A on PR #2782; remains out. |

Existing `p12b_rules_use_global_thresholds`,
`p12b_badges_wire_is_bounded`, `p12b_stored_badges_decode_only_admitted_ids`, and mounted
Cyclopedia badge scenarios cover the retained behavior. Loyalty badge eligibility remains
conditional on the Platform projection becoming available; earned badges stay permanent.
No badge is granted from tournament catalogue text alone.

## Verification evidence

- Before implementation, standalone `rustc --test` ran
  `tests::p12a_source_backed_kill_and_charm_titles_are_admitted` and failed because Beastly
  had no admitted title (exit 101, one failing test).
- Before the earned starter addon fix, the isolated remote durability run
  `durability::character_title::cosmetics::tests::topmodel_counts_an_earned_addon_on_an_available_starter_once`
  compiled successfully, then failed with actual `0`, expected `1` (exit 101; 0 passed,
  1 failed, 552 filtered out; parent log `red-topmodel-4.log`). The predicate was changed
  only after that behavioral RED was recorded. The identical test then passed in the
  isolated remote run (1 passed, 0 failed, 552 filtered out; parent log
  `green-topmodel-6.log`, compilation 9.60 seconds). The final mounted cosmetics run that
  exercises the addon-derived tenth identity remains separately tracked by the parent.
- After all 41 additions, including the accepted Q2/Q3 rules, standalone protocol module execution passed all 16 tests.
  Exact commands (Rust 1.99.1 already available in the managed environment):

  ```text
  rustc --edition=2024 --test crates/protocol-oteryn/src/character_info.rs -o /tmp/achievements-title-rules-green
  /tmp/achievements-title-rules-green --nocapture
  ```

- `rustfmt` was run on the owned Rust files and their title helper children; `git diff --check`
  passed. Workspace Cargo/clippy and PostgreSQL execution are coordinated by the parent on
  the isolated remote build checkout. This inventory does not claim those runs passed.
- The mounted PostgreSQL scenarios return `Ok(())` without the configured admin URL and
  emit `POSTGRES_CASE_COMPLETED` only after their real scenario succeeds.
- `character_title_native_sources_real_dispatch_and_reload` uses real commands 84/113 and
  current-source Bestiary receipt chains; it tests spent balance refusal, wrong boss, stale
  session, automatic grant, replay, selection, durable readback and permanent retention.
- `character_title_cosmetics_real_dispatch_requires_compatible_earned_facts` activates a
  synthetic authored artifact through the production decoder, then drives command 113 and
  existing receipt-backed account facts. It tests missing addon, incompatible provenance,
  ignored Store fact, nine-to-ten mounts and canonical outfits (the tenth comes from an earned
  addon on an available starter), the exact three donor look pairs, replay and later missing
  content with permanent selected title retention.
- `character_title_class_and_boss_completion_real_dispatch` seeds valid native Bestiary/Boss
  receipt chains, refuses a class missing one member and every boss category one kill below
  final mastery, records the final boss kills with the production owner, then verifies
  automatic title grants, explicit command replay, selection and durable readback. Its
  `pg_blocking_pids` assertion also proves requalification waits for the native writer's
  Character root lock before returning the earned completion titles and selection.
