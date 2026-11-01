# E12 evidence: prompt parity, Store, native trade and House projection

This evidence covers the final prompt crosscheck and the Store, projection and native trade integration portions of branch `chatgpt/hireling`, PR 2783. The donor is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`. All source locations below refer to that immutable revision. The aggregate PR records executed Rust/PostgreSQL commands and results; test names in this document identify authored assertions and do not imply that a test has run. The final crosscheck reads the actual protocol routes, native owners and mounted test bodies; the Q16 implementation and its five mounted PostgreSQL cases remain subject to that aggregate verification.

## Parity table

| Element | Implemented rule | Donor file:line / decision | Oteryn owner | Test evidence |
| --- | --- | --- | --- | --- |
| Existing slice before changes | Frozen-base tests exercise original summon, Query, Dismiss and Bank. The recorded baseline is one passing and seven failing cases; current tests assert corrected behavior rather than weakening that evidence. | Existing main `ae4188a9c8d461acb152044814ea23075de02ed4`, migration 0405; source lamp/Bank rules below | `docs/agents/evidence/hireling-baseline.md` and its frozen companion patch; existing mounted PostgreSQL directory | `hireling_postgres_command220_durable_replay_and_lost_ingress_refusal`; `hireling_postgres_scoped_bank_deposit_withdraw_and_replay` |
| Placement and manual return | USE burns the genuine owner-bound Inbox lamp and places its durable Hireling on the player's current native House tile. Refuse another owner/House, a doorway, an occupied Hireling tile or stale authority. Owner-only return waits the source one second, then rechecks current authority, capacity and Inbox room before atomic mint/deactivation. | `data/scripts/actions/items/hireling_lamp.lua:3–38`; `data/libs/systems/hireling.lua:333–381` | Existing USE path and `hireling.rs`; `house_hireling/lamp.rs`, `return_lamp.rs`; existing native item USE and Inbox owners | `hireling_postgres_native_inbox_lamp_fences_replay_and_return`; `hireling_postgres_client_use_management_durable_replay_and_lost_ingress_refusal`; `hireling_postgres_command220_durable_replay_and_lost_ingress_refusal` |
| Ownership loss and restart | Saved identity, owner, placement, sex and appearance survive recreation of the transport owner. An ownership-loss pass returns the lamp to the original owner's Inbox even while offline/full, with authentic original command provenance; it never invents a live session. Valid old 0405 Bankers keep identity and Banker entitlement. | `data/libs/systems/hireling.lua:69–121,299–315,465–496`; ANSWER Q5A/Q9 | `house_hireling/recall.rs`, native offline-recall Inbox owner, House entry/disposition hooks, migration 0719 | `hireling_postgres_full_inbox_refusal_then_offline_recall_and_replay`; `hireling_postgres_0405_upgrade_preserves_owned_banker_and_real_provenance`; exact historical receipt replay plus lost-ingress refusal in both wire cases |
| Banker | Existing command7 Bank conversation, balance, deposit, withdrawal and transfer owners remain authoritative. Current same-House access and the Hireling owner's current Banker entitlement are checked before a new operation; native Bank receipts and audit carry actual House identity. | `data-global/npc/hireling.lua:2340–2347,2398–2399`; `data/libs/systems/hireling.lua:162–178,283–292` | `npc_command.rs`, existing `bank_npc` dialogue and native Bank operation/receipt owner; `durability/bank/hireling.rs` | `hireling_postgres_scoped_bank_deposit_withdraw_and_replay` now also executes a real transfer to native-fixture Bank Friend61/Account60, wire duplicate and frozen-command receipt replay, paired ledger and House transfer audit; `hireling_postgres_guest_service_rechecks_acl_and_house_content_fence` |
| Hireling purchase | Offer 25440 costs 150; at most 10 per character; validate customization and reserve eligibility before the existing Platform purchase. Deliver one native lamp 29432 to the existing Inbox in the same transaction as the Hireling identity and Store delivery receipt. | `data/modules/scripts/gamestore/gamestore.lua:6303–6312`; `init.lua:796–799,1877–1909`; `data/libs/systems/hireling.lua:548–598` | `gameplay_transport/store.rs`, `store/catalogue.rs`; `durability/store/hireling_reservation.rs`, `hireling_delivery.rs`, `house_hireling/store_grants.rs`; migration 0720 | `hireling_store_purchase_lamp_once_and_replay`; `hireling_store_uncertain_tenth_hire_reserves_character_limit` |
| Purchase capacity | Lamp capacity and StoreInbox room are checked before debit. Outstanding lamp reservations count against the existing 20-slot Inbox and 10-Hireling limit. A terminal Platform refusal releases its reservation; an uncertain response retains it for retry of the same request. A failed prepared native delivery keeps the paid operation retryable under a new actual command, without changing the operation/lamp identity or resetting the old attempt budget. | `data/libs/systems/hireling.lua:562–575`; exact native StoreInbox/lamp item facts; existing Store receipt contract | `durability/store/hireling_capacity.rs`, `hireling_reservation.rs`, `storage.rs`; native Inbox prepare/commit wrappers | `hireling_store_full_inbox_refuses_before_debit_and_keeps_delivered_replay`; `hireling_store_refusals_do_not_debit_and_release_reservation`; `hireling_store_uncertain_debit_reserves_entitlement_until_retry`; `hireling_store_paid_lamp_retries_after_prepared_delivery_refusal` |
| Skills | Banker 1001=250, Cook 1002=900, Steward 1003=250, Trader 1004=250. Require at least one Hireling and reject an already-owned entitlement before debit. All current/future Hirelings of the owning character use that entitlement. | `gamestore/gamestore.lua:6334–6372`; `gamestore/init.lua:801–809,2037–2047`; `data/libs/systems/hireling.lua:283–292,709–720`; ANSWER Q1A | Same Store catalogue/reservation/delivery owner; `game_hireling_entitlements` | `hireling_store_skill_once_and_duplicate_refusal`; real Trader stale-window entitlement revocation |
| Outfits | Source dress offers and prices are preserved: profession dresses 500, servant 300, four monster dresses 900. An entitlement is character-wide; the selected look/colors remain per Hireling. Duplicate purchase is refused. | `gamestore/gamestore.lua:6384–6472`; `gamestore/init.lua:810–817,2050–2062`; `data/libs/systems/hireling.lua:205–227,723–734`; ANSWER Q1A | Same Store owner; `house_hireling` selected appearance; native House appearance projection | `hireling_store_outfit_once_and_duplicate_refusal`; projection outfit-change/replay unit tests; management PG |
| Purchase/rename names | ASCII letters/spaces, 3–18 bytes, no leading space, at most one whitespace group, source blocked substrings, source title casing. Consecutive spaces and one trailing whitespace group follow the donor exactly. Names are not unique; independent purchases/reservations may use the same display name. | `gamestore/init.lua:1475–1528,1882–1890`; `schema.sql:697–713` | `durability/store/hireling_model.rs`; Store optional customization binding | `hireling_names_preserve_donor_whitespace_and_title_case`; `hireling_store_retry_binding_includes_normalized_customization`; same-name/distinct-UUID and overlapping-uncertain-name PG regression; `equal_display_names_keep_distinct_house_actor_identity` |
| Paid rename and sex change | Rename 25438=250 and sex-change 25437=120 select a durable Hireling UUID. Require the owning character and an inactive Hireling with its lamp. A repeated operation returns its original effect; sex change cannot toggle twice. Sex change selects the opposite-sex Citizen look and preserves colors. | `gamestore/gamestore.lua:6314–6332`; `gamestore/init.lua:1913–2035`, especially 1929–1930 and 2002–2014; ANSWER Q3A | Existing Store path; `durability/store/hireling_reservation.rs`, migration 0720 effect proof | `hireling_store_rename_once_and_binding_refusal`; `hireling_store_sex_change_toggles_only_once` |
| House NPC projection | The saved name, current look/colors and actual placed position create an existing native NPC slot in the named House. A projection refresh preserves unchanged actor handles; dismissal invalidates them. Other Houses and static NPCs remain intact. Last committed resident exit releases only that House's cached NPC slots. | `data/libs/systems/hireling.lua:318–330`; `data-global/npc/hireling.lua:1–29`; existing House custody/actor-generation contract | `entities/carrier/npc_actor/hireling.rs`; `house_custody.rs`; scoped service context; native visibility/appearance owners | `hireling_projection_is_scoped_to_current_house_and_native_floor`; `hireling_projection_replay_preserves_actor_and_outfit_update_advances_once`; `hireling_dismiss_preserves_other_house_and_static_npcs_and_invalidates_old_handle`; `last_committed_house_exit_retires_only_that_projection_and_reuses_capacity` |
| Visible actor binding | Read-only management projection returns the real existing EntityRef identity/generation for each durable Hireling UUID. Ephemeral actor IDs are not persisted in durable list/commit receipts. | Existing protocol EntityRef contract; ANSWER Q4A retains the current window bound | Existing command 470 management projection variant; scoped House snapshot and appearance owners | Protocol bounded projection tests; real management/USE lifecycle PG |
| Lamp USE uncertainty | Park the exact native request and actual CommandRef before awaiting the native commit. An unknown result or unavailable receipt read closes ingress without a terminal USE acknowledgement. A retry resolves the same plan/receipt; a known refusal clears pending work, and a known commit keeps its truthful result even if current ACL prevents projection refresh. | Existing native `Type2Transaction::commit` and `db::commit_semantic_transaction` uncertainty contract; source lamp placement remains unchanged | Existing `ChannelSpellStates` pending-work owner, native `commit_hireling_lamp`, typed read-only receipt lookup and shared USE routing | `hireling_postgres_lamp_pending_native_commit_and_refusal_recovery`: actual commit before publication, real PostgreSQL receipt-lock contention with no wire acknowledgement, exact retry/replay, known refusal, and frozen uncommitted-plan recovery |
| High-value Trader SELL | Preserve the exact source price, up to 5,400,000 for 43963. The existing native NPC trade owner burns the offered item and mints bounded physical coin stacks; no-room refusal writes nothing. One SELL/SELL_ALL has a combined 5,400,000-gold ceiling under Q8. An explicitly requested SELL above that bound returns native `InvalidInput`; an oversized SELL_ALL plan returns `CapacityExceeded`; insufficient backpack room returns `DoesNotFit`. A retained trade window does not retain Trader authorization. | `data-global/npc/hireling.lua:1221`; native `npc.cpp` sale path; ANSWER Q8A | `durability/npc_trade.rs`, `npc_trade/sell_hireling.rs`; command7/8 adapter | `source_maximum_and_residual_payout_are_bounded_and_conserved`; `highest_offer_uses_native_burn_and_refuses_insufficient_room_before_materialization`; `hireling_postgres_trader_maximum_price_no_room_and_replay` |
| Imbuement packages | All 23 source packages use one native present 2856 with exactly three source child stacks, one native coin/Bank payment and one receipt/audit. The free various service includes packages without granting Trader or Cook. | `data-global/npc/hireling.lua:1821–2033,2304–2324`; ANSWER Q2A/Q3A | Existing NPCBUY owner and `npc_trade/hireling.rs`; source package catalogue and 0702 proof | Cook/package real command7 PG: `bash` costs 6,250 and contains 9657×20,22189×15,10405×10; package D50/extra-child/refusal audit tests |
| Cook purchase integration | Surprise/skill dish 15,000; specific dish 90,000. Q10 distance uses 29409 carrot pie, never 35173 toolbox. Existing fee owner and native Inbox commit are atomic; Bank-only payment works without a fictitious backpack. | `data-global/npc/hireling.lua:2192–2208,2227–2270`; source defect `data/libs/systems/hireling.lua:48`; ANSWER Q10B | `house_hireling/food.rs`, native fee/Inbox owners; real persistent command7 conversation | `hireling_postgres_cook_carrot_purchase_package_and_replay`; `hireling_postgres_cook_bank_only_payment_without_backpack` |
| Gourmet consumption and consumers | All nine registered dishes use native USE burn, shared 600-second cooldown and durable remaining condition ticks. Source bonuses last one hour: speed +80, fishing +30, magic/melee +3, shield/distance +7; recovery dishes restore 30% of current maximum health/mana. Item 29416 is still consumed with cooldown and no buff. Native movement, skill, spell/combat and fishing consumers read the same owned condition state. | `data/scripts/actions/items/hireling_foods.lua:1–34,39–61` | Existing `item_use` owner, `item_use_gourmet.rs`, `spell/actor_conditions/gourmet_food.rs`, native condition/vitals checkpoints | `hireling_postgres_gourmet_use_audit_cooldown_and_persistent_conditions`; `actual_skill_consumers_use_exact_source_bonuses_and_keep_base`; `attributes_replace_haste_coexists_and_relog_preserves_remaining_ticks`; `recovery_uses_current_maxima_clamps_and_creates_no_food_regeneration`; actual movement/fishing consumer tests |
| Steward integration | Current Steward skill belongs to the effective Hireling owner. A permitted guest uses the existing stash command; current ACL is rechecked for every new operation. Historical receipts never restore a revoked live stash view. | `data-global/npc/hireling.lua:2357–2363`; source owner skill lookup `data/libs/systems/hireling.lua:283–292` | Existing character stash owner, sealed House authority and command 79 | `hireling_postgres_steward_native_stash_replay_and_revoked_acl`: real inventory handle, stow 100, withdraw 37, duplicate/recovery replay, ACL revocation |
| Furniture BUY | The complete source capture finds 18 wrappable buy targets. The UI keeps their literal target IDs, but native BUY mints 23398 with immutable original-target provenance. Ground-only target facts stay Ground-only. The existing nonstackable single-item request bound remains. | `data-global/npc/hireling.lua:35–50,117,125`; `src/items/items.hpp:168–170`; `src/game/game.cpp:2761–2767` | Existing NPCBUY transaction plus `house_wrap/purchased_kit.rs`; generated closed target catalogue and 0702 origin proof | `hireling_postgres_trader_wrapped_furniture_native_house_roundtrip`; actual full-backpack refusal before debit, kit mint/audit, immutable target, cmd 402 stale-ACL refusal, unwrap, rewrap, durable replay |
| Furniture lineage | The existing House wrap owner converts the same item 23398↔source target, retaining mint identity. A first unwrap requires native purchase provenance. The original 6355↔6371 path retains its original historical-wrap proof. | Same source createItemBatch rule; existing `0595_house_furniture_wrap.sql` owner | Existing `house_wrap.rs` and command 402; additional pair/origin proof in 0702 only | `purchased_kit_keeps_its_exact_source_target_both_directions`; furniture PG asserts identity, state revision and exclusive custody |
| Malformed/unpriced source rows | Keep all 1,675 literal source rows. Two commented placeholders remain disabled. Buy-loot has 1,376 literal rows, 1,375 priced; 5807 emits no priced offer. Literal 3657/5526/25686 retain source nonportable facts and native refusal. 23721 is a virtual all-loot action and cannot enter physical pouch SELL. | `data-global/npc/hireling.lua:358,1172,1298,1468,1516,1630,1691`; exact Item facts; `src/creatures/npcs/npc.cpp:444–550,590–595`; ANSWER Q7/Q11/Q13B | Source catalogue/page owner; native Item admission; physical adapter and durable selection guard | Catalogue/source-capture checks; owned native refusal guard; full LootPouch integration is excluded by Q13B |
| House movement, loss and fresh relog | Native STEP moves the actual House actor and releases its old entry reservation. The original ordered LastPosition row captures the current House cell, vitals and separate Channel origin together. Retained resume preserves the command namespace; process loss cannot restart a used namespace. A genuinely new authenticated session restores current saved placement only after ACL/content/collision checks. Known refused fresh restoration retains the original native failed-admission cleanup path. | Source placement requires a non-door House cell (`hireling_lamp.lua:4–20`); `src/creatures/players/player.cpp:11612–11628`; `src/io/functions/iologindata_save_player.cpp:240–243` and load counterpart 196–198; ANSWER Q12A/Q16A | Existing native House custody/STEP, `character_last_position`, terminal release and House admission/recovery owners; `docs/contracts/gameplay/hireling-house-position.md` | `hireling_postgres_house_position_ordered_capture_and_same_actor_resume`; `hireling_postgres_house_logout_final_capture_and_authenticated_fresh_relog`; `hireling_postgres_house_process_loss_refuses_started_namespace_then_fresh_restores`; `hireling_postgres_house_acl_loss_gourmet_logout_and_fresh_refusal_cleanup`; `hireling_postgres_house_fresh_restore_refuses_current_hireling_collision` |

Paths in the owner column are relative to `crates/gameplay/dispatch/src/gameplay_transport`, `crates/gameplay/durability/src/durability` or `crates/gameplay/entities/src` as indicated. The Store input is `rulesets/economy/store/hirelings.json`; its generator remains a composition of the existing Store catalogue owner.

## Q1–Q16 reconciliation

The latest PR read reported by the branch coordinator confirms that all sixteen
questions are answered. Repeated display names were settled directly from
Crystal's schema and therefore did not require another rule decision.

| Decision | Final interpretation | Evidence / effect on this implementation |
| --- | --- | --- |
| Q1A | Skills and purchased outfits belong to the character and apply to all current/future Hirelings; chosen appearance remains per identity. | `hireling.lua:205–227,283–292,709–734`; [ANSWER6547428201](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6547428201). No invented per-Hireling entitlement purchase. |
| Q2A | Source various goods remain free of the Trader skill. | NPC `hireling.lua:2367–2375,2402–2412`; same answer. Current authorization is checked again at native trade. |
| Q3A | Full scope includes all imbuement packages and paid rename/sex change through the same Store owner. | NPC packages `1821–2033,2304–2324`; Store `init.lua:1913–2062`; same answer. |
| Q4A | Page source catalogues through existing NPC commands and preserve the 1,024-offer bound. | [ANSWER6547428201](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6547428201); catalogue/page and dialogue bounds tests. No enlarged window or alternate commerce protocol. |
| Q5A | Preserve valid old 0405 identities and their Banker access. | Same answer; migration 0719 maps authentic creation provenance and `hireling_postgres_0405_upgrade_preserves_owned_banker_and_real_provenance` checks it. |
| Q6B | Complete Game reservation/delivery/replay against the existing transport interface; production Platform Store activation remains separate. | Same answer; ten mounted Channel Store cases use the existing `StoreClient`/`StoreTransport` contract with a deterministic producer. The additional House variation uses that same producer. Production mTLS activation is an explicit Left out item below. |
| Q7 | Preserve the complete literal source catalogue. | [ANSWER6547596808](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6547596808); exact comparison of 1,675 ordered rows, 12 categories and 1,409 IDs. |
| Q8A | Extend existing NPCSELL to bounded physical coin stacks for the source maximum 5,400,000; refuse no-room before any write; no Bank/Ground payout. | Source item43963 at NPC line1221; default `AUTOBANK=false`, `config.lua.dist:334`, `configmanager.cpp:95`; [ANSWER6547953322](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6547953322). Combined per-operation ceiling and resource limits remain explicit. |
| Q9 | Preserve and isolate malformed legacy identities without authentic creation receipts. | Same answer; `game_hireling_recovery_quarantine` exposes the exact affected rows. Manual historical repair remains Left out; no session authority or lamp is fabricated. |
| Q10B | Correct the distance skill dish to source-registered food29409 while retaining the 15,000 price. | Source mismatch `hireling.lua:48` versus NPC `2264` and `hireling_foods.lua:27`; [ANSWER6548035915](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6548035915). Purchase and USE tests prohibit toolbox35173. |
| Q11A | Keep literal nonportable rows3657/5526/25686 and refuse them through native item admission. | [ANSWER6548462389](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6548462389); source capture does not grant carried Inventory from shop membership. |
| Q12A | Complete genuine native House STEP so a resident can move off the entry door and use the lamp. | Source lamp `4–20`; PR2783 Q12A. Existing House actor, collision, movement and command owners are composed; no fake Channel position. |
| Q13B | Keep virtual23721 visible with typed refusal; general LootPouch producer/custody and bounded all-loot sweep are separate prerequisites. | `npc.cpp:444–550,590–595`; [ANSWER6548596838](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6548596838). Shared GoldPouch behavior stays with its existing owner. |
| Q14A | Retain the five already admitted native weights for283/284/285/3031/23721 while preserving their raw source facts. | Same answer; importer compatibility pairs are exact native/source values. Any other physical-policy conflict still aborts capture. |
| Q15A | Also retain the exact existing23721 movable=true compatibility value against raw Crystal movable=false. | [ANSWER6549542080](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6549542080). No shared GoldPouch policy/destination changes and no general conflict suppression. |
| Q16A | Extend original LastPosition, terminal-release and authenticated House admission/recovery owners for actual House movement/vitals. | Source player/save/load anchors in the parity table; [ANSWER6549584730](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6549584730). Five mounted lifecycle PG cases cover ordered saves, retained resume, logout/fresh relog, process loss, ACL and occupied saved cells. This is implemented scope, not Left out. |

## Atomicity and scope

The Platform Store transport remains the only coin authority. Game stores a normalized customization binding and serializes eligibility on the existing character root before requesting payment. The same Platform request ID resolves an uncertain response. Accepted delivery is one transaction containing the existing Store receipt, native Inbox mint where needed, Hireling effect and outbox. An unrelated Inbox producer can consume capacity after reservation; that delivery then rolls back and remains retryable under the accepted Store operation. Game does not mint a compensating Store balance.

For a Hireling lamp, the native attempt ID binds the stable Platform operation and the complete
actual CommandRef, including all 64 command bits and the session UUID. This preserves the native
Inbox owner's frozen reservations and per-command work limits when a rejected command is later
retried under a new command or session. The operation, lamp UUID and final Store receipt remain
stable. The receipt's winning native transaction is proven against the same fresh mint/audit,
and the existing post-failure receipt reread reconciles a concurrent winner. The PostgreSQL
regression first commits a preparation while the Inbox is full, checks that no value/audit
committed, then frees one exact ordinary item and retries through real wire bytes. It asserts
two immutable native attempts, one lamp mint/effect, one unchanged Platform debit and one replayed
Store receipt; the old attempt's consumed budget remains unchanged.

Every live House effect uses the actual native House instance/generation. The Channel ID in native events is origin attribution. Additive optional audit fields retain the existing Channel byte encodings: NPC trade field 17, fee field 14, stash fields 6/7, and furniture-kit provenance field 18 in the NPC trade event. Deferred SQL proofs bind these fields to the same native transaction and current House authority. Existing receipts are read before current conversation/eligibility for historical replay.

The House client uses the existing snapshot/delta codecs and real EntityRefs. Generic Channel visibility/occupancy excludes House Hirelings; scoped service lookup checks the player's current House, session, scope generation and NPC generation. Reconnect closes old NPC conversation state according to the existing owner. Multi-step tests keep one actual duplex connection open so they do not rely on weakening reconnect behavior.

The two `durable_replay_and_lost_ingress_refusal` cases send every new USE, management,
Query/Summon/Dismiss and subsequent movement command over real wire frames before replacing the
transport owner. Replacing that owner loses its process-local command directory. The real wire
then refuses the already-started namespace without sending a snapshot or terminal command result,
and the saved command/sequence counters stay unchanged. Only the exact old CommandRefs are retried
through the existing authority and immutable native receipts; these retries prove the original
outcomes without another lamp burn, return mint or appearance mutation. No new command is admitted
after that loss, and no used namespace is reconstructed from a next-command counter. Retained
same-actor resume and genuinely fresh authenticated restoration are covered separately by the
Q16 lifecycle cases.

Pending lamp USE joins the existing actor spell owner's pending-work gate. Its original source,
quantity, placement predecessor, native transaction/event identities and audit time survive
cancellation and uncertain commit. The same resolver runs before new USE, House STEP and native
checkpoints. Receipt lookup distinguishes missing evidence, a found reply, an immutable binding
conflict, and storage failure; storage failure is never converted into a known refusal. Once the
durable outcome is known, the original SessionIngress pending-delivery record retains its exact
native verdict before the actor's frozen work is removed. An active connection still publishes
and terminalizes its own result. If that connection is cancelled first, disconnected recovery
uses the same retained record and original generation, sequence, CommandRef and selected
capabilities to complete it exactly once. A retained verified result can advance only its own
one-command continuity gap; it cannot reconstruct an unrelated counter or a lost namespace.
Legacy Summon keeps its Accepted/boolean codec, including a refused summon. Current House
projection remains derived data, so loss of current ACL cannot leave an already committed
command blocking House exit indefinitely. No additional durable value owner, schema, audit event
or resource ID is introduced by this fix.

The additional `session_ingress/native_tests.rs` cases cover active verdict retention followed
by cancellation and one recovery, unknown work without invented terminal truth, normal active
publication without a second terminalization, and the legacy Accepted/boolean result. The real
pending-lamp PostgreSQL fixture retains its lock-timeout/no-ACK boundary and invokes the native
root's demanded ready-maintenance window after that deliberate holder retirement, as the node
normally does; it does not introduce connection acquisition into gameplay ingress.

Verified execution reported by the root's native gate runs:

- **PG17: all four added ingress units passed:**
  `native_active_verdict_survives_cancellation_and_reconciles_exactly_once`,
  `native_unknown_retains_reservation_without_terminal_truth_or_counter_advance`,
  `native_active_publication_uses_the_original_connection_terminalization`, and
  `native_legacy_summon_completion_retains_accepted_boolean_codec`.
- **PG18: `hireling_postgres_lamp_pending_native_commit_and_refusal_recovery` passed**
  after adding the genuine maintenance step to the deliberate timeout fixture. This exercises
  the full native receipt/ingress/House wire path, immutable committed audit replay, known
  refusal replay, and recovery of the unchanged frozen pre-commit request.

The final focused diff review found no new dead production helper: the pending resolver is
called by normal connection preparation and native resume, and the two typed completion paths
are called by the frozen lamp owner. Existing SessionIngress and PlayerTrade tests remain
present and unmodified. This scoped result does not claim that unrelated final gates passed.

## Fresh House admission and retained session observations

The Q16 fresh-House path runs native initialization through its genuine ACTIVE, unpositioned
Channel source before the existing House handoff. Native admission equipment and seed owners,
pending respawn, committed arrival and tutorial placement take precedence over a saved House.
Progression binding precedes training and quest work; quest obligations and Wheel admission
complete before fresh character facts are loaded for automatic spell repair. The native House
handoff then retires that actual source and supplies the destination's independent session,
lease and scope. No temporary Channel position is published.

The normal handoff moves the progression, quest, Wheel and Atelier observations and clean
weapon-proficiency data through their existing owners. A retained proficiency checkpoint
blocks handoff; its frozen gameplay fence is never retargeted. A lost acknowledgement
reconstructs Character observations with the existing read APIs; it does not rerun an
initializer, automatic spell repair or obligation under a retired source.
The read-only helper checks the actual active destination and controller, exact item fence,
named-House runtime actor and stable Character revision before installing the data. Policy
context/revision mismatch leaves progression unbound; unavailable optional data leaves the
corresponding feature closed. Historical quest claim identities are not rewritten. Moving
these observations does not grant a lease, input eligibility or current gameplay write
capability. An existing quest retry deadline survives a House visit; the generic Channel
obligation writer is suspended for actual House actors. A genuine return to Channel keeps
that deadline or rearms the original retry cadence only for unresolved data.

Premium registration is transferred before the old registration is removed, keeping the
same account refresher and promotion observations through a late source release. Cold House
promotion reads use the existing Character value owner plus independently current House
proof. Resting projections retain their partial stamina and soul counters. Chat and NPC
owners carry their existing mute/talk-rate windows while closing contextual memberships,
NPC handles and confirmation windows. Ordinary transfer carries the two combat intent-rate
windows into the new continuity; command sequencing and domain publication revisions remain
owned by the destination. Actor-owned stamina, Prey, Charm, combat and item-use deadlines
are carried by the separate, independently reviewed native actor transfer token. No current
authority or immutable timer/receipt identity is copied into another scope.

The added outer-owner units are
`house_transfer_keeps_premium_registration_and_promotion_through_late_release`,
`house_transfer_cannot_move_another_accounts_registration_or_conflicting_snapshot`,
`house_handoff_and_retained_refresh_preserve_partial_resting_cadences`,
`house_handoff_preserves_chat_mute_deadline_and_closes_contextual_egress`, and
`house_transfer_keeps_talk_rate_and_starts_no_new_npc_context`. They are authored tests;
their execution is not included in the earlier passing PG17/PG18 results.

### Actual House Store composition

`hireling_postgres_store_named_house_lamp_entitlement_and_replay` enters the House through
the existing native transfer and STEP fixture, then invokes the ordinary Store wire and
the same deterministic Platform test producer as the ten Channel Store cases. It purchases
Hireling 25440 and Trader 1004, checks real Inbox custody and the immutable Store receipt,
decodes the native mint audit with the exact House/session/lease/generation, and checks that
replay and a duplicate entitlement leave exactly two debits totalling 400 coins. No identity,
entitlement or item is seeded for this variation.

The code trace uses `assert_store_fence` and the sealed native Hireling Inbox authority.
Audit operation tag 4 is unchanged: live House authority kind 1 accepts the real StoreOffer
claim; only offline recall kind 2 requires an Item/lamp claim. Migration 0720 retains the
native mint/account/world/current-transaction/outbox proof, and 0722 independently binds the
live House scope to that same native receipt. This genuine House variation closes a coverage
gap in the earlier Channel-only Store tests. At this authoring checkpoint its execution
was still pending; the later focused PG25 result is recorded below.

House logout uses the existing native build/logout receipt owner with a sealed current-House
custody proof. The real login clock is read through committed native transfer ancestry back
to the authentic fresh source and its offline-settlement marker/build receipt. Missing source
proof grants no online time; an intermediate destination timestamp is not substituted. The
source helper, read-only recovery and ancestry changes are authored verification scope in
the final aggregate run; the earlier PG17/PG18 results above do not cover this later delta.

## Bounded native audit shapes

| Shape | Touched / custody / participants | Work units | Existing rows |
| --- | --- | ---: | --- |
| Ordinary BUY | 23/23/23 | 66 | 0328–0331, ordinary shape unchanged |
| Source package BUY | 26/26/26 | 72 | 0328–0331 expanded maxima |
| Source furniture-kit BUY | 23/23/23 | 67 | 0328–0331, same expanded maxima |
| Hireling high-value single SELL | 9/9/9 | 19 | 1114–1117, within existing 23/23/23/66 |

No resource IDs are allocated. Named-House trades use existing 1118/1119 byte ceilings 34,502/34,816. Package D50 assertions cover all 23 exact package definitions with maximum bounded metadata; the largest modeled values are 28,386/28,700 and the real encoder test checks them. Kit D50 assertions cover all 18 exact targets and whole/partial final coin burns; the independently derived maximum is 26,097/26,411 and the real encoder test checks it. A package's three child mints and a kit's one immutable metadata write are separate closed shapes.

Fee existing 0326/0327 admit closed House cause 472 at 26,298/26,612 bytes; ordinary Channel cause limits stay 25,398/25,712. Stash retains 34,000/36,000; its maximum-output House fixture is 22,808 payload bytes. These are source-independent resource limits derived from actual schema encodings, not new gameplay prices or capacities. Exact test execution belongs in the PR's aggregate log.

## Verification performed for this checkpoint

- Independently parsed the cached exact Crystal NPC source, whose SHA256 is `d507c31db74a3558d27bfec6bd08a45076742e6f5ea6da5f5d2baccfc08faf5f`, and compared every generated economic field in source order. All 1,675 rows matched their category, item ID, BUY price, SELL price, subtype and donor line across 12 categories and 1,409 unique item IDs. All 23 packages and their 69 ingredient entries matched both the Rust catalogue and migration 0721 SQL, including names, prices, quantities and donor lines. The largest source SELL price is 5,400,000. This was a read-only source comparison; it does not claim execution of the Rust or PostgreSQL tests.
- Compared `game_store_delivery_proven` against the exact migration 0345 function body. The only change is selecting `COALESCE(NEW.native_transaction_id,NEW.operation_id)` for the native mint transaction; every original atomic item/account/world/current-transaction/count/outbox predicate is byte-identical (`STORE_NATIVE_PROOF_PRESERVED`). `hireling_store_paid_lamp_retries_after_prepared_delivery_refusal` exercises a failed prepared delivery and new-command recovery; its execution remains part of the root verification log.
- Checked all three guarded 0721 replacements against the exact 0595 function bodies; each needle matched once. The patch preserves original fencing, ACL, capacity, occupancy, item-lineage and custody clauses.
- Ran `generate_wrapped_catalogue.generate` against the actual 1,425-record pinned native capture and the generated catalogue, then compared both marked outputs byte for byte: runtime Rust target list and merged 0721 SQL section both matched. The generator verified all source pins/hashes and all 18 targets (`HIRELING_WRAPPED_SOURCE_CAPTURE_VERIFIED targets 18`).
- Independently encoded the kit/package protobuf D50 shapes in Python: kit 26,097/26,411; package 28,386/28,700. These calculations agree with the authored real-encoder assertions; execution of the Rust assertions remains separately recorded.
- Ran Python syntax validation for `tools/content-schema/hireling/generate_wrapped_catalogue.py`. The direct generator/output check above passed against the captured inputs. Its default-path CLI check is also part of the root verification sequence after assembling the complete worktree.
- Kept JSON resource files canonical with a final newline. Rust compilation, Clippy and PostgreSQL results are not claimed here; the root build owner records them after syncing the complete worktree.

## Left out / decisions that affect this slice

1. **Production Platform Store composition, accepted Q6B.** Production `store: None` remains an independent Store-owner dependency. This PR completes Game reservation/delivery/replay and tests the existing `StoreClient`/`StoreTransport` purchase and delivery contract with a deterministic producer. These tests do not validate production mTLS connectivity or credentials; production transport activation remains separate.
2. **23721 all-loot sweep, accepted Q13B.** The source scans the actual LootPouch in scheduled batches and rechecks shop ownership. Existing `NpcShop::SellAll` sells one definition and explicitly excludes this sweep; the repository's general LootPouch producer/custody and bounded sweep are a separate prerequisite. The virtual 23721 row remains visible with typed refusal, and physical pouch sale is refused at the adapter, durable selection and audit. Ordinary Trader and existing per-definition SELL_ALL remain available.
3. **Automatic repair of malformed legacy0405 identities, accepted Q9.** Rows without an authentic historical creation receipt remain preserved in `game_hireling_recovery_quarantine`. They require manual repair using real historical evidence; guessing a session/command or minting an unproven replacement lamp is excluded. Valid legacy identities are migrated and tested.
4. **Production native management UI.** Capability250 starts as bundle-world only, as required by the prompt's capability rollout rule. Its feature/fixture registrations and client-unsupported baseline explicitly report that native management input is not implemented. Server protocol, routing, durable management, real EntityRef projection and literal wire tests are present; production UI activation is separate work. Existing NPC/USE/Store owners remain their original interfaces.

Repeated display names are source-confirmed by Crystal `schema.sql:697–713`; the draft uniqueness/index/name lock were removed, so this is settled parity rather than an outstanding question.

Questions Q1–Q16 are settled. The coordinator separately tracks the later fresh-admission
prerequisite question Q17 in [PR comment6550544216](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6550544216)
and validation-runner availability in Q18
[PR comment6550599243](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6550599243).
The November 1 [execution update6555032232](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6555032232)
reports that the existing Molehill-PC checkout is reachable again; no alternate runner is
being assumed. The interrupted PG21 result remains unverified. Q17 still has no answer.
Aggregate build/PostgreSQL execution and any resulting implementation fixes are verification
work, not omitted gameplay.


## November 1 focused House Store validation

The coordinator verified the terminal PG25 result for
`hireling_postgres_store_named_house_lamp_entitlement_and_replay`: **1 passed, 0 failed,
0 ignored**, 15.72 seconds, with the genuine `POSTGRES_CASE_COMPLETED` marker. This is
a focused pass for real named-House authority/audit, native Inbox lamp delivery, the
Trader entitlement, immutable Store receipt replay and exactly two debits.

The preceding PG24 failure identified capability composition. The established E10 test
contract offers registered but production-unoffered capability 154 only for its test producer. This
fixture now composes the same `StoreClient`/`StoreTransport` producer and binds that offer
in the authentic source session before native House transfer and STEP. The transfer
retains the complete authenticated capability selection, with exact checks before the
write and against the immutable ACK; recovery uses the existing bound policy. The fixture
asserts source/destination selection equality. Global Store offers and production
`store: None` are unchanged, and the test does not validate production mTLS connectivity.

The full 38-test Dispatch `--lib hireling` run contains 30 PostgreSQL scenarios and 8 unit
tests. Its aggregate result, the final implementation head and the remaining final gates
are still pending at this checkpoint; the focused PG25 pass does not stand in for them.


## November 1 complete Hireling filter validation

The subsequent PG25 log (`hireling-pg25-all.log`) has now been read through its terminal
result: **38 passed, 0 failed, 0 ignored**, 179.34 seconds for
`oteryn-gameplay-dispatch --all-features --lib hireling` against PostgreSQL 18.6. Its
38 completed test names match the expected inventory. Exactly 20
`HIRELING_LIFECYCLE POSTGRES_CASE_COMPLETED` markers and 10
`HIRELING_STORE … POSTGRES_CASE_COMPLETED` markers establish 30 real database scenarios;
the remaining 8 are units. This is the completed aggregate for this filter, not 38 separate
PostgreSQL scenarios. The coordinator records it in
[PR comment 6555428344](https://github.com/Oteryn/Oteryn-Game/pull/2783#issuecomment-6555428344).

This run covers the ten Channel Store cases and the new genuine House Store case, together
with the full Hireling lifecycle/services/recovery scenarios. The complete Dispatch unit
run, final static/content checks, node-boot qualification and the final published
implementation head remain separate gates. Q17 remains unanswered; the task is still
implementing, without archive/DONE.


## November 1 complete Dispatch units and package totals

The coordinator subsequently verified the complete Dispatch unit terminal result:
**1760 passed, 0 failed, 14 pre-existing ignored**, 157.79 seconds. The full PG25 script
exited 0. The latest `--lib` result for each of the 11 affected packages sums to
**4533 passed and 16 pre-existing ignored**. That total is assembled from separate
per-package runs, including the corrected Entities fixture; it is not one fresh combined
run and does not erase the earlier 4532-pass/1-fail checkpoint.

This checkpoint preceded the later default-build/cache qualification and native import
correction documented below. The unit results above remain the results of their recorded
runs; Q17 still has no answer.


## November 1 default build provenance and final local gates

Default validation first reported nine `E0433` errors because Cargo reused a Domain
artifact marked `Fresh` whose dep-info omitted Hireling, despite the existing source
export. A timestamp-only refresh of the actual Domain input under the allocated cache
slot preserved its SHA and let normal Cargo rebuild. Those nine errors disappeared;
this was not an unimplemented or newly added module.

The next stage exposed two real default-only imports in `source_progression.rs`.
`CastFactsLoad` and `load_character_cast_facts` now come from native
`gameplay_transport::spell_character_facts`, instead of test-only root aliases. No test
feature was enabled to bypass the error. The subsequent `final-smoke-qualified` run
passed with exit 0 in 99 seconds and recorded the actual server `process_start`.

The coordinator verified these additional terminal results:

- Clippy: PASS, exit 0, 136 seconds; all 11 affected packages with
  `--all-targets --all-features -- -D warnings`, with no owned lint failure.
- Architecture: PASS, 13 seconds; client registrations: 2 passed / 0 failed / 0 ignored,
  0.03 seconds test time.
- Doctests: 48 passed / 0 failed / 0 ignored across 11 packages, 93 seconds overall.
- Content REST: 123 passed / 0 failed / 0 ignored, 2.19 seconds tests / 110 overall.
- `content_world_project_repository`: 5 passed / 0 failed / 0 ignored, 57.89 seconds
  tests / 74 overall.
- Governance: PASS for 22 policy docs and 9 lanes; 129 governance tests passed. Latest
  formatting and diff checks passed; final fmt after PG26 passed in 10 seconds and final diffcheck passed in 0 seconds.
- Internal review of the three final deltas: PASS for that bounded scope.

Current world pin: `cargo run --locked --release -p oteryn-world-bundle-compiler --quiet --
pin-check . /home/mole/chatgpt-hireling-evidence/final-pin` passed with exit 0 in 29 seconds.
Inputs digest: `944661593eecb7e2ac3649b65a32a55ee8b7316a4760ddf3e03d10a9faee6b34`.
Bundle size: 30,953,570 bytes. Bundle digest:
`6edcee04dc8dacbb34f735570e32545ff20793f8da37bd1168f94e941fca8622`.
Data and generators are unchanged since the verified Python source/generator checks.

The fresh numbers helper against main
`03d1a55d06df29f4f110c768364e0e397266d667` exited 1 for unrelated PR2789/2780
overlaps in migrations 716–718 and capabilities 258–259. This task's reserved
0700–0703/250–251/470–473/610–613 block is clear in that snapshot; it is not a global
helper PASS. The earlier conflict report is superseded.

The separate node-boot invocation remains blocked with exit 2
`exact_platform_checkout_missing`; its later preconditions were not reached. The default
smoke PASS is not a node-boot PASS. Remaining final-record values are
the PG26 terminal result recorded below and the published identity in
[PR2783 commit metadata](https://github.com/Oteryn/Oteryn-Game/pull/2783/commits). This file
cannot embed its own commit SHA. No CI-green result is claimed; Q17 remains unanswered.


## Final PG26 and accepted-scope qualification

The final PG26 log (`hireling-pg26-all.log`) was read through its terminal result:
**38 passed, 0 failed, 0 ignored**, 283.40 seconds test time / 501 seconds overall. The
38 exact test names match the earlier inventory; the log contains exactly 20 lifecycle
and 10 Store completion markers, establishing the same 30 genuine PostgreSQL scenarios
and 8 units. The final qualification script completed with **exit 0 in 936.61 seconds**.
Final formatting passed in 10 seconds and final diffcheck passed in 0 seconds.

Accepted-scope self-review and internal review, including the final three deltas, are
complete with no known unaddressed P0/P1. The recorded native tests, units, Clippy,
architecture, registrations, doctests, content REST/World, pin, governance and default
server smoke qualify the accepted implementation. The separate node-boot Platform
checkout prerequisite, unclaimed external CI status and unanswered Q17 are explicit
limits; they are not an unknown local-code result.

Published implementation identity is authoritative in
[PR2783 commit metadata](https://github.com/Oteryn/Oteryn-Game/pull/2783/commits), rather
than a self-referential SHA in this committed evidence file. The task remains DRAFT and
implementing because Q17 is unresolved; this qualification does not mark it archive/DONE.
