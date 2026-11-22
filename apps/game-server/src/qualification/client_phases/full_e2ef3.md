# E2EF3 authoring report — PARTIAL

Journey: `full_e2ef3.rs`, registered once in the real-world journey through
`full_e2ef3_registration.rs`. No fixture-only entry-room registration, SQL item
minting, privileged `qualify` command, server changes or protocol changes.

This patch does **not** complete E2EF3. FAIL records distinguish unfinished
client journeys from missing server implementation. The cited production owners
exist; they are not evidence that their mechanics ran. No MISSING-SERVER claim
is made. `E2EF3_GAP` explicitly lists unexecuted actions; only strict
`e2e_all scenario=...` records are consumed by the existing report.

| Row | Real client assertions in this file | Remaining sub-actions / expected full result |
| --- | --- | --- |
| I01 | Drop and pick up the starter rope through browse-field custody. Buy six five-slot presents from Lubo, USE one in the starter backpack, move and split/merge the gold stack in it, fill its four remaining slots with presents, require `NoRoom` and unchanged custody, then reopen and verify exact quantities after fresh admission. | Node boot must confirm the route and each assertion. |
| I04 | The ground routes target 2818 at (32320, 32254, 6) and 2814 at (32493, 32246, 8). After MOVE into the backpack, the journey checks the 1,024-character refusal, the 2818 → 2834 transform, rewrite refusal, two rewritable write/read cycles, and retained text and definitions after fresh admission. | Expected FAIL on the town bundle at `write-once-transform`. Door 1631 at (32321, 32258, 6) gates 2818 and has no key action. The 2814 room has no connected approach even when ordinary doors are treated as open. The placed scroll sellers are outside the compiled town regions. |
| I06 | Follow a served rat spawn route excluding (32427, 32158, 7). Attack and require the target to leave and a corpse to enter. USE and retain nonempty loot, wait for that corpse to leave while the actor stays in place, require stale corpse USE and stale loot MOVE, then log out, re-admit and walk back to check the exact corpse is absent and custody is unchanged. | PASS requires all five assertions on node boot. Reply/projection owners are `connection/serve/result.rs:230`, `item_ref_admission.rs:115` and `:198`, `corpse_decay.rs:61` and `:92`, `connection/appearance_change/mapping_refresh.rs:49`, `connection/serve/result.rs:129`, `item_move.rs:373` and `:550`, and `connection/serve/setup.rs:114` in gameplay dispatch. |
| I07 | Same served rat route. Click records the result for the actual visible corpse identity. Loot All checks accepted/skipped counts, remaining corpse entries and exact reopened backpack quantities. Exercised policy includes Skip, empty Accept and matching Accept, then fresh admission with custody and a repeat behavioral policy check. Gold→Main assignment records the live reply. The quiver route uses carried or rat-looted gold at an ordinary NPC selling 35562/3447, then hotkey equip, ammunition transfer, container credit and fresh custody. | FAIL at manual click. The corpse exposes an opaque spatial identity and item handle, while command 114 requires the bound item UUID (`item_ref_admission.rs:150`, `kill_reward.rs:3041`, durability `item_quick_loot.rs:197`). Distinct category routing also needs an unpublished recipient UUID and premium; Gold Pouch 23721 needs Inbox custody and the same UUID, with no starter acquisition in the served trade catalogue (`item_quick_loot.rs:88`, `:170`). Other reply owners are `connection/routing/quick_loot.rs:27`, `connection/routing/container_view.rs:43`, `connection/routing/npc.rs:46`, `connection.rs:1538`, `connection/routing/npc_trade.rs:65`, and `hotkey_equip.rs:420` in gameplay dispatch. |
| I08 | Existing conditional USE, one bag consumed, exactly one pinned pool reward and fresh admission inventory equality | Primal Bag 39546: the Hazard 12 Primal Ordeal reward is held, and the compiled town crop has no Plunder Patriarch spawn. Acquisition FAIL when absent |
| I09 | Walk to the nonhouse chair at (32355, 32206, 7), confirm unbound base origin, and step onto and off its tile. USE revision 0 and require the domain-17 update to 2359 at the same ordinal and revision 1. Repeat the same steps to prove movement remains allowed. | PASS requires each observed frame and both collision probes. |
| I12 | Buy pick 3456 from Lubo with corpse gold, hunt served loot sources such as stone golems, gargoyles or enslaved dwarves for Shiny Stone 10310, USE-with, compare exact stone and refinement quantities, repeat the inventory comparison on fresh admission | The first failed route or assertion records its observed value. Exact retained-draw replay remains a FAIL because the public client API cannot resend an acknowledged USE command ID |
| I13 | Existing conditional USE, one bag consumed, exactly one pinned pool reward and fresh admission inventory equality | Bag You Covet 43895: the five Rotten Blood boss loot sources have no spawns in the compiled town crop. Crimson Offering also requires a Bakragore kill. Acquisition FAIL when absent |
| I15 | Hunt served loot sources such as Energy Elementals, Mutated Bats or Dragon Lords for Energy Ring 3051, MOVE into ring slot and verify active 3088, step into a native Energy Field 2126 and compare exact mana and health changes, MOVE to backpack and inspect paused budget before and after fresh admission, then equip, observe expiry and verify retirement on fresh admission | Zuma Magehide has no compiled placement and Demona chest 32515,31596,14 is outside the town crop. The first failed loot route, field route or assertion records its observed value |
| I16 | Route to a placed shrine, seek a Sword from NPC trade and the Basic Strike recipe through ordinary acquisition, then compare the actual slot text, 20-hour duration and 35.00 oz weight before and after fresh admission if application succeeds. | Expected FAIL at `inspect-applied-imbuement` on the town starter unless materials are acquired. The only vendor for 20 Protective Charms (11444) is the Imbuement Assistant, whose placement is absent from the town crop. The crop contains one Valkyrie spawn with a 90-second respawn and a 3.0974% chance of one charm per kill. The existing bounded rat hunt supplies neither the charm nor a confirmed route to the 7,500 gold fee. |

Bag candidates come from `oteryn_gameplay_items::reward_bag::reward_bags`;
the content index only translates production keys to native references. The
journey neither calls the reward-selection function nor predicts a random draw.
The observed inventory must equal the entire prior inventory minus one bag plus
exactly one admitted reward. A refusal, extra mint, missing delta, wrong reward,
wrong session/position or changed reloaded quantities fails. A missing carried
bag logs the observed definition reference and zero carried quantity, then FAIL.
This does not measure the statistical distribution of reward weights.

Server traces read for this patch: audit rows I01/I04/I06/I07/I08/I09/I12/I13/I15/I16;
`container_view.rs`, `loop_move.rs`, routing `depot.rs`, `mail.rs` wire,
durability `item_text.rs`, `corpse_decay.rs`, `quick_loot.rs` and its routing,
`reward_bag_use.rs`, items `reward_bag.rs`, `item_transform.rs`, `mining/mod.rs`,
`equipment_effects_owner.rs`, `timed_item_clock.rs`, `inspection.rs` and its routing.

Node boot was attempted during authoring and stopped at
`NODE_BOOT_RESULT=BLOCKED reason=exact_platform_checkout_missing`. The pinned
Platform revision is `3bc2d0e923db7472ec1f9b534ebe87aaffa1ef05`; CI must run
the privileged topology. No running-node qualification is claimed here.

Validation on base `098c18800b4bc17cd66132370fecd310c70560cf` plus this diff:

- `cargo fmt --all --check` and explicit rustfmt check of `full_e2ef3.rs`: passed.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: passed
  (existing vendor warnings only). Offline resolution first failed on uncached
  `rodio`; the normal locked invocation resolved it without lockfile changes.
- `cargo test --locked -p oteryn-game-server --lib qualification -- --nocapture`:
  16 passed, 0 failed, 3 existing topology-dependent tests ignored. The real
  world-loader smoke compiled and booted the production bundle; this is not
  execution of the new network journey.
- `python3 -m unittest discover -s tools/qualification/e2e_all -p 'test_*.py'`:
  17 passed.
- `git diff --check`: passed. Self-review found no additional defect in the
  implemented bounded assertions; the scope gaps above remain unresolved.

No commit, push or publication was performed. The supplied base includes #2902;
the local `origin/main` tracking ref is older than that base and was not used
to include unrelated upstream changes in the delivered patch.

## Follow-up on base dc4f1b293

The node-boot command stopped at `NODE_BOOT_RESULT=BLOCKED reason=missing_binary_sudo`.
The compiled town route test exercises production map and spawn facts. Wire
journey outcomes below are expectations pending a served run.

| Row | Follow-up and remaining gap |
| --- | --- |
| I01 | Fund and buy the six presents through Lubo's live window. Resolve his named actor within two tiles of the shop and compare the entire purchase delta. The nested custody checks still require the served result. |
| I04 | Try ordinary trade after a blocked ground pickup. Exercise both inscriptions independently and verify any successful inscription after fresh admission. The locked document room and disconnected parchment remain acquisition gaps if no seller is served. |
| I06 | Use native chase, wait for a live rat after a depleted spawn and check that the terminal corpse disappearance could not have occurred before its 60-second fallback lifetime. |
| I07 | Retain the Loot All, filter and quiver journeys. Gold-to-main requires an accepted assignment reply. Manual click and distinct recipients require durable IDs that the current spatial/container projections do not expose. Starter Gold Pouch 23721 acquisition also remains unavailable. |
| I08 | Hazard 12 Primal Ordeal or a Plunder Patriarch drop is needed for Primal Bag 39546. |
| I09 | Exclude the reported occupied western landing in this family's route. Keep both walk-through checks and the exact rotation revision assertion. |
| I12 | Acquire the pick at Lubo's live window and take only the required definition from matching compiled loot sources. Exact acknowledged USE replay still requires client support for retaining the original command ID. |
| I13 | Bag You Covet 43895 requires a Rotten Blood boss or the Crimson Offering path. |
| I15 | Use matching compiled loot sources and reject retirement before the restored active budget. The first failed acquisition, Energy Field, timer or durability step remains a FAIL. |
| I16 | The shared shrine route still depends on the shared-world detour. Basic Strike also needs twenty Protective Charms and 7,500 gold through ordinary play. |
| C03 | Search the compiled movement graph for an ordinary spawn outside protection and approach the live actor. Positive effect sources and attributed critical, perfect-shot and fatal outcome markers remain gaps. |

Reference comparison used the Crystal pin `201189f871d23597b82caf88fd611454a2ddefb8`
as donor evidence and archived Tibia pages for the acquisition gates. Crystal's
bag pools are in `data/scripts/actions/items/reward_bags.lua:26` and `:43`.
The pick action is in `data/scripts/actions/tools/pick.lua:3`. The current
Energy Ring article specifies ten minutes at two mana per damage, matching the
accepted ring rule. The Crystal `rotate.lua` entry turns the player and the
furniture row instead follows the native change to the map item.
