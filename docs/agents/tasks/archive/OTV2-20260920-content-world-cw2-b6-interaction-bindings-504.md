> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical PR #698 is merged on protected main as `a0f05f19d0caf6e44d704f59d383218b5663af73`. Any nonterminal/checkpoint wording below is historical provenance only; live GitHub and current protected state supersede it.

# OTV2-20260920-content-world-cw2-b6-interaction-bindings-504

```yaml
task_id: OTV2-20260920-content-world-cw2-b6-interaction-bindings-504
title: CW2-B6 static spatial interaction binding catalogue
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-cw2-b6-interaction-bindings-504
issue: 162
pr: null
base_sha: f4f1292544b1fffbb09e9df6ebefd5c8bb1f7879
head_sha: null
owner: "Oteryn: content world import"
allocation_comment: 5749276094
capability_amendment_comment: 5749426934
risk_addendum_comment: 5749431370
shard_amendment_comment: 5749765639
owned_paths:
  - tools/reference-world-corridor-census/interaction_binding_catalog.py
  - tools/reference-world-corridor-census/interaction_binding_catalog_self_test.py
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/action-id.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/unique-id.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/teleport-destination.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/house-door-id.json
  - docs/agents/tasks/active/OTV2-20260920-content-world-cw2-b6-interaction-bindings-504.md
  - .github/workflows/content-world-b6-interaction-bindings.yml
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/teleport-destination-0001.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/teleport-destination-0002.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/teleport-destination-0003.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/house-door-id-0001.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/house-door-id-0002.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/house-door-id-0003.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/house-door-id-0004.json
  - docs/agents/evidence/OTV2-20260920-content-world-cw2-b6-interaction-bindings/house-door-id-0005.json
public_contracts: []
production_authority: NONE
closure: CANDIDATE_ONLY
external_repositories:
  - zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a
  - blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce
```

## Outcome

Produce one deterministic whole-map candidate catalogue for exactly four
structural source families:

- `ACTION_ID`;
- `UNIQUE_ID`;
- `TELEPORT_DESTINATION`;
- `HOUSE_DOOR_ID`.

The product remains `OTS_HYPOTHESIS_ONLY / CANDIDATE_ONLY`. It mints no
native Oteryn interaction identity, executes no Lua, promotes no legacy storage
integer to quest state, grants no house ACL/value/runtime authority and makes no
Reference parity claim.

## Exact pinned source

Fresh source generation:

- repository: `zimbadev/crystalserver`;
- revision: `ff7ede593c69d4c658b382c97443e8155926924a`;
- path: `data-global/world/world.otbm`;
- bytes: `52,267,895`;
- Git blob: `e95e8f7c7a95d1b634b49a5dea5a5dc76021406b`;
- SHA-256: `09cce62af6c86644b5579fba460c674585261eb987ca5aa1f52baef9e91f8bbb`;
- source profile: `oteryn-crystalserver-fresh-source-generation-v2`, revision `2`.

Parser lineage:

- `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`;
- `tools/otbm_atlas/__init__.py` blob
  `047d1274022e1d2a71d6aa23d6efcf420a24535d`;
- `tools/otbm_atlas/assets.py` blob
  `25ed2400813bb3ccdc54482967ed05197eb1a850`;
- `tools/otbm_atlas/semantic.py` blob
  `a11343a472145aee4d9cf65c6ce28b3e4a71a2b3`;
- `tools/otbm_atlas/nodefile.py` blob
  `bed6f7a803d9de485c1f03cbdca4be0cb1521d30`.

Protected Game seams reused read-only:

- `tools/game-atlas-fullworld-source/producer.py` blob
  `740a96fe1b1d97f32c56e278725ab9c2de6e724b`;
- `tools/reference-world-corridor-census/census.py` blob
  `f056063ad1ae1476fcb73d97e918553a977a0a15`.

Pinned asset hashes remain:

- ZIP: `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`;
- catalog: `35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85`;
- appearances: `dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075`.

Protected full-stream anchors are exactly:

- MapHeader: 1;
- Tile: 18,997,668;
- Town: 33;
- Waypoint: 18.

## Bootstrap publication protocol

The current worker execution surface cannot materialize the 52 MB binary
directly. The bounded repository-native route from #162 comment
`5749426934` therefore applies:

1. publish generator, self-test, this task record and the one allocated
   workflow only;
2. open one Draft PR on the canonical B6 branch;
3. let the exact-head workflow checkout and verify the pinned fresh source and
   parser;
4. execute two complete normal builds plus one reversed-enumeration build;
5. upload only bounded generated B6 evidence/metadata, never the raw source;
6. read back and verify that artifact;
7. publish the five allocated evidence files to the same branch;
8. freeze the evidence-complete head and rerun normal FULL exact-head
   qualification.

The bootstrap head is not catalogue completion and must not invent family
counts, digests or success evidence before the real hosted pass.

## Real-source checkpoint

Hosted exact-source workflow run `35507934625` on bootstrap head
`bddb0c80f988dca63bc95b73e83e9d6a05f3d69f` completed SUCCESS and proved:

- exact strict stream: 1 MapHeader / 18,997,668 Tile / 33 Town / 18 Waypoint;
- ACTION_ID: 0;
- UNIQUE_ID: 0;
- TELEPORT_DESTINATION: 2,405;
- HOUSE_DOOR_ID: 4,527;
- total: 6,932;
- logical product digest:
  `ae44555700628297e9e4e161928ee7b8bf246484cde402b0cfed342d445cd13e`;
- normal/normal/reversed determinism: PASS;
- `silent_drop=0`, `unclassified=0`;
- no native/executable/quest-runtime promotion.

The original monolithic TELEPORT and HOUSE_DOOR family files exceeded the
high-level publication envelope. #162 comment `5749765639` grants deterministic
storage-only sharding: 3 TELEPORT shards and 5 HOUSE_DOOR shards, each at most
600,000 bytes, with compact family manifests at the original family paths.
Sharding must not change the semantic/product or family records digests.

## Semantic boundaries

Every emitted occurrence is an immutable source structural fact with:

- exact source position/item order/source item ID/value;
- `OTS_HYPOTHESIS_ONLY` evidence classification;
- native binding disposition `UNRESOLVED`;
- semantic disposition `UNKNOWN`;
- one family-specific reason code;
- `executable_eligible=false`;
- no runtime or parity authority.

The catalogue must expose explicit aggregate classes for
`UNKNOWN / UNSUPPORTED / AMBIGUOUS / CONFLICT / LOSS`.
No source fact may disappear silently.

Legacy AID/UID/source-local numbers, coordinates, display names and hashes are
not canonical Oteryn identity. Teleport destinations are not runtime relocation
authority. House-door IDs are not house ACL authority.

## Acceptance

Final evidence-complete candidate requires all of the following:

- exact fresh source size/blob/SHA and exact parser revision/blobs verified;
- exact full-stream anchor counts reproduced;
- exact four-family occurrence and unique-value counts frozen from the real
  source;
- two complete normal builds byte-identical;
- reversed-enumeration build byte-identical;
- `silent_drop=0`;
- `unclassified=0`;
- no native/executable/quest-runtime promotion;
- malformed structural records and duplicate canonical occurrence IDs fail
  closed;
- existing fullworld/census regressions remain green;
- workflow artifact contains no raw OTBM/source checkout/assets;
- final stable head passes repository-selected FULL qualification and aggregate
  `game-gate`.

If any generated family file is too large for the authorized high-level
publication route, stop before evidence publication and return one exact
deterministic sharding/custody amendment request. Do not use raw Git
reconstruction.

## Validation

Bootstrap validation:

- `python -B tools/reference-world-corridor-census/interaction_binding_catalog_self_test.py`;
- `python -B tools/reference-world-corridor-census/self_test.py`;
- `python -B tools/reference-world-corridor-census/content_source_batch_self_test.py`;
- `python -B tools/game-atlas-fullworld-source/self_test.py`;
- governance / repository policy / workflow validation;
- `git diff --check`.

Final validation additionally requires the real B6 workflow SUCCESS artifact,
artifact readback, evidence-file binding checks and exact-head FULL Merge Gate.

## Excluded scope

No shared Content model, compiler, runtime, client, renderer, Cargo/lock,
registry, protocol, production/deployment, external-repository write or
additional workflow mutation. No second OTBM parser/downloader/catalogue
framework.
