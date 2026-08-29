# OTV2-20260825-work-delivery-coordinator

\`\`\`yaml
task_id: OTV2-20260825-work-delivery-coordinator
title: Coordinate the post-blocker gameplay vertical slice and release the clean Durability successor
mode: COORDINATE
status: coordinating_durability_successor_release
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 162
pr: null
delivery_pr: 188
prior_merged_pr: 186
architecture_hold_main_sha: 007183ac7ef09dd4ae8d8f476d7ac943541d7d48
protected_main_sha: a171410de07c2dab718f52f780d4314bdcc53604
architecture_decision_pr: 190
architecture_decision_merge_sha: 2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
foundation_successor_issue: 192
foundation_successor_allocation_pr: 194
foundation_successor_allocation_merge_sha: 1063caf409af6cd4b25fa844e17a483b87e76ad6
registry_successor_issue: 193
registry_successor_merge_pr: 195
registry_successor_merge_sha: 9878d42a21815027ef88067bfc59f8b40e78b473
recovery_allocation_issue: 240
recovery_allocation_pr: 241
recovery_allocation_final_head_sha: ca012ff2b42eefe4f27075455f097199aea63f8f
recovery_allocation_merge_sha: a171410de07c2dab718f52f780d4314bdcc53604
ownership_correction_authority: Oteryn/Oteryn-Game#187 comment 5424765487
ownership_correction_scope: active Durability task status/provenance/blocker/no-write/next-action only; no worker or runtime change
initial_published_pr_head_sha: e205b0620d433733e306777ab8e491d471b62677
candidate_validation: PR #188 final head 0e26fa0c216cadf34ff5c83fa3be508f81106c41 passed exact-head governance, architecture, authority and merge-gate checks plus independent review
candidate_pr_state: PR #188 squash-merged as 29576afa621bbe6a46c51fa1117c94efb6c7a644; temporary delivery branch deleted
base_sha: 2e3b05e7e1e916bd3210ce2184ad7e23482f324d
head_sha: null
final_head_sha: 0e26fa0c216cadf34ff5c83fa3be508f81106c41
final_head_frozen_at: null
owner: ChatGPT Work Delivery Coordinator
created_at: 2026-08-25T23:13:10+02:00
updated_at: 2026-08-28T13:45:24Z
execution_budget_minutes: 720
large_budget_reason: coordinator lifecycle spanning independently reviewable lane allocations, integration, and closeout; no single worker owns the programme
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-interaction-lifecycle.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-ability-engine.md
  - docs/superpowers/plans/2026-08-25-oteryn-game-durability-journal.md
  - docs/superpowers/plans/2026-08-26-oteryn-game-ai-bootstrap.md
  - docs/agents/evidence/OTV2-20260826-wave-a-post-merge-review-reconciliation.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
public_contracts: []
depends_on:
  - Oteryn/Oteryn-Game#154
blocks:
  - P0: historical PR #212 branch ancestry contains destructive cross-scope commit cd808d396018832b632be26911105a36f0cb7a20 and unallocated restoration 73e17f418c63ec038f5aa7ef8f0888ac74b75aa2; it remains immutable evidence only
  - #167 and Server Seam remain fail-closed until the separately allocated clean successor completes its independent TDD, review and merge lifecycle
cross_repository_coordination_id: OTV2-WORK-DELIVERY-POST-BLOCKER
external_repositories: []
\`\`\`

The coordinator owns creation of each child task packet only through the
allocation-PR merge. Immediately after the protected-main allocation merge,
exclusive write ownership of each child task packet transfers to its named
worker for that worker's one branch/PR and returns/releases only at that
worker's closeout. The coordinator retains only the live allocation record,
the child plans and this coordinator packet; no concurrent write authority is
permitted.

## Outcome

Create the durable execution lifecycle for the post-blocker gameplay vertical slice. This coordinator records live readiness, allocates only path-disjoint lanes through separate merged allocations, integrates only exact-head verified deliveries, and emits \`ARCHITECTURE_ESCALATION_REQUIRED\` instead of inventing material architecture decisions.

## Current recovery authority — 2026-08-28

- PROVEN: recovery allocation PR #241 final head ca012ff2b42eefe4f27075455f097199aea63f8f passed exact-head governance, architecture, authority and merge-gate workflows plus native Codex review, then merged as protected main a171410de07c2dab718f52f780d4314bdcc53604.
- PROVEN: PR #212 remains Draft immutable evidence. Its only admitted reconstruction source is fb30fba2a888835dfc7cbde27f940b79d7bfe05d under the canonical #241 packet; no historical mutation is ratified.
- PROVEN: `OTV2-20260828-impl-durability-successor` is the separate successor task on `impl/game-durability-journal-recovery-240`; it receives the clean implementation lifecycle from the protected allocation merge, not from PR #212 ancestry.
- DERIVED: the sole current coordinator action is to dispatch that one successor from main a171410de07c2dab718f52f780d4314bdcc53604, retain #212, and wait for fresh successor-owned evidence before reconsidering Server Seam.

## Architecture and source of truth

- \`PROVEN\`: protected \`main\` is \`2e3b05e7e1e916bd3210ce2184ad7e23482f324d\` when this task was admitted.
- \`PROVEN\`: Issue #154 is closed completed; its packaged authority is canonical on this admission main.
- \`PROVEN\`: Issues #91, #93, #115, #116, #123 and #131 are closed completed. Their terminal state does not grant downstream runtime write authority.
- \`PROVEN\`: PR #98 merged as \`dc22e0da8efcc6f4458416191261063b295af5b4\`; its QA shell exists, while real gameplay Tier 1 and Tier 2 remain \`NOT_EVALUATED\`.
- \`PROVEN\`: PR #144 (\`c1020b2db62ecfa18c411bee56fa004430b28923\`) and PR #151 (\`2d0e951ce37c2e28773c22966bb816c00bebaa0a\`) are merged; the prior registry/Foundation-Cargo lease is released.
- \`PROVEN\`: Issue #139 is deliberately non-current until its exact Movement plan, Interaction, compatible Client, and real QA prerequisites are integration-ready.
- \`PROVEN\`: open PR #150 is a draft root-\`AGENTS.md\` governance change on a separate path; it owns no coordinator or Wave A implementation path.
- \`PROVEN\`: no open Wave A implementation PR or active Wave A path allocation exists on the admission main.
- \`PROVEN\`: Task 1 coordinator lifecycle merged as \`c57ddb5253cdfec126a768232d53f8a9bb292e3f\`; protected main readback confirmed it.
- \`PROVEN\`: Issue #164 is terminally resolved by owner decision merge \`a1a868dc3a7cbe5d3f6c2d3732038ae6cd5d4a3d\`, which accepts only a pure-local AI bootstrap; Issue #174 needs this fresh exact allocation before implementation.
- \`PROVEN\`: PR #186 is merged. Its recorded review claims remain historical; this coordinator hold does not revise them.
- \`PROVEN\`: PR #188 final head \`0e26fa0c216cadf34ff5c83fa3be508f81106c41\` passed exact-head governance, architecture, authority and merge-gate checks plus independent review, then squash-merged as protected \`main\` \`29576afa621bbe6a46c51fa1117c94efb6c7a644\`; its temporary delivery branch was deleted. Its initial published head \`e205b0620d433733e306777ab8e491d471b62677\` remains exact original admission provenance.
- \`PROVEN\`: the architecture-hold protected-main checkpoint \`007183ac7ef09dd4ae8d8f476d7ac943541d7d48\` had PR #182's shared prerequisite but no Durability worker PR or durable adapter delivery; the prior reconciliation readback at \`29576afa621bbe6a46c51fa1117c94efb6c7a644\` retained that no-delivery state.
- `HISTORICAL`: `impl/game-durability-journal@7ac06bd84a1a31fc9a3ea2560de8ae20cea96741` and its local documentation checkpoint are pre-incident provenance only; neither is a dispatch target.
- `HISTORICAL`: PRs #190, #194, #195, #199 and #210 completed the Foundation/registry predecessors. #192 and #193 are terminal and have no active worker/dispatch path.
- `PROVEN`: PR #241 replaced the invalid legacy Durability continuation with one clean successor allocation. The legacy branch is immutable evidence; the successor is the only Durability execution target.
- `PROVEN`: Server Seam remains `WAITING_DEPENDENCY` on the clean successor's independent merge, not on Foundation #192.
- `DERIVED`: Interaction, Ability and journal-only Durability have disjoint primary semantic paths. Their shared composition/Cargo surfaces remain serialized coordinator paths.

Governing authority:

- \`AGENTS.md\`
- \`docs/agents/AGENTS.md\`
- \`docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md\`
- \`docs/agents/prompts/OTV2_IMPLEMENTATION_COORDINATOR.md\`
- \`docs/agents/programs/OTERYN_V2_IMPLEMENTATION_EXECUTOR_DAG.md\`
- \`docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md\`
- \`docs/architecture/reviews/OTERYN_GAME_POST_BLOCKER_WORK_ORCHESTRATION_2026-08-25.md\`
- \`docs/superpowers/plans/2026-08-25-oteryn-game-work-delivery-orchestration.md\`

## Historical readiness matrix — non-actionable

This retained matrix records the pre-recovery state only. Every row below is historical and non-actionable: it grants no dispatch, merge-up, TDD or worker authority. Current delivery state is exclusively the recovery authority above, the archived allocation record and the separately owned successor packet.

| Lane | Historical readiness | Historical evidence | Historical original record |
|---|---|---|---|
| Interaction | \`COMPLETED_RELEASED\` | PR #172 merged as \`73f82e4864aa15ece50625bda8bac7868f779ba3\`; post-merge reconciliation is recorded | Issue #165 may close after this coordinator closeout merges and protected-main readback confirms the evidence. |
| Ability | \`COMPLETED_RELEASED\` | PR #171 merged as \`2faa280b406a313d02ee1330c65651bc36e215a9\`; post-merge reconciliation is recorded | Issue #166 may close after this coordinator closeout merges and protected-main readback confirms the evidence. |
| AI | \`COMPLETED_RELEASED\` | PR #178 merged as \`cb9c5f4f53dd880c9d338dafd21b6184a4419993\`; post-merge reconciliation is recorded | Issue #174 may close after this coordinator closeout merges and protected-main readback confirms the evidence. |
| Foundation successor | \`CONFLICT\` | #187 is resolved by PR #190 and #194 is merged, but the worker-owned #192 packet/live allocation still say \`waiting_allocation_merge\` / \`allocation_pending_merge\` | worker #192 must reconcile its own active packet from current `main`, then execute TDD; this coordinator packet grants no execution authority |
| Durability | \`WAITING_DEPENDENCY\` | #167 remains fail-closed while #192's worker-owned allocation-state conflict is reconciled and its Foundation boundary later integrates | no write authority until #192 integrates and #167 receives fresh resume authority |
| Server Seam | \`WAITING_DEPENDENCY\` | #167 cannot release the durable adapter until the #192 conflict is reconciled, Foundation integrates, and #167 is freshly resumed | not released by #167 |
| Client | \`WAITING_DEPENDENCY\` | Server Seam remains blocked by #167 \`WAITING_DEPENDENCY\` | no compatible production Server Seam is merged |
| Movement | \`WAITING_DEPENDENCY\` | #139 remains non-current and its Client/Server Seam dependency chain is blocked by #167 \`WAITING_DEPENDENCY\`; real QA is also not integration-ready | no dispatch authority |
| Combat | \`WAITING_DEPENDENCY\` | Movement and its Client/Server Seam/Durability dependency chain is blocked by #167 \`WAITING_DEPENDENCY\` | no dispatch authority |

## Acceptance criteria

- [x] GitHub-backed protected-main, Issue, PR and active-task preflight completed.
- [x] Completed blocker work is reconciled without reopening #93, #115, #116, #123 or #131.
- [x] Current Wave A candidate readiness is classified with no implicit worker authority.
- [x] Task 1 coordinator lifecycle passed exact-head governance lifecycle and merged as \`c57ddb5253cdfec126a768232d53f8a9bb292e3f\`.
- [x] Task 2 allocation PR #168 merged exact, path-disjoint authority before any mutating lane worker was dispatched.

## Excluded scope

No gameplay/runtime, Cargo/workspace, registry, stable-ID, public-contract, workflow, production, protected-environment, secret, live account/session/data, Platform, Atlas or META mutation is authorized by this coordinator. Task 2 is authorized to create and maintain only the exact coordinator documentation that allocates path-isolated workers; it does not itself implement runtime behavior, and worker mutation remains deferred until this allocation authority merges.

## Implementation / findings

Task 1 established Issue #162 and merged the coordinator-only packet from fresh protected \`main\`. The previous next-wave blocker coordinator is terminal historical evidence and is not resumed. Task 2 created readiness-backed Issues #165, #166 and #167, three child plans/task packets, and Issue #164 as the required AI architecture escalation. Owner decision merge \`a1a868dc3a7cbe5d3f6c2d3732038ae6cd5d4a3d\` closed #164 and authorized only the separately allocated pure-local AI bootstrap through Issue #174.

PRs #171, #172 and #178 are merged and their packets released. PRs #190, #194, #195, #199 and #210 are terminal historical predecessor evidence; none creates a current #192/registry dispatch path. PR #241 is the current Durability recovery decision: legacy #212 remains immutable evidence, the separate clean successor is the sole Durability execution target, and Server Seam remains fail-closed until that successor independently merges.

## Validation

### Focused

- command/run: \`python tools/agents/validate_governance.py\`
- result: PR #188 final head \`0e26fa0c216cadf34ff5c83fa3be508f81106c41\` passed exact-head governance, architecture, authority and merge-gate checks plus independent review before squash merge.

### Component/integration

- command/run: \`python tools/repository/validate_repository_policy.py\`
- result: baseline-only LICENSE canonical-text mismatch was outside PR #186; no repository-policy change is made by this architecture hold. PR #188 is merged; its historical protected-main merge result is \`29576afa621bbe6a46c51fa1117c94efb6c7a644\`.

### E2E

- scenario: \`NOT_APPLICABLE\` — coordinator-only governance/task lifecycle creates no runtime behavior or user journey.
- result: \`NOT_APPLICABLE\`

### Exact-head CI

- initial published head: \`e205b0620d433733e306777ab8e491d471b62677\`
- final exact-head: \`0e26fa0c216cadf34ff5c83fa3be508f81106c41\`
- authoritative source: PR #188 final-head checks and merge record
- trigger source: pull_request
- workflow/run/job: exact-head governance, architecture, authority and merge-gate checks for PR #188 final head
- runner assignment: GitHub-hosted workflow policy
- classification: coordinator lifecycle reconciliation
- result: PASS; final head was accepted and squash-merged to protected main

## Self-review

- initial reviewed head: \`e205b0620d433733e306777ab8e491d471b62677\`
- final reviewed head: \`0e26fa0c216cadf34ff5c83fa3be508f81106c41\`
- authoritative source: PR #188 final-head review evidence
- method/reviewer: coordinator full-diff review
- material findings: none on the final PR #188 head
- verdict: PASS; final head accepted for merge

## Independent review

- required: exact-head non-authoring review before merge
- initial reviewed head: \`e205b0620d433733e306777ab8e491d471b62677\`
- authoritative source: PR #188 final-head review evidence
- method/auditor: independent task review
- material findings: none on final head; bounded lifecycle scope remained authorized by Issue #187 comment \`5424765487\`
- verdict: PASS; final head independently reviewed and merged

## PR and closeout

- changed-file review: PR #188 final head \`0e26fa0c216cadf34ff5c83fa3be508f81106c41\` passed independent review
- unresolved review threads: none reported at merge
- related/superseded PRs: #168 allocation; #171, #172 and #178 deliveries; #181/#185 leases; #182 shared integration
- protected auto-merge: not used
- merge commit/result: PR #188 final head \`0e26fa0c216cadf34ff5c83fa3be508f81106c41\` squash-merged as \`29576afa621bbe6a46c51fa1117c94efb6c7a644\`; that protected-main readback is historical
- ownership release: Ability, Interaction and AI are released; legacy Durability has no write authority, while the separate clean successor owns its post-allocation lifecycle; coordinator remains active and the vertical-slice programme is not complete

## Context checkpoint

\`\`\`yaml
last_progress: PR #241 merged the canonical clean Durability recovery allocation as main a171410de07c2dab718f52f780d4314bdcc53604; legacy PR #212 remains immutable evidence and the separate successor is ready for coordinator dispatch
status: coordinating_durability_successor_release
branch: null
head_sha: null
pr: null
recovery_allocation_issue: 240
recovery_allocation_pr: 241
recovery_allocation_final_head_sha: ca012ff2b42eefe4f27075455f097199aea63f8f
recovery_allocation_merge_sha: a171410de07c2dab718f52f780d4314bdcc53604
delivery_pr: 188
prior_merged_pr: 186
final_head_sha: 0e26fa0c216cadf34ff5c83fa3be508f81106c41
final_head_frozen_at: null
historical_pr188_ci_trigger_source: pull_request
historical_pr188_ci_check_generation: final_0e26fa0c216cadf34ff5c83fa3be508f81106c41
historical_pr188_ci_checks_for_final_head: passed_exact_head_governance_architecture_authority_merge_gate
historical_pr188_ci_run_ids: recorded_on_pr188_final_head
historical_pr188_ci_job_ids: recorded_on_pr188_final_head
historical_pr188_runner_assignment_state: GitHub-hosted workflow policy
historical_pr188_terminal_ci_wait_started_at: null
historical_pr188_terminal_ci_checks_for_final_generation: passed
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: create the one separately allocated successor branch from main a171410de07c2dab718f52f780d4314bdcc53604; do not dispatch legacy PR #212
blocker: clean successor has not yet produced its mandatory test-only RED generation, implementation GREEN generation, fresh exact-head CI or fresh independent review
next_action: dispatch OTV2-20260828-impl-durability-successor on impl/game-durability-journal-recovery-240 from the recorded merge SHA; retain PR #212 and recompute downstream readiness only after the successor merges
\`\`\`

## Current live control-plane checkpoint — 2026-08-29

This section supersedes the older operational checkpoint above for current execution. Historical fields remain preserved for provenance only.

```yaml
protected_main_sha: 0135deb100109b910dada366d7a1b05484357e51
parent_issue: 162
status: REVIEW_RECONCILIATION_REQUIRED
current_durability_issue: 250
current_durability_pr: 252
current_durability_allocation_pr: 251
current_durability_head_sha: 99b1c13ebfe4edef120e5c89b3c3bf9dfe15114d
current_durability_base_sha: 0135deb100109b910dada366d7a1b05484357e51
current_durability_behind_by: 0
current_durability_pr_state: draft_unmerged
current_changed_path_count: 12
historical_source_pr: 243
historical_source_authority: read_only_evidence_only
server_seam_issue: 247
server_seam_state: WAITING_DURABILITY_MERGE
work_owned_integration_guard: PRRT_kwDOT8SzxM6dX3eH
owner_action_required: none
blocker_owner: "Oteryn: sol durability lead"
blockers:
  - P1: retained reconnect-attempt budget is not preserved across terminal GameSession replacement under the same ControlLossEpoch
  - P1: Foundation final revalidation cannot represent genuinely fresh current authority before V2 COMMIT
qualification_evidence:
  rust_workspace_run: 33240209169
  merge_gate_run: 33240263818
  agent_governance_run: 33240263805
  architecture_semantic_audit_run: 33240209246
  merge_authority_audit_run: 33240209170
  whole_diff_self_review: PASS_on_99b1c13ebfe4edef120e5c89b3c3bf9dfe15114d
  codex_review: CHANGES_REQUIRED_2xP1
work_mutation_authority_now: control_plane_docs_only
worker_branch_mutation_by_work: forbidden
server_seam_release: forbidden_until_durability_merge_readback
next_action: "Wait for Oteryn: sol durability lead to repair both P1s within merged #251 authority, preserve fresh RED -> GREEN evidence, freeze a new exact head, regenerate exact-head CI/self-review and fresh lane-owned Codex with zero blocking findings/required lane threads, then return READY_FOR_INTEGRATION to Work. Work then performs independent exact-head preflight, resolves only its own integration guard, expected-head squash-merges #252, reads back protected main, reconciles #250/#167/#243, and only then creates the fresh Server Seam #247 allocation."
```

Durable GitHub control-plane mirrors for this checkpoint:

- Issue #162 comment `5462518299` records the current Work classification and the two blocking P1s.
- Issue #167 comment `5462519470` supersedes stale `READY_TO_RESUME` / historical #243 execution wording and points current execution to #250/#252.
- Issue #247 body now names Issue #250 / PR #252 as the current prerequisite and keeps Server Seam fail-closed at `WAITING_DURABILITY_MERGE`.
