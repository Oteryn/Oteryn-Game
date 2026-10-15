# BUNDLE-GATES-1

```yaml
task_id: BUNDLE-GATES-1
title: "BUNDLE-GATES-1: audited, test-enforced inventory of the qualified_room gates on a bundle World"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bundle-gates-1-20261014
issue: 1622
governing_issue: 1622
pr: 2115
terminal_pr_policy: archive_pending
base_sha: 75ec7985
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-15
depends_on: ["BUNDLE-COMBAT-ENABLE-1 (#2025)"]
owned_paths:
  - apps/game-server/src/gameplay_transport/bundle_gates_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs   # test module declaration only
  - docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md   # BUNDLE-GATES-1 row only
  - docs/agents/tasks/active/BUNDLE-GATES-1.md
  - docs/agents/tasks/archive/BUNDLE-GATES-1.md
  - Cargo.toml   # syn dev-dependency of the guard test's parser only
  - Cargo.lock   # syn dev-dependency of the guard test's parser only
  - apps/game-server/Cargo.toml   # syn dev-dependency of the guard test's parser only
  - content/world/pins/oteryn.json   # inputs_digest only: the Cargo inputs of the World bundle changed (rustls 0.23.46 precedent)
public_contracts: []
external_repositories: []
```

## Scope

Plan row BUNDLE-GATES-1 (residue of #2025): inventory every `qualified_room` gate of the gameplay seam on main, record which are open and by which merged PR, name the batch that opens each closed one, and add a guard test. No runtime, wire, persistence, migration or registry change.

## Result

Inventory of the gameplay seam's room reads, on main@75ec7985. A `combat_room` read is open on a bundle World; a `qualified_room` read is closed there (`serve.rs` withholds `qualified_room` from a bundle World).

| Gate | State | Opened by / owner |
|---|---|---|
| attack target (`mod.rs`), auto-attack drain (`attack.rs`), native combat cast and commit (`native_combat_cast.rs`), timer continuation (`spell_timer_callbacks.rs`) | open | BUNDLE-COMBAT-ENABLE-1, #2025 |
| monster AI (`monster_ai_cycle.rs`) | open | LOOP-MONSTER-1 child BUNDLE-MONSTER-AI-1, #2036 |
| periodic player damage (`spell_periodic.rs`) | open | MAGIC-COND-1 child COND-1c, #2086 |
| `locate_message` parameter cast (`parameter_cast.rs`) | open | MAGIC-CAST-1, #2112 (the bundle World locate gate) |
| `house_access`, `vertical_move` parameter casts | closed | MAGIC-SUMMON-1 (SPELL-RT-WORLD-1) |
| world item casts (`world_item_cast.rs`): `random_item_grant`; `conjure` | closed | MAGIC-SUMMON-1 (SPELL-RT-WORLD-1); MAGIC-ITEMS-1 (RUNE-CONJ-1) |
| field step ingress (`field_step_ingress.rs`) | closed | LOOP-WORLD-1 (WORLDINT-ADMIT-1) |
| source map initialization (`source_item_cycle.rs`) | closed | MAP-CUTOVER-1c (the bundle Ground persistence) |
| door overlay, step blocking, USE (`mod.rs`) | not a gate | a source-World branch; a bundle World is served by `bundle_map` |

`bundle_gates_tests.rs` enforces the table. It fails in each of these cases:
- a read is added, removed or switched;
- an open gate lacks its batch and merged PR;
- a closed gate has no owner, or an owner that is not a plan batch;
- the plan row stops naming an opener or owner;
- `serve.rs` stops withholding `qualified_room`;
- `parameter_room` opens another key.

The plan row BUNDLE-GATES-1 carries the same inventory and stays open until no gate is closed.

## Notes

- `conjure` goes to MAGIC-ITEMS-1, not MAGIC-CAST-1: that row owns "rune use with target and conjure".
- Source map initialization goes to MAP-CUTOVER-1c, because on a bundle World it is that row's Ground persistence.
- The active record `BUNDLE-MONSTER-AI-1.md` is stale: its PR #2036 merged. It is outside this task's paths.

## Validation

- cargo fmt --all --check: pass
- git diff --check: pass
- cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings: pass
- cargo test --locked -p oteryn-game-server --lib: 3036 passed, 0 failed, 29 ignored
- negative check (world_item_cast.rs switched to combat_room): `every_seam_room_read_is_inventoried` fails as intended
- negative check (plan row loses `auto-attack drain`): `every_gate_is_recorded_in_the_plan_row` fails as intended
- negative check (a comment and string literals naming `self.combat_room` added): guard still passes
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: OK
- review: Codex round 1 (two P2: count only code reads; tie each plan assertion to its gate) fixed in one push; Codex round 2 (one P2: count every receiver and form, not six spellings) fixed in one push, negative-checked with `(*x).combat_room`, a split `admission\n.qualified_room` and a destructuring pattern; Codex round 3 (two P2: walk nested modules; filter comments and strings in the `serve.rs` and `parameter_room` assertions) fixed in one push, negative-checked with a nested `mod.rs` read, a swapped `serve.rs` initializer and a swapped `parameter_room` key each kept as a comment; Codex round 4 (two P2: leave test code out of the scan; key each read by its enclosing function) fixed in one push, negative-checked with a read moved to a helper of the same file and receiver (fails), an inline `#[cfg(test)]` module and function read (passes) and a production read beside them (fails); Codex round 5 (two P2: key each read by its own site; leave every `#[cfg(test)]` item out, whatever its kind) fixed at the root in one push: the scan parses each source with `syn` (a test-only dev-dependency already in the lockfile), drops every `#[cfg(test)]` item, statement, expression, field and arm at any depth, and keys each read by its syntax path and its position in its function, with a unit test for each case, negative-checked with a gate statement replaced by another read of the same form in the same function (fails), a `#[cfg(test)]` `impl` and `const` read (passes) and a production read beside them (fails); Codex round 6 (three P2: follow `#[path]` modules outside the directory; skip `#[cfg(test)]` enum variants; name the enclosing operation in each site) fixed in one push: the sources are the files the compiler loads from `mod.rs` through production `mod` declarations and their `#[path]` (so `interaction/corpse_open.rs` is scanned), test-only variants are skipped, and each call, method and macro on a site's path carries its name, with a unit test for each case; the World bundle pin `inputs_digest` that the `syn` dev-dependency changed is refreshed in 7b669c5f (rustls 0.23.46 precedent; control-plane approved); Codex round 7 (three P2: name each method with its impl; take a bare `combat_room` pattern as a read of the open-gate field; recognize compound test-only cfg predicates) fixed in one push with the `main` merge: a method is keyed `Type::f`, `<Type as Trait>::f` or `Trait::f`, a gate's field is judged by its name in any form and a `Declaration` must be a declaration or initializer, and a cfg predicate is test-only when `test` false makes it false (`test`, `all(.., test, ..)`, `any` of only such terms), with a unit test for each case; Codex round 8 (three P2: key each match arm by its branch; leave test code inside a macro body out; follow a `cfg_attr`-selected module path) fixed in one push: each production arm is keyed by its position and pattern name, a macro body is parsed as expressions, statements or items and visited cfg-aware (a body that parses as none is walked token by token, each `#[cfg(test)]` attribute dropping the tokens it governs), and a `cfg_attr` path is taken when its predicate holds without `test` (a predicate naming another key fails the guard), with a unit test for each case; Codex round 9 (three P2: require the not-a-gate marker in the source-World clause; name a function with its inline module; check the `GameplaySeamOwners` initializer and `parameter_room` themselves, not their text anywhere in the file) fixed in one push with the `main` merge: the source-World clause must say the branches are not gates and name `bundle_map` (negative-checked by renaming them closed gates in the plan row: fails), a function in an inline module is keyed `m::f`, and the `serve.rs` and `parameter_room` checks parse the production code and judge every `GameplaySeamOwners` initializer's `qualified_room` value and `parameter_room`'s sole match, with a unit test for each case; Codex round 10 (two P2: tell sibling control-flow nodes apart; end a test-only macro entry at its comma) fixed in the same push: an `if`, `while` or `for` names the identifiers of its condition (or pattern and source) and an `if`'s branches are `then` and `else`, with the inventory regenerated from these keys, and the cfg skip in a macro token walk stops at a `;`, `,` or braced group, with a unit test for each case; Codex round 11 (three P2: skip test-only variants in the production visitor; keep generic-argument commas inside a test-only macro entry; key an impl by its full self-type and trait paths) fixed in one push: the `GameplaySeamOwners` visitor skips `#[cfg(test)]` variants, the macro cfg skip keeps commas inside `::<..>` and `<T as ..>` arguments (falling back to the first comma when they never balance), and impls are keyed by full paths (`super::super::ComposedFreshAdmission::cast_*` in the inventory), with a unit test for each case; Codex round 12 (one P2: tie a seam field hidden in a `macro_rules!` definition to its invocations) fixed in one push: a seam field named inside a `macro_rules!` definition fails the guard, as no inventoried site could stand for each invocation, with a unit test, negative-checked with a `macro_rules!` reading `$o.combat_room` in `world_item_cast.rs` (fails); Codex round 13 (two P2: follow or refuse sources loaded by `include!`; keep generic arguments in impl identities) fixed in one push: a production `include!` fails the guard (negative-checked with an `include!` in `world_item_cast.rs`: fails), and an impl is keyed by its self type and trait as written, with generic type and const arguments, references and pointers (`Owner<u8>` apart from `Owner<u16>`), with a unit test for each case; at the control plane's request the resolver then fails closed at the root: any two production functions of one file that resolve to one name (whatever the cause: an impl form the identity does not render, `cfg` alternatives, nested functions) fail the guard, with a unit test, negative-checked with two `cfg`-alternative `zz_probe` functions in `world_item_cast.rs` (fails), and a function declared inside another is named with it; Codex round 14 (two P2: keep `cfg` alternatives apart; tell repeated sibling nodes apart) fixed in the same push: `cfg` alternatives of one function are refused by that root check, and a syntax node that repeats an earlier sibling's form (a second `if armed`, a later `let`) is numbered by its position among them, so a read moved between them changes its site, with a unit test for each case; main merged, with the chase step of LOOP-COMBAT-MODES-1 #2113 inventoried (its `combat_room` read open, its `qualified_room` door blocking a source-World branch); Codex round 15 (two P2: tell arguments and operands of one operation apart; refuse a file shared by two module declarations) fixed in one push: every node after its parent's first child is numbered by its overall position, so a read moved to another argument, operand, statement or branch changes its site, with unit tests for call arguments and `&&` operands, and a file loaded by two module declarations fails closed, negative-checked with a second `#[path]` declaration of `attack.rs`; Codex round 16 (three P2: number file-scope items; account for macros defined outside the scan; count a shorthand initializer's read) fixed in one push: the file root counts its items like any node, a shorthand `S { field }` records its initializer and the read of its binding (and a `{field}` format capture is a read), and a macro that is neither upstream (a short named list) nor defined in a scanned module fails closed, negative-checked with a `gate!(owner)` probe in `attack.rs`; Codex round 17 (three P2: read captures only from format strings; evaluate nested negation in test-only cfgs; verify trusted macro names) fixed in one push: a capture counts only in a formatting macro's format-string argument (or any literal of a macro that may forward one), a cfg predicate is evaluated in three-valued logic with `test` false (`not(not(test))` is test-only), and no macro outside the scanned modules may answer to an allowlisted or scanned name (`macro_rules!`, `#[proc_macro]` or `as` rename, in the game server and its path-dependency crates), with a unit test for each case, negative-checked with a `macro_rules! json` in `crates/foundation` (fails); Codex auto-review on the new head
