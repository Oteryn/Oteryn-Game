# OTV2-20260913-wp3-v2-gate1-acceptance-allocation

```yaml
task_id: OTV2-20260913-wp3-v2-gate1-acceptance-allocation
title: Prepare WP3-v2 Gate 1 acceptance and A4 allocation package
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/wp3-v2-multi-agent-delivery-20260912
pr: 589
base_sha: 253b5c0c464e9b73c8398bcf479bdcca9a1f0932
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
final_head_frozen_at: external_pr_evidence
owner: Oteryn: astra wp3-v2 programme coordinator
activation_authority: OTV2_WORK_DELIVERY_COORDINATOR
created_at: 2026-09-13
updated_at: 2026-09-13
execution_policy: continuous_progress
owned_paths:
  - docs/agents/programs/OTV2_WP3_V2_GATE1_ACCEPTANCE_ALLOCATION_20260913.md
  - docs/agents/tasks/active/OTV2-20260913-wp3-v2-gate1-acceptance-allocation.md
depends_on:
  - issue:162
  - issue:351
  - pr:356
  - issue:364
  - pr:590
blocks:
  - WP3-v2 Gate 1 activation
  - A4 material mutation
```

## Outcome

Persist one bounded prospective Gate-1 package that reconciles protected Revision 3, the clean exact-head independent review and the existing #351/#356 canonical implementation lineage without transferring the active control plane or granting implementation authority from an unprotected documentation branch.

## Live authority

- Protected `main@253b5c0c464e9b73c8398bcf479bdcca9a1f0932` contains merged #590.
- Protected #162 task state selects `OTV2_WORK_DELIVERY_COORDINATOR` as the active mutating control plane.
- This task does not change that selector.
- The A0 role prepares/reconciles the package only; protected integration plus explicit active-control-plane activation is still required before A4 mutation.

## Architecture evidence

- exact Revision-3 architecture head: `60b2018ded8be9be9b404eafcd25eba9c43097cd`;
- Architecture Semantic Audit `34719732556`: SUCCESS;
- Agent Governance `34719818428`: SUCCESS;
- Merge Gate `34719818411`: SUCCESS;
- independent exact-head HIGH-risk review: `P0=0`, `P1=0`, `P2=0`, `BLOCKING_EVIDENCE_GAP=0`, PASS;
- the review grants neither architecture acceptance nor implementation authority by itself.

## Canonical implementation disposition

- reuse #351 / PR #356 / `agent/sqlx-driver-budget-351`;
- observed reconciliation head: `fe7891989b1247012e32c89c10cff6a10bacb943`;
- no replacement worker/branch/PR;
- preserve compatible resource/finality/AWS-LC/TLS/PostgreSQL evidence and primitives;
- supersede the old direct-connect/two-connection/overlap/retry/ring-terminal assumptions as specified by the Gate-1 package;
- do not release #335 or #247 from this task.

## Activation rule

This task and its programme package remain `PROSPECTIVE_NOT_ACTIVE` until:

1. PR #589 (or a repository-normal successor carrying the same package) is protected-integrated;
2. protected-main readback proves the exact package;
3. `OTV2_WORK_DELIVERY_COORDINATOR` performs fresh #162/#364/#351/#356 overlap and custody reconciliation;
4. the active control plane records explicit A4 activation naming exact protected main, exact #356 head/branch and bounded owned/shared paths.

Before that explicit activation:

```text
ARCHITECTURE_ACCEPTED = NO
A4_IMPLEMENTATION_AUTHORITY = NONE
```

The package records evidence sufficient for acceptance but does not self-accept.

## Acceptance criteria for this documentation task

- [x] protected Revision-3 evidence pinned exactly;
- [x] independent exact-head review result pinned exactly;
- [x] active control plane preserved rather than transferred by alias/prompt;
- [x] canonical #351/#356 lineage preserved;
- [x] retain/supersede map made explicit;
- [x] frozen Option-B implementation contract made explicit;
- [x] activation gate requires protected readback + fresh control-plane activation;
- [x] #335, WP5, #247, Platform, workflow/ruleset and production authority excluded;
- [ ] exact-head repository/governance checks on the final #589 successor;
- [ ] applicable independent review of the final documentation successor;
- [ ] protected integration/readback;
- [ ] explicit active-control-plane A4 activation.

## Excluded scope

No runtime Rust, vendor, Cargo/lockfile, SQL/migration, workflow/ruleset, #356 material mutation, #335 mutation, WP5 source mutation, Server Seam mutation, Platform write, production/deployment/secret action, direct merge or Merge Queue bypass.

## Next action

Qualify the exact #589 successor as documentation/governance only. If clean, use the repository's normal protected integration path. Only after protected readback may the active control plane perform the separate A4 activation step.
