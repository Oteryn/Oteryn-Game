# OTV2-20261031-events-items

## Task and authority

- Status: AUTHORING; implementation and validation remain in progress.
- Branch: `chatgpt/events-items`; one task lead owns commits and publication.
- Base: `ae4188a9c8d461acb152044814ea23075de02ed4` (verified protected main at task start).
- Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- Accepted request: W19a-e item actions, W05b/d world and seasonal events, W05e Autumn Vintage, W16 Summer 2026 encounters.
- Bound organization policy: version 3.1.0, `Oteryn/Oteryn@1bfb5ff98c8aa156e73669a14e083a1d464c29fb`.
- Deliver one draft PR against main. No merge, rebase, force-push, review request, production activation or heavy-tests label.

## Owned implementation slices

| Slice | Existing owner to extend | Allocated migration, only if needed |
|---|---|---|
| W19a-e | item actions, reward bags, consume/transform, appearance ownership | 0712 |
| W05b/d | world-change draws, map/spawn projections, scheduled events | 0713 |
| W05e | scheduled events, loot, NPC services, potion restoration | 0714 |
| W16 | shared boss encounter and creature condition owners | 0715 |

Capability IDs 256-257, command IDs 482-485 and mechanic ordinals 622-625 are reserved by the task. Reuse existing client commands whenever possible. Check current main and open PRs before using or publishing any reserved number.

## Acceptance

1. Every sourced action reaches an existing live command or server consumer; no test-only implementation or duplicate owner.
2. Every value has donor file:line or accepted content evidence. Unsourced choices are raised on this PR as individual `QUESTION:` comments with alternatives and a recommendation.
3. Durable item, unlock, event and encounter transitions use the existing session-generation fence and atomic receipt/transaction patterns. Replays return recorded results.
4. World changes project their recorded state into the map or spawn owner. Seasonal effects are bounded by their sourced windows and success conditions.
5. Autumn Vintage has collection, NPC delivery, treading and world-success consumers only after the required rules are sourced or answered by the owner.
6. Summer encounters use the shared encounter owner; absent donor admissions or transitions are explicit questions, never guessed encounters.

## Verification plan

- Read the required repository skills and nearest applicable instructions before edits.
- Unit coverage for rules, boundaries and refusals; route/dispatch PostgreSQL coverage for each durable transition in the existing mounted directory, with completion markers and route-less handling.
- Format, clippy and changed-package tests; focused PostgreSQL cases on the local test database where available.
- Regenerate content and world pins through repository tooling; run content-loader, pin and repository integration checks for affected inputs.
- Review full diff and exact changed-file list, run `git diff --check`, recheck number allocation and read PR answers before publication and closeout.

## Initial evidence

- Main includes rust remover and lottery admission through PR #2761, and the world-change/schedule owners through PR #2757.
- Crystal provides source-backed item tables and world-change scripts; Summer monster statistics alone are not evidence for encounter admission or phase rules.
- Official Autumn Vintage news confirms October 1-8 and October 17-24, with +33% health/mana potion effect for seven days after world success; thresholds, exact monster pool and treading rules require further source evidence or owner answers.

## Outstanding work

Implementation, source parity tables and test results are pending. This initial record establishes scope and the early draft question channel; it makes no completion claim.
