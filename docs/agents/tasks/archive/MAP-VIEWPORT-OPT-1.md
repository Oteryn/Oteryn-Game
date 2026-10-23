# MAP-VIEWPORT-OPT-1 — bounded map codec optimization

```yaml
task_id: MAP-VIEWPORT-OPT-1
title: Reduce repeated world-map codec sizing without changing wire bytes
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy
pr: 2287
base_sha: 59eee9429228d50cbc2e3f5fa959509eb00c4a1c
head_sha: da32209a86bebae5851c3278e75375ab75b10b52
owner: Codex standalone worker, local uncommitted allocation
created_at: 2026-10-22
updated_at: 2026-10-22
execution_policy: continuous_progress
owned_paths:
  - crates/protocol-oteryn/src/world_map.rs
  - crates/protocol-oteryn/src/world_map_tests.rs
  - apps/game-server/tests/support/real_map_bundle.rs
  - docs/agents/evidence/MAP-VIEWPORT-OPT-1-viewport.md
  - docs/agents/tasks/archive/MAP-VIEWPORT-OPT-1.md
public_contracts: []
depends_on: [MAP-VIEWPORT-MEASURE-1, MAP-CUTOVER-1b]
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **DONE**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2287](https://github.com/Oteryn/Oteryn-Game/pull/2287). Checked tile sizing is shared by map encoding with preserved wire bytes; measured snapshot savings are retained in the evidence. Evidence: `crates/protocol-oteryn/src/world_map.rs`.

Tracking closeout: delivered on main; no active implementation ownership is retained by this historical card.

## Scope and authority

Owner authorizes local edits and validation only; leave changes uncommitted.
This continuation read the latest WIP `da32209a8` and retained its completed
150-snapshot/150-delta byte-identity test vectors. The original task base is
`59eee9429`; current HEAD is the WIP above. No task branch, PR, remote frozen
candidate, publication, CI or external review trigger is created by this worker.

PROVEN: the bound META policy at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`
was refreshed through the read-only repository file API. Run 1 verified the
packet's prerequisite commits `52712f633` (#1873), `0e8345333` (#1916),
`e953f1ef6` (#1859) and `22539da5f` (#1864) as ancestors. The current codec,
measurement, fixture registration and production callers were reverified.

Continuation option A explicitly extends ownership only to correcting the
fixture's `repo_root()` join and stale consumer-location comment. PROVEN:
this was an ADR-0022 path regression. The sole mounted consumer now compiles
under `crates/gameplay/dispatch`, requiring `CARGO_MANIFEST_DIR + "../../.."`.
The previous `../..` resolved to `crates/`, causing the original deterministic
pre-measurement failure. No other fixture change was made.

Only private codec encoding/sizing implementation and existing test assertions
changed. This is the transport codec in `oteryn-protocol-oteryn`, outside
ADR-0022's gameplay layers; dispatch remains composition. Preserve public
signatures, bytes, ordering, rejection precedence and all registered bounds.
No migration, registry, capability, limit, gameplay composition, pickup, overlay,
equipment, bags or content change. Authority/recovery qualification:
NOT_APPLICABLE; this slice performs no authority-bearing or durable mutation.

## Outcome and measured acceptance

PROVEN: byte-bound validation shares the existing private sizing pass. The
standalone `check_tile_bytes` path is deleted, and all callers use the same
checked tile sizer. Valid-path item sizing falls from five evaluations to three.
No new allocation, public API, helper, fixture, harness or dependency was added.

Seven release invocations passed: three baseline, one preliminary candidate
before signature formatting, then three consecutive final-source candidates.
All passed the existing 400-step staged-versus-real wire comparison, with
identical content digest, update counts and mean byte sizes. Every run, including a
heavily contended candidate, is retained in the evidence:
`docs/agents/evidence/MAP-VIEWPORT-OPT-1-viewport.md`.

DERIVED: retain the bounded change for its real-path snapshot benefit. At
1/4/8 threads, median snapshot encode p50 is 54.7→42.9, 57.2→44.8 and
68.4→50.8 us (21.6% / 21.7% / 25.7% lower). Whole-snapshot median p50 is
139.0→128.9, 141.9→134.6 and 167.7→148.2 us (7.3% / 5.1% / 11.6% lower).
The one-thread snapshot saving is 10.1 us versus a 1.5-us baseline repetition
range; plan time did not improve. Minima, medians, p99, maxima, stage splits
and CPU estimates are all in the evidence, with the shared-host limitations.

DERIVED: no aggregate delta/CPU improvement above noise is established.
Median CPU/update is 86.4→86.6, 93.3→94.8 and 107.8→104.3 us. Candidate
median estimates at 500 views are 0.217 / 0.237 / 0.261 cores, exceeding
0.200 by 0.017 / 0.037 / 0.061. The full capacity row remains OPEN; this is
an independently reviewable codec slice. Production limits and capability
offering remain decisions for their owning lane. No composition work was
attempted to rescue the budget.

## Validation

Results on the final uncommitted Rust sources over WIP HEAD above. The fixture
is compiled only through dispatch's existing test module, so its relevant
package is `oteryn-gameplay-dispatch`, not a new game-server test target.

| Exact command | Result |
| --- | --- |
| `cargo +1.95.0 test --locked -p oteryn-protocol-oteryn --lib encoded_lengths_match_raw_fields_across_varint_boundaries --quiet` | PASS: 1 passed, 284 filtered out |
| `cargo +1.95.0 test --locked -p oteryn-protocol-oteryn --lib world_map --quiet` | PASS: 12 passed, 273 filtered out |
| `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib world_map --quiet` | PASS: 27 passed, 3 existing ignored measurements, 881 filtered out |
| `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib a_session_with_18_joins_steps_and_resumes_with_domain_17_and_never_domain_2 --quiet` | PASS: 1 passed, 910 filtered out |
| `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib a_committed_bundle_door_use_is_a_domain_17_change --quiet` | PASS: 1 passed, 910 filtered out |
| `cargo +1.95.0 test --locked --release -p oteryn-gameplay-dispatch --lib map_viewport_real_measure -- --ignored --nocapture` | PASS seven times: 1 passed, 0 ignored, 910 filtered out per invocation; B1–B3, P, C1–C3 |
| `cargo +1.95.0 test --locked -p oteryn-protocol-oteryn --quiet` | PASS: 285 passed; doctests 0 |
| `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --quiet` | PASS: 897 passed, 14 existing ignored; doctests 0 |
| `cargo +1.95.0 check --locked -p oteryn-protocol-oteryn --quiet` | PASS |
| `cargo +1.95.0 check --locked -p oteryn-gameplay-dispatch --quiet` | PASS |
| `cargo +1.95.0 clippy --locked -p oteryn-protocol-oteryn --all-targets --quiet -- -D warnings` | PASS |
| `cargo +1.95.0 clippy --locked -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings` | PASS |
| `cargo +1.95.0 fmt --all --check` | PASS after fixing intermediate formatting-only differences |
| `python3 tools/agents/validate_governance.py` | PASS: 22 policy documents, 9 lanes |
| `python3 -m unittest discover -s tools/agents/tests` | PASS: 124 tests |
| `git diff --check` | PASS |

No new ignored/disabled/deleted tests or weakened assertions. The required
existing ignored release measurement ran explicitly. PostgreSQL:
NOT_APPLICABLE for this codec/fixture allocation; no persistence, PG harness
or migration change. Full dispatch tests ran normally through the queued
wrapper. Existing vendored Tokio `missing_docs` warnings at
`vendor/tokio-1.53.1/src/sync/oneshot.rs:502` were not suppressed or edited.

## Self-review and reuse

PASS: re-read the entire task diff against
`/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and repository checklist; no hit.

PROVEN: workspace `rg` of `tile_len`, `encoded_len`, `push_tile` and the
removed `check_tile_bytes` found and updated every relevant caller, including
raw-wire test construction. No enum variant, struct field, trait method or
pub fn was added; no constructor/match/impl updates or new public E2E seam
are needed. Existing actual `serve_admitted` tests above cover domain-17
join/step/resume and committed door use without new registration.

Reused existing private varint/field/item/tile/header helpers and semantic
validation, the WIP's existing raw-field test/builders, the sole shared
`compile_real_map` fixture, `staged_update`, `staged_equals_update`,
`map_viewport_real_measure` and actual connection tests. The evidence lists
each symbol. No copied body, wrapper/re-export or second `#[path]` mount.
No file grew by more than 200 lines against the original task base.

## Checkpoint and handoff

Local bounded implementation, measurement, validation and self-review complete.
All five allocated files remain uncommitted. No frozen SHA, PR, CI or independent
review state exists. Integrator owns publication, frozen-head review and integration;
retain this task card until its final authoring commit can archive it.
Full MAP-VIEWPORT-OPT-1 capacity acceptance remains open under its own lane.
The run-1 blocker is resolved by option A; no pending owner question.
