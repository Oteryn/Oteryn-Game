# Acceptance

## Proven gates

- Dedicated Issue and branch allocation from an exact protected-main baseline.
- Version pins selected for gh-aw and OpenSpec.
- Existing Oteryn governance remains the intended integration authority.

## Pending gates

- Compiler-generated lock files committed from the pinned gh-aw compiler.
- Exact-head deterministic repository checks.
- Representative runtime behavior canary or exact capability blocker.
- Independent deep review on the stable material candidate.
- Explicit owner authorization before any control-plane integration decision.
- Merge Queue qualification and protected-main readback if integration is later authorized.

## Terminal state

`OTERYN_AGENTIC_PILOT_IMPLEMENTING`

## Integration boundary

OpenSpec completion and gh-aw output are not merge authority. This pilot must stop before integration unless the existing Oteryn control plane and owner separately authorize the exact candidate.
