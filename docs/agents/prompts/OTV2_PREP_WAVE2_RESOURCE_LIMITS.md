# OTV2-WAVE2-RESOURCE-LIMITS — Next-Wave Resource Limits Preparation

Short alias:

```text
Oteryn: prep resource limits
```

Mode: `PREPARE / DECIDE`, not runtime implementation.

Work only under an exact coordinator allocation for Issue #93. Read live `main`, root/nearest governance, `RESOURCE_LIMITS_REGISTRY.json`, accepted GAME-ABILITY-01, GAME-INTERACTION-01, GAME-AI-01, VSL-MOVE-01 and the next-wave master plan plus hardening amendment.

Produce `docs/architecture/reviews/OTERYN_GAME_WAVE2_RESOURCE_LIMITS_DECISION_PACKET_2026-08-24.md` with the complete classified inventory required by #93. For every dimension use exactly one of `REGISTERED_EXACT`, `CONTRACT_EXACT_UNREGISTERED`, `EVIDENCE_CANDIDATE`, `OWNER_DECISION_REQUIRED`, `NOT_APPLICABLE_TO_FIRST_SLICE` and record unit, owner contract, amplification source, failure behavior, allocation impact, client visibility and boundary tests.

Do not invent numeric policy. Do not reuse generic FND ceilings unless the owning contract explicitly equates the resources. Ability/Interaction/AI may be released only when every exercised dimension is terminally registered or excluded fail-closed.

Movement inventory must be prepared now, but before Movement allocation every exercised Movement dimension must later close to `REGISTERED_EXACT` or explicit `NOT_APPLICABLE_TO_FIRST_SLICE`; `EVIDENCE_CANDIDATE` and `OWNER_DECISION_REQUIRED` remain blockers.

Any registry write is a separate serialized coordinator mutation after accepted evidence/owner decision. No runtime, protocol ID, production or external-repository mutation.

Validate documentation/governance, run whole-diff self-review and persist exact evidence before merge/closeout.
