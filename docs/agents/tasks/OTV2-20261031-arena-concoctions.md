# OTV2-20261031-arena-concoctions — local takeover qualification

```yaml
task_id: OTV2-20261031-arena-concoctions
title: W17 Svargrond and W09 concoctions / Drome Cube
mode: IMPLEMENT
status: PARTIAL
repository: Oteryn/Oteryn-Game
pr: 2763
base_sha: 38641fecd4558a4a14e6b15b3874110ffa2dff19
head_sha: 092aa601d7ab8b1623d8a27de37eba90c508cbff
branch: null
frozen_sha: null
updated_at: 2026-11-01
owned_paths:
  - apps/game-server/migrations/0637_drome_cube.sql
  - apps/game-server/migrations/0638_concoctions.sql
  - crates/gameplay/dispatch/src/gameplay_transport/drome_cube_use*
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/drome_cube*
  - crates/gameplay/durability/src/durability/item_use/drome_cube*
  - crates/gameplay/durability/src/durability/item_use/concoction*
  - crates/gameplay/items/src/concoctions.rs
  - crates/gameplay/items/src/drome_cube.rs
  - crates/protocol-oteryn/src/arena*
  - tools/content-migration/drome_item_profiles.py
  - tools/content-migration/test_drome_item_profiles.py
  - tools/content-schema/native-gameplay/drome-item-profiles.json
  - docs/agents/tasks/OTV2-20261031-arena-concoctions.md
```

This is an AUTHORING worktree, detached at the supplied PR head with main already
merged into its index. No freeze, commit, staging, push, branch operation, remote
write, paid review or merge was performed. GitHub CI/review has not qualified this
uncommitted tree. The owner retains publication authority.

## Recovery and merge resolution

The three supplied conflicts were `register_spell_families.py`,
`content/items/definitions/spell-native-profiles.json` and
`content/spells.manifest.json`. The source registration retains Drome plus main's
enchanting and Task Board admissions. The derived files were regenerated, never
merged by selecting individual generated rows. A full record comparison against
main proves all 2,210 existing native Item profiles unchanged, plus exactly 21
Drome profiles. Conflict markers are absent; unmerged index entries remain for
the owner to stage under the explicit no-Git-mutation instruction.

Main's capacity calculation now requires an Item weight lookup. The Cube passes
its existing native content lookup to that calculation, retaining main's capacity
semantics. All changed Rust files, including explicitly mounted children, were
formatted with the pinned toolchain.

Recovered the unpublished Kooldown candidate
`85c594afb365080a759e6cf06db6205887e63f86` through read-only GitHub commit/compare/raw
APIs. Its sole parent is the supplied `092aa601…`; it adds nine paths/deltas and
was not available in the local object database. A three-way file merge retained
main and the local capacity repair. Fixed the recovered writer's three Type 2
transaction dereferences and its test fixture's audited dynamic SQL call.

Kooldown uses canonical `item_use::write`, the existing activation table, a
per-kind 24-hour cooldown, the retained source reservation, exact wire receipt,
and current fences before publishing its one-time runtime reset. It expires spell
and group deadlines and preserves movement pacification. Its PostgreSQL scenario
uses the real connection/USE path, not an alternate transaction owner.

## Migrations

No renumbering was necessary. `0637_drome_cube.sql` and recovered
`0638_concoctions.sql` have no prefix collision with supplied main or held numbers
0596–0597, 0690–0691, 0704–0707, 0719–0722. The duplicate-migration script emitted
nothing; an independent working-tree prefix check found zero duplicates. No
merged migration was changed. The new trigger function revokes PUBLIC access;
existing runtime-only mutation grants on the activation table remain sufficient.

## Generators and checks

- `bash tools/merge-driver/regen.sh --no-stage`: refused because Git's index still
  contains unmerged entries. The normal wrapper would stage/check out files,
  which this packet explicitly prohibits.
- `python3 tools/content-migration/register_spell_families.py`: passed.
- `python3 tools/content-migration/world_project_v2_to_tree.py`: passed.
- `python3 tools/content-migration/regenerate_content.py --skip-checks`: passed;
  this runs the wrapper's materialization/registration steps without Git mutation.
  Temporary predecessor/materializer files were confined to this worktree.
- `python3 tools/contracts/registry_fragments.py`: passed.
- `python3 -m unittest discover -s tools/content-migration -p test_drome_item_profiles.py`:
  15 passed, 0 failed.
- `python3 tools/content-migration/regenerate_content.py --check`: passed its
  registration, World-tree, materialized-content, reference, and authoring checks,
  then stopped because the unchanged historical Item test creates temporary
  fixtures in `root.parent`, outside the allowed worktree. The same test was run
  through a temporary runner redirecting only `TemporaryDirectory` into this
  worktree: 599 checks passed (including its four unit tests). No assertion or
  checked input was changed.
- `python3 tools/content-schema/item-authoring/item_weapon_proficiency.py --check`:
  passed, up to date.
- `python3 tools/content-census/g4_item_crystal_binding_generator.py --check`:
  passed, 33,971 bindings.

All commands below used `/home/mole/oteryn-cache/codex-bin/cargo`, with
`CARGO_NET_OFFLINE=true`; no dependencies were downloaded.

| Exact arguments after the absolute cargo path | Final result |
|---|---|
| `fmt --all -- --check` | passed (also explicit rustfmt checks for all 26 changed Rust paths, including mounted children) |
| `check --locked --workspace --all-targets --all-features` | passed; initial merge/recovered-candidate compile errors described above were fixed |
| `clippy --locked -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability -p oteryn-gameplay-items -p oteryn-protocol-oteryn -p oteryn-client-ui -p oteryn-game-server --all-targets --all-features --quiet -- -D warnings` | passed, six packages |
| `test --locked -p oteryn-gameplay-items --lib --quiet -- drome_cube concoctions` | 4 passed, 0 failed |
| `test --locked -p oteryn-gameplay-durability --lib --quiet -- drome_cube` | 9 passed, 0 failed |
| `test --locked -p oteryn-protocol-oteryn --lib --quiet -- arena` | 6 passed, 0 failed |
| `test --locked -p oteryn-gameplay-dispatch --all-features --lib --quiet -- drome_cube w22c_ kooldown --test-threads=1 --nocapture` | nominal 14 passed, 0 failed; seven PostgreSQL cases returned early, **not PG qualification**; seven actual unit/content tests passed |
| `test --locked -p oteryn-gameplay-dispatch --all-features --lib --quiet -- kooldown_aid_lost_ack --test-threads=1 --nocapture` | absolute-wrapper rerun: one nominal pass, but PostgreSQL returned early again; **not qualified** |
| `test --locked -p oteryn-game-server --test content_world_project_repository --quiet` | 5 passed, 0 failed; regenerated World byte/digest inventory |
| `run --locked -p oteryn-architecture-check --quiet -- workspace .` | passed, workspace-boundaries PASS and client registrations checked |
| `run --locked -p oteryn-world-bundle-compiler --quiet -- pin-refresh .` | blocked, exit 1: unmerged native-profile index entry |
| `run --locked -p oteryn-world-bundle-compiler --quiet -- pin-check . .arena-temp/pin-check` | blocked, exit 1: same unmerged index entry; initial invocation without output directory gave usage/exit 2 |

`python3 tools/agents/validate_governance.py` passed (22 policy documents,
nine project lanes); `python3 -m unittest discover -s tools/agents/tests` passed
129 tests, zero failures. The generated registration/World checks also ran 87
Python unit tests with zero failures before the historical fixture-directory
restriction. `git diff --check` passed.

The absolute cargo wrapper's current implementation contains no export of
`OTERYN_TEST_POSTGRES_ADMIN_URL`, despite its header describing that feature.
No URL or credentials were read or printed. The owner was asked to restore the
wrapper's PostgreSQL routing; editing it is prohibited by the current-directory
boundary. Seven skipped database paths are Cube full/retry/restart, Cube consumed
handle replay, Cube existing stack, Cube concurrent last unit, Kooldown lost-ACK/
cooldown, and the two `w22c_` Wealth Duplex kill cases. No PG completion markers
were emitted. CI-only or previous-head passes are not substituted for this tree.

## Whole-diff review

Read the full branch delta against supplied main and the recovered nine-file
candidate, with `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`. Searched constructor,
match and implementation sites under `apps/`, `crates/` and `tools/` for the new
Cube kinds, receipt/errors, retained request field, Kooldown case and arena wire
types. Cube/Kooldown production callers and real-dispatch PostgreSQL coverage
exist. No tests were deleted, skipped or weakened. Main's native profile rows,
capacity semantics, registered mechanics and restricted CI paths are preserved.

The inherited arena match wire API and several Concoction conversion helpers
still have only test callers. This is a completeness finding, not an approved
exception or evidence of a playable arena. The draft must not be marked ready
on the strength of Cube/Kooldown qualification alone.

## Unfinished scope

- Stamina Extension has explicit narrow shared-owner approval in PR comments
  6556150903 and 6557625165, but its authored code was not saved in the supplied
  head, local files or the recovered Kooldown object. The approved canonical
  +60/cap2520 build transaction, receipt, sequencer integration, flush/reconcile,
  projection and real-dispatch tests remain absent.
- W17 has catalogue and match wire definitions but no command-436 runtime,
  durable match/rewards, real combat/death/emergency-exit qualification or native
  client implementation. The client registration remains explicitly unsupported.
- Remaining W09 timed effects/online accounting, including the approved legacy
  Wealth Duplex online-budget conversion, remain absent. Existing Wealth Duplex
  still uses its original wall-clock implementation.
- The original `PROMPT_CHATGPT_5_arena.md` and a contemporaneous task card were
  not present. Current scope was read from PR description/latest owner comments;
  historical ARENA-1 is not treated as current completion evidence.
- PostgreSQL qualification is blocked by the supplied absolute wrapper lacking its
  PG environment export. Pending owner action: restore the wrapper routing; rerun
  the filtered dispatch command above. No database success is claimed.
- Pin refresh/check cannot consume this unmerged index without staging its
  resolved entries. The owner prohibited staging; no pin was hand-edited.
- No current-tree CI, independent review, game-gate or merge-readiness claim.
  No changes to the prohibited re-audit ledger or CI/repository-policy paths.

## Complete changed-file list against supplied main

These 45 paths include the inherited PR delta and recovered candidate;
the incoming main merge is excluded.

- `apps/game-server/migrations/0637_drome_cube.sql`
- `apps/game-server/migrations/0638_concoctions.sql`
- `content/content.lock.json`
- `content/items/definitions/spell-native-profiles.json`
- `content/items/index.json`
- `content/manifest.json`
- `content/spells.manifest.json`
- `crates/client-ui/src/features/0170-arena/fixtures/registrations.json`
- `crates/client-ui/src/features/0170-arena/registrations/handlers.json`
- `crates/gameplay/dispatch/src/gameplay_transport/arena_catalogue.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/use_item.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/use_effects.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/drome_cube_use.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/drome_cube_use/gate.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/drome_cube_use/kooldown_aid.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/drome_cube_use/ports.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/item_use.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/drome_cube.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/drome_cube/cases.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/drome_cube/fixture.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/drome_cube/kooldown_aid.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/drome_cube/wire.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/drome_cube_profiles.rs`
- `crates/gameplay/durability/src/durability/item_use.rs`
- `crates/gameplay/durability/src/durability/item_use/concoction.rs`
- `crates/gameplay/durability/src/durability/item_use/concoction/kooldown_aid.rs`
- `crates/gameplay/durability/src/durability/item_use/drome_cube.rs`
- `crates/gameplay/durability/src/durability/item_use/drome_cube/plan.rs`
- `crates/gameplay/durability/src/durability/item_use/drome_cube/tests.rs`
- `crates/gameplay/durability/src/durability/item_use_audit.rs`
- `crates/gameplay/items/src/concoctions.rs`
- `crates/gameplay/items/src/drome_cube.rs`
- `crates/gameplay/items/src/lib.rs`
- `crates/protocol-oteryn/capabilities/0170-arena-v1.json`
- `crates/protocol-oteryn/src/arena.rs`
- `crates/protocol-oteryn/src/arena/match_wire.rs`
- `crates/protocol-oteryn/src/tests/capabilities_golden/0170-arena-v1.json`
- `docs/agents/tasks/OTV2-20261031-arena-concoctions.md`
- `docs/contracts/protocol-oteryn/v1/arena-v1.md`
- `docs/contracts/protocol-oteryn/v1/registry/capabilities/0170-arena-v1.json`
- `docs/contracts/protocol-oteryn/v1/registry/command_types/0436-arena-match.json`
- `tools/content-migration/drome_item_profiles.py`
- `tools/content-migration/register_spell_families.py`
- `tools/content-migration/test_drome_item_profiles.py`
- `tools/content-schema/native-gameplay/drome-item-profiles.json`

## Next step and pending owner action

Restore PostgreSQL routing in the supplied absolute cargo wrapper, then rerun
`cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib --quiet -- drome_cube w22c_ kooldown --test-threads=1 --nocapture`
using that absolute path. Require real database completion (five Cube/Kooldown
markers plus the two Wealth Duplex scenarios); early returns do not count.
After reviewing/staging the three resolved merge entries, rerun the recorded
pin-refresh and pin-check commands. These actions alone do not complete the
missing W17/W09 implementation listed above. Keep the PR draft and AUTHORING.
