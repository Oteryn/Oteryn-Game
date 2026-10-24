# LOOP-MAP-1 — server validation and handoff, 2026-10-24

## Candidate and scope

Local AUTHORING only, repository Oteryn/Oteryn-Game, branch
`claude/happy-davinci-ijz3wy-srv-loopmap`, unchanged base HEAD
`cb5b3de92d2e19df53e05a7e0f5b9d98fa11d299`. No commit, staging, push,
force/reset/rebase, external write or review trigger. Client/UI and CI routing
are untouched. The owner authorized option A with labelled basic Oteryn defaults.
Bound META 3.1.0 was refreshed at its immutable policy commit; no new permission
question was opened. This document is local evidence, not a frozen-candidate,
independent-review, CI or release qualification.

## Results and exact commands

PASS counts: dispatch PostgreSQL E2E 1; domain 2; Cyclopedia durability/audit 1;
revision-sequencer structural checks 2; entity respawn 13; map boot 18; protocol
393. Clippy (seven packages, all targets, warnings denied), cargo fmt and explicit
changed Rust-file rustfmt pass. Governance passes (22 documents, 9 lanes) with
129 governance unit tests; repository policy passes (23 files, 68 workflows).
Existing vendored Tokio/SQLx dependency warnings are not product warnings.
Standalone entities unit tests need the existing `test-support` feature, as
exercised by dispatch's dev dependency; the first attempt without it failed on
pre-existing test-only Content APIs. No manifest or dependency was changed.

```bash
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib cyclopedia_postgres_real_dispatch --quiet -- --nocapture
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-domain --lib cyclopedia --quiet
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib cyclopedia --quiet
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-durability --lib writer_registration_tests --quiet
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-entities --features test-support --lib respawn --quiet
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-world --lib map::boot::tests --quiet
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-protocol-oteryn --lib --quiet
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo clippy --locked -p oteryn-gameplay-domain -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch -p oteryn-gameplay-world -p oteryn-gameplay-entities -p oteryn-protocol-oteryn -p oteryn-game-server --all-targets --quiet -- -D warnings
CARGO_NET_OFFLINE=true /home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
python3 tools/repository/validate_repository_policy.py
git diff --check
```

Only `cyclopedia_postgres_real_dispatch` used PostgreSQL. It is auto-mounted in
`gameplay_transport::monk_save::postgres_tests` by the existing dispatch build
script and therefore belongs to the existing dispatch `--lib` CI tree. The
absolute wrapper supplied the administration URL; it was never printed. The
scenario uses the existing runtime-role Harness and its canonical migrations.
Final nocapture run: `POSTGRES_STARTED LOOP-MAP-1 cyclopedia_postgres_real_dispatch`,
1 PASS, 1,000 filtered out, 23.66 s. No route-less early return qualified this run.

The scenario proves admitted command framing, capability refusal, current
Premium checking, resettable deterministic exploration, private positions,
seven distinct finds, PostgreSQL reload after session-state/sequencer replacement,
lost-ACK exact-result replay, main-backpack coin burn plus bank shortfall,
negotiated inventory delta, no second charge, insufficient-funds rollback,
stale lease and stale boot-epoch refusal, retained pool selection and native
home-spawn timing halved from 60 seconds to 30 seconds. It seeds an activated
next-epoch snapshot; reset control-plane publication is owned/tested elsewhere.

## Whole-diff review

Read the owner's `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and re-read the full
tracked delta plus every new source/SQL/contract file. Swept every added enum,
field and trait-method constructor/match/impl in `apps/`, `crates/`, `tools/`:
`FeeBurnCause`, `FeeBurnCauseV1`, `SpawnPointDefinition`, `BundleMap`, BasicArea,
Point, Exploration, MapProgress, MapAction, MapRefusal, MapRequest, MapIntent,
MapResult, CyclopediaWorldBinding, CyclopediaPremiumGate and the connection
Cyclopedia authority methods. Both SpawnPointDefinition constructors and the
sole BundleMap constructor are updated. No new production public function is
only test-called: the real wire scenario reaches the codecs, rules, sequencer,
Character writer, epoch preparer and runtime home-spawn consumer.

Reviewed unique-key/small-bound reads, root/coin/bank/pool lock order, current
recovery/node/session/lease/scope fences, same-transaction fee/receipt/pool/state
proofs, append-only cause codes, command receipts and content pins. Corrected
whole-class issues in this batch: current Premium delivery before transaction
borrow, actual viewport asymmetry, immutable door exclusion, explicit catalogue
pinning, negotiated inventory refresh, current owner check on the runtime setter,
and retryable epoch initialization in the independent spawn loop. No existing
test was removed, weakened or ignored. No forbidden files changed.

## Remaining qualification

Reference catalogue/parity, catalogue normalization/reconfiguration, automatic
POI presentation events, achievements/outfits/titles/raid-warning consumers and
nonzero discovery speed contributions remain unqualified/outside this basic
server interface. Changed catalogue/bundle state fails closed and retains value.
Independent frozen-candidate review and repository CI remain with the control
plane. No unrelated PostgreSQL case failed; there is no CI-only contention item.

## Changed files (47)

```text
apps/game-server/migrations/0176_character_cyclopedia.sql
content/cyclopedia/basic-area.json
crates/gameplay/dispatch/src/gameplay_transport/connection.rs
crates/gameplay/dispatch/src/gameplay_transport/cyclopedia.rs
crates/gameplay/dispatch/src/gameplay_transport/cyclopedia/content.rs
crates/gameplay/dispatch/src/gameplay_transport/cyclopedia/respawn.rs
crates/gameplay/dispatch/src/gameplay_transport/mod.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/cyclopedia.rs
crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/cyclopedia/fixture.rs
crates/gameplay/dispatch/src/gameplay_transport/world_respawns.rs
crates/gameplay/domain/src/domain/cyclopedia.rs
crates/gameplay/domain/src/domain/cyclopedia/basic.rs
crates/gameplay/durability/src/durability/character_cyclopedia.rs
crates/gameplay/durability/src/durability/character_cyclopedia/storage.rs
crates/gameplay/durability/src/durability/character_cyclopedia/world.rs
crates/gameplay/durability/src/durability/character_revision_sequencer.rs
crates/gameplay/durability/src/durability/character_revision_sequencer/cyclopedia.rs
crates/gameplay/durability/src/durability/character_revision_sequencer/writer_registration_tests.rs
crates/gameplay/durability/src/durability/core_modules.rs
crates/gameplay/durability/src/durability/item_fee_burn.rs
crates/gameplay/durability/src/durability/item_fee_burn/purchase_cause.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit/purchase_cause.rs
crates/gameplay/durability/src/durability/item_fee_burn_audit/tests/purchase_cause_tests.rs
crates/gameplay/entities/src/carrier.rs
crates/gameplay/world/src/map/boot.rs
crates/protocol-oteryn/src/capability_tables.rs
crates/protocol-oteryn/src/cyclopedia.rs
crates/protocol-oteryn/src/lib.rs
docs/agents/evidence/LOOP-MAP-1-server-validation-20261024.md
docs/agents/evidence/LOOP-MAP-1-source-history-20261024.md
docs/agents/tasks/LOOP-MAP-1.md
docs/contracts/game-events/v1/native_one_item_transaction.proto
docs/contracts/protocol-oteryn/v1/cyclopedia-basic-v1.md
docs/contracts/protocol-oteryn/v1/registry/capabilities/0057-cyclopedia-map-v1.json
docs/contracts/protocol-oteryn/v1/registry/command_types/0072-cyclopedia-query.json
docs/contracts/protocol-oteryn/v1/registry/command_types/0073-cyclopedia-intent.json
docs/contracts/resource-limits/entries/0870-CYCBASIC-RL-01.json
docs/contracts/resource-limits/entries/0871-CYCBASIC-RL-02.json
docs/contracts/resource-limits/entries/0872-CYCBASIC-RL-03.json
docs/contracts/resource-limits/entries/0873-CYCBASIC-RL-04.json
docs/contracts/resource-limits/entries/0874-CYCBASIC-RL-05.json
docs/contracts/resource-limits/entries/0875-CYCBASIC-RL-06.json
docs/contracts/resource-limits/entries/0876-CYCBASIC-RL-07.json
docs/contracts/resource-limits/entries/0877-CYCBASIC-RL-08.json
docs/contracts/resource-limits/entries/0878-CYCBASIC-RL-09.json
docs/contracts/resource-limits/entries/0879-CYCBASIC-RL-10.json
```
