# Play feature adapters

CLIENT-STRUCT-5b moves mechanic reducers and command construction out of the play
orchestrator. These adapters still consume the app's `PlayView`, `Scene`, and map
facades, so they remain below the app while those composition types are app-owned.
Attack/spell session adapters and spell feedback live in the matching
`crates/client-runtime/src/features/` directories; spell-bar input/vitals rendering
live in `crates/client-ui/src/features/local-spell-action-bar/`. Independent
client-domain/simulation/UI concerns extracted by steps 2–4 remain in their owning
crates. This step does not start the `lib.rs` migration.

Each participating feature has a `play.json` fragment listing its concern modules
and pushed-event entry points. `apps/client/build_support/play_features.rs` scans
the directories, checks mechanic names against the existing capability facts,
rejects duplicate event ownership, and generates private modules and typed event
dispatch in `OUT_DIR`. A new adapter adds only its feature files and fragment;
there is no hand-maintained module or handler list. Base-protocol and cross-cutting
view behavior uses `local-*` directories because it has no server capability ID.
These registrations describe the existing play projection, not full command or
mechanic support; they do not change the UI's unsupported coverage baseline.

Concern files preserve the existing `PlayView` methods and compatibility exports.
Feature tests live beside their code. Play-view tests mount under the shared
`play::tests` harness to reuse its private snapshot/session fixtures; the remaining
tests there cover cross-feature ordering, bounded delivery, and session lifecycle.

Generated module paths are invisible to rustfmt's source discovery. In addition
to `cargo fmt --all --check`, format/check these sources explicitly:

```sh
rustfmt --edition 2024 --check apps/client/src/features/*/*.rs apps/client/src/features/*/tests/*.rs
```
