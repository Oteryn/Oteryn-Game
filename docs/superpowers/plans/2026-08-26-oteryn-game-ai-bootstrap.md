# Oteryn Game AI Bootstrap — Implementation Plan

**Goal:** Deliver the owner-accepted pure-local GAME-AI bootstrap v1 structural slice; no gameplay-authoritative integration.

**Worker paths:** `apps/game-server/src/ai/{mod,snapshot,perception,resolution,path_proposal,tests}.rs`, `apps/game-server/tests/ai_bootstrap.rs`, and `docs/agents/tasks/active/OTV2-20260826-impl-game-ai-bootstrap.md` only. `apps/game-server/src/lib.rs` is coordinator-only.

## Constraints

- Start only from the allocation-recorded protected-main SHA, one Issue #174 branch/PR.
- Use only limits: active actors 256; authored units 4; evaluation work 8; perception candidates 64; path requests/actor 2 (slice config at most one); path-search work 1024; route steps 128; route bytes 4096.
- Every max+1/checked-overflow rejects before allocation/publication/partial mutation; shuffled input/ties are deterministic.
- Stale owner/actor/revision results reject. Path proposals are data only and cannot adopt a route.
- Exclude Ability/Interaction/Movement integration, persistence/value/reward, spawn/memory/timer/retry/controllers/scripts, protocol/content/schema, production wiring and Reference parity.

## Steps

1. Write failing focused tests for each limit/max+1, overflow, shuffled tie breaks, stale provenance, direct foreign-mutation absence and zero mutation on exhausted budget. Build an allocated standalone harness while `lib.rs` remains shared.
2. Implement bounded snapshot, perception, resolution and route proposal with checked accounting and canonical ordering. Return only typed proposal/rejection outcomes.
3. Run focused AI and required repository checks; if local Rust is unavailable record it and require exact-head CI. Self-review and task-review all material findings before merge. Coordinator alone may later compose `lib.rs`.
