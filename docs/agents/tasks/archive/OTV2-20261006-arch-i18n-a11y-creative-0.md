# OTV2-20261006-arch-i18n-a11y-creative-0

```yaml
task_id: OTV2-20261006-arch-i18n-a11y-creative-0
title: "ARCH-I18N-A11Y-CREATIVE-0: localisation, accessibility and creative direction"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: cand/arch-i18n-creative
issue: 162
pr: 1880
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
owner: claude-code-session_01WQyZ8BUWVpmDLpSTpXHvn1 (Sol Supervising Architect)
created_at: 2026-10-06
updated_at: 2026-10-06
execution_policy: continuous_progress
owned_paths: [docs/architecture/reviews/OTERYN_GAME_ARCH_I18N_A11Y_CREATIVE_DIRECTION_2026-10-06.md, docs/agents/tasks/archive/OTV2-20261006-arch-i18n-a11y-creative-0.md]
public_contracts: []
depends_on: []
blocks: [I18N-CATALOG-0, A11Y-BASELINE-1, ASSET-PROVENANCE-2, CREATIVE-DOC-3, I18N-TEXT-RENDER-4, I18N-CONTENT-KEYS-5, UI-LAYOUT-6, MAP-VIEW-EXTENT-7]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome
- Owner ruling 5b of 2026-10-06: i18n, accessibility and creative direction are decided now.
- Fluent message catalogues in the client with per-message fallback; the server sends codes and typed references, never display text; content keys deferred until a content locale is chosen.
- An accessibility baseline: remapping, UI scaling per the UI baseline §7, colour-only audit, flash rate limit and a legibility check.
- Asset provenance records with CI checks; a creative direction document the owner fills.
- Amendment 3 to ERR-CODES §1.9 replaces a different sentence than ARCH-LIVE-READINESS-0 §2 A3.
- Owner rulings of 2026-10-06 are recorded in §4: Q1 b; Q2 a; Q3 both layouts (A first); both
  field-of-view arms built and switchable in the client, independent of the layout, final policy
  open. Q5 (the larger fixed size) is open with PROPOSED default c (the largest candidate that measures within budget).

## Owner rulings

- 2026-10-06, given directly to the architect: Q1 b (`en` + `pl` UI and outcome text, content
  English); Q2 a (Tibia-sourced NPC, quest, item and achievement text extended as
  OWNER_CLEARED_THIRD_PARTY; not the architect's recommendation, the owner's ruling and risk;
  distribution still gated by ASSET-PROVENANCE-2 records and the §1.17 validator); Q3 both
  layouts switchable in client settings, A classic-faithful first, then B modern, AI-produced
  art with provenance records, theme tokens and layouts as data; field of view: both arms built
  and switchable from the client for the A/B test, FOV-fixed (plan Variant B, a standard and a
  larger fixed size) and FOV-responsive (plan Variant A), separate from `ui_layout`, server-set
  cap, measured against ARCH-MAP-VIEWPORT-BUDGET-V1, final policy undecided.

## Acceptance criteria

- [x] Decision text with facts, rulings, amendments, packets, owner questions and unknowns.
- [x] Owner rulings recorded and the body made consistent with them.
- [x] Review round 1 findings answered (below).
- [x] Review round 2 findings answered (below).
- [ ] Independent exact-head review on the frozen head.
- [ ] Protected Merge Queue integration.

## Review findings

- Round 1 (external review, three P1):
  - 4195134861: `TextRef` is capability-gated (§1.7, F20): strict decoders
    `account_achievements.rs:316` and `quest_log.rs:276` reject unknown fields; the client
    advertises a new capability, the server sends `TextRef` only to selected sessions and the
    legacy encoding otherwise; packet 5 tests the byte-equal legacy path, selection and resume.
  - 4195134871: Q2 ruled a by the owner; §4 records what it permits and that distribution stays
    gated by the corpus records and the §1.17 validator.
  - 4195134876: §1.17 interim manifest enumerates server inputs (achievement `include_str!`
    shards, NPC, dialogue, quest and item families; F21) so an unrecorded input fails.

- Round 2 (external review, three P1):
  - 4195950440: packet MAP-VIEW-EXTENT-7 adds the server side of the field of view (F23 server
    facts; §1.21 b, c, e): an extent capability over capability 18, a request command, domain 17
    type 2 with the granted extent, a per-channel cap, legality on `REFERENCE`, limit rows and a
    snapshot measurement against `MAP01-VIEWPORT-SNAPSHOT-US`.
  - 4195950449: `fov_arm` is a separate setting from `ui_layout` (§1.20, §1.21 a); no packet or
    layout file carries the other; tests cover every pair and the unchanged grant on a layout
    switch.
  - 4195950459: §1.17 scans only shipped inputs: release dep-info of the production roots' bin
    targets, production loader trees and packaged client assets; `NON_ASSET_INPUT` for embeds
    with no media or player text; tests for test-only embeds (F24).

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass at round 0; at round 1 its only
  failures are five E8002 error-registry entries (2014, 3010, 3011, 4001, 6007) that `origin/main`
  added after this branch's base; no owned path touches the registry, and the merge ref has them.
  At round 2, after merging `origin/main`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass

## Context checkpoint

```yaml
last_progress: review round 2 answered (FOV server packet, FOV arm apart from layout, scan scope)
status: completed
branch: cand/arch-i18n-creative
pr: 1880
next_action: lead freezes the head and requests re-review
```
