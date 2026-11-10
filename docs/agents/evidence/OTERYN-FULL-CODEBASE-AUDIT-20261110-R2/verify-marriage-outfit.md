# CL-018 — Earned Newly Wed outfit omitted by the native appearance path

**CONFIRMED · P2 · static integration finding.** Source base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. Independent root review confirms the worker report; no native, Cargo or SQL test was executed.

## Required behavior and bounded trigger

`docs/contracts/OTERYN_NPC_MARRIAGE_V1.md` requires both Newly Wed base looks at reciprocal acceptance, permanently and without a quest requirement. Consider successful normal Lynda acceptance with maintained cosmetics content, a current native session and the resulting account grant, without a separate older quest grant for the same outfit. The stored entitlement remains present, but Newly Wed is absent from the selectable catalogue and a direct otherwise-valid selection is rejected.

## Actual producer and consumer

The current `crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs` enters the marriage adapter after ordinary NPC checks. Its `npc_command/marriage.rs` binds prompt expiry, current NPC generation, item fence, node and native definitions before the normal sequenced commit. `crates/gameplay/durability/src/durability/npc_marriage.rs` recognizes reciprocal acceptance as operation2 and calls `character_appearance/marriage.rs` in the same transaction. That owner writes the dedicated marriage grant and its provenance under existing account grant locks.

`crates/gameplay/content/src/content/native_gameplay/cosmetics_view.rs` includes that grant among declared read keys and implements its neutral lookup correctly. `crates/gameplay/durability/src/durability/character_appearance/earned_unlocks.rs` retains the resulting account-owned NPC fact. The current observation path constructs `EarnedCosmetics.with_sex(state.sex)` in `crates/gameplay/dispatch/src/gameplay_transport/appearance_change/catalogue.rs`. Its lookup reaches `NativeCosmeticsView::earned_unlock_for_sex`, whose special handling includes Drome and Store but omits marriage. It instead calls the authored grant-map resolver, which has no marriage entry and returns Absent. The durable appearance writer independently uses the same sex-qualified adapter before validation.

The maintained manifest pins the inspected cosmetics descriptor and authoring bytes. Outfit40 is Newly Wed, male328/female329, nonstarter for both sexes. Inspection of all295 authored mapping keys finds no marriage key. The sole Newly Wed mapping, record116, has a different quest key and provenance. It cannot authorize the actual marriage fact.

## Counterguards and practical limits

Current actor, session, content digest, character revision, sex and account ownership checks remain effective; none converts the marriage fact into that unrelated quest grant. An existing compatible quest grant can mask the problem. Failed acceptance or absent cosmetics content is outside this trigger. The neutral lookup succeeding does not refute the actual sex-qualified route. The inspected SQL test helper counts stored grants; it does not itself assert selectable appearance.

There is no established entitlement deletion, unauthorized outfit access or deployed-state observation. This is a normal feature failure after a successful authorized operation. Repair should make the current sex-qualified path preserve the explicit marriage mapping, with a regression covering successful acceptance, the subsequent catalogue and durable selection for both sexes, plus independence from the older quest grant.

`marriage-outfit-validation.json` records independent exact-source checks for23 evidence files and the bounded data validation. Full semantic review of the complete authoring file or manifest is not claimed by these selected checks.
