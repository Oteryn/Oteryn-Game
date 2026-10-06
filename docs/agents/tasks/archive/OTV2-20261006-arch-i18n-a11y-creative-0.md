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
blocks: [I18N-CATALOG-0, A11Y-BASELINE-1, ASSET-PROVENANCE-2, CREATIVE-DOC-3, I18N-TEXT-RENDER-4, I18N-CONTENT-KEYS-5]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome
- Owner ruling 5b of 2026-10-06: i18n, accessibility and creative direction are decided now.
- Fluent message catalogues in the client with per-message fallback; the server sends codes and typed references, never display text; content keys deferred until a content locale is chosen.
- An accessibility baseline: remapping, UI scaling per the UI baseline §7, colour-only audit, flash rate limit and a legibility check.
- Asset provenance records with CI checks; a creative direction document the owner fills.
- Amendment 3 to ERR-CODES §1.9 replaces a different sentence than ARCH-LIVE-READINESS-0 §2 A3.
- Three owner items are listed with recommendations.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
