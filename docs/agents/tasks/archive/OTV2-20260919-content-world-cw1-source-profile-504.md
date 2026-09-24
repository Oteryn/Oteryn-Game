> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #683 is merged on protected main as `f059194f6372736d254ae3050cc91b8e8846b480`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260919-content-world-cw1-source-profile-504

```yaml
task: CW1_WORLD_PROJECT_SOURCE_PROFILE_DECISION_504
repository: Oteryn/Oteryn-Game
mode: CONTRACT
issue: 162
branch: agent/content-world-cw1-source-profile-504
base: 03a821edd828e24ccff6e2cb7fc819a776cbd238
status: validating
owner_direction: ACCEPTED_JSON_WITH_CONDITIONAL_MEASURED_JSONL
implementation_authority: NONE
production_authority: NONE
merge_authority: REPOSITORY_CONTROL_PLANE_ONLY
```

## Purpose

Deliver the bounded source-profile decision authorized by Issue #162 comments `5745395297` and `5745433817`. The decision supplies one canonical editable JSON project destination for the existing Reference typed model/linker, with conditional measured JSONL, non-authoritative editor metadata, exact import provenance, semantic three-way reimport and coherent saves.

It does not select the World Bundle encoding/chunking/compression, create source/runtime code, set production/full-world maxima or change bootstrap v1.

## Exact custody

1. `docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V1_DECISION.md`
2. `docs/agents/tasks/active/OTV2-20260919-content-world-cw1-source-profile-504.md`

No other path is owned or changed. The branch was allocated from protected `main@03a821edd828e24ccff6e2cb7fc819a776cbd238` after a fresh 28-open-PR path census found no collision.

## Source authority consumed

- ADR-0005: editable project / canonical graph / compiled bundle separation and source authoring properties;
- DUR-04: serializer-independent typed graph, exact Content Lock, deterministic compiler and later measured physical-format decision;
- `CONTENT-504_D1_TYPED_SOURCE_OWNER_BINDING_CONTRACT_CANDIDATE.md` and merged `ReferencePlayableContentSource` / `link_reference_playable` implementation;
- Content/World owner synthesis, execution design, delivery programme and bulk catalogue plan;
- D3 real-batch measurement, including its explicit `SPIKE_RESULT != OWNER_FORMAT_DECISION` and non-production limits;
- owner-approved direction and execution allocation in Issue #162.

## Decision boundary

The decision freezes only what the next source implementation needs:

- strict JSON default and a family-specific measured JSONL extension rule;
- minimum project manifest/profile/lock/document roles;
- one semantic authority through the existing Reference source/linker;
- typed gameplay fields separated from author/editor metadata and tags;
- deterministic canonical writing and strict bounded parsing;
- exact source/importer/mapping/baseline/conflict provenance;
- semantic three-way reimport without silent local overwrite;
- identity independent of paths, shards, lines, chunks and enumeration;
- coherent publication/recovery of a complete project revision.

Bundle carrier, compression, chunking, production maxima, broad future schemas and implementation remain deferred.

## Accepted P1 repair

Independent HIGH whole-diff review of head `86363f5daa5bf11a6265658f5795e21f17ec9b1a` reported P0=0, P1=1, P2=0. Issue/PR comment `5745510543` accepted the P1: relative document locators lacked explicit root containment, traversal, symlink and alias-uniqueness semantics.

The repaired decision now defines lowercase ASCII canonical root-relative locators; fixed root control filenames; rejection of traversal, absolute/drive/UNC/alternate-separator/device/ADS spellings; no-follow root-anchored reads; rejection of symlink/reparse/special/hard-linked source documents; unique filesystem identity across all admitted files; and containment/identity checks for staged writes and commit publication. The first proof includes focused traversal, alias, symlink/reparse, hard-link and output-escape negatives. No filesystem framework or runtime implementation is added.

## Validation and review state

The candidate is documentation-only and must be validated from its published immutable branch head. Required qualification is:

1. exact two-path changed-file readback;
2. `git diff --check` against exact base;
3. repository governance validation applicable to the two documentation paths;
4. architecture decision-discipline self-review;
5. whole-diff review for accidental bundle/resource/runtime/schema expansion;
6. hosted Agent Governance, Architecture Semantic Audit and authoritative `game-gate` on the exact PR head;
7. one independent HIGH whole-diff schema/semantic review after deterministic qualification.

External review is intentionally not triggered by this worker. The repository control plane owns the standing-authorized review invocation, finding disposition, final readiness and Merge Queue action. Hosted run IDs and independent-review evidence belong in live PR/Issue comments so this committed task record does not become stale or require a no-op evidence commit.

## Self-review checklist

- [x] Owner-approved JSON basis is preserved.
- [x] JSONL is conditional on representative family measurement and is not blanket map acceptance.
- [x] Existing `ReferencePlayableContentSource`, `PackageManifestBinding`, `ContentLockBinding` and `link_reference_playable` remain the semantic path.
- [x] Tags/editor metadata have no gameplay, owner, value, collision or projection authority.
- [x] Provenance, baseline, local corrections and explicit three-way conflicts are retained.
- [x] Stable identity is independent of file/storage layout.
- [x] Strict parsing, deterministic serialization and coherent-save invariants are explicit.
- [x] Canonical locators, root containment, alias uniqueness and v1 symlink/reparse/hard-link rejection cover reads and staged writes.
- [x] No production/full-world numeric maximum is invented.
- [x] D3 retains World Bundle/codec/chunk/resource ownership.
- [x] No source code, Cargo, registry, runtime, client, protocol or activation mutation is included.
- [x] Future schemas are extended incrementally; the decision does not demand every future field now.

## Handoff

After repaired published-head local/hosted qualification, return the exact head, PR, two-path diff, check results and this review packet to the #162 control plane. A fresh independent HIGH review must verify the repair and whole diff. Do not trigger independent review, merge, auto-merge or Merge Queue from this worker.

```text
NEXT_REQUIRED_ACTION: CONTROL_PLANE_TRIGGER_ONE_INDEPENDENT_HIGH_WHOLE_DIFF_SCHEMA_SEMANTIC_REVIEW
READY_STATE_BEFORE_REVIEW: LANE_BLOCKED_REVIEW_PACKET_READY
DIRECT_MERGE_OR_ENQUEUE: FORBIDDEN
```
