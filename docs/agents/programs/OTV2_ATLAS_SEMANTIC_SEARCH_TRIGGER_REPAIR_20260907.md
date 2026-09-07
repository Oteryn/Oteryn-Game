# Atlas semantic-search producer trigger repair allocation

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
  - tools/repository/test_validate_game_atlas_semantic_search_triggers.py
read_only_dependency_evidence:
  - tools/game-atlas-semantic-search/export.py
  - tools/game-atlas-fullworld-source/producer.py
  - tools/game-atlas-thais-fixture/export.py
production_authority: FORBIDDEN
external_repositories: []
```

## Fresh protected dependency proof

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

No current broken product is claimed. This is a latent trigger-completeness P2.
Shared Rust/CodeQL qualification is not equivalent to the semantic digest and
Sam/Thais/count oracle.

## Exact repair contract

The material repair must preserve existing semantic-search permissions, pinned
actions, concurrency, self-tests and real pinned-data oracle. It may only add the
minimum protected dependency-trigger coverage required for:

```text
tools/game-atlas-fullworld-source/producer.py
tools/game-atlas-thais-fixture/export.py
```

The preferred first slice is explicit exact paths in both `pull_request.paths`
and `push.paths`. Do not widen to `tools/game-atlas-*/**` merely for convenience.
If implementation proves an exact protected-base dependency helper is safer, stop
and amend this allocation before adding another existing path.

Producer sources are read-only evidence under this allocation. The repair must not
change their semantics to manufacture a trigger test.

## Required TDD / regression

Create one repository-level regression at the allocated new test path. Before the
workflow change, RED must prove independently that each upstream producer path is
absent from both PR and push trigger families while the real workflow still
executes the producer chain.

GREEN must prove:

1. `tools/game-atlas-fullworld-source/producer.py` independently selects the
   semantic-search workflow for PR and protected-main push;
2. `tools/game-atlas-thais-fixture/export.py` independently selects it for PR and
   push;
3. the original semantic-search tool/contract/workflow paths remain selected;
4. unrelated representative paths remain outside this workflow;
5. deleting/renaming either required trigger entry causes the regression to fail;
6. the workflow still contains and executes its deterministic/negative self-test
   and exact semantic digest/Sam/Thais/count qualification commands;
7. no `continue-on-error`, `if: false`, permission widening or test/oracle removal
   is introduced.

The material PR's hosted evidence must also show the real semantic-search workflow
runs when the candidate itself changes the workflow. For producer-trigger behavior,
the regression must validate the exact protected YAML semantics. A later
producer-source PR/push provides natural lifecycle evidence; no benchmark/no-op
producer mutation is authorized solely to manufacture a run.

## Secondary identity finding

The historical audit also mentioned `tools/game-atlas-creatures/identity.py`.
This allocation does **not** add it. Before any later amendment, fresh dependency
proof must establish whether static-creatures' exact oracle lacks equivalent
coverage despite gameplay-profiles. Unknown equivalence is not authority to widen
this repair.

## Activation / integration

```text
this allocation independently reviewed
-> canonical exact-head checks PASS
-> normal FULL Merge Queue
-> protected main readback
-> fresh workflow/path custody readback
-> explicit Work application to one sole #418 writer
-> test-only RED
-> minimal workflow + regression GREEN
-> independent exact-head control-plane review
-> canonical checks + normal FULL Merge Queue
-> protected readback
-> #418 terminal closeout after natural trigger semantics are proven by protected
   regression and required workflow evidence
```

## Excluded scope

No FullWorld/source semantics, Thais fixture semantics, static-creatures identity
change, Atlas product/runtime code, unrelated CI optimization, ruleset, required
status, Merge Queue semantics, workflow permission expansion, production/live
data, secret, external repository or deployment action.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`; no direct merge/bypass.
Runtime product E2E is NOT_APPLICABLE to this allocation-only document.
