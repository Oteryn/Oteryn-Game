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
| I01 | Open the starter backpack through CONTAINER_TREE; decode its delta; assert capacity, parent and direct inventory agreement; fresh admission and reopen | Ground/inventory/nested moves, stack split/merge, capacity refusal and quantity persistence are unimplemented: FAIL |
| I04 | None | Write/read rewritable text, text limit, write-once transform, rewrite refusal and fresh-login text/transform are unimplemented: FAIL |
| I06 | None | Kill, leave loot, wait for real decay, assert remaining loot retirement and fresh-login retirement are unimplemented: FAIL |
| I07 | None | Manual click/loot-all, accept/skip filters, category recipients, Gold Pouch, quiver ammunition and durable policy/custody are unimplemented: FAIL |
| I08 | If a Primal Bag is already carried: ordinary USE, committed outcome, exactly one bag consumed and one member of its pinned weighted pool minted, entire direct-inventory quantities unchanged after fresh admission | No ordinary acquisition route implemented. Expected FAIL on the starter character; conditional PASS only after all these assertions and fresh admission succeed |
| I09 | None | Approach eligible unbound nonhouse furniture, USE, assert rotated definition and collision are unimplemented: FAIL |
| I12 | None | Obtain pick/stone, USE-with, assert stone consumption and retained refinement across fresh login are unimplemented: FAIL |
| I13 | Same conditional bag journey for Bag You Covet, using its separate production pool | No ordinary acquisition route implemented. Expected FAIL on the starter character; conditional PASS only after all assertions succeed |
| I15 | None | Energy Ring equip, 2:1 mana cost, HP spill, unequip/pause, expiration and durable timer are unimplemented: FAIL |
| I16 | Inspect the actual starter instance through INSPECTION; assert definition/count/nonzero total weight; fresh admission and compare the complete inspection projection | Applied imbuement family/tier/time and weight projection across fresh login are unimplemented: FAIL |

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
