# Owner decision batch D154-D158

- Decision: `OWNER-DECISION-BATCH-D154-D158-V1`
- Status: owner decisions recorded; docs only. Acceptance requires validation and protected integration.
- Origin: the owner answered the control-plane question batch directly on 2026-09-29
- Numbering: D152 and D153 belong to the A14 ruling (#162 comment 5897060635); this batch starts at D154
- Runtime, schema, registry, Platform and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Decisions

| # | Topic | Owner choice (2026-09-29) | Source | Q |
|---|---|---|---|---|
| D154 | 15.30 client files in the repository | The files under `content/assets/files/` (#1251-#1253) stay in the repository and agents must not delete them. Retention only: the 2026-09-29 owner supersession note governs redistribution and confirms the rights (Q12a). Amends the client asset version decision | #162 5898471146; Q12a, #162 2026-09-29 | Q8a, Q12a |
| D155 | Large assets and Git LFS | Plain Git, no LFS (delegated to the control plane); revisit if `content/assets` grows materially or clone/CI cost becomes a problem | #162 5898471146 | Q9 |
| D156 | Pickupable item ids | One proposal table for 247 client-only pickupable ids, 9 donor ids and 10 UNSURE ids, approved as a whole; the table is pending | #162 5898471146 | Q10a |
| D157 | Task-session pilot | Confirmed: up to three concurrent task sessions | #162 5898317577 | Q6a |
| D158 | Workflow and CI authorizations | `.github/workflows` changes authorized in #1160, #1170 and #1189 after #1160's Codex P1/P2 fixes and a green Merge authority audit (Q1a); the CodeQL action bump (#1249) authorized, later carried by #1275 (Q2a); the ADR-0020 N5 client closure change authorized (Q4a); one combined merge-authority rotation for N5 and the CodeQL bump (Q7c), which supersedes the #1249-only rotation (Q5a) | #162 5897283986, 5897339487, 5898411039 | Q1a, Q2a, Q4a, Q5a, Q7c |

Not given a D number (a one-time action, not a durable rule): **Q3b** (#162 5897333404), refresh and
merge #633; it is merged.

## 2. Notes

- D154 amends `docs/architecture/OTERYN_CLIENT_ASSET_VERSION_OWNER_DECISION_2026-09-27.md`
  (dated amendment section; its history is unchanged).
- D156: no id is approved until the proposal table exists; approval applies to the table as a whole.
- D158 authorizes the listed changes only; each still needs its own PR, checks and Merge Queue
  under the bound META policy.

## 3. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
owner_decisions: [D154, D155, D156, D157, D158]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D154_D158_2026-09-29.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
```
