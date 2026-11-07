# COUNT-SLICE-1 — insert-only registration inventories

```yaml
task_id: COUNT-SLICE-1
mode: IMPLEMENT
status: local_validation_complete_with_baseline_failures
repository: Oteryn/Oteryn-Game
pr: null
publication: uncommitted (owner instruction)
```

## Summary

Keep registration changes local to individual entries without changing behavior.
This is an owner-authorized local edit; no Git publication or lifecycle transition.
Bound META policy 3.1.0 was read at
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

## Reason

STRUCTURE-TARGET-1 identified shared-line modifications that the additive merge
driver cannot resolve: sequencer registration lengths and a pipe-joined allowlist
were touched by 10 task IDs in 30 days; the Character receipt SQL inventory by
16 task IDs; and the full-manifest item count by TUTORIAL-1 and ITEM-TIMED-1 on
the same lines. RUST_KEY33_SITES also requires a shared array-length edit.
The measured benefit is fewer merge conflicts between parallel lanes.

## Scope

- `crates/gameplay/durability/src/durability/character_revision_sequencer/writer_registration_tests.rs`:
  sorted writer and fee-source slices, retaining bypass checks.
- `apps/game-server/tests/integration_rest/spell_lane_key33_pin.rs`:
  sorted RUST_KEY33_SITES slice, retaining inventory and lane-proof checks.
- `crates/gameplay/durability/src/durability/character_authority.rs`:
  sorted receipt-source inventory and one lazily constructed integrity query.
- `crates/gameplay/content/src/content/native_gameplay.rs`:
  exact item-policy census against the pinned input profiles, reused in JSON proof.
- This card; local validation evidence under `target/COUNT-SLICE-1/`.

The sequencer registrations and key33 test already moved from the locations
named in the audit; their current locations above are the owned paths.
Receipt UNION ALL order is immaterial because the query only tests whether an
invalid row exists; revision joins and aggregates determine chain validity.

## Excluded scope

No changes to gameplay behavior, schema, fencing, public APIs, dependencies,
workflow/policy files or unrelated registrations. No new enum variants, struct
fields, trait methods or public functions. Existing ignored qualification stays
ignored. No commit, push, external review trigger or merge.

## Validation

- Package rustfmt and explicit formatting/checks of the included durability files:
  PASS. The two tuple inventories intentionally preserve one entry per line with
  `#[rustfmt::skip]`; all other touched code follows rustfmt.
- All-target Clippy `--locked -- -D warnings` for durability, content and
  game-server: PASS. Only existing vendored Tokio/SQLx dependency warnings remain.
- Durability sequencer units: **12 passed**; authority inventory unit: **1 passed**.
- Content library: **345 passed, 1 existing ignored** full-manifest test.
- Key33 integration module: **7 passed, 2 pre-existing failures**, detailed below.
- Filtered Character authority PostgreSQL tests: **3 passed** through the absolute
  wrapper. Each ran one test; no unconfigured-URL/skip marker appeared.
- Governance validation: **22 policy documents / 9 lanes**, PASS; governance unit
  tests: **129 passed**. `git diff --check`: PASS.
- Whole-diff review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`: PASS for
  this refactor. Workspace `rg` covered every new inventory and consumer. No new
  enum variants, struct fields, trait methods or public functions require updates.
- Equivalence checks preserved all **14 sequenced writer registrations**, **4 fee
  permissions**, **13 key33 sites** and **12 receipt sources**. Every receipt
  projection is unchanged and the integrity query tail is byte-identical. Only
  UNION ALL arm order and insignificant whitespace changed.

Exact Rust qualification commands (formatting also explicitly checked each touched
file with `rustfmt --edition 2024 --config skip_children=true --check`):

```sh
/home/mole/.cargo/bin/cargo fmt -p oteryn-gameplay-durability -p oteryn-gameplay-content -p oteryn-game-server
/home/mole/.cargo/bin/cargo fmt -p oteryn-gameplay-durability -p oteryn-gameplay-content -p oteryn-game-server -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-gameplay-durability -p oteryn-gameplay-content -p oteryn-game-server --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib --quiet -- character_revision_sequencer::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib --quiet -- character_authority::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-content --lib --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test integration_rest --quiet -- spell_lane_key33_pin::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test character_authority_postgres --quiet -- bootstrap_is_atomic_and_audit_is_published_held_and_expired --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test character_authority_postgres --quiet -- character_progression_postgres_cases::exact_replay_reconcile_and_restart_readback_are_durable --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test character_authority_postgres --quiet -- bestiary_progress_postgres_cases::restart_reopens_character_authority_and_reads_back_bestiary_progress --exact --nocapture
```

Detailed logs, command results and equivalence evidence are in the ignored
`target/COUNT-SLICE-1/` directory. Initial compilation required passing the lazy
query's static `&str` to SQLx; the final Clippy and tests compile successfully.

## Unfinished

Two existing key33 checks fail against the unchanged
`spell_item_transaction.rs::apply_physical_item_line`:

- `spell_lane_key33_pin::lane_proof_authorities_are_minted_only_behind_a_permit`:
  the function mints `SpellItemScopeAuthority` outside the pinned minter set.
- `spell_lane_key33_pin::rust_key33_writers_are_pinned_and_hold_a_lane_proof`:
  the function writes a key33 table without a recognized lane-proof parameter.

Both failures reproduce with identical diagnostics using the original `HEAD`
test code in a dependency-free, standalone baseline fixture under
`target/COUNT-SLICE-1/key33-baseline/`. The sole fixture adaptation supplies the
original game-server source locator. That run passed 6 original tests and failed
the same 2; this refactor adds the seventh passing inventory-order test. Command:

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked --manifest-path target/COUNT-SLICE-1/key33-baseline/Cargo.toml --test key33 --quiet
```

These are deterministic baseline failures, not load contention. The checks and
their requirements remain intact; repairing that separate production proof path
is outside this behavior-identical refactor. The existing ignored
`actual_full_manifest_qualifies_source_world_and_all_owner_profiles` requires
`OTERYN_FULL_SPELL_TEST_MANIFEST`; it was compiled but not executed. Full
unfiltered PostgreSQL suites remain CI-only under the owner's shared-PC scope
rule. No Git publication was performed.
