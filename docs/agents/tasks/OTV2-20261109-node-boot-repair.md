# OTV2-20261109-node-boot-repair

```yaml
task_id: OTV2-20261109-node-boot-repair
title: Repair and qualify the real-World Node boot path
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/node-boot-world-followup-20261109
pr: null
base_sha: fb031b8ddf0bf1349ef7944b0584c744a17dd34b
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-node-boot-repair-20261109
created_at: 2026-11-09T13:00:08Z
updated_at: 2026-11-09T16:51:10Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - .github/workflows/merge-group-gate.yml
  - .github/workflows/node-boot-qualification.yml
  - .github/workflows/pr-postgres-harness.yml
  - apps/game-server/Cargo.toml
  - apps/game-server/src/qualification/client_phases/full_e2ef7.rs
  - crates/gameplay/dispatch/src/gameplay_transport/creature_field.rs
  - crates/gameplay/dispatch/src/gameplay_transport/monster_profile_cycle.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/creature_field_bundle.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/loop_monster/pass_cost.rs
  - crates/gameplay/world/src/movement/monster_map.rs
  - tools/qualification/node_boot/run.sh
  - tools/qualification/node_boot/test_diagnostics.py
  - tools/repository/test_heavy_gate.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/OTV2-20261109-node-boot-repair.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome and current generation

Complete the owner's requested Node boot repair, with one canonical writer and
three parallel evidence/implementation workers. The actual Node boot leaf and
its full-World final `NODE_BOOT_RESULT=NODE_BOOT_PASS` are required. Informational
aggregate success cannot close this task.

PROVEN: prior PR #3069 head `b029df4a40b4c74996c0a6eada3c766ee2627b16`
passed Rust (9,800 tests), all 24 PostgreSQL partitions (5,216 executed tests),
focused checks and independent review. Node boot job `129207003632`, run
`42631014207`, passed only its basic seam and was cancelled after about 60 minutes.
Full World hit S11's 180-second journey deadline, then transport/cleanup errors;
its final result and runtime node log were absent. No full Node boot PASS exists
for that candidate. The exact cancellation actor/mechanism remains UNKNOWN.

PROVEN: GitHub shows #3069 merged outside this repair session at 15:07:33 UTC,
as `f4249e005c9ab6a7b6b54adced7ac3546092b672`. The coordinator did not initiate
that merge. This task returned explicitly to AUTHORING on a new branch from
`91786a1013c129daa579a5f436f8adf5d321ab20`. A normal fast-forward then incorporated
main `fb031b8ddf0bf1349ef7944b0584c744a17dd34b`, whose sole change from that base
reverts the client rodio update and its lockfile entries. Earlier validation is
historical and does not qualify this new
candidate. The original SQL grant, immutable-result test and scheduler repair
remain intact on main.

## Accepted findings and bounded repair

1. PROVEN: ordinary passive creatures are rejected by native AI admission but
   still consumed by `tick_creature_fields`. AI-only prefetch misses their cells.
   Prefetch the whole field census without changing which creatures receive
   condition ticks. Add a sparse 512-creature mixed rat/deer cost case with the
   unchanged Ground-scan bound and explicit passive AI refusal.
2. PROVEN: pickupable stacks deliberately make spell tile facts Unknown, while
   retained PZ remains independently known. The field consumer currently refuses
   the whole pass on that lookup. Existing logout code and WorldBase contracts
   already qualify retained zones independently. Read only PZ through the same
   pinned Map/static-owner boundary; leave Unknown collision/spell facts and
   mutable-source receipt requirements unchanged. Cover Bundle PZ true/false,
   real residual condition damage/cursors, Source mutable stacks, clipping,
   independent pin mismatch, absent cells and invalidated transactions.
3. PROVEN: raw Light/PvP/Forge clients omit the boot-required capability closure.
   Selecting that closure requires map domain 17 instead of legacy domain 2 and
   inventory/container domains. Decode them with existing native codecs and
   accept only correlated, bounded replacement snapshots. Preserve all journey
   assertions. Exercise real capability selection and encoded positive/negative
   snapshot/delta transfers.
   Light no longer declares the deliberately unoffered presentation capability
   23; its EntityLights, mana, cooldown, decay and relog oracles are unchanged.
   The static boot-offer test accounts only for Forge's exact dynamic 25/104
   additions, which the existing appearance owner selects at session admission.
   Declare Tokio macros and paused-time support as server test dependencies so
   standalone package tests do not rely on workspace feature unification.
4. PROVEN: the 3,600-second job limit is shorter than the serial clock/light
   scenario envelopes (3,750 + 3,300 seconds), before build/basic/World setup.
   Set the three orchestration jobs to 240 minutes. No runtime, connection,
   transaction or individual scenario deadline changes. This is a practical
   healthy-run budget, not a claim that every report-only failure envelope fits.
5. PROVEN: the old harness removes the detached node log during cleanup, and
   reproduced process-group cancellation loses its stage failure diagnostics.
   The b029 cancellation mechanism and cleanup execution remain UNKNOWN.
   Stream bounded, allowlisted event evidence independently;
   flush before a single EXIT cleanup, preserve INT/TERM exit codes, and tag
   basic/full-World context. Retain S11's last logical query and progress outside
   its timed future. Exit-code/cleanup tests deliver signals to the harness.
   GitHub entry-shell cancellation or SIGKILL can retain only already-streamed
   evidence; a final cleanup marker is not guaranteed at that separate boundary.

DERIVED: repeated refusal of field passes can keep periodic work due and delay
client input. UNKNOWN: the cancelled b029 log does not identify the first S11
stall or prove the runtime contribution of either Map defect. Fresh physical
qualification must establish the repaired behavior.

## Authority and review

META 3.1.0 remains bound to `Oteryn/Oteryn` commit
`1bfb5ff98c8aa156e73669a14e083a1d464c29fb`. Game's integration override is normal
squash followed by the full post-merge gate and protected-main readback.

The complete follow-up includes a PROPOSED one-line audit pin rotation from
`a652e0a5e1e045c546d739e7e543df79b43e7dd3` to the exact new merge-group workflow
blob `24c8b5f32a5e086d40bfd2c863e721146d920730`. It is not active on protected main.
The audit's self-modification rejection, other pins, exact-head/base checks,
required fragments, forbidden behavior and rulesets remain unchanged. Its
expected rejection is not product-test success or permission to bypass controls.
Integration requires current explicit owner approval for this rotation and one
independent deep review of the frozen candidate. No such rotation approval is
claimed; first finish the concrete proposal, deterministic checks and review.

Map zone evidence is immutable geometry, not permission to use mutable collision
or Item facts. Its consumers retain current runtime scope/generation, content,
frame/map pins and physical-transaction validation. Negative tests vary clipped
cell, map pin, absent data, missing initialization receipts and SQL abortion
independently. Existing stale/foreign scope and transaction regressions remain.
No new authority grant, persisted schema, wire version or recovery interpretation
is introduced. Retry/deadline relaxation and test-result suppression are excluded.

Finding-family sweep covers AI and passive field consumers, Bundle and Source
Unknown tiles, both zone outcomes, map replacement framing and all three raw
observers, all Node boot entry points and both catchable cancellation signals.
The report-only character-state dependencies (partial S08 to S16a and P21 to
C19a) are separate fixture issues, not proven causes of the hard town failure.
Keep the hard town assertion and `profile_result` error propagation intact.

## Publication and validation

Normal Git push lacks configured authentication in this session. The root selects
the bound policy's API-native NEW-candidate route on the fresh canonical branch
`codex/node-boot-world-followup-20261109`: one writer, live predecessor/tree,
complete bounded delta, one commit with that predecessor as sole parent, one
non-force ref update and immediate readback. No evidence transfers from a prior
candidate. Any drift/ambiguous mutation stops for reconciliation. No deployment,
live-data, credentials, direct-main writes or external-repository mutations.

The committed record is a truthful authoring snapshot. The PR will bind the
actual freeze SHA, run evidence and review without moving the frozen head for
metadata. Complete archive closeout only after authorized merge and post-merge
qualification, under the existing archive-only exception when required.

- PASS: workflow budget regression demonstrated RED at 3,600 < 7,050 seconds in
  all three entry points, then GREEN; 12 heavy-gate tests passed.
- PASS: actual shell cancellation fixture, INT and TERM, retained bounded safe
  evidence before cleanup and final FAIL after an earlier basic PASS; 2 tests.
- PASS: capability regression failed on the old declarations with
  `MissingSupport([4, 11, 15, 18, 30])`, then passed after the repair. A subsequent
  full server-library run found the independently unoffered Light 23 declaration;
  the corrected focused module passed all 27 tests, including the S11 deadline
  and native map framing regressions. The earlier 502/503 server result is not
  represented as a successful final suite.
- PASS: World library tests, 234 passed and one PostgreSQL case excluded by name.
  This does not qualify the new configured-PostgreSQL scenarios.
- PASS: strict Clippy for World, dispatch and server with all targets/features;
  workspace architecture check; workspace formatting and explicit formatting of
  build-mounted test files. Debug information and incremental compilation were
  disabled for the 8 GiB local environment.
- PASS: repository policy (23 files/68 workflows), native PowerShell failure
  propagation and 47 workflow mutation canaries; governance (22 documents/9 lanes)
  and all 129 governance tests; complete owned diff and partition reviews.
- RESOURCE LIMIT: local dispatch-library test code generation was killed by the
  8 GiB cgroup OOM limit. That execution is not a test PASS; CI must run its suite.
- PENDING: fresh exact-head validation after API publication, independent
  frozen-head review and actual Node boot.
- NOT RUN locally: configured PostgreSQL 18.6 and full Docker/Platform Node boot;
  Docker is unavailable and package installation cannot perform UID/group setup.
  Existing CI is required. Unconfigured PostgreSQL early returns are not evidence.

## Acceptance

- [ ] Both Map regression families pass against configured PostgreSQL 18.6.
- [x] Actual raw observer support, framing and S11 diagnostics regressions pass locally.
- [x] Final integrated affected Clippy/architecture checks and governance pass locally.
- [ ] Required independent review completes on the frozen head.
- [ ] Actual full-World Node boot finishes with PASS.
- [ ] Owner approves the exact proposed audit rotation before integration.
- [ ] Normal authorized squash, full post-merge gate and main readback pass.

Reference parity: NOT_APPLICABLE; this repairs existing runtime/qualification
contracts and adds no mechanic. Next: complete integrated validation, freeze a
reviewable candidate and obtain exact-head evidence before requesting integration
approval for the protected audit rotation.
