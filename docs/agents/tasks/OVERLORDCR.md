# OVERLORDCR

Base: e2152d050038cecdb3728c222af2ac184e1cac5a. Branch: claude/happy-davinci-ijz3wy-overlordcr.

Crystal reference checkout: /workspace/crystal at 201189f871d23597b82caf88fd611454a2ddefb8; the original packet revision 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d was also inspected.

`data-global/scripts/quests/elemental_spheres/creaturescripts_overlord_kill.lua:2-5` maps energy, fire, ice and earth overlords to the same player `Storage.Quest.U8_2.ElementalSpheres.BossStorage`. Lines 24-26 call `onDeathForDamagingPlayers`, check `< 1` and set `1`. `data-global/lib/core/storages.lua:1084` defines BossStorage as 41652. The distinct globalStorage keys in lines 2-5 control spawn availability, not per-character kill credit.

Implementation plan: register four source-death rules against one quest track. Use the existing lethal damage-share capture, quest recipient freeze, native death queue and fenced quest-state transaction. Do not introduce a second storage owner or infer nearby/party participants. Verify two damaging characters, a nearby observer, other creature deaths, duplicate settlement and all four source mappings through PostgreSQL.

WIP: production rules are registered in `crates/gameplay/quests/src/quest/source_deaths.rs`. Mounted dispatch regressions cover shared donor mapping, two damaging Characters, a nearby observer, a non-overlord death and duplicate projection/settlement. The pre-change quest dependency was compiled before production registration; that attempt failed during dispatch compilation before any test ran. The single-job retry is pending. Formatting passed. Governance validation and all 135 governance unit tests passed. PostgreSQL execution, clippy and pin verification remain pending.

Validation attempt: `cargo test -p oteryn-gameplay-dispatch --lib overlordcr -- --nocapture` failed before test execution: `error: could not compile `oteryn-gameplay-dispatch` (lib test)`; rustc ended with `(signal: 9, SIGKILL: kill)`. The cgroup reported one OOM kill. This is not behavioral RED evidence. Cleaned only dispatch build output and retried with `CARGO_BUILD_JOBS=1`; the retry includes the production rules.

Whole-diff self-review: no new storage owner, SQL migration, capability, command, content input or generated pin. The four rules share the donor flag, require damage rather than social-party/proximity membership, and use the existing native quest writer and occurrence receipts. Existing authority checks are unchanged. New tests retain the original source-death scenario assertions and add independently admitted damager/observer assertions. Runtime PostgreSQL validation remains pending.

Second dispatch test attempt: after `cargo clean -p oteryn-gameplay-dispatch`, `CARGO_BUILD_JOBS=1 cargo test -p oteryn-gameplay-dispatch --lib overlordcr -- --nocapture` again ended with `(signal: 9, SIGKILL: kill)` before any test executed. The repository PG workflow documents extra swap for this target; this container has no swap or administrative capabilities. Full lethal-path tests remain intact. Added a separate mounted authority-case proof of the quest-owner transaction to obtain supplemental PostgreSQL evidence without compiling dispatch library tests.

Passed: `cargo clippy --locked -p oteryn-gameplay-quests --all-targets --quiet -- -D warnings`. Supplemental authority PG test compilation is in progress.

Current validation table (ci-local.sh is absent at this base):

| Check | Result |
| --- | --- |
| cargo fmt plus explicit mounted-file rustfmt | Passed |
| quest crate clippy, all targets, warnings denied | Passed |
| governance validator and unit tests | Passed, 135 tests |
| full dispatch lethal-path PG regressions | Two compiler OOM failures, including clean single-job retry; no test execution |
| supplemental authority PG proof | Building with one job |
| dispatch clippy, all targets/all features | Pending |
| game-server touched authority target clippy | Pending |
| pin/lock verification | Pending |
| behavioral red-to-green proof | Not obtained; initial red attempt failed in compiler |

Donor key rechecked on the current 201189f8 checkout: `data-global/lib/core/storages.lua:1084`, `BossStorage = 41652`. Its overlord death script is byte-identical to packet revision 9f5a72c for the relevant file. The supplemental build has progressed past the content crate into gameplay dependencies.
