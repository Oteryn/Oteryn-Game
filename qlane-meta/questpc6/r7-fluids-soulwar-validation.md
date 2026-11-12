# Run 7 slot 2: fluid and Soul War validation evidence

Recorded 2026-11-06 from the existing logs; no new tests were run to write this
record. Repository: `/home/mole/oteryn-cache/rs-wt/questpc`, detached HEAD
`1fa04bbbddb5d96e21fecca0bc456f3c5e1109b7`, with local uncommitted authoring changes.
These are command-execution snapshots, not a frozen candidate or CI result.
Other lanes continued integration between commands.

| Check | Exit | Result | Cargo-reported build | Test duration | Log |
| --- | ---: | --- | --- | --- | --- |
| Fluid owner, restored-service baseline before expanded assertions | 0 | 1 passed, 0 failed, 627 filtered | 3m 15s | 8.03s; completion marker 8.023554164s | [r7-fluid-pg-before.log](r7-fluid-pg-before.log) |
| Fluid owner, expanded milk/wine/Present assertions | 0 | 1 passed, 0 failed, 627 filtered | 2m 16s | 4.05s; completion marker 4.052489751s | [r7-fluid-pg.log](r7-fluid-pg.log) |
| Fluid authoring module | 0 | 17 passed | Not applicable | 2.388s | [r7-fluid-authoring.log](r7-fluid-authoring.log) |
| Soul War final-retry regression, RED | 101 | 0 passed, 1 failed, 69 filtered | 1m 15s | 0.00s | [r7-soul-war-entry-red.log](r7-soul-war-entry-red.log) |
| Soul War final-retry regression, GREEN | 0 | 1 passed, 0 failed, 69 filtered | 13.36s | 0.00s | [r7-soul-war-entry-green.log](r7-soul-war-entry-green.log) |
| Soul War PostgreSQL progression/reward/terminal cases | 0 | 2 passed, 0 failed, 2491 filtered | 3m 05s | 55.86s; party case marker 53.61s | [r7-soul-war-pg.log](r7-soul-war-pg.log) |
| Existing endgame entry-refusal PostgreSQL case | 0 | 1 passed, 0 failed, 2493 filtered | 5m 15s | 6.74s | [r7-soul-war-entry-pg.log](r7-soul-war-entry-pg.log) |

The entry-refusal log separately records `/usr/bin/time -p` values `real 297.55`,
`user 275.09`, `sys 71.66` seconds. Its Cargo build field (5m 15s) exceeds that
reported wall time. Those raw measurements disagree; no reconciled total or sum
is asserted here. The other commands did not record an independent wall timer.

Exact commands, run from the repository root unless a working directory is given:

```bash
QPC_SLOT=2 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres filled_fluid_claims_commit_root_and_children_and_replay_without_refilling -- --nocapture > .questpc/r7-fluid-pg-before.log 2>&1
QPC_SLOT=2 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres filled_fluid_claims_commit_root_and_children_and_replay_without_refilling -- --nocapture > .questpc/r7-fluid-pg.log 2>&1
```

The first command qualified the restored PostgreSQL route. The second ran the
expanded owner case: milk subtype 9, wine subtype 2, Present child ordinals 1 and 3
with water subtype 1 and ordinals 2 and 4 without fluid rows, replay after draining,
and transaction rollback when a trigger omits or substitutes a fluid write.
The owner case uses synthetic Item facts; production chest-selector qualification
requires the separate catalogue USE evidence, which is outside this record.
The successful final run opened a database on `server_version_num=180006` and emitted:

```text
POSTGRES_CASE_COMPLETED filled_fluid_claims_commit_root_and_children_and_replay_without_refilling 4.052489751s
```

Working directory for the authoring command:
`/home/mole/oteryn-cache/rs-wt/questpc/tools/content-schema/reward-claim-authoring`.

```bash
python3 -m unittest test_container_native > ../../../.questpc/r7-fluid-authoring.log 2>&1
```

The final 17-test result uses the actual generated Item facts. It includes exact
milk/wine/Present projections, immutable input checks, and refusals for sibling
holds, incomplete source status, altered quantities/positions/UIDs/identity,
missing fluid semantics, unadmitted vessels, and incompatible charge semantics.
The initial targeted fluid authoring RED run had three positive subtest failures
(`native` unexpectedly `None`) across two test methods in 15.116s; that output
exists only in the worker tool transcript, not a retained `.questpc` log.
Its exact command, from the same authoring working directory, was:

```bash
python3 -m unittest test_container_native.ContainerNativeTests.test_wiki_milk_wine_and_present_donor_fills_preserve_source test_container_native.ContainerNativeTests.test_fluid_choice_never_clears_sibling_holds_or_unproven_item_facts
```

```bash
QPC_SLOT=2 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-encounters --lib endgame_entry_forwards_primal_soul_policy_and_canonical_final_phase -- --nocapture > .questpc/r7-soul-war-entry-red.log 2>&1
QPC_SLOT=2 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-encounters --lib endgame_entry_forwards_primal_soul_policy_and_canonical_final_phase -- --nocapture > .questpc/r7-soul-war-entry-green.log 2>&1
```

The RED assertion was exactly `retry_seconds: 259200` versus `244800` at the
Megalomania lever `(33675,31634,14)`. The production policy was then corrected to
244800 seconds (68h), following Wiki spoiler revision 1134584 over donor 72h.
GREEN covers the purple entry and canonicalization of both blue/green aliases.

```bash
QPC_SLOT=2 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib soul_war_postgres_ -- --nocapture > .questpc/r7-soul-war-pg.log 2>&1
```

The filter selected these two tests:

- `gameplay_transport::monk_save::postgres_tests::soul_war::soul_war_postgres_native_taints_xp_reset_and_replay`
- `gameplay_transport::monk_save::postgres_tests::soul_war::soul_war_postgres_mixed_and_all_expired_terminal_replay`

The successful run emitted all of these completion markers:

```text
SOUL_WAR_REWARD POSTGRES_CASE_COMPLETED source_use_no_room_mint_replay_fences
SOUL_WAR POSTGRES_CASE_COMPLETED npc_commands_native_xp_taints_reset_reload
SOUL_WAR_VICTORY POSTGRES_CASE_COMPLETED expired_victory_reward_once_bystander_replay
SOUL_WAR_TERMINAL POSTGRES_CASE_COMPLETED mixed_all_expired_replay
PARTY-XP-1 POSTGRES_CASE_COMPLETED case=party_xp_30 elapsed_seconds=53.61
```

```bash
QPC_SLOT=2 /usr/bin/time -p /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib endgame_entry_postgres_real_dispatch_cannot_allocate_or_reset_without_w02 -- --nocapture > .questpc/r7-soul-war-entry-pg.log 2>&1
```

The successful entry-refusal run emitted:

```text
ENDGAME_ENTRY POSTGRES_CASE_COMPLETED
```

That existing case proves actual wire refusals and unchanged activity/cooldown/
reward counters for its low-level fixture; it does not admit a qualified Soul War
battle room. The concrete remaining room-composition boundary is
`crates/gameplay/dispatch/src/gameplay_transport/endgame_entry.rs`: the qualified
`enter` callback still returns `EntryRefusal::ContentUnavailable` because W02's
native activity/room owner is unmounted. No six-boss room-playability claim follows
from these progression/reward/refusal results. Combined fmt/clippy and final-tree
integration belong to the root coordinator.
