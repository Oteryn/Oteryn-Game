# PVP-LIVE-FLAKE-1

```yaml
task_id: PVP-LIVE-FLAKE-1
title: Remove competing owner turns from live PvP PostgreSQL tests
mode: REPAIR
status: validated
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: e9f3d1e8aabc43aa5dce35e9e0584600ae58887c
branch: null
pr: null
owner: local Codex worker
created_at: 2026-10-24
updated_at: 2026-10-24
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/pvp_live.rs
  - crates/gameplay/dispatch/src/gameplay_transport/pvp_live.rs
  - docs/agents/tasks/PVP-LIVE-FLAKE-1.md
public_contracts: []
depends_on: []
external_repositories: []
```

## Summary

Remove the test driver's competing combat drains and reconciliation calls while
`serve_admitted` is running. Replace the fixed 2,100 ms sleep with a bounded wait
for the serving loop to consume each injected fault and reconcile its lane.
Preserve the exact rollback, durable harm, HP reduction, spell, death, replay and
reload assertions.
The existing thread-local fault seam now returns a one-shot consumption receipt
and refuses to overwrite an unconsumed fault. Production behavior is unchanged;
the seam is compiled only under `cfg(test)`.

## Reason

The owner reported intermittent failure at the strict `live melee changes player
HP` assertion, followed by failures in other cases. Before this repair, two local
`--lib pvp_live` PostgreSQL runs passed all five cases (10/10 cases; 172.13 s and
188.72 s). The reported failure did not reproduce in those runs.

Adding fault confirmation alone did not fix it: a three-run filtered sequence
passed twice and failed once (bundle scenario). A further diagnostic filtered
sequence passed once and failed once (wire scenario). Eight bundle-only diagnostic
runs passed five times and failed three times. These intermediate tests retained
the driver's explicit owner drains and are diagnosis, not final qualification.

The failing trace identifies the actual path:

1. `BeforeCommit` is consumed and the rollback assertions pass.
2. Attack command 4 returns `Ok`. The next swing becomes due and increments the
   swing sequence to two.
3. Its `pvp_target_legal` call enters `current_quest_fence`;
   `FreshAdmissionStore::current_session_at` returns **`RootUnavailable`**.
4. The legality check returns `None`, so `drain_auto_attacks` clears the held
   target before any second harm transaction. `AfterCommit` remains armed and HP
   stays unchanged.

`DurabilityRoot::connect_test_runtime` configures exactly one database connection.
`try_acquire_ready` refuses when that holder is busy. The test driver's manual
equipment/drain calls raced the real serving loop's other database work for that
holder. Waiting longer or retrying drains does not restore a cleared target. This
is a test-driving race; production's unavailable-authority refusal remains intact.
All temporary runtime diagnostics were removed after tracing this path.

Fault state is already thread-local; the fixture runs on a current-thread Tokio
runtime within each case's separate large-stack thread. It is not a process-global fault shared by
these parallel cases. The cases also hold a shared bootstrap mutex: a panic in
one case poisons that gate, explaining secondary case failures without proving a
second PvP legality defect.

The repaired helper sends each original attack command once and services client
liveness while the real connection ticks drive combat. Within the same ten-second
bound it requires the exact fault notification, then observes the spell lane
until the serving loop has reconciled it. Acquiring and dropping an unresolved
lane does not run a database operation or take its parked attempt. The driver
does not call `spell_lane_permit` to perform reconciliation itself. It then runs
the original assertions. A stopped melee path still fails; there is no HP polling,
retrying the command, synthetic damage, RNG override or relaxed assertion.

## Scope

- Test-only fault notification and prevention of accidental fault replacement.
- A unit test proving that only the matching fault confirms consumption, once.
- Both existing live melee PostgreSQL scenarios, through the real transport and
  owner writer; the refusal scenarios retain their existing assertions.
- This task record. Local detached-worktree authoring only; publication belongs
  to the coordinator.

## Excluded scope

Production combat, RNG, clocks, fault placement, SQL, schemas, protocol, fixture
serialization and test assertions. No dependencies, workflow, repository-policy,
vendor, credential or external-repository changes. No Git mutations.

## Validation

All local verification passed. All Cargo builds/tests use the queued absolute
wrapper with `--locked --offline`; formatting uses the installed rustfmt entry
point. Real PostgreSQL qualification uses only this mechanic's library filters.
The requested unfiltered library run removes the PostgreSQL URL in the test
runner so it does not run unrelated PostgreSQL mechanics on the shared PC; that
run is unit/route-less coverage, not PostgreSQL qualification.

Final PostgreSQL command, **eight consecutive invocations on unchanged Rust**:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib pvp_live --quiet
```

| Run | Passed | Failed / ignored | Duration |
| --- | --- | --- | --- |
| 1 | 6 | 0 / 0 | 165.55 s |
| 2 | 6 | 0 / 0 | 164.82 s |
| 3 | 6 | 0 / 0 | 209.31 s |
| 4 | 6 | 0 / 0 | 218.33 s |
| 5 | 6 | 0 / 0 | 166.29 s |
| 6 | 6 | 0 / 0 | 165.68 s |
| 7 | 6 | 0 / 0 | 167.01 s |
| 8 | 6 | 0 / 0 | 166.00 s |

Each invocation executed all five existing PostgreSQL scenarios plus the new
fault-notification unit test: **40 PostgreSQL scenario executions and eight unit
executions passed**. No missing-PostgreSQL-configuration early return was seen.
Before the sequence, the formerly failing bundle scenario also passed alone
with `--lib pvp_live_bundle_melee_spell_death_replay_and_reload` (31.43 s).

Completed checks:

- `/home/mole/oteryn-cache/codex-bin/cargo --config 'target.x86_64-unknown-linux-gnu.runner=["env","-u","OTERYN_TEST_POSTGRES_ADMIN_URL"]' test --locked --offline -p oteryn-gameplay-dispatch --lib --quiet`:
  1,027 passed, zero failed, 14 pre-existing ignored; 65.24 s on the final code.
- `/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`:
  passed. Existing vendored Tokio/SQLx warnings remain; vendor was not edited.
- `/home/mole/.cargo/bin/cargo fmt -p oteryn-gameplay-dispatch` and the same
  command with `-- --check`: passed.
- `python3 tools/agents/validate_governance.py`: passed.
- `python3 -m unittest discover -s tools/agents/tests`: 129 passed.
- `git diff --check`: passed. Read the entire Rust diff and this record against
  `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and the repository review checklist.
  No enum variants, struct fields, trait methods or public functions were added.
  Searched `apps/`, `crates/` and `tools/` for all uses of the modified fault
  API; both fault-taking writer sites and the scenario's two arm sites are
  accounted for. The new driver is private test code.

### Unfinished

- None locally. No CI-only contention failures remain for this task.
- Coordinator publication and repository CI; no commit or push is authorized.
