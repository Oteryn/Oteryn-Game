# Atlas semantic-search oracle and producer-trigger amendment

Coordinator: #162. Programme: #364. Safety issue: #418. Existing material PR: #426.

## State

```yaml
allocation_id: OTV2-ATLAS-SEMANTIC-ORACLE-TRIGGER-20260908
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: dfc0fd3a9148cb85b7c75e7cad3b15a1fe70d2eb
allocation_state: NOT_ACTIVE
preparation_branch: coord/atlas-semantic-oracle-closure-418
worker_branch: agent/atlas-semantic-trigger-418
source_material_head: 07846952f6f2e3888410e2e89c0e6c2107e2a662
source_material_tree: bcaa3df6befee07c7c5fab137548157a1f66bcf2
source_rebaseline_comment: 5580864385
risk: CONTROL_PLANE
```

This is a prospective allocation-only amendment for the SAME #418/#426 material worker. It grants no present workflow mutation authority and creates no replacement branch or worker. Material application is permitted only after independent exact-head review, canonical checks, normal FULL Merge Queue, protected-main readback and fresh Work custody verification.

## Proven stale oracle

Fresh hosted semantic-search execution on exact #426 head reproduced the same failure twice after the new trigger-regression itself passed 5/5. The current source-derived semantic product is deterministic and differs from the workflow's historical frozen digest.

Independent read-only reproduction in comment `5580864385` proved on protected Game source `8ec8fedd23e7cd6b0acb3c0848baf4f7b629919f` with pinned legacy `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`:

- current semantic digest: `sha256:035c911b11e588a969e8fb642965772eee598c30ad80d5d04f9ceda32461e530`;
- historical workflow digest: `sha256:0cc0546aff0e9a8f85716dcbe5babc6043e148e946a448b1d87f083410cb1005`;
- records: `88684`;
- kinds: `monster=87565`, `npc=1068`, `town=33`, `waypoint=18`;
- `input_floor_aliases['7'] = -7`;
- static input digest: `sha256:81505e91d7089f91e71813ec43f97118932db9cc7fd76d291fa399447ee2dfa4`;
- static input census: `1068` NPCs and `87565` monster spawns.

Two independent protected-base exports were byte-identical for both the static and semantic serialized products.

The accepted semantic spot checks remain stable and were independently reproduced:

- Sam: exactly one record, id `npc:726487438c8308abf291622a52d91b24`, position `{x:32361,y:32198,floor:-7}`, contains `shop`, service-resolution state `RESOLVED`;
- Thais: exactly one record, id `semantic-record:23716a35099a04179f7b9e3e6c9198ee`, position `{x:32369,y:32241,floor:-7}`, `bounds = null`.

Therefore the current digest is a valid rebaseline only together with the unchanged counts/Sam/Thais/floor/static-product assertions. This amendment does not authorize weakening or deleting those semantic oracles.

## Proven drift attribution

The original semantic workflow/oracle at commit `25d628d6f7ebf4976ebd6d2a7dee9df21f19e3e3` reproduced the historical semantic digest `0cc0546a...` and prior static digest `01921968...` exactly.

Commit `0161b80c351b644b47c28b290a6f54b44f775de7` (`feat(atlas): export factual NPC roles`) changed the canonical static product to current digest `81505e91...` and changed the semantic product to current digest `035c911b...` while leaving the required record census and Sam/Thais semantics stable. This is the proven origin of the stale semantic oracle.

Commit `b56ce339281d252a9e01a5a2bed583582bf29e68` introduced the shared `tools/game-atlas-creatures/identity.py` seam but did not further change the pinned product. It nevertheless created a real direct producer dependency: future identity changes can change creature IDs and therefore both static and semantic products.

The stale oracle was able to persist because semantic-search never selected on the creature producer paths that changed after the workflow was created.

## Minimal same-repository semantic producer closure

The exact Game-owned product-producing source closure is:

```text
tools/game-atlas-semantic-search/export.py
tools/game-atlas-creatures/export.py
tools/game-atlas-creatures/identity.py
tools/game-atlas-fullworld-source/producer.py
tools/game-atlas-thais-fixture/export.py
```

The semantic-search workflow already covers `tools/game-atlas-semantic-search/**`. PR #426 already adds the FullWorld and Thais producer paths. The smallest missing direct producer additions are therefore exactly:

```text
tools/game-atlas-creatures/export.py
tools/game-atlas-creatures/identity.py
```

Both must select semantic-search on `pull_request` and protected-main `push`. A sibling-workflow cross-trigger is not a substitute for direct product-source closure.

Test/docs/routing paths such as semantic `self_test.py`, the repository regression, contracts and workflow files are validation/control surfaces, not additional semantic JSON producer inputs. External legacy parser/NPC/monster/map inputs remain pinned migration evidence and are not same-repository trigger paths.

## Exact future material authority

After protected application, the SAME #426 worker may modify only these already-owned files for this amendment:

1. `.github/workflows/game-atlas-semantic-search.yml`
   - add `tools/game-atlas-creatures/export.py` to both existing semantic-search PR and protected-main push path sets;
   - add `tools/game-atlas-creatures/identity.py` to both existing semantic-search PR and protected-main push path sets;
   - replace only the stale exact semantic digest `sha256:0cc0546aff0e9a8f85716dcbe5babc6043e148e946a448b1d87f083410cb1005` with the independently reproduced deterministic digest `sha256:035c911b11e588a969e8fb642965772eee598c30ad80d5d04f9ceda32461e530`;
   - preserve the existing exact record/kind counts, Sam identity/position/shop/service-resolution assertions, Thais identity/position/bounds assertion and floor-alias assertion;
   - preserve current permissions, pinned actions, concurrency, event model, all original triggers and PR #426 FullWorld/Thais/regression/cross-workflow additions.

2. `tools/repository/test_validate_game_atlas_semantic_search_triggers.py`
   - require both creature paths in semantic-search PR and protected-main push trigger closure;
   - add mutation negatives proving removal/rename of either creature dependency fails the regression;
   - require the current proven semantic digest while continuing to require all existing semantic/static exact oracles and safety properties.

`.github/workflows/game-atlas-static-creatures.yml` is not required to change for this amendment and must remain byte-identical to the current #426 candidate unless a separately proven blocker is raised. The original #418 allocation remains the authority for its existing changes.

## Required TDD and hosted qualification

The material worker must preserve the current failing exact-head hosted semantic run as truthful RED evidence. GREEN must prove:

- creature `export.py` independently selects semantic-search for PR and protected-main push;
- creature `identity.py` independently selects semantic-search for PR and protected-main push;
- FullWorld and Thais direct triggers from #426 remain selected;
- the shared regression/cross-workflow self-protection remains intact;
- the exact current source-derived semantic digest is `035c911b...`;
- record/kind counts remain `88684` / `87565,1068,33,18`;
- Sam and Thais exact assertions and floor alias remain unchanged;
- static input digest/census remain protected;
- no permission widening, action-pin weakening, event-model expansion, `continue-on-error`, `if:false`, producer/helper semantic mutation or unrelated routing change occurs.

Run the focused regression and Python compilation, repository-policy/governance validation and diff checks before publication. Fresh hosted evidence must include both allocated Atlas workflows when selected by the candidate, with semantic-search passing the full real pinned-data oracle. Runtime product E2E is NOT_APPLICABLE to this control-plane/oracle synchronization; the real pinned-data producer run is mandatory.

## Explicit exclusions

No producer/helper source semantics, FullWorld/Thais source, creature identity/export implementation, Atlas runtime/product schema, broad Atlas glob, static-creatures event expansion, unrelated workflow, ruleset, status, Merge Queue, permission, action pin, secret, production, deployment or external repository mutation is granted.

Do not reinterpret the new digest as a general authority to rebaseline future drift. Any future source-derived oracle change requires fresh deterministic proof and an exact protected amendment.

## Integration lifecycle

```text
this allocation-only amendment
-> independent exact-head review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> fresh Work custody verification
-> explicit application to the SAME #418/#426 worker
-> minimal semantic workflow + regression edits
-> focused GREEN + real pinned-data hosted semantic/static runs
-> independent exact-head review
-> canonical checks + normal FULL Merge Queue
-> protected readback
-> #418 terminal closeout when trigger/oracle semantics are protected
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
