# Evidence

## PROVEN

- Protected allocation baseline was read as `main@253b5c0c464e9b73c8398bcf479bdcca9a1f0932` before the pilot branch was created.
- Root `AGENTS.md` says live GitHub Issue/PR/check state governs lifecycle and repository-native GitHub/CI is preferred.
- Existing `agent-governance.yml` verifies the live PR head SHA, checks out that exact target and runs governance validators.
- Bound organization policy is `OTERYN_ORGANIZATION_AGENT_POLICY@3.1.0` at META commit `3b39e0be05aef008f1bd442821daefa898a201dd`.
- GitHub release API reported gh-aw `v0.88.7`, whose tag resolves to commit `bde367913adeb3132f0a171594c88a17f4b7d08c`; the pilot pins that version/commit.
- OpenSpec release API reported `v1.13.0`; the qualification job pins npm package `@fission-ai/openspec@1.13.0`.

## DERIVED

- The safest initial behavior canary is an actual router-to-worker dispatch with the worker restricted to read-only GitHub access and staged human-facing output.
- OpenSpec should model the handoff contract but must not duplicate live GitHub lifecycle state.

## UNKNOWN

- Whether the organization has the `copilot-requests: write` entitlement required to execute the agentic jobs. Compilation can qualify without proving runtime inference entitlement.
- Actual token/cost and owner-intervention improvement versus the current manual flow until behavior trials run.

## CONFLICT

`NONE` at allocation.
