# questpc3 concrete remaining boundaries

Authoritative status: QUEST-STATUS.json. Sinatuki is now QUALIFIED and is absent from this blocker list.

## Rascoohan Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Rathleton Quest

BLOCKED: Atomic voting-point debit/world tally and fortnightly map/spawn selection owner.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## Ravenous Madness Quest

BLOCKED: Quest-bound corpse Item/progress and Trollwut encounter/access composition.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97`.

Tried: Read this title's captured walkthrough and searched production NPC/callback owners and the canonical quest packet. Traced state-only QuestState and the four closed Rookgaard barter rules; neither executes the listed physical Item/progress/world effects as a complete quest. No seeded end-state or standalone reward was counted as qualification.

Concrete shared boundary: `crates/gameplay/quests/src/quest/world_callbacks.rs:78; crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:42`: Source-bound runtime corpse/special-tool callback with corpse identity, transformation/capture and quest credit. ItemOnTarget currently names only a fixed Item/count/replacement and execute_quest_callback resolves static map entries; an NPC module cannot establish the required live corpse evidence.



Attempt: Compared this quest source with the ItemOnTarget schema and production map-target resolution. The existing map callback is not a corpse-capture proof; a seeded captured Item would bypass the missing operation.

## Realm of Dreams Quest

BLOCKED: Live area-clear, field/switch/lever puzzle and key/book-gated access composition.

Boundary: `content/quests/definitions/quests-00100-00199.json:1; .questpc3/wiki/Realm of Dreams Quest_Spoiler.wiki:18`.

Tried: Compared the Draconia book/key, live-area clear and timed field/switch/lever steps with canonical chayenne_realm native_lowering and the registered world callbacks. No interaction/progress bindings compose the source puzzle; an unconditional final corpse claim would bypass access.

## Research and Development Quest

BLOCKED: Atomic NPC Item/progress and captured-corpse/Spirit Cage composition.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97`.

Tried: Read this title's captured walkthrough and searched production NPC/callback owners and the canonical quest packet. Traced state-only QuestState and the four closed Rookgaard barter rules; neither executes the listed physical Item/progress/world effects as a complete quest. No seeded end-state or standalone reward was counted as qualification.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

Concrete shared boundary: `crates/gameplay/quests/src/quest/world_callbacks.rs:78; crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:42`: Source-bound runtime corpse/special-tool callback with corpse identity, transformation/capture and quest credit. ItemOnTarget currently names only a fixed Item/count/replacement and execute_quest_callback resolves static map entries; an NPC module cannot establish the required live corpse evidence.



Attempt: Compared this quest source with the ItemOnTarget schema and production map-target resolution. The existing map callback is not a corpse-capture proof; a seeded captured Item would bypass the missing operation.

## Rest in Hallowed Ground Quest

BLOCKED: Atomic NPC Item/progress and fifteen-grave flask completion path.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97`.

Tried: Read this title's captured walkthrough and searched production NPC/callback owners and the canonical quest packet. Traced state-only QuestState and the four closed Rookgaard barter rules; neither executes the listed physical Item/progress/world effects as a complete quest. No seeded end-state or standalone reward was counted as qualification.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

## Revenant Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Rift Warrior Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Rise of Devovorga

BLOCKED: Shared seasonal stage/reset, coordinated encounter and participant-reward owner.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## River Runs Deep Mini World Change

BLOCKED: Native daily river-layer/fishing/achievement binding.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## Rootwalker Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Roshamuul Quest

BLOCKED: Shared contribution-to-world-stage/map/encounter composition.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## Rotten Blood Quest

BLOCKED: Qualified W02 encounter admission: endgame_entry.rs still returns ContentUnavailable; existing progression/payment owners remain.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/endgame_entry.rs:93`.

Tried: Traced the existing access-payment, taint/death, reward and entitlement owners to the production command178 handoff. The live W02 callback returns ContentUnavailable even with prerequisites satisfied; seeded native deaths do not prove lever-to-instance admission.

## Rottin Wood and the Married Men Quest

BLOCKED: Physical ribbon hand-outs/hand-ins, corpse callback and later world composition.

Boundary: `crates/gameplay/quests/src/quest/classic_talk.rs:441; .questpc3/wiki/Rottin Wood and the Married Men Quest_Spoiler.wiki:10`.

Tried: Read the pinned Rottin script and traced classic dialogue. The selector explicitly defers ribbon/net effects; Mission2 acceptance alone omits seven physical ribbons, knife-on-rabbit acquisition and the later missions.

Concrete shared boundary: `crates/gameplay/quests/src/quest/world_callbacks.rs:78; crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:42`: Source-bound runtime corpse/special-tool callback with corpse identity, transformation/capture and quest credit. ItemOnTarget currently names only a fixed Item/count/replacement and execute_quest_callback resolves static map entries; an NPC module cannot establish the required live corpse evidence.



Attempt: Compared this quest source with the ItemOnTarget schema and production map-target resolution. The existing map callback is not a corpse-capture proof; a seeded captured Item would bypass the missing operation.

## Royal Bounacean Advisor Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Royal Costume Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Sam's Old Backpack Quest

BLOCKED: Atomic Item/progress, filled-backpack destruction and Kroox quest-door access owner.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97; .questpc3/wiki/Sam's Old Backpack Quest_Spoiler.wiki:9`.

Tried: Read Sam/Kroox walkthrough and native quest tracks; traced carried-item barter and state-only dialogue. Neither composes filled-backpack destruction, Backpack Tourist, Sam/Kroox permission and the quest door in one qualified path.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

## Sanctuary of the Lizard God Quest

BLOCKED: Dawnport is explicitly excluded by the retained owner scope. No in-scope production quest qualification exists for this title.

Boundary: `.questpc3/OWNER-SCOPE-ALL.md:3; .questpc3/wiki/Sanctuary of the Lizard God Quest_Spoiler.wiki:11`.

Tried: Checked the captured Dawnport location against the retained explicit Dawnport exclusion. No qualification was attempted for this excluded title; it remains outside implementation scope, represented as BLOCKED so every inventory row uses a terminal status.

## Sea of Light Quest

BLOCKED: Atomic NPC Item/progress, timed study and corpse/device completion composition.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97`.

Tried: Read this title's captured walkthrough and searched production NPC/callback owners and the canonical quest packet. Traced state-only QuestState and the four closed Rookgaard barter rules; neither executes the listed physical Item/progress/world effects as a complete quest. No seeded end-state or standalone reward was counted as qualification.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

Concrete shared boundary: `crates/gameplay/quests/src/quest/world_callbacks.rs:78; crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:42`: Source-bound runtime corpse/special-tool callback with corpse identity, transformation/capture and quest credit. ItemOnTarget currently names only a fixed Item/count/replacement and execute_quest_callback resolves static map entries; an NPC module cannot establish the required live corpse evidence.



Attempt: Compared this quest source with the ItemOnTarget schema and production map-target resolution. The existing map callback is not a corpse-capture proof; a seeded captured Item would bypass the missing operation.

## Secret Service Quest

BLOCKED: Atomic quest-bound physical hand-ins/hand-outs and exclusive faction completion.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97`.

Tried: Read this title's captured walkthrough and searched production NPC/callback owners and the canonical quest packet. Traced state-only QuestState and the four closed Rookgaard barter rules; neither executes the listed physical Item/progress/world effects as a complete quest. No seeded end-state or standalone reward was counted as qualification.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

## Serpentine Tower Quest

BLOCKED: Source requires placing a pot on fire to enable a teleport before its reward. The pot-conditioned world trigger remains missing; do not grant the pearl through an unconditional chest.

Boundary: `.questpc3/wiki/Serpentine Tower Quest_Spoiler.wiki:9; content/quests/world-callbacks.json:1`.

Tried: Traced the source pot-on-fire condition and searched native quest callbacks for its binding. No source-bound pot-conditioned teleporter is registered; the pearl alone would bypass the puzzle.

## Shadows of Yalahar Quest

BLOCKED: Atomic NPC Item/progress and device/world/encounter composition.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; .questpc3/wiki/Shadows of Yalahar Quest_Spoiler.wiki:3`.

Tried: Compared Telas source stages with the Isle of Evil adapter and native callback inventory. The latter is another quest; physical golem parts/devices and the ritual/encounter chain are not composed by state-only dialogue.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

## Shaman Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Shards of a Broken Moon Quest

BLOCKED: Atomic Item/progress, time-conditioned world callbacks and encounter/access composition.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97`.

Tried: Read this title's captured walkthrough and searched production NPC/callback owners and the canonical quest packet. Traced state-only QuestState and the four closed Rookgaard barter rules; neither executes the listed physical Item/progress/world effects as a complete quest. No seeded end-state or standalone reward was counted as qualification.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

## Skeleton Decoration Quest

BLOCKED: Reward is inside a world barrel; shared map-container pickup/reset ownership is required. Current personal once-claim owner cannot represent floor-reset custody.

Boundary: `crates/gameplay/content/src/content/project/native_spell_world.rs:446; crates/gameplay/spell/src/spell/source_map_initialization.rs:79`.

Tried: Compared the source barrel with native mutable-placement materialization. Nonzero-depth Items are skipped, and the B3 initializer lacks a source-world proof; a personal once claim would change shared reset custody.

## Small Ruby Quest

BLOCKED: The production reset finalizer exists. Remaining: committed B3 mutable Item pickup/remint bridge; `NativeMovementCollisionIndex::Bundle` has no `source_world`, required by `CurrentNativeMapInitialization`. Actual reward is Item3030 count1 at (32437,32174,15), beneath Item2123. Preserve shared floor-reset custody; do not substitute a personal claim.

Boundary: `crates/gameplay/content/src/content/native_cell_lookup.rs:75; crates/gameplay/spell/src/spell/source_map_initialization.rs:49`.

Tried: Decoded the committed (32437,32174,15) tile; traced production source_item_cycle initialization and the reset activation finalizer. The reward is present, but Bundle supplies no source-world initialization proof. A source-only seeded fixture does not qualify production B3 pickup/remint.

## Soil Guardian Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Soul War Quest

BLOCKED: Qualified W02 encounter admission: endgame_entry.rs still returns ContentUnavailable; existing progression/reward owners remain.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/endgame_entry.rs:93`.

Tried: Traced Flickering Soul access, taints, native death settlement, the reward chest and Revenant entitlement to command178. The live W02 callback returns ContentUnavailable; no fabricated party admission or captured death counts as a complete quest.

## Spider Nest Mini World Change

BLOCKED: Daily nest spawn/portal and three-kill-to-entry/achievement binding.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## Spike Tasks Quest

BLOCKED: Atomic NPC Item/progress and task-specific world/corpse/cooldown composition.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97`.

Tried: Read this title's captured walkthrough and searched production NPC/callback owners and the canonical quest packet. Traced state-only QuestState and the four closed Rookgaard barter rules; neither executes the listed physical Item/progress/world effects as a complete quest. No seeded end-state or standalone reward was counted as qualification.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

Concrete shared boundary: `crates/gameplay/quests/src/quest/world_callbacks.rs:78; crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:42`: Source-bound runtime corpse/special-tool callback with corpse identity, transformation/capture and quest credit. ItemOnTarget currently names only a fixed Item/count/replacement and execute_quest_callback resolves static map entries; an NPC module cannot establish the required live corpse evidence.



Attempt: Compared this quest source with the ItemOnTarget schema and production map-target resolution. The existing map callback is not a corpse-capture proof; a seeded captured Item would bypass the missing operation.

## Spirit Grounds Mini World Change

BLOCKED: Native daily portal/layer and independent monster selection binding.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## Spirithunters Quest

BLOCKED: Captured-corpse/Spirit Cage and atomic tiered NPC Item/progress completion.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97`.

Tried: Read this title's captured walkthrough and searched production NPC/callback owners and the canonical quest packet. Traced state-only QuestState and the four closed Rookgaard barter rules; neither executes the listed physical Item/progress/world effects as a complete quest. No seeded end-state or standalone reward was counted as qualification.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

Concrete shared boundary: `crates/gameplay/quests/src/quest/world_callbacks.rs:78; crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:42`: Source-bound runtime corpse/special-tool callback with corpse identity, transformation/capture and quest credit. ItemOnTarget currently names only a fixed Item/count/replacement and execute_quest_callback resolves static map entries; an NPC module cannot establish the required live corpse evidence.



Attempt: Compared this quest source with the ItemOnTarget schema and production map-target resolution. The existing map callback is not a corpse-capture proof; a seeded captured Item would bypass the missing operation.

## Spring into Life

BLOCKED: Seasonal activation and native nest/reset/hatching/spawn composition.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## Stampede Mini World Change

BLOCKED: Daily Terrified Elephant spawn and kill/achievement binding.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## Steal From Thieves Quest

BLOCKED: Atomic bag hand-in/reward/mission completion and source acquisition/capacity path.

Boundary: `crates/gameplay/dispatch/src/gameplay_transport/npc_command/quest_dialogue.rs:1; crates/gameplay/durability/src/durability/item_craft/npc_exchange.rs:97; .questpc3/wiki/Steal From Thieves Quest_Spoiler.wiki:4`.

Tried: Read Ahmet acquisition-before-mission and 400oz capacity requirements; traced its existing quest track and closed barter writer. No bound atomic bag hand-in/reward/once completion path exists; a repeatable barter would bypass it.

Concrete shared boundary: `crates/gameplay/durability/src/durability/quest_state/writer.rs:104; crates/gameplay/durability/src/durability/quest_state/writer.rs:232; crates/gameplay/durability/src/durability/quest_world_callback.rs:39`: Shared CharacterRevision/QuestState plus Item hand-in/hand-out transaction composition for an NPC occurrence. The current Quest writer accepts only the RottenBloodAccessPayment resource bridge; QuestWorldContext accepts Use/Step map occurrences, not NPC_TALK.

A standalone NPC/craft module cannot write canonical Quest tracks and their CharacterRevision receipt. It needs a shared Quest writer composition seam, not another selector or a test-only seeded end-state. The authorized NPC routing hook supplies no such seam.

Attempt: Traced the private commit_quest_inner resource parameter and begin_type2_transaction branch, and the world callback cause gate. Implemented the receipt-derived cumulative Sinatuki path in the lane-owned craft submodule; that path has no existing canonical Quest track to update and does not supply the missing cross-owner primitive for this quest.

## Steamship World Change

BLOCKED: Atomic shared coal contributions, reset and travel composition.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.

## Summoner Outfits Quest

BLOCKED: Canonical outfit/addon catalogue population and per-field admission decisions; preserve existing entitlements and prerequisite quests.

Boundary: `content/cosmetics/outfits/index.json:7; tools/content-schema/cosmetics-authoring/README.md:13; crates/gameplay/content/src/content/native_gameplay/cosmetics_loader/grant_mappings.rs:68`.

Tried: Read this title's captured walkthrough, checked the production outfit index and traced grant mapping validation/resolution. The index is READY_UNPOPULATED and the canonical sex/look/addon/premium/provenance mappings are absent. Numeric donor look IDs and synthetic test packets cannot supply the required admitted production data.

## Swamp Fever World Change

BLOCKED: Atomic medicine contribution and world daily/spawn binding.

Boundary: `crates/gameplay/world-change/src/content.rs:117; crates/gameplay/world-change/src/daily.rs:96`.

Tried: Compared this title's pinned walkthrough with the selection candidates and native daily effects. require_w1_bindings returns WorldChangeBindingsUnavailable for the candidates, and the daily owner does not project this title's required geometry/spawn/contribution effects. Existing catalogue entries do not constitute activation.
