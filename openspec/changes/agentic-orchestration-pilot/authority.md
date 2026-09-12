# Authority

## Live locators

- Repository: `Oteryn/Oteryn-Game`
- Issue: `#591`
- Base branch: `main`
- Allocation baseline: `253b5c0c464e9b73c8398bcf479bdcca9a1f0932`
- Candidate branch: `agent/otv2-agentic-openspec-pilot-01`
- PR: resolved from live GitHub after creation

## Writable scope

- Pilot-owned paths are enumerated by `docs/agents/tasks/active/OTV2-20260913-agentic-openspec-pilot.md`.
- The gh-aw agent jobs receive repository read permissions only, except `copilot-requests: write` for inference.
- The router may request one same-repository dispatch to the allowlisted pilot worker.
- Worker human-facing output is `staged: true` and therefore preview-only.

## Non-authority

OpenSpec artifacts, gh-aw prompts, workflow output, task packets and agent narratives do not grant merge, production, credential, external-repository, protection or Merge Queue authority. Existing GitHub checks and bound Oteryn policy remain authoritative.
