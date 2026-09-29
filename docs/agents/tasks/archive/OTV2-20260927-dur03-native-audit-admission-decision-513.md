# OTV2-20260927-dur03-native-audit-admission-decision-513

```yaml
task_id: OTV2-20260927-dur03-native-audit-admission-decision-513
title: Native MINT audit snapshot binding and staged admission decision
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/dur03-native-mint-audit-binding-513
issue: 513
pr: 1002
base_sha: a0ef47246903041d76810c230137cf2cc5bef4e0
head_sha: 91544690ef6453095a7240987f8603b912a576fd
final_head_sha: 91544690ef6453095a7240987f8603b912a576fd
final_head_frozen_at: 2026-09-27T16:53:20Z
freeze_reference: "Issue #162 comment 5857850828"
owner: /root/dur03_max_shape_writer-local-architecture-author
created_at: 2026-09-27T16:45:00Z
updated_at: 2026-09-27T17:08:44Z
completed_at: 2026-09-27T17:08:44Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/agents/tasks/active/OTV2-20260927-dur03-native-audit-admission-decision-513.md
public_contracts:
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and allocation

[Allocation 5857774964](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5857774964)
authorizes only this additive documentation candidate following
[escalation 5857709826](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5857709826).
§39.2 supplies native MINT snapshot/identity/compatibility binding for the next
schema/codec/item-profile work, while preserving later separate TRANSFER.
It does not accept production collection or an executable admission path.
Root is sole remote API publisher/control plane; local architecture author
cannot publish, review-trigger, canonicalize or integrate this decision.

## Architecture and source of truth

- `PROVEN`: protected base above; architect prompt v1.1, architecture discipline,
  DUR-03 §39.1, GAME-ITEM-01 native ItemTypeKey/revision and typed legality rules,
  ANL-01 semantic-envelope/exact-payload admission rules, existing item-specific
  P90D purpose/readers/deletion/holds/evolution. Bound META remains
  `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb` (3.1.0).
- `PROVEN`: integrated synthetic v1/v2 remain evidence only; current event/profile
  registry has only the Character family and no native DUR-03 admission binding.
  Shared ANL ceilings do not establish missing DUR-03 resource acceptance.
- `DERIVED`: MINT-first native binding separates actual preceding schema/codec/
  profile design from later Inventory-root representation without changing §39.1
  transaction sequence or owning materialization timing.
- `UNKNOWN`: actual native source/cause domain/eligibility, Gold Coin typed legality/
  max_stack, native direct-root legality/reference, accepted actual schema/event/
  profile/resource entries, physical SQL/runtime/atomicity/recovery evidence.
- `CONFLICT`: historical P90D integration-status wording is stale, not a missing
  owner selection. That historical document remains read-only in this allocation.

## Decision and limits

Must decide now YES: do not freeze private fixture mappings into native evidence.
Selected native complete MINT snapshot; rejected v2 promotion and unnecessary
coupling to all TRANSFER root choices. Reviewable supersession requires accepted
domain/compatibility/security/privacy changes or measured native encoding evidence.
No event/field IDs, source/root domain, key grammar, gameplay/resource number,
security purpose, registry entry or physical mechanism is selected. Root/profile
unknowns still gate their owning admission; audit cannot create source eligibility.
ANL payload bytes remain exact while equivalent envelope ordering is semantic,
not raw-byte, identity. Existing §39.1/history and P90D terms stay intact.

## High-risk authority/recovery qualification

`NOT_APPLICABLE` to production mutation: two documentation paths only; no
PREPARE/COMMIT, persistence/recovery consumer, controller installation or runtime
authority change. The contract explicitly preserves independent current facts,
immutable expected bindings, all-or-none mandatory evidence and later applicable
fence/DB/recovery qualification. No runtime proof is claimed here.

## Acceptance criteria

- [x] One additive §39.2 after §39.1; all original contract text preserved.
- [x] Native stable identity and complete MINT/Ground/source binding; unknown
  owning inputs remain fail closed and no synthetic canonical promotion occurs.
- [x] Later separate TRANSFER/root prerequisite and ANL byte/semantic distinction.
- [x] Existing item-P90D constraints; actual schema/profile/resource/physical
  admission remains gated; no product number or new identifier domain chosen.
- [x] Root-published exact-head freeze, applicable CI and independent review.
- [x] Normal protected integration; lifecycle archive separately allocated.

## Excluded scope

Schema/proto, registries, Content, runtime/example/evidence, foundation, Cargo,
SQL, foreign tasks, R7 P03, historical P90D-status repair, production, external
repositories and any integration authority. No old-head review/CI reuse.

## Authoring validation and self-review

Local authoring checks: exact two-path delta, append-only original-text equality,
whitespace, UTF-8/LF byte manifest and semantic consistency with unchanged owning
contracts. Parent receives exact final hashes externally after the last metadata
write; the task does not embed its own or a superseded candidate SHA.
Local whole-diff self-review: native identity/source/root boundaries, no numeric
promotion, no retention reopening, no blanket source legality assumption, exact
payload versus semantic Envelope comparison, no runtime/SQL admission authority.
No unresolved material authoring finding; final exact-head self-review is root's.

Focused checks for parent after publication/freeze:

```sh
git diff --check <base> <frozen-head>
python3 tools/agents/validate_governance.py
python3 tools/repository/validate_repository_policy.py
```

Prepublication checks are not candidate-specific CI. Runtime/compiler/E2E:
`NOT_APPLICABLE` to this docs-only candidate. Exact-head workflow/run/job and
independent review remain pending the final remote freeze. Known foreign stale
active packets at the protected baseline are not repairs or waivers in this task.

## Protected integration and terminal disposition

- PR #1002 source candidate `91544690ef6453095a7240987f8603b912a576fd` has sole
  parent `a0ef47246903041d76810c230137cf2cc5bef4e0`; root froze it under Issue #162
  comment `5857850828`; exact freeze time `2026-09-27T16:53:20Z`.
- Independent review is recorded in PR comment `5857883379`.
- Terminal Merge Queue workflow/group `36335231869`, `game-gate` check
  `108666180792`, completed for the queued candidate. PR #1002 merged to protected
  main `8b53439b716ff3a86c823c6a4795d15fd7e7444b`.
- Protected-main blob identities: DUR-03 contract
  `f77fa7365b930be7b1678d52cc1cac82b4d517e3`; this task record
  `aed2be69e1dc3d5eb69214da599e1288aa326195`. Root terminal/allocation comment:
  `5857988240`.

This two-path closeout archives the completed documentation decision only; it has
no further implementation authority and makes no CI claim for its own closeout
head. Issue #513, Jira KAN-12, and playable Combat remain open. Next work is one
coordinated generic-Game vertical slice: native event/profile/resource admission
and legal Content/loot-source admission; production MINT SQL/runtime; then separate
TRANSFER/client/E2E leases. No Rat hardcoding or product numbers are authorized.

## Context checkpoint

```yaml
last_progress: PR #1002 merged and protected-main contract/task blobs identified; record archived
status: completed
branch: codex/dur03-native-mint-audit-binding-513
head_sha: 91544690ef6453095a7240987f8603b912a576fd
pr: 1002
final_head_sha: 91544690ef6453095a7240987f8603b912a576fd
final_head_frozen_at: 2026-09-27T16:53:20Z
ci_trigger_source: pull_request/opened and ready_for_review
ci_check_generation: 36335162840
ci_checks_for_current_head: 2
ci_run_ids: [36334915639, 36335162840]
ci_job_ids: []
merge_group_run_id: 36335231869
runner_assignment_state: success
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: coordinated successor work belongs to fresh separate leases under control-plane authority
```
