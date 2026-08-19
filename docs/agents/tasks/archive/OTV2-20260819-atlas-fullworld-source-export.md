# OTV2-20260819-atlas-fullworld-source-export

```yaml
task_id: OTV2-20260819-atlas-fullworld-source-export
title: Game-owned Atlas full-world source projection adapter
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
coordination_origin_branch: feat/OTV2-20260819-atlas-fullworld-source-export
pr: 25
base_sha: 9e594ceb292cb2a54bf968fb057501b743443728
head_sha: af1ec5bcc021fdd62841bc0c88da2e8d579d0c4b
final_head_sha: af1ec5bcc021fdd62841bc0c88da2e8d579d0c4b
merge_sha: d7c207876920dfe7c7026a0d1316a2ee603ebf0d
owner: released
created_at: 2026-08-19T09:52:53+02:00
cross_repository_coordination_id: ATLAS-FULLWORLD-COORDINATOR
external_repositories:
  - Oteryn/Oteryn-Atlas
blocks_released:
  - ATLAS-FULLWORLD-LOCAL-GENERATION-FABRIC
  - ATLAS-FULLWORLD-COMPILER-PUBLICATION
```

## Terminal outcome

Completed and merged. The Game-owned offline source-projection adapter now provides the authoritative Game-side input used by the Atlas full-world local-generation fabric. It broadens spatial selection while preserving the already-qualified per-tile DYN projection semantics.

Full-world qualification found exactly one visible appearance absent from the exact pinned 15.32 catalogue: source ID `2141` at legacy `(33572,32528,14)`. The producer preserves it explicitly as `UNRESOLVED_APPEARANCE` with no resolved primitives and never substitutes or infers another appearance or sprite.

## Exact source and qualification evidence

- Game delivery head: `af1ec5bcc021fdd62841bc0c88da2e8d579d0c4b`.
- Squash merge to Game `main`: `d7c207876920dfe7c7026a0d1316a2ee603ebf0d`.
- Producer content SHA-256 used by the successful generation: `b3fcb59a8a5df3f5e9acb25036086215a60b8d66c8a01985172707559edf1a2f`.
- Legacy importer: `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.
- `world.otbm` SHA-256: `3bd40d14fefec41f24c4b3ae879e420be1a831ef55b95dcbec721e587a09b034`.
- exact `15.32.zip` SHA-256: `1a6bad8b7598cd874f534cd4aae2d249fb3d9b4458b3ccfa75754f91bb27870f`.
- catalog SHA-256: `35639e000c4c108665a091cfbdf699d549d995b37670bc08de575ab6cd380d85`.
- appearance SHA-256: `dc4f4c01e3701c77877c67895168e4399837046122d6d17e3e608a12a2fed075`.
- Bounded Thais Z7 replay after the repair: PASS byte-identical, 24,311 tiles / 39,282 primitives, SHA-256 `ff14efee3fc376d8f18432c628294c64ffe89450a59aaa498a28e6d705815984`.

## Full-world consumer result

- floors: **16** (`-15..0`)
- tiles: **18,997,668**
- presentation records: **24,502,036**
- resolved primitives: **24,502,035**
- unresolved presentations: **1** (`appearance_source_id=2141`)
- unique appearance source IDs: **25,198**
- unique sprite source IDs: **27,394**
- Atlas fabric root: `sha256:ef72ccea156283eea1efd103577e2933b15b38d1a67aa05c89594cc3a731ea6f`
- independent full-shard handoff verification: **PASS**

## Validation and review

Focused/local validation:

- Python compile: PASS.
- `tools/game-atlas-fullworld-source/self_test.py`: PASS.
- `python3 tools/agents/validate_governance.py`: PASS; 26 required policy documents / 9 project lanes.
- full changed-file scope review: PASS.
- `git diff --check`: PASS.

Exact delivery-head GitHub validation for `af1ec5bcc021fdd62841bc0c88da2e8d579d0c4b`:

- Agent governance run `32244642319`: SUCCESS.
- Merge gate run `32244642400`: SUCCESS.
- Architecture semantic audit run `32244499973`: SUCCESS.
- Merge authority audit run `32244499944`: SUCCESS.
- Merge-gate scope, dependency review, Rust Linux workspace, governance, Rust policy/metadata, CodeQL Python, CodeQL Actions, Rust Windows client and Rust supply-chain jobs: SUCCESS.
- unresolved PR review threads at readiness: 0.

Mandatory implementing-agent full-diff self-review on the exact delivery head: **PASS**, zero material findings.

Independent review: **NOT_REQUIRED** under the trusted-base risk triggers. This offline adapter changed no protocol/session/admission/persistence/value/security/production authority, weakened no governance gate and introduced no new untrusted parser.

## Cross-repository hand-off

Atlas PR #13 consumed the verified local generation hand-off and was subsequently squash-merged to `Oteryn/Oteryn-Atlas` main as `6a1ca4f4a20182b84aca0a824d038e6d086fa959`.

Canonical local compiler input remains:

`/home/mole/oteryn-fullworld/output/fullworld-fabric-v2/handoff.json`

with SHA-256 `1d1ab30a59819e41592d701adb188ea619b1122ccf7298255c2afd08d2841659`.

## Closeout

- implementation PR #25: merged
- task acceptance criteria: satisfied
- exact-head CI: green
- self-review: green
- material review findings: none
- ownership: released
- downstream compiler/publication hand-off: unblocked
- original task branch: safe to delete after this archive closeout lands
