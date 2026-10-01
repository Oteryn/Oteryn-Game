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
amendment of the programme's close-out mode. The branch and the two owned paths
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
