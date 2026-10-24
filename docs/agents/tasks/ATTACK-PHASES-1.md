# ATTACK-PHASES-1 — isolate attack drain phases and shared experience

```yaml
task_id: ATTACK-PHASES-1
mode: REFACTOR
status: local_validated
repository: Oteryn/Oteryn-Game
publication: uncommitted (owner instruction)
pr: null
```

## Summary

Behaviour-identical extraction of shared-experience implementation/tests and the
auto-attack drain from `gameplay_transport/attack.rs`. Existing public item names,
Channel attack state and the channel-owner/gate test mounts stay in place.
Bound META policy 3.1.0 was read at immutable revision
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`; this is local authoring only.

## Reason

STRUCTURE-TARGET-1 measured `attack.rs` at 2,063 lines, with 23 commits / 24 tasks
in 30 days (9 in 7 days). PVP-RT-1 and PARTY-XP-1 both changed the approximately
375-line `drain_auto_attacks` body, causing real mid-function conflicts that the
additive merge driver cannot combine. PARTY-XP-1 also appended shared-experience
implementation and tests to that file. The split targets these demonstrated merge
conflicts; it makes no compile-time or runtime performance claim.

## Scope

Owned paths:

- `crates/gameplay/dispatch/src/gameplay_transport/attack.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/attack/drain.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/attack/shared_experience.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs` (gate call path inventory only)
- `docs/agents/tasks/ATTACK-PHASES-1.md`

`shared_experience.rs` holds the native activity implementation, aggression/heal
qualification helpers and both existing activity tests. Module-relative paths and
visibility are adjusted only to preserve their original access scope.

`drain.rs` holds the existing owner method and private phases `select_swing`,
`draw_swing_damage`, `record_hit_activity` and `settle_kill`. Selection preserves
all gates, character/weapon reads, timer polling and ordinal advancement. The
attacker lease check still precedes damage drawing. Damage drawing still marks
creature combat before rejecting a zero draw. Activity still precedes the
committed-hit Charm hook, which still precedes lethal settlement/target clearing.

The owner pass retains its runtime -> spell states -> door -> attack lock order,
prune, activity snapshot and explicit disjoint `entries`, `blocks`, `kills`
borrows. PvP legality/batch installation and creature block/mitigation/physical
commit stay inline in that pass to retain their existing async permit and guard
lifetimes without introducing another parameter bundle. They retain every
original early `continue`; extracted rejection paths return `None` to the same
loop continuation. No lock or asynchronous boundary moves into a helper.
The parent file shrinks **2,063 -> 1,333** lines; the owner method shrinks
**373 -> 270** lines. New files have **484** lines (`drain.rs`) and **354** lines
(`shared_experience.rs`).
`SelectedSwing` contains only already-captured copyable facts; it adds no new
read, allocation, clone, public API, enum variant or trait method.

## Excluded scope

No gameplay, balance, protocol, authority, persistence, content, dependencies,
Soul War, vendor, workflow or repository-tool changes. No tests removed, ignored,
weakened or added; all original test functions remain registered. The existing
`gate_auto_attack` call inventory follows its new source path; its count and
assertions stay unchanged. No Git staging, commit, branch mutation or
publication. Unrelated PostgreSQL mechanics remain
CI-only under the shared-PC scope rule.

## Validation

Evidence and complete logs are retained under ignored
`target/ATTACK-PHASES-1/`.

Parity baseline: **1,036** dispatch lib tests; **55** match `attack`; **29** are
under `gameplay_transport::attack::tests` (including chase-budget tests), rather
than the task packet's older 28. Baseline `attack` execution: **55 passed**.
After extraction: **1,036** tests, **55** matching `attack`, **29** under the
channel-owner mount. Sorted before/after inventories are identical after
normalizing only the two relocated activity tests' private module prefix.
Non-PG inventories are **898 before / 898 after**; **138** PostgreSQL cases are
excluded from that run under the owner's shared-PC rule.

Final `attack`: **55 passed**, including the real PostgreSQL attack-to-kill XP,
corpse and loot case. Filtered `party_xp::`: **12 passed**, with **12** real
database-start markers and durable assertion-completion markers. No unconfigured
PostgreSQL-route marker was observed. Filtered `pvp_live::`: **5 passed**,
covering live melee/spell/death/reload, optional-world refusal, protection-zone
refusal, no-PvP-tile refusal and the bundle-map melee path.

Final non-PG library execution: **884 passed, 0 failed, 14 existing ignored**
(**898** selected; **138** PG cases filtered out). No test is newly ignored. The
initial full run identified the relocated `gate_auto_attack` inventory entry;
only its expected source path was updated, then the full non-PG run passed.

- Package formatting and repository-wide formatting check: PASS.
- Package all-target Clippy `-D warnings`: PASS on the final source, including
  the inventory edit. Existing vendored Tokio/SQLx dependency diagnostics remain;
  the touched crate has no warnings. No vendor changes.
- Governance: **22** required policy documents / **9** project lanes validated;
  **129** governance unit tests passed.
- `git diff --check`: PASS, with only the five owned files changed.
- Whole-diff review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`: PASS.
  `review_moves.py` independently compares the retained parent, shared-experience
  implementation/tests, every extracted phase and expanded orchestration with
  the original source. Only module/visibility paths, reference reborrows,
  continuation-to-`Option` propagation, equivalent `?` syntax and formatting are
  normalized. Every executable statement is accounted for. The sole existing
  test edit is the moved gate's expected source path, with count/assertions intact.
- Workspace-wide `rg` over `apps/`, `crates/`, `tools/`: all `SelectedSwing`
  field/constructor/destructuring sites reviewed (declaration, one constructor,
  two destructures; no impls). No enum variant, trait method or new external
  public function. Production helpers are called by the existing real drain and
  exercised by the named real-dispatch PG cases.

Other exact validation commands:

```sh
/home/mole/.cargo/bin/cargo fmt -p oteryn-gameplay-dispatch
/home/mole/.cargo/bin/cargo fmt -p oteryn-gameplay-dispatch -- --check
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
python3 target/ATTACK-PHASES-1/review_moves.py
git diff --check
```

Exact Cargo test commands (the wrapper supplies the PG route; no dependencies
were downloaded):

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib -- --list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib attack --quiet -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib gameplay_transport::monk_save::postgres_tests::party_xp:: --quiet -- --nocapture --test-threads=2
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib gameplay_transport::monk_save::postgres_tests::pvp_live:: --quiet -- --nocapture --test-threads=2
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib --quiet -- --skip postgres --skip two_nodes_of_a_world_exchange_room_and_private_lines --skip a_listening_relay_stores_its_world_lease --skip a_relay_role_without_the_set_grant_fails_closed --skip every_new_relay_connection_pins_logging_off --skip a_send_on_a_session_that_may_log_bind_parameters_sends_nothing --skip native_npc_receipt_core_preserves_training_and_replays_after_reopen
```

The exclusion flags select the full non-PG library. Most database tests have
`postgres` in their full path; the other six exclusions are existing chat-relay
and native-NPC database cases whose names omit that marker. These exclusions apply only to this local non-PG run; test
assertions, registrations and ignore attributes remain intact. A separate filtered PG run qualifies each affected mechanic.
The original baseline `attack` run omitted only `--nocapture`; its filter and
**55 passed** count are identical.

## Unfinished

No implementation or affected-test work remains. The **14** pre-existing
ignored non-PG cases require retained native/full-spell source captures or
explicit release measurements; this structural task does not change their
qualification. Unrelated PostgreSQL mechanics remain CI-only under the owner's
scope rule. No load-contention failure was deferred. Publication/CI are outside
this local-only task; changes remain uncommitted as requested.
