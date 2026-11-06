# questpc3 shared integration notes

## Current continuation: multi-stack carried resources

- New focused fixture `quest_world_callbacks/multi_stack_handin.rs` uses the
  source-pinned three-tear callback to test shared consumption, not to claim the
  Gravedigger quest is qualified. Pickup autostacks, so the fixture must call the
  existing Item loop split owner to establish two real stacks. The first fixture
  attempt observed one proof because pickup merged them; the two-proof assertion
  is retained. Release the split's `SpellLanePermit` before literal USE to avoid
  holding its mutex while the dispatch reacquires it. The attempt that retained
  that permit was interrupted; it is not qualification evidence.
- No shared lease, journal, admission, migration trigger or test assertion was
  relaxed. No database URL was read or printed. The active regression command
  and its final result belong in RESUME.md and MECHANISM-BATCHES.json.

- `crates/gameplay/dispatch/src/gameplay_transport/newhaven_travel.rs`: the
  lane-owned exchange hook must capture engagement before ordinary dialogue.
  Otherwise a first `hi honey scented flowers` both opens Lee'Delle's conversation
  and consumes/mints an Item. Preserve the existing receipt-first replay route;
  new offers require the exact previously engaged NPC identity/generation and
  the adapter's existing current-context checks. The shared Rookgaard test adds
  mixed-greeting/no-inventory/source-greeting assertions for all four NPCs.
  Red: `rookgaard_studded_legs_lee_delle_exchange_atomic_repeatable_replay`
  (restart-greeting-red.log). Green: all four tests under `rookgaard_exchanges`,
  four completion markers, 201.71 seconds (restart-greeting-green.log).

- `crates/gameplay/dispatch/src/gameplay_transport/npc_command.rs`: the interrupted run's state field and dispatch hooks were removed; the only delta is one sorted `questpc3_exchanges` module declaration. The NPC adapter lives in `npc_command/questpc3_exchanges.rs`; its conversation state is held by the existing quest-dialogue state and its routing uses `newhaven_travel.rs` / the existing retained-NPC resume chain. Qualification: all four filtered NPC PG cases passed in `restart-pg-npc-fixed.log`, with completion markers and durations.
- `crates/gameplay/content/src/content/world_reward_claims.rs` and `tools/content-schema/quest-authoring/quest_callback_coverage.py`: synchronize the digest of the regenerated Crystal container source packet after adding the two R–S charged-leaf witnesses. Without this, every existing filled-container projection is refused. Qualification: `questpc3_charged_amulet_cohort_once_and_replay` passed; all 23 restart chest claims passed in `restart-pg-chests.log`.
- `tools/content-migration/test_quest_callback_profiles.py`: its local Rapier fixture now supplies all six definitions required by the extended admission function. Preserve the full-value/idempotence and rejected physical-drift assertions. Native recipe closure now has 135 Item keys because the Protection Amulet admission completes the existing Desert bag and Small Sapphire adds two document carriers. Qualification: all 14 focused tests passed in `small-sapphire-native-items.log`.

No shared stale-lease, replay-cursor, fresh-admission or migration-trigger fixture change has been made in this restart. The new migration 1040 is the mechanic itself, not a shared fixture repair.

- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc3_npc_exchanges.rs`: use `game_item_text_revisions.last_revision` (migration 0101), and start the literal wire connection at its real fresh cursor. Unframed guard probes use separate command IDs and do not manufacture retained ingress continuity. The red run failed with the missing `text_revision` column and then an early EOF at `prepare_quest_callback_resume` for cursor3 without retained ingress state. All four tests under filter `questpc3_npc_exchanges` passed in `restart-pg-npc-fixed.log`.
- Native Small Sapphire import marker64993 was absent from every current B3 map tile and from executable Rust/Lua references. It is distinct from donor UIDs (the source packet keeps an empty legacy list), is recorded in the native projection, and is applied only at its exact position/appearance. Coordinator: preserve this marker allocation when combining content lanes.

- `crates/gameplay/content/src/content/world_reward_claims.rs`: the closed Small Sapphire container has two source-pinned written children; the older decoder only expected the single `written_text` envelope. Preserve the old donor rule and require exactly both pinned texts for the new native recipe. Red/green: `small-sapphire-decoder.log` / `small-sapphire-decoder-fixed.log`; production proof: `small-sapphire-pg-fixed.log`.

- Regenerated `tools/content-schema/reward-claim-authoring/samples/migration/reward-claim-variants.json` and the map overlay source metadata after Small Sapphire’s final canonical identity replaced the provisional source alias. No recipe or B3 tile changed in this repair. Reproducibility proofs: `restart-variant-check-fixed.log`, `restart-map-overlay-check-fixed.log` (changed=0).

- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/rookgaard_exchanges.rs`
  (renamed from `questpc3_npc_exchanges.rs`): the expanded guard probes must advance
  the existing fixture clock between messages. The initial batch reached the real
  chat limiter and failed with `RateLimited` instead of `Ok`; no limiter, lease,
  admission, trigger or assertion was changed. All four `rookgaard_*_exchange_atomic_repeatable_replay`
  tests now pass with completion markers (`continuation-keywords-pg-fixed.log`).
  The earlier cursor/text-revision repair above remains unchanged.

- Active restart: `npc_command/questpc3_exchanges.rs` renamed to
  `npc_command/rookgaard_exchanges.rs` as requested by NOTE-coordination.
  Updated the one shared mod declaration, state type and existing prepare/apply/
  resume references. No additional NPC routing hook was needed for Sinatuki.
- `postgres_tests/rookgaard_exchanges.rs`: extracted the existing literal wire
  dialogue driver as `wire_talk` so the Sinatuki cohort also reaches
  `serve_admitted`. Original NPC dialogues and assertions are preserved.
  Sinatuki PG proof: `sinatuki-stack-order-green.log` (two completion markers).
  The new Sinatuki fixture reverses seed insertion order because production
  carried inventory projects ordinal DESC; no shared inventory ordering changed.
- Migration1041 adds only Sinatuki's exact whole-stack/zero-output and five-shard
  arms to existing craft checks, plus a once/cumulative prerequisite trigger.
  It is mechanic code, not a fixture trigger bypass. PostgreSQL executes all
  admission/custody/deferred completion checks under the runtime role.
