# OTV2-GAMEPLAY-SERVER-SEAM-PREP — Production Gameplay Server Seam Preparation

Short alias:

```text
Oteryn: prep server seam
```

Mode: `PREPARE / DECIDE`, not runtime implementation.

Work only under an exact coordinator allocation for Issue #96. Read live `main`, root/nearest governance, merged Foundation framing/codec/runtime/admission/reconnect implementation, FND-02/FND-03/FND-04, NET-TRANSPORT-01, QA-E2E, current `apps/game-server`, current registries and the next-wave master plan plus hardening amendment.

Produce `docs/architecture/reviews/OTERYN_GAME_PRODUCTION_GAMEPLAY_SERVER_SEAM_PLAN_2026-08-24.md`. Freeze exact listener/transport/composition paths, Cargo/shared-path needs, resource-limit ownership, admission/GameSession/reconnect fencing and the real Tier-1 boundary journey:

`connect -> frame/decode -> admission -> GameSession -> reconnect/resume -> resync/fail-closed gameplay entry`.

Do not invent gameplay command/state/event IDs. Unsupported gameplay remains unavailable until owning registrations exist. Define TDD negatives for malformed/oversized/unknown messages, stale generation/session evidence, reconnect/fencing races and authority-before-mutation.

The output must include the exact later implementation allocation proposal for lane `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM`, worker alias `Oteryn: impl server seam`, prompt `docs/agents/prompts/OTV2_IMPL_GAMEPLAY_SERVER_SEAM.md`, and required child plan `docs/superpowers/plans/2026-08-24-oteryn-production-gameplay-server-seam.md`.

This preparation task grants no listener code, dependency, stable ID, production port/deployment/secrets or external-repository authority. Validate governance/diff quality and whole-diff self-review before merge/closeout.
