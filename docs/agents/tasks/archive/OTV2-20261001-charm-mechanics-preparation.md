# OTV2-20261001-charm-mechanics-preparation

```yaml
task_id: OTV2-20261001-charm-mechanics-preparation
title: Prepare all 25 Charm mechanics and progression rules with qualified source evidence
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-complete-preparation-20261001
pr: 1434
base_sha: edad9408b6996297d9768fe299a6c351e08dc393
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root of this owner-authorized preparation task
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/charm-authoring/mechanics.schema.json
  - tools/content-schema/charm-authoring/charm_mechanics.py
  - tools/content-schema/charm-authoring/test_charm_mechanics.py
  - tools/content-schema/charm-authoring/test_charm_authoring.py
  - tools/content-schema/charm-authoring/test_tibiapal_evidence.py
  - tools/content-schema/charm-authoring/verify_tibiapal_browser.py
  - tools/content-schema/charm-authoring/samples/test-tibiapal-planner.cjs
  - tools/content-schema/charm-authoring/samples/tibiapal-description-calculator-evidence-2026-10-01.json
  - tools/content-schema/charm-authoring/samples/tibiapal-planner-execution-2026-10-01.json
  - tools/content-schema/charm-authoring/samples/tibiapal-browser-verification-2026-10-01.json
  - tools/content-schema/charm-authoring/samples/charm-mechanics-sources-2026-10-01.json
  - tools/content-schema/charm-authoring/samples/charm-source-resolution-2026-10-01.json
  - tools/content-schema/charm-authoring/README.md
  - tools/content-schema/charm-authoring/INTEGRATION.md
  - rulesets/progression/charms/index.json
  - rulesets/progression/charms/mechanics.json
  - rulesets/progression/charms/progression.json
  - docs/agents/tasks/archive/OTV2-20261001-charm-mechanics-preparation.md
public_contracts: []
depends_on: [CHARM-0, CHARM-1, CHARM-3, CHARM-4, CHARM-5-REG, D186]
blocks: []
external_repositories: []
cross_repository_coordination_id: null
```

## Outcome and owner scope

The owner requested all possible reference/schema/data completion in this session and explicitly
authorized subagents. Gameplay connection remains the coordinator and workers' task. The local
branch and its newly created remote counterpart have one publisher (root); child implementation
owns only the path subset listed in its task packet, and all reference/audit children are read-only.
This owner instruction authorizes this bounded preparation despite the earlier programme's
closeout/no-new-allocation snapshot. It grants no integration or review-trigger role.

The supplemental package binds all 25 existing Charm keys and their catalogue digest, expresses
mechanics/progression with a closed schema, and retains source-qualified parameters, source
conflicts and field-level unknowns. Existing content identities, revisions, runtime, migration,
protocol, capability and entitlement implementations are outside the changed paths.

The human integration packet is `tools/content-schema/charm-authoring/INTEGRATION.md`; commands
and schema boundaries are documented in that directory's README. An OTS reference candidate
is not automatically a new accepted runtime rule or a claim of live Tibia verification.

## Architecture and source of truth

- PROVEN: accepted CHARM-0 owner answers §§7–8; D186 in the accepted owner decision batch
  D174–D235 refines secondary-target behavior for critical/leech.
- PROVEN: all 25 stage costs, bonus values and categories agree with separately extracted
  TibiaPal `61ffa3e0502879ccec44e59ead859e92b6d88531`, Canary
  `47dfd51f45280a59a1d3e50ba7edd573d7234446`, and Crystal
  `00ce02a57ca5a12e48f32a3476e37471167e4c3f` source data.
- DERIVED: detailed OTS mechanics are facts about those revisions, classified
  OTS_HYPOTHESIS_ONLY for Tibia parity. Shared ancestry is not independent official corroboration.
- CONFLICT: Cleanse eligibility, incoming effect order, Parry armor callsites, leech and critical
  algorithms, Scavenge success scaling, Carnage static cap leakage and first reset fee.
- UNKNOWN: exact official speed formulas, condition stacking/immunity, source-specific rounding,
  some area geometry/leech details and excluded Store/potion/reset lifecycle.
- Bound META policy: 3.1.0 at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, resolved at admission.

## High-risk authority/recovery qualification

NOT_APPLICABLE: offline source preparation and validation; no production mutation, persistence,
session/lease fence implementation or authority-bearing recovery interpretation. The selected
local Git publication uses the exact bound META publication guard, an expected remote predecessor,
one exclusively owned task branch and verified recovery before remote head readback. No merge,
enqueue or owner-funded review trigger belongs to this worker.

## Validation

- Original catalogue: build, schema validation and content regeneration check.
- Supplemental package: schema validation, catalogue/source bindings, complete 25-key coverage,
  independently extracted three-source comparisons and negative cases.
- Existing Charm workflow test entry includes the supplemental checks; no workflow change.
- Native Chromium 151.0.7922.173 executed the pristine pinned TibiaPal `_site`: all 25 cards,
  150 purchase/refund transitions, 42 major budget boundaries, minor budget/refund guards,
  reset and 66 calculator cases passed. Source/deployed-snapshot JS hashes match.
- Independent Node VM execution of the pinned planner: 412 checks passed. All 25 descriptions'
  costs/bonuses match; observed calculator rounding and omitted level cap are retained.
- Live domain was blocked by the proxy (`ERR_TUNNEL_CONNECTION_FAILED`). Local snapshot
  browser evidence is not a deployed-site or official battle verification. `cargo` was not
  available, so Rust runtime tests were not rerun in this session.
- Python Ruff lint/format, governance validator/tests and changed-file whitespace check.
- Baseline full-game tree validator: FAIL `SOURCE_ID_BOUNDARY_MISSING` on admission main,
  before this task's changes; already named by programme STATE. Outside this task's owned paths.
- First preparation head: PASS, 8 authoring-entry tests including 18 supplemental tests;
  31 source-file hashes across four repositories and 75 independent numeric comparisons;
  Ruff lint/format; governance validator and 36 governance tests; materialized tree 97/97.
- Successor: PASS, 9 authoring-entry tests including 18 mechanics and 9 TibiaPal evidence
  tests; opt-in portable Node replay 412/412; native browser run 25/150/42/66; the source,
  catalogue, Ruff, governance and materialized-tree checks remain required at the frozen head.
- Independent preparation audit: PASS by `charm_independent_review`; Gut source qualification,
  generated-damage bindings for Carnage/Parry and historical README clarity findings resolved.
  Fatal Hold's targeting binding preserves CHARM-0/D186. This is a preparation review,
  not the programme's protected-integration qualification or an owner-funded review trigger.
- Hosted exact-head CI and programme review: coordinator qualification after publication.

The owner's subsequent instruction required checking the available references before claiming
completion. The branch returned to AUTHORING for portable execution harnesses and persisted
evidence. Successor qualification supersedes the first published preparation head; the current
PR metadata names the exact frozen candidate. No runtime connection was added.

Hosted Charm CI then exposed a Ruff import-classification difference between repository-root
and authoring-directory execution. The branch returned to AUTHORING for a test-local jsonschema
import; both exact Ruff invocations pass. Fixtures and browser/Node harnesses are unchanged.
Final qualification must include the authoring-directory commands used by CI.

## Closeout boundary

This record is included in the preparation PR's final authoring commit, so it reaches protected
main only with that PR. The PR URL and exact frozen SHA are recorded by GitHub and the final
owner-facing report after publication; a commit cannot contain its own hash. Archive placement
does not assert integration. The coordinator retains review-trigger and protected-integration
ownership; gameplay connection and source-conflict adjudication remain named follow-ups.

## Owner-requested source continuation, 2026-10-01

The owner resumed this exact preparation task and asked for subagents using Canary and Crystal.
Root remains the one publisher of `codex/charm-complete-preparation-20261001`; all five source
workers owned only separate local evidence outputs. The preparation returned to AUTHORING
from predecessor `ca991aa1fb12fb656ea3d39f5736d9745769027e` for this bounded evidence repair.
Gameplay connection, protected integration and provider review triggers remain coordinator work.

The new qualified source packet covers every one of the original 32 question entries and
retains concrete consumer recommendations, exact source quotations and 45 pinned file hashes.
Two claims that Crystal omits Adrenaline/Numb on mana drain were disproved and removed.
The captured facts, closed schema cardinalities and generated mechanics were updated together.
Fatal Hold's Crystal area filter and generated-damage callback, Carnage summon provenance,
and the physical-damage owner locator are corrected/clarified. No source defect is promoted
to an accepted runtime rule, and official parity remains distinct from source behavior.

Five lanes passed 227 source/formula/reference-model checks. Independent source review checked
all 32 entries, 190 citation hashes/ranges and 127 excerpts; its Fatal callback finding was
repaired and the critical threshold operator made explicit. Offline checks reject missing or
duplicate coverage, pin/inventory drift, OTS promotion and accidental runtime activation.
These checks are preparation evidence; no fork server or connected Oteryn gameplay was executed.

Publication is one new API-native candidate with this complete bounded delta and the exact
predecessor as its sole parent, on this exclusively allocated preparation branch. The native
atomic expected-head operation is unavailable; the bound META connector-compatible route
uses a fresh predecessor read, one non-force ref update and immediate exact-head readback.
Local edits select no material Git commit for publication. Candidate freeze and exact-head
qualification follow the remote readback; a changed head requires reconciliation.
