# A..M remaining source-owner boundaries

The current owner instruction says to record BLOCKED and move on when a missing engine feature prevents finishing a quest. These are full-quest blockers, not reasons to drop an existing test or weaken a guard.

Production trace: `durability/quest_state/npc_program.rs::evaluate_source` intentionally calls `plan_storage` with an empty inventory/outfit snapshot, level0, empty vocation and fixed calendar month1. The storage-only owner is not a composed native item/money/XP/outfit owner. `dispatch/npc_command/catalogue.rs::prepare` refuses an unsupported plan. Existing native request ids and capability bits remain unchanged.

- BLOCKED: A Father's Burden full quest — eight exact material burns, per-material2500XP, progress and final OldCape/8000XP still require a composed source item/XP owner. The existing atomic11-track acceptance is tested separately.
- BLOCKED: Bigfoot's Burden full quest — Xelvar's four-item grant and later item/level/world effects have no composed catalogue owner. The existing scoped case must retain its whole-branch refusal.
- BLOCKED: Hot Cuisine full quest — item/cooking effects and authoritative calendar/character facts are absent from the storage-only catalogue owner. No fixed August/month value will be substituted to make it pass.
- BLOCKED: In Service of Yalahar full quest — later material/kill/world effects are not composed by the storage-only NPC owner. A mission-start receipt is not completion.
- BLOCKED: A Pirate's Tail full quest — raid/world progress and source item/context branches remain outside storage-only NPC execution.
- BLOCKED: Hero of Rathleton full quest — source item/combat/world continuation remains outside the greeting/storage adapter. The source greeting/access write has a focused test.
- BLOCKED: Beggar, Druid, Hunter and Knight outfit quests — native item hand-ins and outfit ownership are not composed by this catalogue storage owner; no outfit capability or cosmetic grant is inferred.

LostWisdom's exact51977→3572+800XP exchange has its existing native owner and now has successful real PG proofs. Its quest-log/full-World admission is not certified by these reward-source fixtures. PhaseA's54 current reward-set cohorts are separate from these storylines.

Actual final mixed PG failure: `catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic`, command2603 `hi` toJeanPierre, `Rejected != Ok`, classic_quests.rs:168. Hero+Palimuth steps passed first. `npc_rules/evaluate.rs` rejects calendar guards under storage_only before interpreting the fixed input defaults, so no current calendar authority is fabricated. Test retained unchanged beyond fixture fixes; no whole-suite green claim. FocusedHero test PASS81.139063654s and bothFather cases PASS as separate bounded slices.
