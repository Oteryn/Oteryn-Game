# CAST-SPLIT-1 — separate player state, cast gates and owner preparation

```yaml
task_id: CAST-SPLIT-1
mode: IMPLEMENT
status: local_validated
pr: null
repository: Oteryn/Oteryn-Game
publication: uncommitted (owner instruction)
```

## Summary

Behaviour-identical relocation of the existing cast implementation into three
private per-mechanic modules. Existing public names remain available through
`spell::cast` re-exports. No function, field, enum variant or trait method is
introduced, and the existing test modules retain their names and paths.

Owner-authorized local work only; no Git mutation or publication. The bound
META organization policy 3.1.0 was read at immutable commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

## Reason

STRUCTURE-TARGET-1 supplied concrete merge-conflict evidence: the 1,788-line
`cast.rs` received 26 commits across 18 tasks in 30 days. ITEM-USE-1c
(`385c01b26`) and CHAR-VITALS-1 (`a53e16480`) inserted at the same line within
`impl PlayerSpellState`; REGEN-FOOD-1 also added fields and changed `new()`.
State transitions and the owner preparation pipelines therefore shared one
hotspot. Separating their ownership follows the existing
`cast/vocationless_vitals.rs` convention and allows those lanes to edit distinct
files. This is an ownership/conflict reduction, with no compile-time or runtime
performance claim.

## Scope

Owned paths:

- `crates/gameplay/spell/src/spell/cast.rs`: bundle loading, Character facts,
  player state declaration, `new()`, `solo()`, private modules and re-exports.
- `crates/gameplay/spell/src/spell/cast/state.rs`: all existing state transition
  impls, including Harmony/conditions and the trailing premium/respawn impl.
- `crates/gameplay/spell/src/spell/cast/gate.rs`: `CastContext`, disposition,
  condition admission gates, `cast` and `cast_resolved`.
- `crates/gameplay/spell/src/spell/cast/prepare.rs`: all `Paid*` structs,
  `prepare_*` functions, `resolve_ordinary_due_effects` and `commit_effect`.
- Existing `cast/tests.rs` and `cast/gate_tests.rs`: imports only.
- This task card; ignored local validation artifacts in `target/CAST-SPLIT-1/`.

Moved references to the former parent use `crate::spell::` so they resolve to
the same items. All existing field visibilities remain unchanged. The only
declaration visibility adjustments are private to `pub(super)`:

| Declaration | Reason |
| --- | --- |
| `state::PlayerSpellState::caster` | Called by sibling gate/preparation code and the existing cast tests. |
| `gate::cast_resolved` | Called by sibling preparation code. |
| `prepare::commit_effect` | Called by sibling gate code. |

The modules remain private and these helpers are not publicly re-exported.

## Excluded scope

No behaviour changes, public renames, new API, test removal or weakening,
dependency/manifest changes, Soul War changes, workflow/repository-tool changes,
vendor changes, external writes or Git publication.

## Validation

Local evidence is retained under ignored `target/CAST-SPLIT-1/`.

| File | Before lines | After lines |
| --- | ---: | ---: |
| `cast.rs` | 1,788 | 189 |
| `cast/state.rs` | — | 454 |
| `cast/gate.rs` | — | 284 |
| `cast/prepare.rs` | — | 922 |

- **Test parity: 224 before / 224 after.** The complete library inventories
  (`lib-before.txt`, `lib-after.txt`) are byte-identical, including test paths.
  All **224** library tests passed, with **0** ignored or failed. The spell
  library has no PostgreSQL tests, so its full library run does not consume an
  unrelated PostgreSQL suite.
- Spell all-target Clippy with `-D warnings`, package formatting and the
  downstream dispatch check passed. Existing dependency warnings originate in
  vendored Tokio/SQLx; vendor sources are untouched.
- `review_moves.py` reconstructs each extracted module from the original source,
  permitting only the documented namespace and visibility changes, then runs
  rustfmt and requires exact equality with the resulting module. The retained
  loaders, state declarations, constructor and `solo()` also match. Both existing
  test modules' bodies, helpers and attributes are byte-identical; only imports
  changed. Evidence: `structural-review.txt`.
- Workspace `rg` across `apps/`, `crates/` and `tools/` found **462** sites for
  the existing moved/retained state and cast types (`type-sites.txt`) and **85**
  direct cast consumers (`cast-consumers.txt`). All **26** existing public
  top-level declaration names are preserved, and moved names are explicitly
  re-exported. No new enum variant, struct field, trait method or public function
  needs wiring, constructor or exhaustive-match changes.
- Governance validation passed for **22** policy documents and **9** lanes;
  all **129** governance unit tests passed.
- Focused dispatch unit tests passed: actor spell state/casts **19**, owner
  payment/condition commits **7**. Workspace formatting passed.
- The exact PostgreSQL rune-use bridge case passed **1** test encompassing
  **10** real-dispatch scenarios: replay, protection/target/capability/source
  refusals, stale session/content, commit faults and stale revision. It used the
  absolute Cargo wrapper, executed against PostgreSQL and emitted no
  unconfigured-URL or noncanonical early-return marker. No whole PostgreSQL
  suite ran. Evidence: `dispatch-rune-postgres.log`.
- Whole-diff self-review against
  `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` covered all changed source and
  test imports, the new modules and this record. The move comparison proves
  unchanged admission/error ordering, payment/revision fences, state transitions
  and effect application. No persistence, transport, protocol or schema changes
  are present; no tests or assertions were deleted, weakened or newly ignored.

Commands executed (all Rust commands use the offline cache, with no dependency
downloads):

```sh
~/.cargo/bin/cargo fmt -p oteryn-gameplay-spell
~/.cargo/bin/cargo fmt -p oteryn-gameplay-spell -- --check
~/.cargo/bin/cargo fmt --all -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-spell --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-spell --lib --quiet -- --list
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-spell --lib --quiet
/home/mole/oteryn-cache/codex-bin/cargo check --locked --offline -p oteryn-gameplay-dispatch --quiet
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib --quiet -- gameplay_transport::actor_spell::tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib --quiet -- gameplay_transport::actor_spell::owner_commit::tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib --quiet -- gameplay_transport::monk_save::postgres_tests::rune_use_bridge::rune_use_bridge_postgres_real_dispatch_casts_once_and_reloads_one_debit --exact --nocapture
python3 target/CAST-SPLIT-1/review_moves.py
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
git diff --check
```

The library listing command ran both before and after the split. The initial
all-target Clippy run identified a missing test-only `SpellDefinition` import;
the import was restored before the successful Clippy and library runs.

The first downstream filter attempts used source filenames instead of their
mounted module names and matched zero tests; those runs do not qualify the
change. `dispatch-inventory.txt` established the actual paths before rerunning
the successful **19**-, **7**- and **1**-test filters above.

## Unfinished

No implementation or requested local-validation work remains. Changes are
uncommitted as requested. Publication and repository CI are outside this
local-only task.
