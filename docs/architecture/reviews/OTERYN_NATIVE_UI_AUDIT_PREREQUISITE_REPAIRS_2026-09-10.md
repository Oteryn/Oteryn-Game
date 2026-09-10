# Native UI source-audit prerequisite repairs

- Task: `OTV2-NATIVE-UI-AUDIT-PREREQUISITES-20260910`
- Repository: `Oteryn/Oteryn-Game`
- Source baseline: protected `main@0a89d66d6954a05000f49ebfef49b0db717f46be`
- Source audit: PR #557, head `c76858b23298e9f012b72ce834b3f826722940e5`
- Branch: `agent/native-ui-audit-prerequisites-20260910`
- Status: **REGRESSION QUALIFICATION / NOT READY FOR INTEGRATION**
- Scope: minimal repairs of existing input/renderer code, not UI-P1 implementation.

## Authority and ownership

The owner's continuation requests completing the native UI source audit and correcting
its findings. This separate prerequisite keeps runtime repairs out of the documentation
candidate #557 and does not authorize or start a UI foundation, HUD, protocol, server,
production activation, FOV decision or addon programme.

Root AGENTS, immutable bound META 3.1.0 and the repository execution protocol apply.
The scoped live open-PR search identified no input/renderer runtime writer; matching #356
owns unrelated durability/root Cargo work, while #520 and #552 are documentation only.
No nearer AGENTS exists in the affected crate/src ancestors. Root Cargo, lockfile,
workspace-boundaries, shared composition roots and workflows are not writable here.

Owned paths, including the planned corrective delta:

- `crates/input-actions/src/{physical,error,semantic,router}.rs`;
- `crates/input-platform/src/lib.rs` and `audit_regressions.rs`;
- `crates/renderer/src/resources.rs` and `windows.rs`;
- this bounded task/evidence record.

No replacement of #557, no force/rebase, no main write and no integration authority.

## Findings and acceptance

### Input modifier identity

The physical adapter emits supported left/right modifier keys as well as the canonical
modifier snapshot. Semantic chords declare modifiers separately from non-modifier
inputs, but the existing router adds modifier atoms to the chord. Real Control+A can
therefore fail while shortened hand-built test streams pass.

The repair must retain normalized physical events for the app/UI, prevent modifiers
from becoming non-modifier chord atoms, reject invalid modifier-containing binding
shapes, cancel a held action when its modifier snapshot ceases to match, and require
a fresh eligible press rather than implicitly restarting a cancelled held letter.
No keymap, command authority, UI focus policy or duplicate platform mapper is added.

### Renderer generation before physical acquisition

The native render path currently validates generation only after queue submission and
presentation. Its phase-only entry check is factored into one crate-private acquisition
boundary so pure tests can count whether a physical acquisition callback was entered.
The Windows renderer uses that same boundary; this is not a parallel mock-only path.

The repair must reject stale generations before acquisition, reject non-presentable
phases, call the backend exactly once on eligible entry, and preserve state/outcome
semantics. Acquisition is not presentation; timeout, occlusion and recovery must not
be reclassified as a displayed UI frame. Existing recovery bounds remain unchanged.

## Qualification plan and honest evidence

The first regression candidate preserves the old phase-only generation behavior in
that extracted seam. The new stale-generation tests and full modifier-stream tests
are expected to fail; expected failure is not an observed result until hosted CI runs.
Do not merge this candidate. The next coherent repair must make the same tests pass.

Focused actual Rust targets:

```text
cargo +1.94.0 test --locked -p oteryn-renderer acquisition_tests
cargo +1.94.0 test --locked -p oteryn-input-platform audit_regressions
cargo +1.94.0 test --locked -p oteryn-input-actions
```

The existing required Linux workspace job runs these package tests. Final qualification
also requires repository-selected metadata/fmt/boundary/Clippy/supply-chain checks,
Windows production-client build/Clippy, existing regression/smoke targets and game-gate
on the exact unchanged final head. No workflow weakening or custom gate is introduced.

The local isolated source subset is byte-verified against Git blob identities. Rust is
not installed locally and direct repository network access is unavailable; actual Rust
execution is repository-native CI, not a claimed local run. The acquisition callback
oracle proves preflight call ordering, not hardware GPU performance or complete native
UI behavior. Physical HUD/Tier-2/FOV A/B qualification remains outside this prerequisite.

## Exit, review and rollback

Exit requires observed negative discrimination, final passing component/required CI,
whole-diff review and an exact source/evidence record. A failed required check is FIX,
not permission to merge. Independent review required by the owning risk contract is
not replaced by author checks or the automatic architecture workflow.

Only normal authorized Merge Queue, merge_group proof and protected-main readback can
establish integration. Runtime repair reversals are bounded to these existing client
seams; no data/schema migration or server rollback is introduced. UI-P1 still requires
its separate architecture acceptance and shared-workspace allocation.

## Observed first negative population and input correction

At exact candidate `8ceb0fc57a43c30e5bbf84fb989a03d393022ac9`, Merge gate run
`34534642921`, Linux job `103063412799`, build and strict Clippy succeeded.
The three new adapter/router regression tests all failed with zero actions where one
start was required; the 11 existing adapter tests passed. This is observed Rust
execution, not source-only inference. Cargo stopped at this failing package, so the
renderer negative tests were not executed in this population.

The next delta corrects the input contract: modifier usages remain normalized events
but are excluded from non-modifier chord atoms; invalid modifier-atom bindings return
`ModifierChordInput`; changed modifier snapshots cancel incompatible active actions
before new routing. Existing consumers are qualified by workspace compilation/tests;
no serialized binding format or product keymap exists or is changed by this repair.
Additional tests cover every supported modifier side, malformed binding shapes,
exact release counts and the four-non-modifier chord limit.

The initial population also exposed an incorrect PR heading (`Owned scope` instead of
the required `Scope`) and one Rustfmt import-line difference. These are authoring
mistakes, not intended runtime failures; both are corrected without policy changes.
Renderer generation repair remains pending its negative test population. This
intermediate candidate is still not ready for integration.
