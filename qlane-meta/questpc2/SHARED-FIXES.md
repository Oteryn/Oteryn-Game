# questpc2 shared fixture fixes

Oriental Trader continuation: no shared test-support or fixture fix. Under the
explicit 2026-11-06 16:20 routing grant, `npc_command.rs` adds only the sorted
`mod oriental_trader` declaration and one `open_alias` call. The mechanic-named
submodule opens the existing window for Yasir's `ariki` keyword after ordinary
NPC admission/current-conversation checks. Other NPC routes are unchanged.
`npc/src/npc_talk.rs` admits only Yasir's literal captured dialogue and existing
catalogue offers; it versions the talk catalogue. Existing P11 trade/achievement
transactions own all value. Proving regression:
`oriental_trader_catalogue_ariki_sale_and_achievement_once` was red with
`committed Yasir dialogue is not admitted`, then green with an actual
POSTGRES_CASE_COMPLETED/46850ms marker; final61158ms. Final clippy and fmt pass.
No migration or Item grant added.

Panpipe Key4055 continuation: `world-authoring/quest_chest_bindings.py` now
selects the exact complete key-only variant already supported by production.
`content-migration/quest_callback_profiles.py` reuses that same bounded check;
the source-derived native carrier set adds only stone1777. No shared fixture,
lease, replay, fresh-admission or migration-trigger guard changed. Proving test:
`quest_reward_claims_panpipe_key_4055_once_and_replay` (PG initially red with
`ItemReference`; green POSTGRES_CASE_COMPLETED/26039ms in
`panpipe-key-pg-green.log`; bag regression25998ms). Qualification ledger
regenerated281->280 for only this key; all other280 records are unchanged.

No shared fixture/test-support file has been changed by this lane. All fixture
repairs are confined to its own `postgres_tests/quest_reward_claims.rs`:

- Current active generation's qualified room, real frame digest and SQL content
  activation agree before movement. Proved by the Naginata journey PG marker.
- Source map entries retain Item classification and native physics. Offered and
  accepted Item/map capabilities follow the existing placed-door fixture.
  Proved by the Naginata/Noble Armor level-gate PG cases.
- Live duplicate commands share one connection; a cold owner retains the proper
  command cursor and uses the existing durable reward replay path. Proved by the
  Naginata pick/reward PG cases and the reward replay cases.

No lease, replay, fresh-admission or migration-trigger production guard was
weakened. These are lane-private fixture repairs, not patches for other lanes to
merge into shared support.

Phase A `ItemReference` repair: no shared test-support edit. The bounded native
carrier overlay (`build_quest_chest_profiles.py`, `quest-chest-item-profiles.json`,
`quest_callback_profiles.py`) supplies missing real source policies for map
Items2436/2438/2473/2474. The proving tests are the filtered Phase A container,
key-container and leaf cohorts. Strict Item classification and map boot remain.


Restart self-review, qualification ledger repair (not a runtime/fixture change):
- `tools/content-schema/quest-authoring/check_quest_claim_qualification.py` and
  its existing test: exact remaining denominator297->281 because16 N--Q source
  selectors now have real dispatch PG proofs. The existing drift, missing-row,
  duplicate-key, locator and source-witness guards remain; the test also fixes
  the complete336-claim inventory and27/28/281 disposition totals.
- `docs/agents/evidence/quest-claim-qualification.json`: regenerated; only those
  sixteen proved N--Q records leave the unqualified ledger. Every other281
  record is byte-equivalent as a JSON value. Proved red then green by
  `test_quest_claim_qualification.py` (7 tests) and the checker `--check`.
- Coordinator integration: other lanes may qualify more claims; recompute the
  exact combined denominator and regenerate the ledger once after combining
  their coverage maps. Do not keep either lane's stale count or loosen the guard.

Poison Daggers target correction is shared authoring, not test support:
`reward-claim-authoring/poison_daggers.py`, `container_native.py`,
`reward_claim_variant_authoring.py` and `world-authoring/quest_chest_bindings.py`
correct only UID6068's recipe, preserving the immutable donor and all source
holds. Thirty authoring units,28 registered authoring checks, strict source
capture replay and the real
`quest_reward_claims_batch_two_poison_daggers_two_instances_once_and_replay`
PG marker (39427ms) prove two independent dagger instances and atomic replay.
No lease, fresh-admission, replay or migration-trigger guard changed.

Panpipe locked-door continuation: only lane-private
`postgres_tests/quest_reward_claims.rs` changes its new replay call to carry
the returned AdmittedSession continuity, as existing M06 does. Reusing the
pre-command cursor failed connection setup with broken pipe; using the returned
cursor passes `quest_reward_claims_panpipe_locked_door_to_reward_and_replay`
(POSTGRES_CASE_COMPLETED/27802ms). No shared fixture, lease, migration trigger,
fresh-admission or replay guard changed.

Owner-stop-rule continuation: migration1240_al_dee_item_exchange.sql is a new
feature, not a fixture repair. It extends game_npc_trade_consistency_guard's
current body for the closed Al Dee side3; existing Buy/Sell arms are preserved.
No old migration, lease, fresh-admission or replay guard was edited. Proving
PG test is pick_exchange_atomic_confirmation_refusal_and_replay (pending green).

Pick fixture repair: postgres_tests/pick_exchange.rs reopens Al Dee's actual
conversation with hi after every connection closes. The first engine run reached
PG but returned NoConversation for a reconnect starting with pick. This is
correct production disconnect behaviour, not a lease or routing defect. No
shared fixture or production admission guard was changed. All success, replay
and quantity/receipt/audit assertions remain strict; rerun pending.

Pick migration repair proved: the 1020 guard extension now changes the first
exact per-line definition clause with overlay/strpos instead of replacing every
identical key token. The second token lives in Sell All's differently scoped
query. Diagnostic PG error was `missing FROM-clause entry for table "l"`; repaired
PG test passed POSTGRES_CASE_COMPLETED/56367ms with all original receipt/audit
and replay assertions. Temporary constraint forcing/logging was removed. This
changes only the new unmerged1020 migration; no shared fixture or old migration.

Current continuation shared-hook correction: npc_command.rs now has exactly
one additive mod declaration and one dispatch call for questpc2. The mechanic
file oriental_trader.rs is mounted beneath questpc2_item_exchanges.rs; it keeps
Yasir's source alias on the same real ordinary-talk and window owner. The
previous two-hook shape came from separate continuations. No shared test
fixture/admission/lease/replay/trigger was changed. Pick+Uzgod regression passed
2/0 with markers63180/61480ms; Oriental Trader regression passed67591ms.


Seymour continuation: no shared fixture, authority, lease or replay repair.
The existing lane-private pick_exchange PG scenario gains a filled-box branch;
all original unfilled-path assertions remain. Its first run proved the normal
NpcIntentDisposition::Rejected -> CommandStatus::Rejected result mapping, so
only the new branch's incorrect Accepted assertion was corrected. Nested item
initial custody uses the existing replica-mode seed convention; all commands
and refusals use production dispatch and the fenced item owner.

Production sharing: pick_exchange::plan_with_input and audit check_recipe
reuse the exact one-take/one-give shape for Al Dee and Seymour; the closed NPC,
input and output validators remain in Rust and SQL. New migration1022 extends
the current guard without changing1020/1021 or any merged migration. Native
Seymour dialogue joins the existing book; the touched lane module is renamed
to npc_item_exchanges per the owner naming rule, retaining one root mod/call.
Final validation evidence is in present-exchange-verification.tsv, not implied
by this note. No full Present quest completion or independent review claimed.
