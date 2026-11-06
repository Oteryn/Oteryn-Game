# questpc restart7 readback

All 178 pinned A..M titles have final statuses in QUEST-STATUS.md/json: 22 QUALIFIED standalone reward contracts; 156 BLOCKED, including one owner-excluded Dawnport. This is the requested terminal-status condition, not complete implementation of all quests.

Current-tree audit: all 174 inherited product hashes and all status inputs match; all 427 blocker file:line/JSON pointers resolve. Each blocked title names its missing obligation and attempts. Nine retained PG cohorts contain 82 positive-duration native POSTGRES_CASE_COMPLETED markers matching current test source and log hashes. No new qualification or PG run is claimed. Unchanged heavy reruns were avoided under the bound policy.

Fresh commands, all exit 0 (CARGO_BUILD_JOBS=4):

```sh
/home/mole/oteryn-cache/codex-bin/cargo check --locked -p oteryn-gameplay-dispatch --quiet
/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
rustup run 1.99.1 rustfmt --edition 2024 --check --config skip_children=true crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-dispatch -p oteryn-gameplay-quests --all-targets --all-features --quiet -- -D warnings
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-quests --lib storage_owner_ -- --nocapture
```

Storage-owner units: 3 passed, 0 failed. Dependency warnings from existing vendored Tokio/SQLx remain; no dependency changed.

Retained production-dispatch PostgreSQL command, not rerun in this continuation:

```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib -- quest_catalogue_chests_phase_a_01_use_replay_and_refusal quest_catalogue_chests_phase_a_02_use_replay_and_refusal quest_catalogue_chests_phase_a_03_use_replay_and_refusal quest_catalogue_chests_phase_a_04_use_replay_and_refusal quest_catalogue_chests_phase_a_05_use_replay_and_refusal quest_catalogue_chests_phase_a_06_use_replay_and_refusal quest_catalogue_chests_phase_a_08_use_replay_and_refusal quest_catalogue_chests_phase_a_09_use_replay_and_refusal quest_catalogue_chests_phase_a_11_use_replay_and_refusal --test-threads=1 --nocapture
```

Exact PG filters:

- `quest_catalogue_chests_phase_a_01_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_02_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_03_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_04_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_05_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_06_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_08_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_09_use_replay_and_refusal`
- `quest_catalogue_chests_phase_a_11_use_replay_and_refusal`


Changed files in this continuation:

- .questpc/RESUME.md
- .questpc/restart7-validation.md
- .questpc/restart7-initial-check.log
- .questpc/restart7-fmt.log
- .questpc/restart7-clippy.log
- .questpc/restart7-storage-units.log

Review: both checklists re-read; the record-only delta was reviewed, including terminal counts, hash preservation, qualification boundary and retained failure. No production code, fixture, test or status changed. No enum variant, struct field, trait method or public production function was added, so no new workspace constructor/match/impl/caller update is applicable. No claim of independent review of the inherited 174 product files, full quest completion, CI, freeze or mergeability.

Unfinished: 155 nonexcluded full quest paths are explicitly BLOCKED with exact missing obligations and attempts in QUEST-STATUS.json. Hot Cuisine retains catalogue_npc_scoped_a_m_starts_are_source_ordered_and_atomic failing at command2603 hi, Rejected!=Ok, classic_quests.rs:168, 114.00s, without a completion marker. CalendarMonth and delayed EndConversation plus cooking exchange composition remain missing there. World pin refresh, migration allocation, independent candidate review and CI/integration remain with the coordinator. No shared fixture fix, Git/index/publication, credential, Docker, dependency or outside-worktree mutation.
