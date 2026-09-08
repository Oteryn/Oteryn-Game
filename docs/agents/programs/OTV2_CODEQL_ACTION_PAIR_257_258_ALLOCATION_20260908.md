# Coordinated CodeQL 4.37.9 action-pin allocation

Refs #162, #257, #258, #420, #422, #448.

## State

```yaml
allocation_id: OTV2-CODEQL-ACTION-PAIR-257-258-20260908
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: c9cec0f746e549ff96151bcf1e3582522dfea0ee
allocation_state: NOT_ACTIVE
preparation_branch: coord/codeql-action-pair-448
canonical_material_pr_after_activation: 257
canonical_material_branch_after_activation: dependabot/github_actions/github/codeql-action/init-4.37.9
held_overlapping_pr: 258
risk: CONTROL
```

This document is allocation-only. It grants no present workflow, validator, ruleset, Merge Queue, runtime or production mutation authority. Material application requires exact-head qualification of this allocation, normal FULL Merge Queue, protected-main readback and a fresh Work custody/overlap check.

## Why #257 and #258 must become one candidate

Dependabot opened two overlapping PRs against the same three workflow files:

- #257 updates `github/codeql-action/init` from pinned `ff2f1c621b7f889edc0d3c761ac2e6a3f8cdb0dd` to pinned `cdf488f595d80d6e07e03d4674febd5ab45fa938`;
- #258 updates `github/codeql-action/analyze` from the same old pin to the same new pin.

These are not safely serializable as two independently integrated workflow generations. GitHub CodeQL validates that all `github/codeql-action` steps within a workflow use the same version and reports incompatible mixed generations. A public GitHub-hosted example records both the explicit warning — `Not all workflow steps that use github/codeql-action actions use the same version` — and an analyze failure after loading configuration from a different action version.

The repository also deliberately protects the Merge Queue CodeQL contract from candidate self-relaxation. At the allocation base, `tools/repository/validate_repository_policy_core.py` requires:

- the full reviewed `merge-group-gate.yml` blob;
- the exact old CodeQL `init` fragment;
- the exact old CodeQL `analyze` fragment.

Therefore neither existing three-file Dependabot PR is semantically complete by itself. The smallest truthful candidate must update both action phases atomically and update only the trusted expected representation that binds that exact workflow change.

## Canonical lineage and serialization

After this allocation is protected and Work explicitly applies it:

1. Preserve **existing PR #257** as the single material candidate. Do not create another dependency implementation branch/PR.
2. Rebase #257 from then-current protected `main` before material edits.
3. Extend the same #257 branch only inside the exact authored scope below so every CodeQL `init` and `analyze` callsite in the three current CodeQL workflows uses the same new pinned commit.
4. Keep #258 open but **HELD / NO_WRITER** throughout #257 qualification and Merge Queue.
5. Close #258 as superseded only after the combined #257 candidate is protected-integrated and exact protected-main readback proves the analyze bump is present. If #257 fails terminally or authority is withdrawn, #258 is not automatically activated.

This preserves one material writer for the overlapping workflow surface and avoids a temporary mixed-version protected-main generation.

## Exact additional material lease after protected application

Material writes are limited to exactly four paths:

### 1. `.github/workflows/codeql.yml`

Only replace the currently pinned CodeQL Action commit on existing `init` and `analyze` steps so both equal:

`cdf488f595d80d6e07e03d4674febd5ab45fa938`

No trigger, permission, language, query, build-mode, category, timeout, schedule or other action change.

### 2. `.github/workflows/merge-gate.yml`

Only replace the currently pinned CodeQL Action commit on existing `init` and `analyze` steps so both equal the same new commit.

No scope classifier, trusted-base lane, job graph, permissions, PostgreSQL/SIM routing, `game-gate`, trigger or other action change.

### 3. `.github/workflows/merge-group-gate.yml`

Only replace the currently pinned CodeQL Action commit on existing `init` and `analyze` steps so both equal the same new commit.

No Merge Queue identity validation, dependency review, Linux/Windows/PostgreSQL/supply-chain lane, job graph, permissions, `game-gate`, trigger or other action change.

### 4. `tools/repository/validate_repository_policy_core.py`

Only update trusted expected values made stale by the exact paired pin replacement above:

- the required CodeQL `init` fragment in the merge-group `codeql` job;
- the required CodeQL `analyze` fragment in that same job;
- `EXPECTED_MERGE_GROUP_GATE_BLOB` to the Git blob of the otherwise byte-identical merge-group workflow containing only those two pin replacements.

Do not weaken/remove the exact-blob check, fragment checks or any other repository-policy assertion. Do not add a generic allowlist, tag-based action reference, version range, candidate-derived expected value or fallback.

## Explicit exclusions

No authority is granted for:

- any fifth repository path;
- CodeQL configuration, query selection, languages, categories, build modes or permissions;
- other GitHub Action upgrades;
- Merge Gate/Queue topology, status names, rulesets, branch protection or bypass actors;
- Cargo, runtime, gameplay, Atlas producer semantics, database schema, migrations, production, secrets or external repositories;
- #258 material mutation while #257 is active;
- direct merge or bypass of canonical checks/Merge Queue.

Any observed need for another path or semantic change is `SHARED_LEASE_REQUIRED` before mutation.

## Required RED/GREEN and qualification

The material #257 candidate must prove, on the exact current base at application time:

1. **RED / incomplete Dependabot generation:** the three-file init-only or analyze-only state is ineligible. The repository-policy validator must reject the stale trusted representation and CodeQL's same-version rule must not be bypassed.
2. **GREEN / paired pins:** all six existing CodeQL callsites across the three workflows use exactly `cdf488f595d80d6e07e03d4674febd5ab45fa938` for their respective `init`/`analyze` phases.
3. `tools/repository/validate_repository_policy.py` and its core validator pass only after the exact expected fragment/blob update; all unrelated policy checks remain unchanged.
4. Exact changed-path inventory is exactly the four paths above, with workflow textual changes limited to the six pin lines.
5. Agent governance and Architecture semantic audit pass on the exact material head.
6. Canonical pull-request Merge Gate passes, including actual CodeQL Python and Actions jobs on the paired version.
7. Zero unresolved actionable review threads on the final material head; apply the current META risk-based external-review policy without inventing a stricter local requirement.
8. Normal FULL Merge Queue passes on the exact merge-group candidate, including candidate/governance, dependency review, CodeQL Python+Actions, supply chain, Linux workspace, Windows client, real Durability PostgreSQL harness and final `game-gate`.
9. Protected-main readback proves the exact paired CodeQL pin and matching validator authority before #258 is closed as superseded and ownership is released.

No no-op/retrigger commit, test suppression or temporary mixed-version merge is acceptable.

## Integration lifecycle

```text
allocation-only PR
-> exact-head governance/semantic/canonical checks
-> current-policy CONTROL review if selected
-> normal FULL Merge Queue
-> protected-main readback
-> fresh Work custody/overlap readback
-> apply to SAME existing #257 branch/PR
-> paired init+analyze + trusted-validator RED/GREEN
-> exact-head qualification/review
-> normal FULL Merge Queue
-> protected-main readback
-> close #258 as superseded
-> release CodeQL CONTROL ownership
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.