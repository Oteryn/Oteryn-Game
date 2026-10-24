# SERVE-PHASE-1 — ordered private connection phases

```yaml
task_id: SERVE-PHASE-1
title: Split serve_admitted_actor into ordered private phases
mode: REFACTOR
status: waiting
repository: Oteryn/Oteryn-Game
branch: claude/happy-davinci-ijz3wy-serve1
base_sha: d5bf993672235af9df044c0acd4585bd60a5c29e
pr: null
owner: standalone Codex worker
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/connection.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/serve/**
  - docs/agents/tasks/SERVE-PHASE-1.md
```

## Result

PROVEN: `serve_admitted_actor` is now **48 lines**, down from **2,183** on current main.
The largest extracted function is **248 lines** (`result::encode`); the largest
new source file is **514 lines** (`snapshot.rs`). All twelve private source
files are below 700 lines. No public API, wire, SQL, dependency, test or assertion
changes. `connection.rs` is byte-identical outside the removed function and its
two module/import lines.

The entry preserves admission refusal and observation before preparing the
snapshot, polling, handling a refresh or frame, and executing the next command.
`ServeState` explicitly carries the old locals. Its fields drop in reverse order
of the original declarations, retaining pacing-claim release before the command
stream's ingress lock on teardown. `ServeEnd` carries the original terminal
reason; the entry attaches the current admitted session. Protocol closure still
calls the existing `close_admitted` helper before returning that same end.

Snapshot phases move encoded payload buffers into the existing owned domain
representation, then borrow them for the existing encoder. No payload is cloned.
Observation, encoding, domain ordering, transmission and continuity advancement
remain ordered as before. Input permits still cover execution and inventory
refresh, and drop before result encoding or sends. Logout/read polling, cadence
priority, buffered-step ownership, replay and relocation retain their ordering.

Bound META policy: 3.1.0 at
`Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, resolved from the
immutable organization-policy path. Local uncommitted authoring only; the driver
owns publication. `PHASE-SPLIT-B.md` is absent in this checkout, so the YAML header
follows the adjacent `CAPS-SPLIT-3.md` and `CONN-TESTS-SPLIT-1.md` cards.

## Old → new mapping

Old line numbers refer to `connection.rs` at the original patch base
`2435e07a9450549fbdd252e9ad00327749e37ea1`. New paths below are
relative to `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/`.

| Old section | New private phase |
|---|---|
| 1779–1801: actor eligibility, observation and initial revisions | `mod.rs::serve_admitted_actor` |
| 1802–1894: overlay, vitals and achievement notices | `snapshot.rs::initial`, orchestrated by `setup.rs::prepare` |
| 1895–2004: items, appearance/spatial visibility and notice domain insertion | `snapshot.rs::items`, `setup.rs::prepare` |
| 2005–2151: container, quest, chat, NPC and combat snapshots | `snapshot.rs::sessions` |
| 2152–2220: map, clock and weather snapshots | `snapshot.rs::world` |
| 2221–2244: snapshot encoding, sends and confirmed continuity | `setup.rs::prepare` |
| 2245–2339: command stream, timers and pacing setup | `setup.rs::live` |
| 2341–2461: event selection and ordered polling | `poll.rs::next`, existing `Next` enum |
| 2462–2800: refresh/frame/probe handling | `inbound.rs::receive`, seven `refresh.rs` functions |
| 2801–2916: envelope, ingress, input/capability gates and retained bank | `command.rs::execute` |
| 2917–3199: ordered command branches, step pacing and logout/read polling | `route.rs::{step,execute,services,logout}`, called from `command.rs::execute` |
| 3200–3237: bank inventory refresh, permit release and result sequence | `command.rs::{bank_inventory,execute}` |
| 3238–3440: result disposition/payload and terminal error encoding | `result.rs::encode` |
| 3441–3483: terminalization, result write and logout teardown | `outbound.rs::finish` |
| 3484–3512, 3645–3676, 3723–3755, 3790–3835: step/spell/item/NPC effects | `effects.rs::{step,spell,item_move,npc_trade,npc}` |
| 3513–3644: inventory, vitals, overlay and achievement use effects | `use_effects.rs::use_effects` |
| Other 3484–3916 follow-up branches and final relocation | `outbound.rs::emit` |

## Re-application onto current main

Re-applied onto `d5bf99367` on 2026-10-24. The requested base-to-main diff contains
only LOOP-TRAIN-1 additions in `connection.rs` and `connection/routing.rs`.
All six additions within the moved loop
from LOOP-TRAIN-1 (#2359) are preserved below; source lines refer to current main.

| Source in `connection.rs` | Destination under `connection/serve/` |
|---|---|
| 2926–2932: training activity under the input permit | `command.rs::execute` |
| 2993–3010: commands 66/67 and control-loss outcome | `route.rs::execute` |
| 3288–3289: capture training logout before result encoding | `command.rs::execute`, carried in `CommandOutcome` |
| 3298–3312: training result encoding and rejected/accepted status | `result.rs::encode` |
| 3957: training has no follow-up domain effect | `outbound.rs::emit` |
| 3963–3965: return logged-out after the result, before relocation | `outbound.rs::emit` |

The three training authority methods at lines 663–687 stay in `connection.rs`.
`connection/routing.rs::Dispatch::Training` is already on main and is unchanged.
No additional stamina change belongs to the moved body; the existing real-wire
exercise test calls `stamina::qualify` after logout and successor admission.

## Validation

Current re-application checks, 2026-10-24. Clippy and tests use the absolute wrapper
`/home/mole/oteryn-cache/codex-bin/cargo`, with `--locked --offline`. Only the
connection and mechanic-owned training filters execute locally. The current main
test inventory is **1,036** (the original patch base had 1,033); no test changed.

- `~/.cargo/bin/cargo fmt -p oteryn-gameplay-dispatch`: PASS.
- `~/.cargo/bin/cargo fmt -p oteryn-gameplay-dispatch -- --check`: PASS.
- `cargo clippy --locked --offline -p oteryn-gameplay-dispatch --all-targets -- -D warnings`: PASS; existing vendored Tokio/sqlx warnings remain dependency diagnostics.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --lib gameplay_transport::connection::`: PASS, **73** passed, 963 filtered out.
- `cargo test --locked --offline -p oteryn-gameplay-dispatch --lib loop_training_postgres -- --nocapture --test-threads=1`: PASS, **3** passed, 1,033 filtered out. Real PostgreSQL starts were observed, including exercise real-wire stop/replay/fence/reload, offline training logout and stamina qualification; no route-less early return qualified.
- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 -m unittest discover -s tools/agents/tests`: PASS, **129** tests.
- `git diff --check` and new-source whitespace checks: PASS.
- Whole-diff review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md` and repository checklist: PASS. No SQL, wire identifiers, authority gates, durable outcomes or test bodies changed.
- Workspace searches under `apps/`, `crates/`, `tools/` for every new private state/output type, enum constructor/match and implementation: PASS; all sites remain inside the private module, with production callers. Existing real-dispatch tests cover the extracted path.
- Size and move review: PASS against current main. The poll body, all seven refresh bodies and six extracted effect bodies are token-identical after undoing state access, return plumbing, local shadow renames and rustfmt-only changes. All six ported additions match main after the same normalization. A byte comparison verifies `connection.rs` is unchanged outside the moved function and its two module/import lines.

Evidence remains under ignored `target/serve-phase-1/`: current main source,
clippy/test/governance logs, `move-review.log`, and workspace-site searches.

## Unfinished

- The unfiltered dispatch library suite is CI-only under the owner's PG scope rule.
- No local test failures or contention cases.
- The working-tree conflict markers are resolved. The index remains unmerged
  because the owner prohibits `git add`; the driver stages the resolution.
- Changes remain uncommitted; the driver handles the commit/publication lifecycle.
