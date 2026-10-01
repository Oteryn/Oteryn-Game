# OTV2-20261001-wheel-authoring-reference

```yaml
task_id: OTV2-20261001-wheel-authoring-reference
title: "Wheel of Destiny and Gem Atelier: complete reference authoring schema"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/wheel-authoring-20261001
pr: 1435
base_sha: edad9408b6996297d9768fe299a6c351e08dc393
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: codex-wheel-authoring-reference
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/wheel-authoring/
  - docs/agents/tasks/archive/OTV2-20261001-wheel-authoring-reference.md
  - .github/workflows/wheel-authoring-schema.yml
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The owner's direct request prepares a complete reference authoring schema for the
five vocations and gems, with a populated candidate, offline rebuild, semantic
validator, allocation/gem checks and an interactive standalone comparison.
This is a separate authoring delivery, not a W-R/GEM-R runtime allocation or an
amendment of the programme's close-out mode. The branch and the three owned paths
have one writer. Publication uses the proven guarded local Git route from the
bound META policy, never sequential Contents API commits.

`completed` describes preparation of this one-PR authoring deliverable. External
review, protected integration and runtime admission are pending. The PR is created
after publication; its description binds the exact frozen head and validation
packet. A commit cannot contain its own SHA. No follow-up metadata-only write is
needed after freeze.

## Architecture and source of truth

- `PROVEN`: WHEEL-0, the Wheel state candidate and WHEEL-GEM-0 define the separation
  of content authoring, Character writers, protocol, combat effects and client UI.
- `DERIVED`: five-vocation slot/perk captures from TibiaPal
  `61ffa3e0502879ccec44e59ead859e92b6d88531`; the topology and mitigation from Canary
  `99902524e052f37574194466c2949c576e4ab269`; Crystal `summer-update`
  `00ce02a57ca5a12e48f32a3476e37471167e4c3f` corroborates the modules inspected.
- `PROVEN` within the committed captures: 180 legal planner allocations and
  1,080 unlock observations match the extracted graph in the tested sequences.
- `CONFLICT`: dedication resistance wording versus observed mitigation; planner
  Lord of Destruction stage-2 25.5% versus the project's corroborated 22.5%; OTS
  fragment yields versus official manual ranges. The candidate records these and
  selects existing official/project evidence as described in its README.
- `UNKNOWN`: live TibiaPal/Fandom parity, visual icon availability and client
  crosswalks. Destination access was denied. No new live-site fetch is claimed.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: reference content tooling only. It performs no production,
Character, persistence, economy, protocol or combat mutation. Runtime admission
is schema-constrained to false. The local publication helper verifies and
preserves the exact commit in a recovery bundle before publication.

## Acceptance criteria

- [x] Closed JSON Schema and reproducible populated candidate: 180 slots, 20
  vocation/domain Revelation records, 46 basic mods, 94 supreme mods.
- [x] Typed stage values, explicit units, corrected source conflicts, gem families,
  item keys, fees, yields, grade costs, compatibility and resonance slot bindings.
- [x] Positive/negative validation and all 180 captured planner allocations.
- [x] Five-profession standalone comparison and search, without JavaScript errors.
- [ ] Exact-head repository CI and independent content review.
- [ ] Protected integration by the active programme control plane.

## Validation

- `python wheel_authoring.py build --check` and `validate`: PASS.
- `python -m unittest discover -s tools/content-schema/wheel-authoring -q`: PASS, 60 cases.
- Standalone HTML: five vocations × 36 rows, four Revelation cards, mitigation
  search and no browser JavaScript errors: PASS.
- Governance validator: PASS; governance tests: PASS, 36 cases.
- `git diff --check`: PASS before publication.

## Review packet and excluded scope

Review the schema's reference fidelity, unit conversions, directed unlock graph,
mod compatibility, grade cap, official-yield precedence and revision restrictions.
The package does not claim live runtime parity, migrate paid gems, introduce
admitted native effect keys, distribute proprietary icon sheets, populate runtime
rulesets or alter accepted architecture. The active control plane retains paid
review dispatch, Merge Queue and merge authority. No worker review trigger is sent.

## Source-verification supplement

Owner continuation on 2026-10-01 returns this branch to AUTHORING and supersedes
candidate `71f772b459e52100aea47dc7cd5e1aff3fd3c195`. The sole writer and owned paths
are retained. The successor is frozen only after the complete supplement is
published and read back; no merge or paid-review authority is acquired.

- Replay against the exact pinned TibiaPal checkout: PASS for every source slot,
  all perk descriptions/values, all mod catalogues and all vocation availability
  lists; five full 4,000-point wheels also match their source captures.
- Nine unique Conviction definitions now have structured numeric parameters,
  conditions and targets. Revelation behavior/range/area and reference spell
  costs/base powers are captured from the existing accepted spell evidence.
- Progression, Atelier operation/refusal policy, extra-point references,
  vendor prices and separately labelled Crystal loot hypotheses are included.
- Renderer crop formulas are confirmed for all five sprite categories. The
  bundled 16-cell Revelation sprite is visually verified and hash-pinned.
- Live-source verification is still blocked by the enforced cloud destination
  policy. The requested allowlist adjustment is pending; the Desktop Commander
  connector reports no online devices, and there is no in-app browser reader.
  No proxy bypass or fabricated verification claim is used.
- The owner-captured official manual notes remain valid evidence. Its private
  original attachment could not be downloaded through the same destination policy;
  no proprietary manual text or sprite bytes are copied into this public repository.

`verification-evidence.json` records what is source-confirmed, OTS-only,
PARITY_PENDING, or blocked. Source completion is not equated with live Global
parity or runtime implementation.

The complete augment comparison also finds official 8944 cooldown corrections
(Mystic Repulse I and Thousand Fist Blows II: -4 s), and source conflicts for Great
Fire Wave I and Mystic Repulse II. The candidate selects existing project target
evidence and retains the differing planner parameters explicitly; live parity
remains unconfirmed. Special Spells' Canary-only secondary cooldown is recorded
as an unselected hypothesis. Flurry of Blows now binds the enlarged-area reference.
The verification record is schema-validated and SHA-256-bound to the candidate.

## Tavily live continuation (2026-10-01)

The owner's direct request to continue PR #1435 with Tavily returns the existing
branch to AUTHORING and supersedes frozen `65433321efa136bef99123996b461cb87842360c`.
Live branch/PR equality and #162 STATE/latest-page reads establish no competing
Wheel writer. The original two owned paths and sole writer remain; programme
close-out mode is not expanded. This section supersedes historical destination
blocks above. Review dispatch and integration stay with the control plane.

- `PROVEN`: Tavily extracted TibiaPal, the official manual and separately pinned
  BR wiki pages. Seven requested Fandom pages failed. No Fandom success is claimed.
- `PROVEN`: live-served module, string catalogue and renderer bytes equal pinned
  TibiaPal inputs; complete replay passes. The provided code renders an invalid
  code message. This does not establish interactive browser behavior.
- `PROVEN` within the fresh manual extract: the audit's listed eligibility,
  Atelier operations/refusals, fragment ranges and effective-grade facts.
- `DERIVED`: BR Lord of Destruction II total 52.5% minus Master of Decay base 30%
  corroborates the selected 22.5 pp. Both -4 s cooldowns have live BR corroboration;
  fresh original official 8944 access remains unsuccessful.
- `CONFLICT`: Mystic Repulse II now also has BR +15% versus planner +60% and
  selected +40%. BR Great Fire Wave I lists both critical effects, in a published
  March 2025 revision. Two target-version conflicts remain unresolved; raw pinned
  observations and selected numbers are preserved.
- `UNKNOWN`: all five original sprite URLs return HTTP 403; the four missing
  sheets remain visually unverified. Numeric fees/cap, exact mitigation, initial
  gems, grade costs and quest/scroll points are not newly qualified.

`samples/live-source-audit.json` retains extraction request IDs, revisions, content
hashes, HTTP observations, bounded source facts and conflict outcomes. Its SHA-256
is bound by the verification record and tested. No original manual, proprietary
sprite bytes or upstream implementation are redistributed. Candidate changes
are evidence/classification and the planner-content verification flag; numerical
parameters are unchanged. Global parity, runtime admission, client asset mapping,
independent review and protected integration remain pending.

Qualification: deterministic rebuild/validation, all 60 authoring tests,
governance validation and all 36 governance tests, pinned planner replay and
whitespace/owned-path checks are required for the successor candidate. Its exact
published SHA and results are reported in the PR after freeze, without a
metadata-only follow-up commit.

## Audit repair (2026-10-01)

The owner's direct instruction to fix all audit findings returns the branch to
AUTHORING and supersedes frozen `9ede8c40f2e734104f9a74a96d6e05a9d6f6cc31`.
The same sole writer retains the original paths; the requested CI repair adds
only `.github/workflows/wheel-authoring-schema.yml` to this task's owned paths.
The live PR head and #162 STATE/latest page were refreshed; no competing Wheel
writer was identified. Review dispatch and protected integration remain with the
programme control plane. No runtime allocation or architecture change is made.

The eight audit findings are repaired in five related areas, within 500 changed
lines of hand-written code: source/effect completeness, reference and policy
bindings (including provenance), revision chaining, strict JSON, and CI.

- Great Fire Wave I preserves both critical effects from the existing project
  spell capture. Flurry I retains range +1 as an unselected hypothesis and selects
  only the enlarged affected area. Corrections cannot replace effect kinds, and
  unresolved area/duration observations are no longer silently cleared.
- Candidate source revisions, topology crosswalks, perk identities/placement,
  complete effect kinds/units, areas, targets and icon indices are checked against
  the input captures. Atelier operation invariants are schema-bound; adjacency
  thresholds/caps and loot chances are checked for feasible bounds.
- A supplied previous candidate forbids `initial`. Value-only restrictions and
  predecessor requirements remain enforced; structural source changes require
  corresponding source-capture updates.
- Readers and validators reject NaN, Infinity and numeric overflow. Writers use
  standard JSON. Default CLI validation qualifies exact-file evidence; custom
  candidates explicitly report semantic-only validation unless evidence is supplied.
- CI checks byte-identical schema/candidate/report rebuilds, evidence, regressions
  and the existing asset input bindings. No upstream network fetch is required.

The owner's asset-directory link was inspected. All 18 Atelier item appearances
resolve to sprite IDs and three existing atlas files; the reference is digest-bound
and reproducible. This supplies item presentation evidence, not the missing Wheel
perk UI crosswalk, client asset admission or proprietary sprite redistribution.

Qualification for the successor: authoring regressions, deterministic checks,
pinned planner replay, governance validator/tests, repository policy validator and
matching repository tests, whitespace and owned-path checks. Exact-head results
are recorded in the PR after publication/freeze. Global target-version conflicts,
runtime behavior and independent review remain outside this reference repair.

Local repair evidence: 78 authoring regressions, all 19 mutation probes rejected
(including the 17 formerly accepted malformed candidates), deterministic rebuilds,
complete pinned planner replay and the five-vocation browser check pass. Governance
validation/36 tests, repository policy validation and content/post-merge/PR routing
regressions pass. The broader merge-group and canonical PR PG/SIM scripts reach
native PowerShell canaries and stop because `pwsh` is absent in this workspace;
those unchanged native checks remain for repository CI. No check is weakened.


## Completed authoring handoff (2026-10-01)

The owner's instruction to complete all owned work returns the task to AUTHORING,
superseding frozen `126fa467b3b82a9e6e2e8fcced9ecdf0d64ca432`. The same branch,
sole writer and three owned paths are retained. This section supersedes historical
statements that the four reference sheets or reference client crosswalk are missing.

- [x] Five hash-pinned reference sheets inspected; immutable URLs, dimensions,
  205 explicit crops and all 520 candidate bindings supplied. Older client layout
  differences are recorded and cannot silently overwrite current planner IDs.
- [x] Reference manifest and source selection are bound into default exact-file
  validation, deterministic regeneration and CI. Reference source bytes can be
  independently hash/dimension-checked without committing proprietary files.
- [x] All 205 icons have a browser catalogue; five-vocation comparison uses the
  crop manifest and preserves a readable ID fallback when reference images fail.
- [x] Existing spell snapshot date/source precedence fixed explicitly. Mystic
  Repulse +40% is selected for Sept 27; Great Fire Wave includes both critical
  effects; Flurry selects enlarged area with cast-range hypothesis unselected.
- [x] Existing WHEEL-GEM-0 fee/grade/cap decisions represented completely. Global
  parity uncertainty is preserved and is not an incomplete authoring field.

The complete review input is PR #1435 at the final frozen SHA in its description:
closed schemas, populated candidate, deterministic source/evidence bindings,
planner replay, asset references, browser comparison and regression results.
Review attention: source precedence; all icon crops/IDs (including the documented
older Focus Mastery difference); full augment effect sets; finite JSON; revision
chaining; Atelier invariants; no runtime admission or proprietary redistribution.

The authoring delivery is complete. Independent review dispatch and protected
integration remain with the active programme control plane. WHEEL-0 dedication
wording reconciliation and native runtime effect/asset admission belong to their
accepted owners; the evidence and complete reference bindings are provided here.
No worker paid-review trigger, merge, protocol or runtime mutation is performed.


Completion qualification: deterministic schema/candidate/report/manifest checks;
85 authoring cases; source bytes of all five PNG references hash/dimension-checked;
205 browser crops loaded and bounds/offsets checked across five vocations; offline
ID fallback checked; complete pinned planner replay; 18 item appearances; governance
validator/36 cases; repository policy validator, three routing suites/6 unittest
cases; whitespace and owned-path checks. No local PowerShell result is newly claimed.
The final frozen SHA, exact-head check results and review packet are recorded in
PR #1435 after publication, with no tracked writes after freeze.
