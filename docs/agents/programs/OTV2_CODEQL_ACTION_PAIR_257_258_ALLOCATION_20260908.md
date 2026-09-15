# Coordinated CodeQL 4.37.9 action-pin allocation

Refs #162, #257, #258, #398, #420, #422, #448, #449.

## State

```yaml
allocation_id: OTV2-CODEQL-ACTION-PAIR-257-258-20260908
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: c9cec0f746e549ff96151bcf1e3582522dfea0ee
allocation_state: NOT_ACTIVE
preparation_branch: coord/codeql-action-pair-448
canonical_material_pr_after_stage_a: 257
canonical_material_branch_after_stage_a: dependabot/github_actions/github/codeql-action/init-4.37.9
held_overlapping_pr: 258
risk: CONTROL
required_sequence: ALLOCATION_PROTECT -> AUDIT_PIN_ROTATION_PROTECT -> MATERIAL_257
```

This document is allocation-only. It grants no present workflow, validator, ruleset, Merge Queue, runtime or production mutation authority. Its own exact-head qualification, normal FULL Merge Queue and protected-main readback are required before any later control-plane write.

Independent exact-head review of the first candidate correctly returned two P1 findings. Both are accepted here:

- review `3959551286`: `tools/repository/test_validate_merge_group_pg_sim.py` also pins the exact approved merge-group workflow blob and must move with the final gate candidate;
- review `3959551293`: protected `.github/workflows/merge-authority-audit.yml` preapproves only the current merge-group blob and deliberately rejects candidates modifying the audit itself, so its pin must be rotated in a separate protected stage before #257 may carry the future gate.

No executable authority is created by documenting those findings.

## Why #257 and #258 must become one material candidate

Dependabot opened two overlapping PRs against the same three workflow files:

- #257 updates `github/codeql-action/init` from pinned `ff2f1c621b7f889edc0d3c761ac2e6a3f8cdb0dd` to pinned `cdf488f595d80d6e07e03d4674febd5ab45fa938`;
- #258 updates `github/codeql-action/analyze` from the same old pin to the same new pin.

They are not safely serializable as two independently protected workflow generations. GitHub CodeQL requires the action phases used by one workflow generation to remain version-compatible; a mixed `init`/`analyze` generation is ineligible.

The repository intentionally adds two further protected bindings:

1. `tools/repository/validate_repository_policy_core.py` binds the full approved `merge-group-gate.yml` blob plus exact CodeQL `init` and `analyze` fragments;
2. `tools/repository/test_validate_merge_group_pg_sim.py` binds the same approved merge-group workflow blob as its `APPROVED` regression oracle.

A third binding lives outside the future material candidate: protected `.github/workflows/merge-authority-audit.yml` accepts a control-plane candidate only when its merge-group workflow matches the preapproved future blob. That audit refuses candidate self-modification by design.

Therefore the smallest truthful delivery is **two protected stages after this allocation is canonical**:

```text
Stage A: rotate protected audit pin to the exact future gate blob
-> protected readback
Stage B: apply the paired CodeQL change + matching trusted/regression pins to SAME #257
```

This preserves the protected-base trust direction rather than allowing #257 to approve its own future Merge Queue generation.

## Canonical lineage and serialization

After this allocation is protected and Work freshly applies it:

1. Keep #257 and #258 non-integrating while Stage A is prepared.
2. Fresh-read protected `main`, #257/#258 heads and all exact path custody.
3. Rebase the **same #257 branch** to then-current protected `main` if required, but do not yet add material CodeQL changes beyond Dependabot's existing candidate.
4. From that exact protected source generation, construct the intended future `merge-group-gate.yml` by changing only its existing CodeQL `init` and `analyze` pins to `cdf488f595d80d6e07e03d4674febd5ab45fa938`; compute its exact Git blob. Any unrelated drift requires recomputation and re-review rather than reuse of a stale blob.
5. Execute Stage A below and read it back on protected main.
6. Only after Stage A protected readback, apply Stage B to the **same existing #257 branch/PR**. Do not create a replacement dependency implementation PR.
7. Keep #258 **HELD / NO_WRITER** through #257 qualification and Merge Queue.
8. Close #258 as superseded only after Stage B is protected-integrated and readback proves its analyze update is present. Failure or withdrawal of #257 does not automatically activate #258.

## Stage A — protected merge-authority audit pin rotation

Stage A is a separate serialized CONTROL mutation whose sole material path is:

- `.github/workflows/merge-authority-audit.yml`.

Authority is limited to changing exactly `EXPECTED_MERGE_GROUP_GATE_BLOB` from the currently protected gate blob to the precomputed exact Git blob of the future Stage B `merge-group-gate.yml` containing only the paired CodeQL pin replacements.

No other line in the audit workflow may change. Preserve:

- `pull_request_target` and same-repository/exact-head/main-base identity checks;
- inert candidate reads and no candidate checkout;
- self-modification refusal;
- exact one-blob approval;
- required/forbidden merge-group fragments;
- pinned actions, read-only permissions and no dispatch/`continue-on-error` behavior.

The audit's own protected-base self-audit is expected to reject Stage A solely because the candidate modifies `.github/workflows/merge-authority-audit.yml` itself. That failure must remain truthful and must not be weakened, relabeled or made required-success. This is the same fail-closed rotation pattern previously used by protected PR #398.

Stage A therefore requires:

- fresh explicit Work application of this protected allocation;
- exact one-path diff/readback;
- independent exact-head deep CONTROL review with no unresolved material finding;
- applicable canonical PR checks;
- normal FULL Merge Queue;
- protected-main readback proving the new audit pin before Stage B mutation proceeds.

Stage A grants no #257/#258 write and no permission to change the future gate itself.

## Stage B — exact material lease on existing #257

Only after Stage A is protected-integrated/read back, Stage B material writes are limited to exactly five paths.

### 1. `.github/workflows/codeql.yml`

Only replace the currently pinned CodeQL Action commit on existing `init` and `analyze` steps so both equal:

`cdf488f595d80d6e07e03d4674febd5ab45fa938`

No trigger, permission, language, query, build-mode, category, timeout, schedule or other action change.

### 2. `.github/workflows/merge-gate.yml`

Only replace the currently pinned CodeQL Action commit on existing `init` and `analyze` steps so both equal the same new commit.

No scope classifier, trusted-base lane, job graph, permissions, PostgreSQL/SIM routing, `game-gate`, trigger or other action change.

### 3. `.github/workflows/merge-group-gate.yml`

Only replace the currently pinned CodeQL Action commit on existing `init` and `analyze` steps so both equal the same new commit.

The resulting file must have the exact Git blob preapproved by protected Stage A. If protected-main drift makes the reconstructed future file differ, stop and rotate the audit pin again through the same protected sequence; never make the candidate derive or relax its own approval.

No Merge Queue identity validation, dependency review, Linux/Windows/PostgreSQL/supply-chain lane, job graph, permissions, `game-gate`, trigger or other action change.

### 4. `tools/repository/validate_repository_policy_core.py`

Only update trusted expected values made stale by the exact paired pin replacement:

- the required CodeQL `init` fragment in the merge-group `codeql` job;
- the required CodeQL `analyze` fragment in that same job;
- `EXPECTED_MERGE_GROUP_GATE_BLOB` to the exact preapproved Stage B merge-group Git blob.

Do not weaken/remove the exact-blob check, fragment checks or any other repository-policy assertion. Do not add a generic allowlist, tag-based action reference, version range, candidate-derived expected value or fallback.

### 5. `tools/repository/test_validate_merge_group_pg_sim.py`

Only update `APPROVED` to the same exact preapproved Stage B merge-group Git blob.

Preserve every regression assertion, lifecycle command, native PowerShell canary, mutation-negative and full-policy invocation unchanged. No test suppression, condition skip or weaker oracle is authorized.

## Explicit exclusions

No authority is granted for:

- any Stage A path other than `.github/workflows/merge-authority-audit.yml`;
- any Stage B sixth path;
- changing Stage A audit semantics beyond its single expected blob value;
- CodeQL configuration, query selection, languages, categories, build modes or permissions;
- other GitHub Action upgrades;
- Merge Gate/Queue topology, status names, rulesets, branch protection or bypass actors;
- Cargo, runtime, gameplay, Atlas producer semantics, database schema, migrations, production, secrets or external repositories;
- #258 material mutation while #257 is active;
- direct merge or bypass of canonical checks/Merge Queue.

Any observed need for another path or semantic change is `SHARED_LEASE_REQUIRED` before mutation.

## Required RED/GREEN and qualification

The current #257 init-only generation is retained as truthful RED evidence: after metadata is valid, repository-policy validation rejects its changed merge-group gate because the protected full-blob oracle and old `init` fragment no longer match. Do not weaken or bypass that RED.

Stage B must then prove on its exact current base:

1. **Paired version:** all existing CodeQL `init` and `analyze` callsites across the three workflows use exactly `cdf488f595d80d6e07e03d4674febd5ab45fa938`; no mixed generation remains.
2. **Protected preapproval:** the resulting `merge-group-gate.yml` Git blob exactly equals the Stage A pin already present on protected main.
3. **Trusted GREEN:** `tools/repository/validate_repository_policy.py` and its core validator pass only after the exact expected fragment/blob updates; all unrelated assertions remain unchanged.
4. **Regression GREEN:** `tools/repository/test_validate_merge_group_pg_sim.py` retains its full negative/positive suite and its `APPROVED` blob equals the same protected future gate blob.
5. **Exact scope:** exactly the five Stage B paths change; textual workflow changes are limited to existing CodeQL pin lines, while validator/test edits are limited to the three trusted expected values plus the one regression `APPROVED` value.
6. Agent governance and Architecture semantic audit pass on the exact material head.
7. Canonical pull-request Merge Gate passes, including actual CodeQL Python and Actions jobs on the paired version.
8. Current META risk-based independent CONTROL review is applied to the stable material candidate when selected; unresolved actionable threads are zero.
9. Normal FULL Merge Queue passes on the exact candidate, including candidate/governance, dependency review, CodeQL Python+Actions, supply chain, Linux workspace, Windows client, real Durability PostgreSQL harness and final `game-gate`.
10. Protected-main readback proves the paired CodeQL pins, matching audit preapproval, trusted validator and regression oracle before #258 is closed and CONTROL ownership is released.

No no-op/retrigger commit, test suppression, candidate self-approval or temporary mixed-version merge is acceptable.

## Integration lifecycle

```text
allocation-only #449
-> exact-head checks + independent CONTROL review
-> normal FULL Merge Queue
-> protected-main readback
-> fresh Work custody readback
-> compute exact future paired merge-group blob from current protected generation
-> Stage A: audit-pin-only rotation
-> independent deep review + canonical checks
-> normal FULL Merge Queue
-> protected-main readback of future blob preapproval
-> rebind/rebase SAME #257 to current protected main if required
-> Stage B: paired init+analyze + trusted validator + regression oracle
-> exact-head qualification/review
-> normal FULL Merge Queue
-> protected-main readback
-> close #258 as superseded
-> release CodeQL CONTROL ownership
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.