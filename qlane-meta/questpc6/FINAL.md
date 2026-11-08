# questpc6 local result

All 33assigned titles have final status rows:1 QUALIFIED original SOURCE contract,
32 BLOCKED (31 shared-engine dependencies and owner-excluded Dawnport).
Each active blocker names the missing engine piece, its file:line evidence and
what was attempted. These are not missing-test dispositions.

QUALIFIED: Deeper Fibula Quest -
`questpc6_source_deeper_fibula_well_and_complete_reward_inventory`:
`POSTGRES_CASE_COMPLETED`,85.824043755s,`source-final.log`.
The test asserts the exact original feature set and complete six-claim inventory,
then drives the well and every native source reward placement through real framed
USE. It covers stale-target refusal, successful travel, replay, unchanged quest
state, exact item contents/charges and chest refusals. The separately authored
key/expertise/eight-dragon recipe remains BLOCKED; this is not a full chosen
storyline or deployed-map certificate.

Changed Rust files:

- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/questpc6_access.rs`
  (new auto-mounted lane tests and source-contract guard).
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_catalogue_chests.rs`
  (existing test helper visible to sibling lane cases).
- `crates/gameplay/dispatch/src/gameplay_transport/npc_command/catalogue.rs`
  (one inherited rustfmt line break).
- `crates/gameplay/durability/src/durability/character_revision_sequencer/npc_program.rs`
  (narrow documented argument-count lint attribute on the existing forwarder).
- `crates/gameplay/durability/src/durability/quest_state/npc_program/fathers_burden.rs`
  (remove an unused import).

Changed/new lane records and evidence:
`QUEST-STATUS.json`, `QUEST-STATUS.md`, `RESUME.md`, `SHARED-FIXES.md`,
`ENGINE-BLOCKERS.json`, `engine-dispositions.py`, `finalize_status.py`,
`final-status-audit.json`, `phase-a-callers.txt`, this report and the focused
validation logs/result records. Other copied `.questpc6` history was preserved.

Exact executed PostgreSQL commands (one at a time, the absolute queued wrapper):

```sh
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc6_ -- --nocapture --test-threads=1
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc6_source_ -- --nocapture --test-threads=1
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib questpc6_source_deeper_fibula_ -- --nocapture --test-threads=1
```

- First command:5 passed/0 failed,426.13s execution;`access-current.log`.
  This preceded the two strict-source tests and the additional stale USE check;
  its original mounted-source hash is retained separately.
- Second command: narrative cache cohort PASS, marker95.635899022s;
  Fibula RED because a stale USE consumed transport cadence before its immediate
  successor. Overall1 pass/1 fail;`source-red.log`. All assertions were retained.
- Third command: repaired Fibula PASS,1/0, 85.85s execution and the positive
 85.824043755s marker. Advance the existing fixture clock between distinct
  commands; production cadence was unchanged. No full PG suite ran.

Final checks:

```sh
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-dispatch -p oteryn-gameplay-durability --all-targets --all-features -- -D warnings
CARGO_BUILD_JOBS=3 /home/mole/oteryn-cache/codex-bin/cargo fmt --package oteryn-gameplay-dispatch --package oteryn-gameplay-durability -- --check
```

Both PASS; Clippy reports2m 42s. Direct rustfmt checks of the mounted lane file
and touched sequencer file PASS. `git diff --check` PASS. Whole retained diff
reviewed against both checklists; no unresolved finding. No enum variant,
struct field, trait method or production function was added. All shared helper
call sites in apps/,crates/,tools/ are recorded in`phase-a-callers.txt`.
The two durability hygiene changes after the PG run are behavior-neutral;
no runtime/CI/frozen-candidate qualification is inferred from them.

Unfinished: the 31 nonexcluded blocked full paths need the reserved qengine or
qengine2 owners (encounter/access, StepOut/reset, atomic NPC hand-in, general
outfit/addon entitlement). Dawnport remains owner-excluded. Fibula's separate
chosen authored supplement also needs its access/encounter path. Per-title
requirements, attempts and references remain in`QUEST-STATUS.json`.
No publication, Git/index mutation, migration, new command/capability, dependency
installation, credential access or external write was performed.
