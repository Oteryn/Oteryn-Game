# OTV2-20261003-charm-desc-fence1

```yaml
task_id: OTV2-20261003-charm-desc-fence1
title: "CHARM-DESC-FENCE-1: D295 hard wiring gate test"
mode: WORK
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charm-desc-fence1-20261003
pr: 1652
base_sha: ac6fdca8
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: implementation worker under control plane session_013KJX6mv8LQveCKKXYgAX94 (#1622)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ability/charm_desc_fence_gate_tests.rs
  - apps/game-server/src/ability/mod.rs
  - docs/agents/tasks/archive/OTV2-20261003-charm-desc-fence1.md
public_contracts: []
depends_on:
  - "PR #1638 (D295), merged"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implements D295 §3 item 3 (`docs/architecture/reviews/OTERYN_GAME_CHARM_DESC_FENCE_DECISION_2026-10-03.md`):
a `game-server` test scans `src/` and fails on any non-test production reference to
`commit_exact_owner_damage`, `commit_exact_owner_primary_damage` or
`commit_exact_owner_charm_damage`. Exempt: the three canonical bridge definitions and bodies in
`src/ability/commit.rs` only (exactly one each), comments, string and char literals,
`#[cfg(test)]` outer attributes on an item or `let` statement only (never on elements, fields,
variants, arms or expressions, and never as macro tokens), files whose
leading inner attributes include `#![cfg(test)]`, and `*_tests.rs`/`tests.rs` files only when
every declaration that may load them (`mod`, `path`, `include!`, found by name anywhere) is
`#[cfg(test)]`-gated or sits in an already-skipped file. A production `path`/`include!` is itself
a finding unless every file it may resolve to (by exact normalised path from the declaring file,
including inline-block and `mod.rs`/non-`mod.rs` readings) is a scanned file under `src/`;
unresolvable loads (macro-built names, non-literal, escaped or `cfg_attr` paths) count as loading
any file and, when explicit and in production, are findings.
`#[cfg(not(test))]`, `#[allow(dead_code)]` and same-named wrappers elsewhere are not exemptions.
The failure message names D295 and A2. Only the A2 PR, with the fence, may relax it.

No production change: one new test module and its `#[cfg(test)]` mod line.

## Acceptance criteria

- [x] Gate test passes on `main` (no production reference exists).
- [x] Negative self-checks on synthetic sources: production references flagged; comment, string,
  test, and bridge-body references ignored.
- [x] Manual mutation: a probe reference appended to `src/world_runtime.rs` fails the gate with the
  D295/A2 message; reverted.
- [ ] Independent exact-head review (combat).
- [ ] Protected Merge Queue integration.

## Validation

- `cargo test -p oteryn-game-server`: all suites pass, 0 failures.
- `cargo fmt --all --check`: clean.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: clean.

## Self-review

- Whole-diff reread. The scanner is lexical, not a parser: it fails closed (a reference it cannot
  classify is reported), and the exemptions are exactly D295's list.

## Review rounds

- Codex on ce9a9458: P1 4173638056 (`#[cfg(test)]` as macro tokens hid a call) accepted, fixed:
  attributes count only in item/statement position outside macro token trees. P1 4173638052
  (name-only definition exemption hid a same-named wrapper) accepted, fixed: exemption only for the
  single canonical definition per bridge in `src/ability/commit.rs`. P2 4173638060
  (`#![cfg(test)]` after other inner attributes) fixed: the whole leading inner-attribute sequence
  is inspected. Each has a synthetic regression case; a real-tree wrapper probe in
  `src/world_runtime.rs` fails the gate (reverted).
- Codex on 2ec180cb: P1 4173705652 (a preceding `,`/`{` let the exemption blank past an attributed
  array element, field or arm) accepted, fixed: the exemption applies only when the attributed node
  is an item or `let` statement, whose extent the scan bounds; any other node exempts nothing.
  Sweep: struct-literal field, match arm and expression-statement cases added.
- Codex on 07bf151b: P1 4173759090 (a Unicode macro name such as `μ!{..}` escaped macro detection,
  so its `#[cfg(test)]` tokens exempted a call) accepted. Beyond the D317 two-round limit; the owner
  approved one more round via the control plane. Fixed at the root: any `!` followed by an optional
  identifier and a `(`/`[`/`{` group is a macro token tree, whatever precedes it, except an inner
  attribute's `#!`. Over-matching (unary `!(..)`, `if !x {..}`) only withholds exemptions.
  Sweep: identifier boundaries now treat non-ASCII bytes as identifier bytes throughout.
- Codex on ac803b36: P1 4173866055 (a test-named file compiled into production by an unconditional
  `mod` was skipped by name) accepted; the owner approved one more round via the control plane.
  Fixed: a test-named file is skipped only when all declarations that may load it are test-only,
  transitively; undeclared files stay skipped. Adversarial sweep of the family (module resolution,
  `#[path]`, `include!`, nested `mod` blocks, macro-built declarations, `cfg_attr` paths, escaped
  literals, loads outside `src/`), each closed fail-closed with a regression case.
- Codex on bb3abda9: P1 4173956057 (the outside-`src/` check compared basenames, so
  `../generated/commit.rs` collided with `src/ability/commit.rs`) accepted; owner decision D345
  approved one more round. Fixed at the root: every production explicit load resolves against the
  declaring file (normalised `.`/`..`, absolute and above-root paths refused, inline `mod` blocks
  with both `mod.rs` and stem readings) and must equal a scanned file exactly; an unresolvable
  explicit load is itself a finding. Regression cases for each.

## PR and closeout

- Record archived in the final authoring commit; it reaches `main` only if the PR merges.
