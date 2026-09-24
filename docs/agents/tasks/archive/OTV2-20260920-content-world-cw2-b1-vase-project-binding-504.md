> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #691 merged as `9a63c653a469649d3dc796482077ad8dbde0b84f`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260920-content-world-cw2-b1-vase-project-binding-504

```yaml
task_id: OTV2-20260920-content-world-cw2-b1-vase-project-binding-504
title: CW3 typed project binding for the approved CW2-B1 vase
mode: BUILD
status: reviewing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-b1-vase-project-binding-504
issue: 162
pr: 691
base_sha: 54869eb9db46d83beaaadfc557270d58a39cc6dd
head_sha: pending
owner: "Oteryn: content world build"
production_authority: NONE
owned_paths:
  - apps/game-server/src/content/project.rs
  - apps/game-server/src/content/cw2_b1_import.rs
  - apps/game-server/src/content/mod.rs
  - apps/game-server/tests/content_world_cw2_b1_import.rs
  - docs/agents/tasks/active/OTV2-20260920-content-world-cw2-b1-vase-project-binding-504.md
```

## Authority and outcome

Owner proposal `#162` comment `5746160107` and session acceptance recorded in
comment `5747828221` authorize exactly one local non-production native binding:

```text
zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
data/items/items.xml item 2876
node b7c5c457cdccf047b251313e27cf283442a7346ddc556eaece8c2fdc30d655c3
-> oteryn:item.decor.vase@definition-r1
```

The native Item values are deliberately Oteryn-authored: `Physical`,
`materializable=true`, `NonStackable` and `ClientSafe`. The existing Item
lowering supplies `CharacterInventory` legality. Source pickup and weight facts,
the source primary type, source name and the protected B3 corroborating row are
provenance/loss only and do not select gameplay semantics.

The implementation adds one typed native-item binding value to the existing
import candidate representation. The binding must resolve to one existing
project `Item` record with the exact family/key/revision. Local executable proof
rejects unresolved reimport conflicts, duplicate binding targets, non-PENDING
access and non-PENDING project licensing. Empty B4 batches serialize exactly as
before because no field was added to `ImportBatch` or `ImportCandidate`.

## Protected evidence

| Evidence | Exact binding |
| --- | --- |
| B1 tracked catalogue | blob `2f0121f3ea6586477b4535840b9a1f1bc28c677c`, 16,877,870 bytes, SHA-256 `7836c78cad130a5c404f648e76e0823f53ae6a34c6952b9b88c8bed2e50d96a7` |
| B1 product | `d773076b576599b6ced7eb53e262cdc6363515d842da3e8518b610e2106db0fc` |
| B1 mapper | blob `904d62e1277ceae76434f75bca104686a7303ff2`, SHA-256 `320ce69f516de2a6e3cec669493ec59a6f3103f405e624478cf2a76acd3d01b6` |
| Pinned `items.xml` | blob `0b1dc3ba1a49094d9c83b90ab399bd2a9dd7a17f`, 3,819,874 bytes, SHA-256 `c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb` |
| Target source node | item `2876`, direct member, field profile `11da415530a0c8678cdb3d05c3205f21f3cc708f13a7342674538f97b911472e`, native disposition `UNRESOLVED` |
| B3 corroboration | `definition:monster:0108:loot:0006`, row digest `f5d87a09806776c70a799abb5b9eb657ed66b93346d943053ba3fad6500b2712`; no loot/chance/count promotion |

The adapter checks the finite B1 byte bound before hashing and accepts only the
exact protected digest. It emits no general catalogue adapter and performs no
full-world promotion.

## Acceptance criteria

- [x] The exact approved source item maps to the exact approved native key and revision.
- [x] Existing project Item and Reference Item semantics remain the sole gameplay model.
- [x] Pickup `1`, raw weight `940`, unknown weight unit/typed field and B3 corroboration remain typed provenance/loss.
- [x] Author/editor metadata cannot select Item semantics.
- [x] Local source correction remains auditable without changing authored Item semantics.
- [x] Reimport conflict, missing/wrong binding target, wrong access and wrong licensing fail closed.
- [x] Canonical project write/strict parse/rewrite is deterministic and reaches `ReferencePlayableContentSource`, `link_reference_playable`, server Item legality and client-safe projection.
- [x] B4 retains its existing candidate-only representation and canonical shape.
- [x] Exact published candidate passes required local focused and repository gates.
- [ ] Exact published candidate passes hosted Merge gate.
- [ ] Parent-owned independent review and protected integration complete.

## Excluded scope

No B1/B3 evidence mutation; no broad source adapter; no new item identity; no
loot/chance/count promotion; no redistribution or production grant; no
Reference parity claim; no compiler, runtime, registry, protocol, persistence,
Cargo/workspace/lock, workflow, resource-limit or publication-path change; no
B5/B6. `CanonicalReferencePlayableContent` remains without a successor compiler
until the separately accepted measured D3 profile extends the existing compiler
lineage. Fixture compilation is not an ordinary-release route.

## Validation

Local repaired-candidate validation completed with Rust 1.94:

- `cargo test -p oteryn-game-server --test content_world_cw2_b1_import` — PASS, 6 tests, including a coherent canonical-document regression that recomputes manifest, package-provenance, lock and root digests before proving non-PENDING licensing fails strict parse.
- `cargo test -p oteryn-game-server --test content_world_project` — PASS, 19 tests.
- `cargo test -p oteryn-game-server --test content_reference_playable` — PASS, 31 tests.
- `cargo test -p oteryn-game-server --test content_world_cw2_b4_import` — PASS, 4 tests; protected B4 remains candidate-only and byte-stable through its canonical round trip.
- `cargo check -p oteryn-game-server` — PASS.
- `cargo fmt --all -- --check` — PASS.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` — PASS.
- `cargo test --locked --workspace` — PASS.
- `python -B tools/agents/validate_governance.py` — PASS.
- `python -B tools/repository/validate_repository_policy.py` — PASS.
- hosted Architecture semantic audit, Agent governance and Merge gate on the superseded pre-repair head — PASS.
- hosted exact-head gates on the repaired candidate — pending after this task-record update.

## Review and integration

The parent control plane owns independent review, CI interpretation and governed
protected integration. This worker publishes normal non-force commits and does
not submit Merge Queue, trigger manual workflows or merge.
