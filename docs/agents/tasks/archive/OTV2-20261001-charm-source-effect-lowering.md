# OTV2-20261001-charm-source-effect-lowering

```yaml
task_id: OTV2-20261001-charm-source-effect-lowering
title: Closed canonical Charm effect lowering
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/charm-exact-source-json-20261001
branch: codex/charm-source-effect-lowering-20261001
pr: null
issue: 162
base_sha: 59befa58e730a9251eb2bacaf1bce2bdd95bf7f2
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, sole publisher; Charm source preparer
created_at: 2026-10-01
updated_at: 2026-10-01
owned_paths:
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/charm_source_effect.rs
  - apps/game-server/src/content/charm_source_effect_tests.rs
  - docs/agents/tasks/archive/OTV2-20261001-charm-source-effect-lowering.md
public_contracts: []
depends_on: [CHARM-0, CHARM-3, CHARM-4, 1508]
blocks: []
external_repositories: []
```

## Bounded result

Lower all25 canonical raw effect objects into the existing18 closed CharmEffect shapes.
Reuse the exact RawValue decimal and duplicate-decoded-member helpers from PR1508; every
known field is typed, required and consumed, then leftovers reject. Element/resource selectors
are closed; damage mitigation flags are faithfully copied. Domain semantic rules remain with
the existing combat definition owner, including zero duration/cap rejection.

357 new module/test lines plus targeted wiring. The temporary unused-code allowance moves
from the JSON prerequisite to this effect module until the actual canonical container decoder
consumes it. Caller must bound raw source bytes with existing Content evidence limits before
invocation. No public API/contract, new limit defaults, source-ID authority, generation issuer,
runtime activation or capability advertisement is introduced.

## Validation and remaining work

Actual25 effects have independently specified expected enums/parameters and18 discriminants.
Each real effect rejects every missing/wrong-type member plus unknown members; tests cover
duplicate decoded Unicode/type/duration/percentage keys, precision/exponent/overflow, element,
resource and flag types. Existing CharmDefinitionError::InvalidParameter explicitly owns zero
duration/cap refusal. Tests retain every oracle and use Result/? without lint allowances.
The first compiler attempt found the test's source include one directory too high; only that
path was corrected to the actual repository file before qualification. Focused4PASS, full
server library1286PASS/2existing ignored, workspace fmt/diff and governance/lifecycle13PASS.
Strict workspace all-target Clippy PASS. Exact independent review belongs to the freeze packet.

The canonical container/domain/cost/stage/budget projection is separately allocated to keep
both coherent batches reviewable. Complete738-line source draft is preserved; no assertions
were dropped or minified. Qualified Content generation attachment and native battle composition
remain unfinished. The original nine-row parent remains IMPLEMENTING. Root publishes guarded
actual Git identity. Stack CI requires base main; CP owns retarget/requalification after PR1508
integration. This archive reaches main only if its PR merges; a commit cannot contain its own
final frozen SHA.
