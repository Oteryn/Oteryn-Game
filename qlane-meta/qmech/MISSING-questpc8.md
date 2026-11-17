# questpc8: code-checked missing mechanics

Checked against qmech `aca84855776f1f68e3d2d61bf720366c6b46d1cf` and the retained lane work `47876e7f5ecc816613f1382502fa5c6497fbe9f8`. These are missing adapters, not claims that the underlying condition or inventory engines are absent.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| The Colours of Magic | Server: selected carried USE against another live player's fenced quest state | `content/quests/definitions/quests-00300-00351.json#/records/38/definition/recipe/stages/2` (`s3`: pair remaining powder with a yellow-marked participant). `crates/gameplay/quests/src/quest/predicate.rs::QuestPredicateFacts` reads the acting Character only; it has no peer-state predicate. The retained opening test `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc8_colours_opening.rs` proves envelope/self marking only. Missing adapter must reject self, stale sessions, wrong target colour and cross-runtime-scope targets before consumption. | Partly: opening is tested; peer-pairing and whole-quest completion are unqualified. |
| Wrath of the Emperor Quest | Server: quest item-look disguise and restoration adapter | `content/quests/definitions/quests-00200-00299.json#/records/65/definition/oteryn_recipe` crate-disguise stage; pinned Crystal `data-global/scripts/quests/wrath_of_the_emperor/actions_mission01_crate.lua` uses item look `11328`. `crates/gameplay/quests/src/quest/world_callbacks/effects.rs::CallbackEffects` has no item-look disguise/restoration effect. Core `ConditionValues::ItemOutfit` exists in `crates/gameplay/domain/src/domain/condition.rs`; the missing piece is its quest-owned adapter, not a new core condition. Retained `questpc8_wrath_opening.rs` tests construction only. | Partly: crate construction and other retained component tests do not qualify the whole quest. |

Source locators: Colours <https://tibia.fandom.com/index.php?curid=50860&oldid=1128664>; crate <https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/wrath_of_the_emperor/actions_mission01_crate.lua>.

## Transfer boundary

PR #3042 (`claude/happy-davinci-ijz3wy-wpkg2`, inspected `440e6d5834f3ffc09c4c8313eefd671f0eaa25aa`) already contains this lane's finished quest work, including Father/Winterlight/Venore repairs. It is left unchanged. No duplicate quest payload is transferred here. Unfinished local shipping-filter work is preserved in a local stash and is not submitted as tested work. The unpublished n3 main-integration/generated-file changes are not a new quest/mechanic slice and are not copied over the coordinator-owned package.

## M43 implementation

The new `RewardCallback.peer_requires` hook is implemented in `reward_claim_mint/quest_peer.rs` and resolved by `gameplay_transport/quest_peer_use.rs`. The selected USE ingress retains its target binding and immutable consumption replay. The peer is locked read-only through the actor's reward transaction; busy session/root locks refuse without an opposite-order wait. Peer state must match the current content/quest pin and cannot be newer than its root revision.

The focused real-PG case `questpc8_peer_predicate_postgres_fences_locked_state_and_no_peer_mutation` passed with marker `POSTGRES_PEER_CURRENT_YELLOW_SELF_STALE_INGRESS_BUSY_AND_READ_ONLY_PASS` and duration **3.838504071s**. The schema test was red before adding the hook, then green. Actor-only chosen s3 wiring is in `content/quests/npc-callbacks/m43-colours-peer.json`; it attaches only after the exact separately packaged mark-blue opening is present. No copied PR #3042 payload and no whole-quest completion claim. Source/contract: `docs/reference/content-prep/QMECH-M43-colours-peer.md`.
