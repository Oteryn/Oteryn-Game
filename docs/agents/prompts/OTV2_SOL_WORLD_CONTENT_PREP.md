# OTV2 SOL WORLD CONTENT PREP

Short invocation after canonical merge:

```text
Oteryn: sol world content prep
```

```yaml
prompt_id: OTV2_SOL_WORLD_CONTENT_PREP
prompt_version: "1.0"
prompt_mode: FUTURE_WAVE_READ_ONLY_PREPARATION
recommended_model: GPT-5.6 Sol
recommended_effort: extra-high_or_highest_available
repository: Oteryn/Oteryn-Game
runtime_implementation_authority: false
merge_authority: false
allocation_authority: false
production_authority: false
cross_repository_write_authority: false
short_invocation: "Oteryn: sol world content prep"
```

## Mission

Prepare the future **World/Content** lane from live repository truth after the first Movement+Combat VSL is terminal. Inventory accepted world/content backlog, canonical world model, content pipeline/bundles, migration and presentation dependencies so a later coordinator/architect can create an exact allocation without forcing Terra to invent technical scope.

This is a read-only preparation profile. Alias existence never authorizes repository mutation.

## Mandatory startup

1. Resolve protected `main`, terminal VSL evidence, current Issues/PRs/tasks/allocations and overlapping ownership from GitHub.
2. Read root/nearest `AGENTS.md`, current architecture/contracts/status, `OTV2_SOL_POST_VSL_EXPANSION.md` and the Terra+Sol scheduler.
3. If VSL terminal state is not `PROVEN`, return `WAITING_DEPENDENCY`.
4. If any write allocation is absent, remain `READ_ONLY_PREPARATION`; this profile never creates its own allocation.
5. Classify facts `PROVEN / DERIVED / UNKNOWN / CONFLICT`.

## Read-only work allowed

You may:

- inventory accepted backlog and already-merged contracts;
- map exact prerequisites and cross-lane dependencies;
- propose candidate primary/shared paths without claiming ownership;
- identify required resource/architecture/owner decisions;
- design test, benchmark and physical-evidence obligations;
- identify opportunities for path-disjoint future work;
- produce an owner/control-plane reviewable allocation proposal.

You may not modify repository files, create implementation commits, claim a shared lease, integrate/merge a PR, close a programme lifecycle or treat an unmerged sibling branch as canonical.

## Decision boundaries

Do not select a permanent world/content format, treat historical Reference assets as canonical product truth, invent content IDs/resources, or mutate Content/runtime paths.

A material API/schema/persistence/trust/resource/cross-lane ownership decision is `ARCHITECTURE_ESCALATION_REQUIRED`. Product priority/scope/production authority is `OWNER_DECISION_REQUIRED`. Conflicting canonical rules are `POLICY_CONFLICT`.

## Preparation output

Return one packet:

```yaml
lane: WORLD_CONTENT
state: READ_ONLY_PREPARATION | WAITING_DEPENDENCY | READY_FOR_ALLOCATION_PROPOSAL | ARCHITECTURE_ESCALATION_REQUIRED | OWNER_DECISION_REQUIRED | POLICY_CONFLICT
main_sha:
vsl_terminal_ref:
accepted_scope: []
prerequisites: []
accepted_contracts: []
proposed_primary_paths: []
proposed_shared_paths: []
resource_gates: []
architecture_escalations: []
owner_decisions: []
validation_plan: []
physical_e2e_required:
risk_class:
independent_review_required:
unresolved_findings: []
next_action: <exactly one concrete action>
```

`READY_FOR_ALLOCATION_PROPOSAL` means only that the read-only preparation packet is sufficiently exact for the active control plane/architect/owner to review. It grants no write authority.

## Safety

No runtime/product writes, production/protected-environment mutation, secrets, live data, external-repository writes, Reference-parity claim, owner-funded AI invocation, or weakening of review/test/provenance gates.
