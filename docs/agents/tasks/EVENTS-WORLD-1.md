# EVENTS-WORLD-1 — world changes, Fury Gates, Nightmare Isles, hazard (skeleton task record)

```yaml
task_id: EVENTS-WORLD-1
title: "EVENTS-WORLD-1: world changes, Fury Gates, Nightmare Isles, hazard (skeleton)"
mode: IMPLEMENT
status: waiting
pr: null
repository: Oteryn/Oteryn-Game
base_branch: main
branch: skeleton/events-world-1
base_sha: efecd2209e71e209878ac543cd70966c6300e8dd
issue: 1622
wave: 5
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
created_at: 2026-10-15
plan_row: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4 Epic 4 row EVENTS-WORLD-1 (rows 103, 104); §5 Wave 5"
class: "hard (child graph)"
needs_migration: maybe
needs_registry: maybe
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
owned_paths:
  - "crates/gameplay/world-change/src/ (new) + tests"
  - "content/encounters/{world-changes,mini-world-changes}/ source inputs"
  - "docs/agents/tasks/EVENTS-WORLD-1.md"
  - "apps/game-server/src/lib.rs (D1287 cfg(test) world_change registration only)"
  - "docs/architecture/ADR-0021-world-map-runtime-loading.md (D1136/D1287 amendments and status)"
  - "docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md (D1136/D1287 amendments and status)"
  - "docs/architecture/reviews/OTERYN_GAME_EVENTS_WORLD1_DECISION_2026-10-15.md (D1287 W15 source locator correction)"
  - "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md (EVENTS-WORLD-1 row brief reference only)"
slot_only_paths:
  - "regenerate_content.py outputs, content/project.json, manifest.json, content.lock.json, content/world/pins/*, Rust pins"
  - "migration if durable"
  - "docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json"
  - "Common slot-only set (any that the task touches): tools/content-migration/regenerate_content.py outputs, content/project.json, content/manifest.json, content.lock.json, content/interactions/index.json, content/world/pins/*, Rust pins, docs/contracts/*REGISTRY*.json, migration numbers, wiring in crates/gameplay/dispatch/src/gameplay_transport/{connection.rs,mod.rs,capabilities.rs} and crates/protocol-oteryn/src/lib.rs"
```

Tracking audit (CARDS-AUDIT-1, 2026-10-23, `origin/main@ac3f7fbc6`): **PARTIAL**. This supersedes historical status/PR-wait notes below; acceptance and scope are unchanged.

Merged evidence: [#2178](https://github.com/Oteryn/Oteryn-Game/pull/2178). Fury Gates/Nightmare Isles selection-source compiler is merged and rejects unavailable W1 bindings. Evidence: `crates/gameplay/world-change/src/content.rs`.

Remaining: Executable W1 bindings, layer/activation owners and restart/epoch runtime composition remain; source selection alone activates nothing. `pr: null` means this audit identifies no canonical unmerged candidate; merged slices are linked above.

## D1287 review repair — source only

Source base is CP's main-merge head `610d60cdf0f9c11618d085152a683293ddf69448`.
The published `#[cfg(test)] mod world_change;` mount now makes all five component
cases part of native library tests. The earlier five PASS came from the disclosed
private mount at exact source ded94838, not published integration. The acceptance
checkbox remains open until CI passes on this repaired head. Exact source
`c410ebf60cd58aee762fd5164322840abed309ee` compiled through the canonical
native library root, and all five exact selection-source cases PASS without
private mounts or wrappers. Actual inventory: 3259 tests; the source occurs once
in native dep-info. ELF SHA256:
`11631e4177976817d25f69a83d75023ee787bee2beae37def8d1ec4ce1de8c7d`.
Evidence is retained outside the checkout in `evidence/2178-D1287-native/`.
The publication adds only this task-documentation bridge to tested c410ebf6.
Fmt/diff and exact-head governance PASS; strict/all-target/full runtime checks
remain NOT_RUN. The refusal is `WorldChangeBindingsUnavailable`.
ADR/BOSS status lines now record the D1136 amendments; source-only W1 selection
scope and pending separate LAYER/ACT mechanics remain unchanged. The PR body must
name the final published repair head, replacing the stale 95bcebd7 reference.

### Exact-head CI receipt (2124c58d)

Exact head `2124c58d2b3895d301a4b7b219610eb4043c0bb9`: check run
`Merge gate / Rust changed-crate Clippy and unit tests` (id 117302016245,
https://github.com/Oteryn/Oteryn-Game/actions/runs/39000354207/job/117302016245)
concluded success. The six published-mount selection-source cases (the earlier five
plus `partial_binding_gap_set_refuses`) run in that job; this supersedes the five-case
PASS recorded above at older heads. No local run is claimed. If the Rust is unchanged,
the PR's next head is doc/data-only relative to 2124c58d (this task record and the
fury_gates selection-source provenance pin).

## Goal

World changes, Fury Gates, Nightmare Isles and hazard events.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State | Gate |
|---|---|---|
| EVENTS-RAID-1 | NOT MERGED | **BLOCKING** |
| HAZARD-1 (LEVER-1, COND-1) | NOT MERGED | **BLOCKING** |
| brief child graph for world changes | architect confirms (no dedicated child in App. 7) | **BLOCKING** |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

## Owned paths (builder edits freely)

- crates/gameplay/world-change/src/ (new) + tests
- `content/encounters/{world-changes,mini-world-changes}/ source inputs`
- `apps/game-server/src/lib.rs` — D1287 test-only registration.
- `docs/architecture/ADR-0021-world-map-runtime-loading.md` — D1136/D1287 amendments and status.
- `docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md` — D1136/D1287 amendments and status.
- `docs/architecture/reviews/OTERYN_GAME_EVENTS_WORLD1_DECISION_2026-10-15.md` — D1287 W15 source locator correction.
- `docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md` — EVENTS-WORLD-1 row brief reference only.

## Slot-only paths (ONLY in the final commit at the merge-queue turn, after merging main)

- regenerate_content.py outputs, content/project.json, manifest.json, content.lock.json, content/world/pins/*, Rust pins
- migration if durable
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json
- tools/content-migration/regenerate_content.py outputs; content/project.json; content/manifest.json, content/world/manifest.json; content/content.lock.json, content/world/content.lock.json; content/interactions/index.json; content/world/pins/*; Rust pins
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- crates/gameplay/dispatch/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring; crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants; module registration in lib.rs / durability/mod.rs / domain/mod.rs

To compile on the branch the builder may add only module registration lines in shared `mod.rs`/`lib.rs`; they are re-applied in the slot commit after merging main.

## Wiring points (verified on origin/main 5742f30f3)

- raid scheduler (EVENTS-RAID-1)
- crates/gameplay/world/src/world_object_revert.rs
- crates/gameplay/world/src/world_clock.rs
- connection.rs command dispatch chain `else if command.command_type == COMMAND_TYPE_...` (USE_INTENT l.2481, CHAT_INTENT l.2735, NPC_TRADE_INTENT l.2766)
- capabilities.rs `GATED` table l.165, offered sets l.76/104/132 (new capability not offered until CP decides)

## Inseparable bundles (never split across PRs)

- content + regeneration + pins by one author

## Acceptance criteria

D1206 narrows this PR to selection-source. Activation, restart/epoch persistence,
full W1 bindings and concurrent runtime transitions belong to the future
LAYER/ACT mechanics and are NOT_APPLICABLE here.

- [x] Retain Fury Gates/Nightmare Isles source facts and provenance without fake W1 bindings.
- [x] Compile validated inputs into typed WorldChangeSelection.
- [x] Refuse unavailable W1 bindings with WorldChangeBindingsUnavailable.
- [x] Six native component cases (including `partial_binding_gap_set_refuses`) pass through the published mount; exact-head CI success at 2124c58d (see receipt).
- [ ] Required final exact-head repository gates and CP lifecycle before leaving draft.

## Validation commands

```sh
cargo test -p oteryn-game-server world_change
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
```

## Reference evidence pointers

- `docs/architecture/reviews/OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md`
- `docs/architecture/reviews/OTERYN_GAME_WORLD_INTERACTION0_DOORS_LEVERS_FIELDS_AND_WORLD_CLOCK_DECISION_2026-10-01.md`
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 5), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Fill only owned_paths until CP gives the queue turn; then merge main and add slot-only paths in ONE final commit.
- Push rarely and batched: one push per round of fixes (heavy builds per push); no status-only commits.
- Never mark the PR ready, never enable auto-merge, never post `@codex review`; CP does that.
- Max 2 fix rounds; after that CP closes and splits the task.
- Confirm every BLOCKING dependency on main before starting code that needs it.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.


## D1136 acceptance-amendment checkpoint

CP2131 comment6224930839 D1136 assigns this PR the ADR-0021 §4.2/§4.7
and BOSS-RAID-0 §4.1 amendments required by D1053. They now carry W2/W4
atomic reset activation and immutable layer composition/restart behavior, plus
W6's durable World Change prerequisite for raid scheduling. The accepted brief
header records D1052–D1057/owner acceptance without claiming protected-main
integration. No runtime, schema, RNG registry, content pin, map format or other
owner's implementation changes in this checkpoint.

Status NOT_FULLY_QUALIFIED: CONTENT source population/validation, separate
WCHANGE-LAYER-1 and WCHANGE-ACT-1 allocations, runtime/announce integration
and mechanic qualification remain NOT_RUN. LAYER/ACT require separate CP PRs
under D1057; this builder creates none. EVENTS-RAID-1 gates RT, not this
amendment authoring. Nomads uses accepted weight1 per camp; Bank Robbery is
outside v1, and traps alone require HAZARD-1. The checkpoint follows D1122.

## WCHANGE-CONTENT-1 selection-source authoring (local successor)

D1054/D1057 authorizes owned CONTENT work on #2178. The new
`world_change/content.rs` and two `selection-source.json` documents retain the
accepted Fury Gates343/ten city variants and Nightmare Isles1000/three named
variants, all weight1. Per-field provenance names the accepted brief at published
`0bbfbf5856b5bacd722b51d8f22f8a13f663b67b` and pinned Canary reference
`04b83b512114bfd888000d6e1433ed8ecaec7c5b`; full URLs and exact source SHA256
are retained in each document. Canary's startup1000 rule does not replace the
accepted Fury chance343. Reference map paths are evidence locators only.

This is a proposed **selection-source candidate schema**, not the complete W1
World Project family or a native layer/definition. It explicitly carries unresolved
native geometry, target House membership, value-source multiplicity, spawn/NPC/
raid bindings, announcement text and compiler/bundle binding. Empty completed
gameplay arrays, guessed placements/IDs, fake qualified references or implicit
multiplicity are not substituted for those sources. Directory population states
remain READY_UNPOPULATED for the complete admitted family.

The typed source validator checks W14 ceilings256/16, supported trigger shape and
chance/date values, unique keys/variants, positive non-overflowing weights,
per-field source/hash shape and explicit unresolved bindings. It neither fetches
nor attests provenance and does not parse unbounded files or admit a bundle.
Existing bounded World Project capture/compiler integration remains required.
Five authored tests cover the accepted two source documents, malformed bounds,
identity/weight faults, unknown fields, missing provenance, false completion and
the WorldChangeBindingsUnavailable refusal (added in D1206).
Source validation does not prove exclusive-layer composition, House safety,
spawn limits, native compilation, activation or gameplay behavior.

Cheap checks: JSON syntax/accepted selector facts/provenance digest binding,
explicit Rustfmt of all three owned Rust files and `git diff --check` PASS.
At the original four-test source checkpoint, Rust compilation, strict lint and
gameplay qualification were NOT_RUN; exact later component evidence is retained below. Status remains
NOT_FULLY_QUALIFIED; production acceptance **RED** pending the missing bindings
and LAYER/ACT plus later RT/ANNOUNCE. No shared root registration, generator,
shared schema, manifest, content pin, wire/RNG/activation or other owner's source
is changed. The source successor was first retained locally for review; later qualification
and its metadata bridge are recorded below. No review trigger or CI restart is used.


### Native selection-source component qualification

Qualified Rust/JSON source: `a08f37a7bd474f230aab86fc9ceb25280c924979`.
This record-only bridge preserves all three Rust and both JSON hashes. A private
full actual crate adds only `#[cfg(test)] mod world_change;` to the real server
root; canonical wiring remains absent and no source-copy harness is used.

Actual command:
`cargo test --offline --locked -p oteryn-game-server --lib world_change::content::tests:: --no-run --message-format=json`

The real Cargo lib-unit artifact compiled successfully (fresh=false); its native
inventory has 3,183 unique names and exactly four selected tests. Each literal
`--exact` invocation passed one test, zero failed/ignored. Actual compiler dep-info
resolves each of the six mounted source inputs once, including both JSON files.
The retained gzip was decompressed and its ELF independently verified against SHA256
`ffc4f60d054d2b8a9571d6ff204a249aa969f739126d509c2a618271588d1655`.
Retained gzip bytes have SHA256 `f09282f5f2fa03b002c6adc638463e8cd0c604e9c0b63f1abfdbe746e85c9e98`.
Independent source/plan and actual outcome review PASS; no own-source compiler
warnings were emitted. Outcome receipt SHA256: `50d1b281ff8fbcbdd91a57e23a9261d78cb7ab67df66d0c09466ac3d73666071`.

The first native compile ended with LLVM out-of-memory/SIGABRT under an imposed
8GB virtual-address limit; runtime did not run in that attempt. One unchanged-
source attempt used 16GB virtual address space, retaining the 8GB process-group
RSS cap, disk/memory-availability guards and ten-minute timeout. It compiled
and ran the four tests successfully; sampled RSS peaked at 6,784,978,944 bytes,
with no guard abort. First-failure logs and the actual measured sampling cadence
remain retained. An initial receipt check assumed absolute dep paths; normalizing
actual relative/parent paths corrected the receipt without rebuilding or rerunning.

These four PASS qualify the selection-source parser component only. They do not
qualify complete W1 compiler admission, layers, House/property safety, activation,
spawns, announcements or gameplay. Strict all-target Clippy, whole-package tests
and integrated runtime remain NOT_RUN. Status stays NOT_FULLY_QUALIFIED.
The earlier NOT_RUN entries describe the source-authoring stage, superseded only
for this exact normal native compilation and these four tests. D1122 permits a
coherent reviewed source checkpoint with these limits; the draft remains open.


### D1206 scope and typed compiler refusal (source authoring)

CP2131 comment6225820327 supersedes the earlier full-W1 delivery gate: #2178
ships typed selection-source only; full W1/native schema ingress, layers/House/
value-source and spawn/activation belong to future LAYER/ACT skeletons opened by
CP after #2178 merges. Full integration is NOT_APPLICABLE to this PR, rather
than NOT_RUN. No empty gameplay arrays or new ingress hunk is added.

`compile_selection_source` validates before constructing the private-field
`WorldChangeSelection`. Its `require_w1_bindings` boundary returns the typed
`WorldChangeBindingsUnavailable` for both retained source entries; source-only
compilation succeeds without requesting gameplay bindings. A fifth test covers
that refusal and prevents malformed input becoming typed selection. New test
execution remains NOT_RUN at this source checkpoint; the previous four native
PASS remain exact to a08f37a7. Current source review/qualification is pending.


### D1206 native refusal qualification

Qualified source commit: `ded94838925731cf504804912d1d70845481d273`.
One actual private native crate compiled this exact source with the two-line
cfg(test) root mount. It also mounted exact Weather026085f7 and Reset0c745974
owned inputs; no A2 proposal or fake source harness was used. Actual Cargo command:
`cargo test --offline --locked -p oteryn-game-server --lib --no-run --message-format=json`.

Compile exit0/fresh=false; actual unique inventory3,187. Five literal Events
`--exact` invocations each passed1/failed0/ignored0, including the typed W1
refusal. Actual dep-info resolves all nine mounted Rust/JSON inputs exactly once;
source-at-close hashes match the freeze. ELF SHA256
`89fb3206f720c42b168379022f37198e77b8b4785638ec6b0813e6589fe883d8`,
retained gzip SHA256
`57aab3f9a153bbb5eea4fd333ca46bf261a9f76140e1cdf70c2b4854cc9558a6`.
The same artifact's three Weather loader tests passed separately. A first compile
failed on two Weather test Debug assertions; those logs remain retained, and
only the corrected Weather test source changed for the successful attempt.

Resource guards observed peak own-groupRSS6,775,611,392B, minimum disk
2,386,309,120B and MemAvailable2,534,297,600B, with no abort. No PG job ran.
This record bridge changes no qualified Rust/JSON. New strict all-target/whole-
package gates remain NOT_RUN; full W1/runtime integration is NOT_APPLICABLE to
#2178 by D1206. Draft/NOT_FULLY_QUALIFIED is retained for CP's final gates.
