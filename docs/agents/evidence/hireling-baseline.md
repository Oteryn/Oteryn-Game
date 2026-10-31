# E12 baseline regression evidence

Starting base: `ae4188a9c8d461acb152044814ea23075de02ed4`.
Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
Task branch: `chatgpt/hireling`; draft PR: https://github.com/Oteryn/Oteryn-Game/pull/2783.

The companion `hireling-baseline.patch` adds the original regression fixture to
the existing mounted PostgreSQL test directory on that exact base. It is kept as
a patch because the completed owner replaces the old `commit_house_hireling`
API. The production branch carries current lifecycle, wire, bank and Store
regressions in the same mounted directory.

## Environment and commands

PostgreSQL18.6, isolated local test cluster, port56583. Tests were authored and
first compiled in a detached frozen worktree before production source was
changed in the task worktree. The fixture uses the existing safety-checked
temporary-database harness and real House handoff.

```sh
OTERYN_TAG=2783-baseline bash /home/mole/oteryn-cache/slot-run.sh \
  cargo check --locked -p oteryn-gameplay-dispatch --all-features --tests --message-format short

OTERYN_TEST_POSTGRES_ADMIN_URL=postgresql://oteryn_test_admin:@127.0.0.1:56583/postgres \
OTERYN_REQUIRE_PG=1 RUST_MIN_STACK=67108864 OTERYN_TAG=2783-baseline \
  bash /home/mole/oteryn-cache/slot-run.sh cargo test --locked \
  -p oteryn-gameplay-dispatch --all-features --lib hireling_baseline -- --nocapture
```

`cargo check` passed. The complete baseline run returned exit101: **1 passed,
7 failed, 0 ignored**. These failures are intentional regression evidence, not
a passing implementation result. Initial fixture-only failures (URL format and
capability bootstrap declarations) were corrected without editing frozen
production code, then the complete suite was rerun.

| Baseline case | Actual result |
|---|---|
| Native summon and original-command replay | PASS; one placement and one receipt; completion marker printed |
| Native Query with no mutation shape | `InvalidStoredState` |
| Native Dismiss result and receipt | Deleted row, but returned old coordinates and persisted `active=true` |
| Wire native-floor summon | Rejected instead of accepted active reply |
| Wire Query | Rejected instead of accepted inactive reply |
| Wire Dismiss | Rejected instead of accepted inactive reply |
| Bank conversation lookup unit | Greeting accepted dynamic key, but current Bank context did not resolve it |
| Wire House bank deposit/withdraw/replay | Rejected at `hi`; no successful money movement claimed |

The baseline is frozen and is not weakened to make these results pass. Current
tests assert repaired behavior, current named-House authority, native item and
money records, and once-only replay after replacing the transport owner.
