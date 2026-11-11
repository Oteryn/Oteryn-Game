# OTV2-20261110-wpkg-r2-repair

```yaml
task_id: OTV2-20261110-wpkg-r2-repair
title: WPKG-R2 repair, runtime qualification and native World completion
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/pr3076-complete-e48f
pr: 3106
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: PR3106 repair lead; sole publisher and review dispatcher in comment 6786929490
created_at: 2026-11-10T15:59:04Z
updated_at: 2026-11-11T13:19:01Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20261110-wpkg-r2-repair.md
  - docs/agents/tasks/archive/OTV2-20261110-wpkg-r2-repair.md
public_contracts:
  - docs/contracts/protocol-oteryn/v1/cast/cast-v1.md
  - docs/contracts/S17_CAST_CONSENT_PERSISTENCE.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implementation and authoring complete; integration and post-merge acceptance
pending. This is the task's final authoring record for PR #3106, not a claim that
the final candidate has passed CI, merged, or released ownership. The exact
bounded file allocation and remaining acceptance belong to live control
[6786929490](https://github.com/Oteryn/Oteryn-Game/pull/3106#issuecomment-6786929490);
the two owned paths here record only the final documentation move. Source
allocations remain those published in that control, with no additional writes
authorized by this record.

## Architecture and source of truth

- PROVEN: bound META policy `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`,
  version 3.1.0; Game remains the sole product write authority.
- PROVEN: the lead alone publishes and dispatches Codex review. Disjoint worker
  allocations, expected-head publication and exact readback precede FREEZE_SHA.
  A material repair requires explicit return to AUTHORING and fresh qualification.
- PROVEN: Game's override in `docs/agents/AGENTS.md` selects squash merge and the
  full post-merge gate on `main`; protection bypass and self-audit bypass are excluded.
- PROVEN: live PR/check/control receipts govern current state. A commit cannot
  name its own SHA; final head, review and run bindings are recorded externally.

## Retained history

| Cohort | Verified disposition | Evidence boundary |
|---|---|---|
| Original collector repair, #3042 | Externally merged as `f01f18582443f482b7adcfaf32cae3f9d3616876` | Historical control 6767805546; that merge did not establish successful full Node or post-merge qualification. |
| Separate protected pin rotation, #3103 | Merged as `2520d704c05f67d3f63c3637e62f8a8ae14fd304` | Approved workflow blob `f574c67416ff8323327cfc6ac0e5829a1c9cba92`; audit self-modification refusal retained. |
| Grouped P2/P3 follow-up, #3108 | Externally merged as `edf8272737d66bd050519de6c19e32dbf6ad0ddb` | Collected mechanical follow-up; historical backlog 6766071958 retains explicit deferrals. |
| Runtime, CI ledger and native-content continuation, #3106 | Final authoring record; acceptance pending | Canonical branch `codex/pr3076-complete-e48f`, control 6786929490. |

Earlier fixture, cast, collector and scheduling receipts remain in #3042 and
the prior revisions of this record. Their results qualify their own SHAs only.

## Implementation / findings

- PROVEN: valid PR file-count drift discards transported scope evidence. Exact
  protected-base Git recovery may classify lanes; unavailable recovery selects
  FULL. Repository, state, head and base identity refusals remain strict.
- PROVEN: the CI success-ledger repair requires a completed successful execution;
  a failed Clippy step cannot create reusable success evidence. Approved workflow
  pinning remains separate from the workflow's own protected audit.
- PROVEN: the runtime continuation includes the reviewed admission/recovery and
  foreground-holder work; failure diagnostics use closed labels and registered
  event fields. Diagnostics do not establish or replace a runtime success result.
- PROVEN: the PostgreSQL ancestry fixture observes its own command through
  two-stage PID/transaction barriers, retains the competing NOWAIT lock assertion,
  and rejects an unrelated blocked reader. Clock qualification retains the 75 s
  advancement, 8 s owed-delta and 120 s outer budgets and all ten actions.
- PROVEN: cleanup accepts only the defined accepted/rejected auxiliary TURN
  reply pairs; it still requires a final accepted logout. The five real TLS
  regressions and three selected clock regressions passed on predecessor `cd6d149a`.
- PROVEN: main's three content failures exposed noncanonical World bytes, four
  previously admitted Djinn services and the additional Thaian import batch.
  The repair uses the native writer on an explicit package, preserves the complete
  source cohort, and retains strict document, identity and service assertions.
  Standalone Thaian completion and normal regeneration both require native output.
- PROVEN on predecessor `f959dfb444e09eae82f8b592750c5d388ee19acd`: hosted run
  42994570833 reproduced two native eleven-document packages, retained identical
  bytes on a third completion, and generated the three native pin outputs. The
  package tree is `74ff1708c9f85dbe20997265430fff5e5b157fa7c47dbfc4e28cff209bfe8b13`;
  the native bundle digest is
  `61357aebac5cd74b2f0bea6c920455940c6488a609f4b64e1f492a135113d7ab`.
  The source checkpoint still failed its tracked-package, focused-test and pin
  comparisons; its oversized focused log was unavailable. Final committed-source
  qualification remains pending and must pass every required check.

## Complete dependent native closure

- PROVEN on predecessor `93684c40a787d6e1a79aa49dfc77cd6d41cfcbbd`:
  FAST run 43002115193 passed 11,025 distinct actually executed tests, excluding
  success-ledger reuse, plus actual Clippy and native bundle pin-check. Native
  WorldProject run 43002115364 reproduced the eleven documents but passed only
  47 of 48 focused tests; Thaian's separate Wiki binding was missing from the
  fixture census. ContentTree run 43002115033 separately rejected stale Prey inputs.
- PROVEN: native World serialization omits Thaian's empty immunity list and null
  reference-level field. The existing raw definition digest therefore changed;
  the native provider, Prey fingerprint, indexes and manifests required normal
  regeneration. The seasonal validator now accepts only the pinned packet's
  equivalent omitted-empty-immunity projection; source facts, required reference
  level and negative refusals remain strict. Eight regression tests cover that
  boundary, including the observed failing case before the fix.
- PROVEN: the World fixture independently verifies Thaian's exact Wiki source,
  revision, import batch and eight-field binding, requires one Thaian and one
  D44 binding, and retains all other identity and non-Wiki count assertions.
- PROVEN: isolated authoring checkpoint `f68a65d1`, run 43007567729, performed
  the actual 15-case / 777-creature native Prey simulation, then stopped because
  its shallow checkout could not resolve the existing StarterKit seal base.
  Its 27-file partial artifact was not accepted as completed generation.
- PROVEN: checkpoint `f1530f6208cd8467b600ed3457ccf3abea844bb1`, run
  43012720721 / job 130496071582, supplied bounded real history and passed the
  existing seal resolver: main `4e29529a`, base `f01f1858`, the original 239-byte
  ledger, one sealed label and no history errors. The complete existing generator
  succeeded without a provenance bypass, manual model repin or simulation change.
- PROVEN on that authoring workspace: all eight step outcomes passed; eight
  seasonal tests ran before generation and eight after it; Prey checked four
  outputs; registration checked 53 files; all five World repository tests passed
  with zero ignored tests. Native pin-refresh and pin-check agreed on bundle
  `8f578285e03020023b59ca54e3752830f1975b6a660e934ad57e9369572ddbc5`,
  inputs `9b65cddb29ecba139707a4b9c1c42a3fdfbd4973eedf48256ec9dc45695f9b81`
  and 33,657,032 bytes. The eleven World documents and entry_start were unchanged.
- PROVEN: completed artifact 13853567128 has 41 members and 29 changed generated
  paths; ZIP length 1,427,815 bytes and SHA-256
  `6c9502d44c9a602ac0503ca9932872554cffe7f41352cb38956515d6f4b2245a`.
  Each output's bytes, SHA-256 and Git blob were checked and independently reviewed.
  All 27 partial outputs repeat exactly; only the two native bundle pin files
  are additional. The provider blob is
  `3d2c1c301caceefda8cd29e2e3d4cf2562fa5cb8`, already stored by immutable
  transport run 43013807852 and exactly matched to completed generation.
- PROVEN: semantic review found only the native empty-default projection and
  source/hash propagation; all balance and simulation values remain unchanged.
  Pin and topology changes contain only the generated bundle digest update.
  The final successor contains these 29 outputs, the three reviewed source fixes
  and this archived record. Both temporary helper workflows are excluded.
- PROVEN: the managed workspace became unavailable before adoption. Root recovered
  an isolated checkout and the exact artifact on an explicitly authorized remote
  device for light checks; no production or live-account files were touched.
  Publication continues through the already selected GitHub control-plane route.
- PROVEN on the complete 33-path successor composition: governance validated 22
  required documents and nine lanes; all 129 governance tests passed; repository
  policy validated 23 files and 70 workflows; all eight seasonal tests passed;
  the two Python files compiled; Rust 1.99.1 formatting and complete-diff whitespace
  checks passed. Initial governance attempts identified missing sparse-checkout
  dependencies; exact unchanged baseline files were materialized before the
  successful checks. No source assertion was changed to satisfy setup. Final
  archive governance and whitespace checks are repeated before publication.

This authoring evidence does not qualify the committed successor. Final-head
fast, ContentTree, WorldProject, actual PostgreSQL/Node, applicable Codex review,
normal squash and full post-merge acceptance remain pending in live control.

## Quest evidence closure after native index regeneration

- PROVEN on `72cb2fcbb2723c3eb36a22358f7110d6e87c2722`: FAST run
  43021886412 completed all 15 jobs and 11,025 distinct actual nextest PASS cases
  (3,660 / 3,715 / 3,650), with no duplicate or failed case. All five selected
  logout TLS, three selected clock, seven native and five buffered-STEP regressions
  executed once; all ten clock tests passed. Strict Clippy actually ran 67 packages.
  Separate 7 / 8 / 6 support-package ledger reuses are excluded from that test count.
- PROVEN on the same head: ContentTree 43021886442 passed 420 unittest executions,
  15 regenerator checks and the actual StarterKit seal. WorldProject 43021886269
  passed 48 focused integration tests, seven native CLI Rust tests, seven Python
  wrapper tests, two reproductions, idempotence, exact eleven-document comparison,
  native pins, Clippy and fmt. PIN_DRIFT was false; the pin report independently
  confirmed the unchanged `9b65cddb...` inputs and `8f578285...` bundle.
- PROVEN: Quest Authoring Schema 43021886321 found stale downstream evidence.
  Its checks part stopped at `quest_binding_authoring.py --check` on
  `npc/quest-npc-dialogue-links.json`; 120 unit-module summaries reported 1,192
  test executions with one talk-NPC packet equality failure. The donor part passed.
  Commands after the failed binding check were not yet qualified.
- PROVEN: the native regeneration changed the NPC and Dialogue index metadata
  hashes. The NPC binding packet retains both input hashes; its index retains the
  resulting output hash; the talk candidate packet retains the NPC input hash.
  Their underlying shards, candidate records and admission facts did not change.
  The normal content generator's documented scope leaves other authoring evidence
  to its corresponding producers, so this repair does not broaden that scope.
- PROVEN: in an isolated exact-source copy, the existing binding and talk generators
  reproduced both failures before writing. After regeneration both standard
  `--check` commands and all six binding controls plus eight talk tests passed.
  Exactly three generated JSON files changed, with four SHA-256 leaf replacements
  and identical byte lengths; the other 4,181 copied files remained byte-identical.
  No test, predicate, schema, source hold, selected branch or runtime admission was
  changed. Both prior failing cases are now positive in this authoring proof.
- PROVEN after canonical adoption: both standard producer checks, all six binding
  controls and eight talk tests, governance validation (22 policy documents and
  nine lanes), all 129 governance tests and complete-diff whitespace passed.
  Independent byte review reconstructed all three outputs using exactly four
  hash substitutions; eight other registered binding payloads remain identical.
  Final archive governance and whitespace are repeated before publication.
- PROVEN: main advanced to bdb4e0a64b77fe308caa45f8380af57f7b4b551b through
  the externally merged PR #3113, changing only the accepted delegation guidance.
  The prospective squash must retain that guidance and the protected main audit;
  the existing four-path repair allocation does not modify either file.

The three reviewed generator outputs and this archived record form one bounded
successor. Hosted Quest checks must execute the previously unreached commands,
and all required checks must bind the newly frozen source. Final Codex review,
actual PostgreSQL/Node, normal squash and full new-main acceptance remain pending
in live control; the preceding head's green results do not qualify the successor.

## High-risk authority/recovery qualification

Current node, session-generation, binding, lease and durable-root fences remain
the authority boundary. Historical terminal evidence cannot reacquire authority.
Cast retry remains bounded and rechecks node authority, bans and remaining lifetime.
The final review must retain the inherited cohort's negative, restart, retry,
replay, concurrency and PostgreSQL cases, plus the changed fixtures' real dispatch
and TLS evidence. No weaker assertion, added retry budget or ignored test is an
accepted repair. Exact case/run receipts remain pending for the final head.

## Acceptance criteria

The owner's later instruction to obtain **all tests green and merge** supersedes
this record's former blanket statement that Node is informational and the older
Node exception cited from comment 6745293631. Workflow classification alone does
not waive the owner's requested Node, integration or post-merge outcome.

- [ ] Final unchanged PR head: all required checks, `game-gate`, `heavy-gate` and
  all 24 PostgreSQL partitions pass with actual execution evidence.
- [ ] Node reaches `NODE_BOOT_RESULT=NODE_BOOT_PASS` with its required stages and
  the scenario verdicts required by live control 6786929490. Retain the actual
  clock, ranking and disconnect/resume row evidence. Explicitly nonblocking
  full-row coverage gaps remain reported gaps; this record adds no `W20b.full`
  PASS requirement. Real failing Node jobs or required stages remain blockers.
- [ ] Hosted **WorldProject canonical package**
  (`.github/workflows/world-project-package.yml`) reproduces two fresh completed
  packages, proves idempotence and exact eleven-document equality, and passes
  all three repaired integration targets and its existing checks.
- [ ] Native `pin-refresh` outputs are published and the final source passes
  `pin-check`; a runner's uncommitted generated files do not qualify the source SHA.
- [ ] The lead closes the complete changed-file review and the required Codex
  review binds the final material delta with no open concrete P0/P1 finding.
- [ ] Protected squash integration is eligible; the resulting `main` commit passes
  its full post-merge gate, including integration, Windows and dedicated seams.
- [ ] Residual P2/P3 findings have explicit grouped dispositions; ownership is released.

## Reference parity

NOT_APPLICABLE: repair of existing behavior, source serialization and qualification.
No new mechanic, reference value or public protocol change is asserted.

## Validation

Retained local checks below passed before predecessor `93684c40` was published.
They do not establish final successor qualification. The subsequent native-closure
repair has its separate evidence below; applicable light checks are repeated on
its complete composition before publication. The user requested light local
checks and hosted heavy/database validation.

- `python tools/agents/validate_governance.py` — PASS; 22 required policy documents and 9 lanes.
- `python -m unittest discover -s tools/agents/tests` — OK; 129 tests.
- `python tools/repository/validate_repository_policy.py` — PASS; 23 files and 70 workflows.
- `python -m unittest discover -s tools/repository -p 'test_job_result_key.py'` — OK; 31 tests.
- `python tools/repository/test_classify_pr_test_lanes.py` — PASS; 12 routing cases.
- `python tools/repository/test_validate_pr_gate_pg_sim.py` — PASS; 10 placement tests, routing and fan-in checks, 47 workflow mutations and four native failure positions with PowerShell 7.7.0 on Unix.
- `python tools/ci/test_merge_gate_scope.py` — OK; 3 tests.
- `python tools/qualification/node_boot/test_diagnostics.py` — OK; 9 tests.
- `python -m unittest discover -s tools/content-migration -p 'test_native_world_canonicalization.py'` — OK; 7 tests after materializing the unchanged sparse-checkout source packet.
- `python tools/content-migration/test_regenerate_content.py` — PASS; 15 checks.
- `git diff --check` — PASS. Complete candidate diff inspection: source, fixture,
  exact native-output hashes and main-adoption boundaries reviewed; final tree
  identity is recorded externally after publication.
- `python tools/content-migration/test_native_profile_composition.py` — OK; 6 tests.
- Pinned Rust 1.99.1 `rustfmt --edition 2024 --check` for the three changed content
  tests — PASS. Workflow YAML, 11 shell blocks and embedded Python syntax — PASS.
- `cargo fmt --all --check`; affected-package Clippy/tests, including dispatch's
  `--all-features`, and architecture workspace validation — pending; hosted execution.
- `cargo test --locked -p oteryn-game-server --test content_npc_catalogue` — pending; hosted execution.
- `cargo test --locked -p oteryn-game-server --test content_npc_service_model` — pending; hosted execution.
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository` — pending; hosted execution.
- WorldProject canonical package native writer/regeneration, eleven-document
  equality and regenerated-versus-committed pin equality — pending; hosted
  execution through `.github/workflows/world-project-package.yml`.
- Native bundle `pin-check` and its exact command/job receipt — pending in the
  selected World bundle qualification lane.
- PostgreSQL, Node, full integration and post-merge run/job manifest — pending.

## Self-review

The lead inspected the bounded source/test/workflow delta and exact native data
manifest. Independent review verified all eleven package hashes and three native
pin outputs, preserving entry_start and strict comparison results. Main's protected
audit is deliberately not copied into the PR delta: its self-modification refusal
remains intact, and prospective squash must retain main's approved audit blob.
Final remote tree readback and hosted acceptance remain pending in live control.

## Independent review

Required: YES. The historical clean review of `29bb` and earlier #3042 receipts
are retained only for their actual scope. The final material delta needs its
own applicable review receipt; exact reviewed SHA, findings and disposition are
pending in control 6786929490. Only the lead dispatches under the bound review
policy; no duplicate request or unapproved extra round is implied by this record.

## PR and closeout

- Canonical PR: https://github.com/Oteryn/Oteryn-Game/pull/3106.
- Final FREEZE_SHA and review/CI receipt: pending in live control 6786929490.
- Merge commit/result: squash merge of #3106 — pending, not yet claimed.
- Post-merge protected-main readback and full gate: pending.
- Ownership release: pending after verified integration and post-merge acceptance.
- Archive placement closes authoring under `tasks/archive/README.md`. It does
  not make an unmerged repair accepted. Do not move the frozen head merely to
  copy later CI/review state into this file.
