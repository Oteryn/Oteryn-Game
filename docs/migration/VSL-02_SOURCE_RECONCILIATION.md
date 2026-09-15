# VSL-02 Source Reconciliation

- Status: normative evidence for `VSL-02`
- Reconciliation date: 2026-08-06
- Coordination ID: `OTV2-RUST-CLIENT-CUTOVER-20260806`
- Source repository: `blakinio/otclient`
- Source subtree: `oteryn-client/`
- Selected source commit: `c923ad8a1dff17b4933a6110931b0823cec2c590`
- Selected subtree tree: `c0928dafca6df19ff11d7901e503ed85a5199439`
- Result at reconciliation: no Rust-subtree drift

## 1. Reconciliation conclusion

The FND-01 inventory commit was still the live source default-branch head when VSL-02 started. The cutover pins immutable content to:

```text
blakinio/otclient@c923ad8a1dff17b4933a6110931b0823cec2c590
oteryn-client tree c0928dafca6df19ff11d7901e503ed85a5199439
```

The implementation always imports from that selected commit, not from a later mutable `main`.

A later source commit wholly outside `oteryn-client/**` is allowed and does not invalidate VSL-02. Cutover is blocked only when current `main` resolves `oteryn-client/` to another tree, or a new PR/task claims the frozen Rust subtree.

An authorized subtree change requires an owner-approved VSL-02 amendment with a new commit/tree, affected-path reconciliation, regenerated manifests and fresh audit.

## 2. Immutable source identifiers

```yaml
cutover_commit: c923ad8a1dff17b4933a6110931b0823cec2c590
subtree_tree: c0928dafca6df19ff11d7901e503ed85a5199439
root_manifest_blob: 037013e8e4a762a65f0f2a30f7761ee14725a3fc
root_lockfile_blob: 2143408c12c50132883890f0821278320a331fde
crates_tree: c2a5426f764bab0f3a89de3e5a03e88a7f111c20
source_readme_blob: 62f49edd5aa584982ebf31339cc72e2ed8a74b0b
source_agents_blob: 3422f3af440456d16c70df92064ad06fdbf02659
native_correspondence_workflow_blob: 16638fe20af72d772c7f3f8b116c22c3831f26c2
nested_native_task_blob: 2d75c9eb0bed583f3253fee28211994623c97901
```

These identify evidence. They do not imply that manifests, lockfiles, README, governance, workflows or rewritten code are copied unchanged.

## 3. Open source pull requests

| PR | Lane | Rust subtree impact | Cutover decision |
|---:|---|---|---|
| #23 — Oteryn login shell prototype | legacy C++/Lua/OTUI | none | excluded; may continue as legacy-client work |
| #48 — official Tibia Linux package analysis | operational/proprietary analysis | none | reference-only; never imported or treated as distributable provenance |
| #97 — release archive verification | legacy asset installer/security | none | excluded; may continue for the legacy client |

No open PR changes `oteryn-client/**`. No PR is merged, cherry-picked or restacked into the selected Rust source.

A later merge of these PRs is harmless when it leaves the pinned subtree tree unchanged. The source-marker PR does not close or rewrite unrelated legacy PRs.

## 4. Root source task records

| Task | Actual lane/state | Cutover decision |
|---|---|---|
| `OTC-20260712-client-test-foundation` | legacy C++/Lua tests; stale PR #3 | excluded; legacy lifecycle cleanup |
| `OTC-20260714-protocol-session-reentrancy` | legacy C++/Lua protocol; stale PR #9 | excluded; legacy lifecycle cleanup |
| `OTC-20260721-oteryn-identity-login` | merged legacy auth | historical evidence only |
| `OTC-20260724-validation-cost-policy` | repository governance; stale PR #19 | excluded; governance cleanup |
| `OTC-20260802-agent-governance-sync` | completed but under `active/` | excluded; generic governance cleanup |
| `OTC2-20260803-playability-p2-canary-world-protocol` | Rust Canary implementation | reference-only; archive in source marker |

These records grant no destination ownership. The Canary task cannot block omission of `protocol-canary` because ADR-0008 and FND-01 make it reference-only.

## 5. Nested native-programme conflict

The source subtree contains:

```text
oteryn-client/docs/agents/tasks/active/
  OTC2-20260805-native-protocol-single-version-completion.md
```

It proposed an independent `protocol-oteryn`, automatic dual-protocol selection and source transport reuse. That plan is superseded for Oteryn-v2:

- Canary is not migrated;
- the initial destination has no gameplay transport or protocol crate;
- no placeholder or automatic protocol selection is created;
- FND-02 later owns the only native protocol contract.

Associated source-only material is also non-canonical:

```text
oteryn-client/docs/architecture/OTERYN_NATIVE_PROTOCOL_CORRESPONDENCE.md
.github/workflows/native-protocol-rust-correspondence.yml
```

The correspondence remains historical evidence with a superseded notice. The workflow is removed by the later source-marker PR so it cannot continue enforcing a superseded programme.

## 6. Source freeze

The administrative freeze begins when VSL-02 merges:

```text
blakinio/otclient/oteryn-client/**
```

After that:

- no feature, refactor, protocol, dependency, formatting or documentation work is accepted in the Rust subtree;
- legacy work outside it is unaffected;
- import always uses the selected commit;
- current `main` may advance only without changing the subtree tree;
- no bidirectional sync or dual-canonical Rust development is allowed.

An owner-approved critical security or cutover-build correction may amend the subtree only through a VSL-02 amendment.

## 7. Destination implementation preflight

Before opening the atomic PR and again before its merge, prove:

1. selected commit remains reachable;
2. selected commit has subtree tree `c0928daf...`;
3. current source default branch has the same subtree tree;
4. no open PR changes `oteryn-client/**`;
5. no new active task claims a migratable Rust path;
6. destination `main` has no competing import/workspace;
7. FND-01 and VSL-02 exist on destination `main`.

A failure stops import. The implementation cannot silently select a later source.

## 8. Later source-marker PR

A separate owner-authorized task in `blakinio/otclient` runs only after verified destination implementation merge. It changes source ownership documentation, obsolete workflow state and task lifecycle, not Rust runtime code.

Required scope:

```text
oteryn-client/README.md
oteryn-client/AGENTS.md
oteryn-client/MOVED_TO_OTERYN_V2.md

docs/agents/tasks/active/OTC2-20260803-playability-p2-canary-world-protocol.md
docs/agents/tasks/archive/OTC2-20260803-playability-p2-canary-world-protocol.md

oteryn-client/docs/agents/tasks/active/OTC2-20260805-native-protocol-single-version-completion.md
oteryn-client/docs/agents/tasks/archive/OTC2-20260805-native-protocol-single-version-completion.md

oteryn-client/docs/architecture/OTERYN_NATIVE_PROTOCOL_CORRESPONDENCE.md
.github/workflows/native-protocol-rust-correspondence.yml

one source-marker task record
```

Required behavior:

- README begins with a moved/non-canonical notice and exact destination merge;
- nested AGENTS begins with a highest-priority implementation freeze and destination pointer;
- `MOVED_TO_OTERYN_V2.md` records source commit/tree, destination merge, manifests and rollback;
- Canary task archives as reference-only;
- nested native task archives as superseded reference-only;
- native correspondence receives a superseded/non-canonical notice;
- obsolete native correspondence workflow is deleted;
- source code, commits and legal evidence remain present;
- legacy root C++/Lua ownership remains unchanged.

The marker cannot delete the subtree, rewrite history, close unrelated PRs or claim preserved cross-repository ancestry.

## 9. Canonical ownership transitions

```text
Before destination merge:
  source = canonical frozen Rust source
  destination = architecture-only

After destination merge, before source marker:
  destination = canonical Rust implementation
  source = intact frozen historical source

After source marker:
  destination = canonical Rust implementation
  source = explicitly moved/non-canonical evidence
```

Exactly one repository has writable canonical authority.

## 10. Reconciliation verdict

```yaml
selected_source_commit: c923ad8a1dff17b4933a6110931b0823cec2c590
selected_subtree_tree: c0928dafca6df19ff11d7901e503ed85a5199439
rust_subtree_drift_at_reconciliation: none
open_rust_subtree_prs: 0
conflicting_source_programmes:
  - protocol-canary task: reference-only
  - nested dual/native task: superseded reference-only
  - native correspondence workflow: delete on source marker
material_source_ownership_conflicts: 0
result: PASS
```
