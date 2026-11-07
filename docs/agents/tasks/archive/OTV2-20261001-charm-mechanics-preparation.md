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
  - tools/content-schema/charm-authoring/samples/charm-global-parity-2026-10-01.json
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

## Owner-requested Global fidelity continuation, 2026-10-01

The owner requested further research toward official Tibia and supplied
`content/assets/files`. This returns the same preparation branch to AUTHORING from
`b588cd46413c18837643a593002609b02a03fd11`. Root is freshly allocated as the sole publisher;
five source workers and one independent asset/review worker write only separate local
evidence artifacts. There is no runtime, protected integration or provider-trigger allocation.

Public-source research adds 73 source records, 83 qualified claims and all25 profiles,
with 75 current community cost and 75 bonus comparisons. It corrects Cleanse11s provenance
and adds dated2026 Hex evidence. Global recommendations for Gut's probability-only model,
actual-HP-loss leech, post-player-resistance Parry and independent Dodge condition delivery
are explicitly withdrawn or reopened. Accepted owner rules remain distinct from externally
documented behavior; ambiguous minor-stage and D186 evidence do not silently amend contracts.

The supplied 15.30 corpus was decoded independently. Six semantic files match their
admission hashes/sizes; no25 Bestiary Charm payloads or formulas exist in those decoded
tables. Generic Charm Upgrade appearance36726 is not a combat specification. Exact raw
research responses remain local; the committed packet retains qualified excerpts and hashes.

Root selects one new API-native candidate on this exclusively allocated canonical branch:
fresh predecessor read, one complete bounded tree, one sole-parent successor commit,
one non-force ref update and immediate exact-head readback under bound META policy.
Existing executable modes must be preserved. Freeze and fresh candidate qualification
follow publication; prior-head CI/review is not inherited. The task remains a draft
preparation handoff, with current-server and connected-consumer proof explicitly absent.

## Completed owner-authorized browser supplement, 2026-10-01

The owner explicitly authorized public Chrome/CDP fallback on their computer and asked the
existing six source/review workers to finish the preparation. Root returns the exclusively
allocated task branch to AUTHORING from `3584c9a27e1a9d47422301846afd61b5106f800f` and remains
the sole publisher. Remote Desktop is restricted to public browser research; no repository,
system, credential or project operation is performed there.

The completed packet contains 97 sources and 149 qualified claims, adding 24 full browser
source records, 66 assessments and 89 checked literal quotations from seven lane reports.
Same-revision fuller wiki access resolves omitted Notes, not a new independent observation.
Official archive8140 closes the minor Bestiary-stage2 documentation gap. Archive8935 already
announces Hex removal and immunity on25August2026;8960 reiterates immunity on8September.
Full pages qualify Carnage last-hit/summon/armor rules, base mana-leech ceil-per-target scope,
Scavenge fractional relative scaling and Bless ordering. TibiaMaps/Exevo model checks remain
community evidence. Inaccessible Reddit and empty video transcripts supply no positive proof.

The full-page records supersede the earlier broad documentary residuals above. Precise
remaining behavioral questions are retained per lane; hidden RNG implementation is not a
blocker where models are observably equivalent. Accepted contracts and inactive runtime
boundary remain unchanged. Root selects exactly one new sole-parent API-native candidate
from the fresh predecessor, preserves executable modes, performs one non-force branch update
and verifies exact head before freeze and fresh qualification. This completes preparation;
connected-runtime qualification remains the separately allocated coordinator/consumer task.

## Owner-requested execution of all incomplete reference items, 2026-10-01

The owner explicitly requested all remaining items be executed with subagents. Root returns
the same exclusively allocated branch to AUTHORING from
`5e126f273cc052484f6975db13a614891063b8ae` and remains the sole publisher. Five read-only
reference lanes own separate local artifact directories; the sixth worker performs
independent inventory/source/execution review. No runtime activation, accepted-contract
change, protected integration or live-account action is allocated here. Remote Desktop
continues to perform public browser research only.

The supplement covers all 19 exact remaining-question groups and maps the 30 earlier
distinguishing protocols. It executes isolated pinned C++ bodies, Lua loot functions and
explicit mathematical models, retaining actual input/output artifacts, report hashes,
source ranges and per-result authority limits. Five historical reported Life Leech traces
provide a real arithmetic discriminator. Primary news5268 strengthens historical additive
matching-family critical outcomes; news8610 confirms all-charms Physical Pierce exclusion.
Explicit Agony-status documentation strengthens its qualified no-known-removal candidate.

All 25 profiles bind the new executed-question claims. The hidden-RNG identity demand is
closed for the proved observably equivalent cases. Current-server and connected-runtime
proof remain precise per-question observations, not fabricated outcomes or runtime defaults.
Root selects one new API-native sole-parent successor after a fresh predecessor read,
preserves executable modes, performs one non-force branch update and verifies exact head.
Fresh validation/review then qualifies the final inactive preparation candidate.

## Owner-requested deployed calculator continuation

The owner resumed Charm completion. The predecessor9a94780a completed its hosted
Charm, Linux, Windows, governance and aggregate game-gate checks successfully.
Root returned this exclusively allocated branch to AUTHORING for five existing preparation
paths; no runtime code, catalogue arithmetic or accepted CHARM-0/D186 contracts change.

Nineteen public deployed UI fixtures now qualify the supplied TibiaMaps and ExevoPan links.
The packet adds five source records and four scoped claims (146sources/172claims total).
Captured public modules close the earlier unavailable-deployed-source gap. TibiaMaps flooring,
cap transitions, display mitigation and its observed immune-Demon preselection defect remain
community-tool facts. ExevoPan's current fixed scalar formulas and12 observed cases agree with
the historical model, including its missing level cap. Neither captured page exposes stages.
Failed synthetic-input and slider attempts are discarded explicitly, not counted as behavior.

Ordinary page HTTPS was attempted first; module HTTPS403 used only the existing Chrome/CDP
public research fallback. All edits, comparisons and publication use the ordinary workspace
and GitHub API. Root remains sole publisher. The same bounded one-successor/non-force route
creates a new candidate from9a94780a, then freezes and freshly validates/reviews its exact SHA.
No current Global game/account action, connected runtime, merge or paid review trigger is added.
