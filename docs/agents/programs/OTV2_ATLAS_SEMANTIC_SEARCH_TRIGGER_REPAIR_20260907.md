# Atlas producer trigger-completeness repair allocation

Coordinator: #162. Safety issue: #418. Remediation programme: #364.

## Status

Prospective control-plane/test allocation only. **NOT_ACTIVE**. No workflow,
producer, product/runtime, production or external-repository mutation follows
from this document until a later explicit Work application.

```yaml
allocation_id: OTV2-ATLAS-SEMANTIC-TRIGGER-418-20260907
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
programme_issue: 364
safety_issue: 418
allocation_base_main_sha: 4d6139083179b8fd8c5d0497b2abf8c2545de599
allocation_state: NOT_ACTIVE
worker_launch: NONE_UNTIL_WORK_APPLICATION
owned_paths_after_application:
  - .github/workflows/game-atlas-semantic-search.yml
  - .github/workflows/game-atlas-static-creatures.yml
  - tools/repository/test_validate_game_atlas_semantic_search_triggers.py
read_only_dependency_evidence:
  - tools/game-atlas-semantic-search/export.py
  - tools/game-atlas-fullworld-source/producer.py
  - tools/game-atlas-thais-fixture/export.py
  - tools/game-atlas-creatures/export.py
  - tools/game-atlas-creatures/identity.py
production_authority: FORBIDDEN
external_repositories: []
```

## Fresh protected dependency proof

### Semantic-search chain

On protected `main@4d6139083179b8fd8c5d0497b2abf8c2545de599`, the semantic-search
workflow's PR and push path filters include only its own semantic-search tool
family, contract and workflow. The exact qualification nevertheless executes a
deeper producer chain:

```text
game-atlas-semantic-search/export.py
  -> _load_fullworld_producer()
  -> game-atlas-fullworld-source/producer.py
  -> _load_bounded_module()
  -> game-atlas-thais-fixture/export.py
```

The workflow then asserts the exact semantic digest, record/kind counts, Sam
identity/position/capability and Thais identity/position. Changes to either
upstream producer can therefore affect this exact oracle while not selecting the
workflow today.

### Static-creatures identity chain

Fresh follow-up verification from #418 proves a second independent omission:

```text
game-atlas-creatures/export.py
  -> from identity import stable_creature_entity_id
```

`game-atlas-static-creatures.yml` performs the exact pinned world/NPC/monster
static export and freezes the static census and semantic digest, but its PR path
filter omits `tools/game-atlas-creatures/identity.py`.

`game-atlas-creature-gameplay-profiles.yml` does trigger on `identity.py` and runs
the shared static self-test, but it checks out only NPC/monster definition
evidence and builds a distinct gameplay-profiles product with a different census
and digest. It does not run the static workflow's pinned world spawn export or its
exact static census/digest. Therefore gameplay-profiles is partial coverage, not
oracle-equivalent coverage for the static product.

No current broken product is claimed. These are latent trigger-completeness P2s.
Shared Rust/CodeQL qualification is not equivalent to the affected Atlas oracles.

## Exact repair contract

The material repair must preserve existing workflow permissions, pinned actions,
concurrency, self-tests and real pinned-data oracles.

### Semantic-search workflow

Add only the exact missing producer dependencies to both existing trigger
families:

```text
pull_request.paths:
  tools/game-atlas-fullworld-source/producer.py
  tools/game-atlas-thais-fixture/export.py
push.paths:
  tools/game-atlas-fullworld-source/producer.py
  tools/game-atlas-thais-fixture/export.py
```

Do not widen to `tools/game-atlas-*/**` merely for convenience.

### Static-creatures workflow

Add only:

```text
pull_request.paths:
  tools/game-atlas-creatures/identity.py
```

The existing static workflow event model is PR + `workflow_dispatch`; this repair
does not invent a protected-main push event for it. The proven gap is that a PR
changing the shared identity helper can bypass the exact static product oracle.

All producer/helper sources above are read-only evidence under this allocation.
The repair must not modify their semantics to manufacture trigger evidence.

If implementation proves another existing path is materially required, stop and
amend this allocation before mutation.

## Required TDD / regression

Create one repository-level regression at the allocated new test path. The test
must parse/inspect protected workflow semantics without relying on PR-editable
manifests to decide required dependencies.

Before the workflow changes, RED must independently prove:

1. semantic-search PR and push filters omit fullworld producer;
2. semantic-search PR and push filters omit Thais fixture producer;
3. static-creatures PR filter omits shared `identity.py`;
4. the workflow bodies still prove the corresponding producer/import dependency
   and exact oracles, so the failures are semantic trigger failures rather than a
   missing unrelated string.

GREEN must prove:

- each semantic producer path independently selects semantic-search for PR and
  protected-main push;
- `identity.py` independently selects static-creatures for PR;
- original self-owned tool/contract/workflow paths remain selected;
- unrelated representative paths remain outside these specialized workflows;
- deleting/renaming any of the three newly required entries fails the regression;
- semantic-search still contains deterministic/negative self-tests and exact
  digest/Sam/Thais/count qualification;
- static-creatures still contains deterministic producer self-test, exact pinned
  double export, static census/roles and exact semantic digest qualification;
- no `continue-on-error`, `if: false`, permission widening, action-pin weakening
  or test/oracle removal is introduced.

The material PR's hosted evidence must show both changed workflows run on the
workflow-changing candidate because each workflow self-triggers on its own file.
The regression proves future producer/helper trigger semantics. No no-op producer
mutation is authorized solely to manufacture a run.

## Activation / integration

```text
this repaired allocation independently reviewed
-> canonical exact-head checks PASS
-> normal FULL Merge Queue
-> protected main readback
-> fresh workflow/path custody readback
-> explicit Work application to one sole #418 writer
-> test-only RED against protected baseline semantics
-> two minimal workflow trigger edits + one regression GREEN
-> independent exact-head control-plane review
-> canonical checks + both Atlas workflow hosted runs
-> normal FULL Merge Queue
-> protected readback
-> #418 terminal closeout after trigger semantics are protected
```

## Excluded scope

No FullWorld/source semantics, Thais fixture semantics, creature identity
semantics, static/gameplay producer semantics, Atlas product/runtime code,
unrelated CI optimization, broad Atlas glob, ruleset, required status, Merge
Queue semantics, workflow permission expansion, production/live data, secret,
external repository or deployment action.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`; no direct merge/bypass.
Runtime product E2E is NOT_APPLICABLE to this allocation-only document.
