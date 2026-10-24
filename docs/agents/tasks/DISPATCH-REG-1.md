# DISPATCH-REG-1 — mechanic registration fragments

```yaml
task_id: DISPATCH-REG-1
title: Remove shared mechanic registration edits from dispatch
mode: REFACTOR
status: review
repository: Oteryn/Oteryn-Game
branch: claude/happy-davinci-ijz3wy-dispreg
base_sha: 22106ff6bb6e4da7838d430e9dcd95cecf8ab001
pr: null
owner: standalone Codex worker
owned_paths:
  - crates/gameplay/dispatch/build.rs
  - crates/gameplay/dispatch/build_mechanics.rs
  - crates/gameplay/dispatch/mechanics/**
  - crates/gameplay/dispatch/src/gameplay_transport/mod.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/authority.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/authority/**
  - crates/gameplay/dispatch/src/gameplay_transport/connection/routing.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/routing/{daily_reward,house_move_out,party,staff,world_event}.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/{route,result,outbound,registry_tests}.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/route_table_before.txt
  - crates/gameplay/dispatch/src/gameplay_transport/{appearance_change,capabilities,cyclopedia,daily_reward,guild_intent,house_move_out}.rs
  - docs/agents/tasks/DISPATCH-REG-1.md
```

## Summary

Adding a routed mechanic now adds its Rust files and one small registration fragment.
The 28 fragments generate the 40 existing non-step command registrations, the composed
and authority method macro lists, the mechanic transport mounts, and the three dynamic
capability-offer hooks. The existing build-mount helper discovers routing modules.
There is no central mechanic list to edit. No dependency or manifest changed.

`FreshAdmissionAuthority` and its production impl moved to
`connection/authority.rs` and `connection/authority/composed.rs`. The existing
`connection::FreshAdmissionAuthority` re-export preserves every caller. Authority
macro paths use `$crate` so expansion does not depend on the declaration's depth.
The two mounted modules with child modules retain explicit source-relative paths.

## Reason with evidence

STRUCTURE-TARGET-1's owner-supplied three-way simulation of the last 60 mechanic PRs
reported **31 conflicting pairs / 239** for `gameplay_transport/mod.rs` and
**30 / 114** for `connection.rs`. This refactor targets the remaining registration
insertion sites; no compile-time or runtime speedup is claimed.

Before authoring, `rg` and `git log -p --since=30.days -- <both files>` confirmed the
actual code. This head has eight explicit `module::composed_methods!` calls and
seven `module::authority_methods!` calls; the aggregate macros named in the task did
not yet exist. They now expand those same calls from the fragments.

Per-line collision exposure at the original HEAD is below. For each exact anchor
line, count distinct commits whose changed hunk contains that line (context, added,
or removed); overlapping-hunk pairs are `n * (n - 1) / 2`. This is a reproducible
line-level overlap count, **not** a re-run of the owner's three-way conflict test;
pairs can overlap multiple rows and must not be summed as unique merge conflicts.

| Original source line | Commits reaching the same line | Overlapping-hunk pairs |
|---|---:|---:|
| `mod.rs:1095`, production authority impl opening | 8 | 28 |
| `mod.rs:1130`, `async fn observe_quest_log` after the macro list | 11 | 55 |
| `connection.rs:387`, authority trait opening | 6 | 15 |
| `connection.rs:400`, `fn admit` after the macro list | 8 | 28 |
| `connection.rs:1555`, daily-reward offer hook | 2 | 1 |
| `connection.rs:1556`, final Cyclopedia offer hook | 1 | 0 |

The 30-day patches additionally contain registration-line edits in nine commits for
composed macros (24 added/removed call lines), seven for authority macros (7 lines),
four for offer hooks (4 lines), nineteen for routing predicates/command registration
(70 lines), and five for the four reply-only enum variants (8 lines). Structural
moves are included, so these counts describe edits rather than independent mechanics.
The final Cyclopedia line alone has no overlapping pair, but its shared offer list
already has one at the preceding daily-reward line: the list is extracted as one unit.
No speculative lifecycle-hook framework was added.

## Scope

- Scan `.mechanic` fragments in file-name order, following the REGISTRY-SPLIT-1
  directory-scan/OUT_DIR pattern. A trivial `key=value` header plus a `[route]` Rust
  expression avoids adding a build parser dependency (`serde_json` was only a normal
  dependency). Ordinals preserve the old route order; command order within a fragment
  remains unchanged. Generated Rust is included with `include!`.
- Fail the build on duplicate mechanic names, duplicate command ids (also within a
  fragment), invalid headers, duplicate transport modules, or duplicate offer ranks.
  Emit `cargo:rerun-if-changed` for the directory and every fragment. Reuse
  `apps/game-server/build_mounts.rs` unchanged for routing mounts, preserving their
  previous `pub(super)` visibility in the generated declarations.
- Generate `mechanic_composed_methods!`, `mechanic_authority_methods!`, the dynamic
  offer-filter fan-out and `route_registered`. Weather filtering still precedes
  appearance (10), daily reward (20), and Cyclopedia (30). Every old route body is
  whitespace/trailing-comma equivalent. The old `owns` helpers for guild/house move-out
  were replaced by their exact ids in the registry; the internal guild-bank predicate
  remains in use by its owner.
- Use `Dispatch::Reply(CommandStatus, Vec<u8>)` for party, daily reward, house move-out, staff
  and world-event results. Preserve accepted/rejected status, exact payload bytes, unknown
  outcomes, disconnected/control-lost exits, and command-result encoding. Their old
  variants had no subsequent effects. Daily reward calls its unchanged status/payload
  selector in the adapter. Reply payloads are moved into the encoder.
- Keep specialized variants where their semantics extend beyond a pre-encoded reply:
  guild also carries Cyclopedia's post-command map refresh; VIP/house/appearance retain
  the pre-result disconnect handling; training carries its termination flag;
  highscores/achievements carry registered terminal errors. No speculative conversion of those control paths.

## Excluded scope

New gameplay, protocol/wire/SQL changes, durable authority changes, lifecycle transaction
reordering, dependencies, Soul War, workflows, repository policy/tools, vendor, migrations,
Git writes, publication, paid/external review, Jira and other worktrees. Step pacing and
logout's stream polling remain in their existing private phase; logout is called by its
registry arm. No existing test source, assertion, test mount name or qualification is removed.

The META binding was read (3.1.0 at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`).
This is the owner's local worker route; the driver retains policy/publication control.
No remote policy, credentials or external write was used to expand authority.

## How to add a mechanic now

1. Add the mechanic's owner/adapter Rust files. Optional `transport=<module>` mounts
   `src/gameplay_transport/<module>.rs`; files in `connection/routing/` mount automatically.
2. Add one uniquely named `<ordinal>-<name>.mechanic` file with `name`, `commands`,
   optional `authority`/`composed` module names, optional ordered `offer` hook, and the
   small `[route]` expression. Copy the existing unknown-outcome handling appropriate
   to the owning contract. A reply-only mechanic returns `Dispatch::Reply`.
3. Add its real dispatch tests to the already mounted directories and run its scoped
   checks. Do not append a module, macro invocation, route arm, reply variant or test
   mount to a shared source file. No existing fragment needs renumbering.

See `crates/gameplay/dispatch/mechanics/README.md` for the format and an example.
The baseline route fixture stays immutable: the parity test projects out future mechanic
names before comparing the old routes' exact order and membership. New mechanics test
their own registration through real dispatch.

## Validation

All Cargo checks/tests use `/home/mole/oteryn-cache/codex-bin/cargo` with `--locked`.
PostgreSQL ran through that absolute queued wrapper; no URL was printed, no whole PG
suite was run, and no missing-route early return is counted as qualification.

| Parity/check | Before | After/result |
|---|---|---|
| Dispatch lib test names (`--lib -- --list`) | 1,036 | 1,040; every old name remains, four new registry tests |
| Registered non-step route table | 40 ids / 28 branches | Exact ordered baseline fixture comparison passes |
| Authority trait and production impl | Original declarations/bodies | Equivalent after mount/path and rustfmt normalization |
| Route bodies | 28 hand-written branches | All 28 match after whitespace/trailing-comma normalization |
| Generated output | First build | Determinism test compares all four generated outputs byte-for-byte |
| Registry validation | Duplicate names/ids and malformed headers | Rejection unit tests pass |
| `--lib registry` | — | 7 passed |
| `--lib capabilities::tests` | — | 63 passed |
| `--lib connection::` | — | 89 passed; includes registry and real transport routing tests |
| `--lib house_move_out_postgres_real_dispatch --nocapture` | — | 1 passed, 7.23 s; real wire/native mutation/reload |
| `--lib party_membership_real_dispatch_postgres_end_to_end --nocapture` | — | 1 passed, 7.75 s; real admitted membership commands |
| `--lib world_event_postgres_real_dispatch_reset_restart_and_fences --nocapture` | — | 1 passed, 51.14 s; both reset epochs exercised |
| `--lib daily_reward_real_dispatch_delivers_replays_reloads_and_orders_seven_days --nocapture` | — | 1 passed, 16.69 s; additional targeted check for the final daily-reward Reply conversion; PostgreSQL 18.6 confirmed |
| Clippy `-p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --quiet -- -D warnings` | — | Passed |
| Check `-p oteryn-game-server --all-targets --quiet` | — | Passed |
| `cargo fmt --all --check`, explicit rustfmt of generated-mount sources | — | Passed |
| Governance validator and agent Python tests | — | Passed; 129 Python tests |
| `git diff --check`, both review checklists, workspace consumers | — | Passed |

Existing dependency warnings from vendored Tokio/SQLx appear in logs; neither touched
package has a warning. Rustfmt does not walk generated includes: the mounted transport
sources, changed routing files and registry test file were also checked explicitly.
The ordinary game-server/dispatch library PG CI mounts continue to run every old test.

The full diff was reviewed against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and
`docs/agents/review/CODEX-CHECKLIST.md`. Workspace-wide `rg` over `apps/`, `crates/`,
`tools/` covered all `Dispatch` variants/matches and `Mechanic` fields/constructors.
`Reply` is constructed by production routes and handled by both result and outbound
phases; the four scoped PG tests exercise it through real dispatch. No trait method or public
API was added. The new build generator has a production build-script caller.

## Unfinished

None within local implementation and scoped validation. The owner's full three-way
simulation was supplied evidence, not reproduced here; the table above records the
additional line-level overlap measurement. Publication, external review and repository
CI belong to the driver; no PR or frozen/landed candidate is claimed.
