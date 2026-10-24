# PHASE-SPLIT-A — server qualification and boot phases

## Task

Owner-authorized structural refactor in the allocated worktree; local edits only.
No Git publication or lifecycle transition is requested. Bound META policy 3.1.0
was read at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.

## Scope and owned paths

- `apps/game-server/src/qualification.rs`
- `apps/game-server/src/qualification/client_phases/*.rs` (new private modules)
- `apps/game-server/src/node/serve.rs`
- `apps/game-server/src/node/serve/channel_boot.rs` (new private module)
- `apps/game-server/src/node/serve/supervision.rs` (new private module)
- This task card; local validation artifacts under `target/PHASE-SPLIT-A/`.

No dispatch connection paths, manifests, workflows, policy or tests are edited.
The former crate paths and external public API stay intact. Added helpers are
restricted to their parent modules; no public API or trait/enum changes.

## Old → new mapping

| Old function/phase | New private location |
| --- | --- |
| `qualification::seam_clients` transport and negative cases | `qualification/client_phases/negatives.rs` |
| First admission, replay, concurrency and first control loss | `qualification/client_phases/admission.rs` |
| Both same-session resumes and expiry | `qualification/client_phases/recovery.rs` |
| Door USE wire exercise and release | `qualification/client_phases/use_wire.rs` |
| Dev client admission, door exercise and release | `qualification/client_phases/dev_client.rs` |
| Final re-admission | `qualification/client_phases/readmission.rs` |
| `node::serve::boot_and_serve` content activation and Channel preparation | `node/serve/channel_boot.rs::prepare_channel` |
| Serve-loop polling, position flush, readiness withdrawal and shutdown | `node/serve/supervision.rs::supervise` |

The two original functions orchestrate these phases. The existing grant factory
retains nonce order; the transport phase returns the plaintext socket so the
orchestrator retains its original lifetime. Character store/seal/authority stay
in the boot orchestrator. Prepared Channel state retains the activation and
reference catalogues throughout serving. Shutdown still polls in its original
order and withdraws readiness before cancelling loops.

## Validation evidence

- Library inventory: **464 before / 464 after**, zero additions/removals; the
  complete list files are byte-identical (`lib-inventory.diff` is empty).
- Focused unit tests: node serve **27**, qualification **6** (three existing
  topology-only tests ignored), quest boot **8**, runtime status **4**, bundle
  spawn owner **1**. All **46** executed unit tests passed.
- `integration_rest`: server-process qualification **2**, map cutover **3**;
  all **5** passed. This integration target is exercised, not edited.
- PostgreSQL: exact readiness/assignment case **1**, exact fresh-store boot case
  **1**; both passed through the absolute wrapper, with no unconfigured-URL
  early-return marker. No whole PostgreSQL suite was run.
- Formatting and final package/all-target Clippy `-D warnings` passed. Clippy's
  dependency diagnostics retain pre-existing vendored Tokio/SQLx warnings;
  there are no game-server warnings. The initial `cargo check --tests` caught
  an extracted mutable-reference borrow and unused captures; these were fixed
  before final Clippy and tests passed.
- Governance: **22** policy documents / **9** lanes passed; **129** governance
  unit tests passed. `git diff --check` passed.
- Functions: `seam_clients` **44** lines; `boot_and_serve` **218**;
  `prepare_channel` **209**; `supervise` **224**. Every extracted phase is below
  250 lines. New files are at most **340** lines / **15,441** bytes (dev client).
- Whole-diff checklist review covered effect/error ordering, socket/activation/
  authority lifetimes, capability checks and shutdown deadlines/poll ordering.
  `structural-review.txt` compares every moved phase with its original body:
  identical after only explicit borrow, module path, cancellation-return wrapper
  and rustfmt/trailing-comma normalization. Existing test source is byte-for-byte
  unchanged. No tests, assertions, SQL, wire strings or error text were edited.
- Workspace-wide `rg` over `apps/`, `crates/`, `tools/` enumerated all three new
  phase-state struct declarations, constructors and destructuring sites; evidence
  is in `state-sites.txt`. No enum variant, trait method or externally public
  function was added. Production helpers are called by the existing boot path;
  qualification helpers are called by its existing real-dispatch harnesses.

Exact successful Rust commands (each used the queued absolute wrapper):

```sh
/home/mole/oteryn-cache/codex-bin/cargo fmt --all
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib -- --list
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib --quiet -- node::serve::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib --quiet -- qualification::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib --quiet -- quest_catalogue_boot_tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib --quiet -- node::runtime_status::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib --quiet -- bundle_spawn_owner_tests::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test integration_rest --quiet -- server_ci_qualification::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test integration_rest --quiet -- map_cutover_boot::
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test runtime_scope_assignment_postgres --quiet -- assignment_mutations_atomically_fence_readiness_and_reject_stale_publication --exact --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --test character_authority_postgres --quiet -- fresh_store_rerun_is_idempotent_and_interpretation_waits_for_it --exact --nocapture
```

The `--lib -- --list` command ran before and after the refactor, saving stdout
as `target/PHASE-SPLIT-A/lib-before.txt` and `lib-after.txt` respectively.
Other exact checks:

```sh
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
python3 target/PHASE-SPLIT-A/review_moves.py
diff -u target/PHASE-SPLIT-A/lib-before.txt target/PHASE-SPLIT-A/lib-after.txt
git diff --check
```

Initial compilation probe (repaired findings as described above):

```sh
/home/mole/oteryn-cache/codex-bin/cargo check --locked -p oteryn-game-server --tests --quiet
```

## Unfinished

The three pre-existing ignored qualification scenarios require their disposable
Platform/node topology and published ephemeral trust material. No test is
changed or newly ignored; ordinary PostgreSQL wrapper configuration alone does
not supply that topology. Publication and CI are outside this local-only task.
