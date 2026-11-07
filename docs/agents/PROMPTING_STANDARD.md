# Game prompting extension

Game adopts the prompting standard at the immutable META revision in `META_AGENT_POLICY_BINDING.json`, path `docs/agents/policy/PROMPTING_STANDARD.md`.

Reusable Game prompts are task deltas. Retain aliases, outcome, writable scope, Game-specific invariants and dependencies, and acceptance evidence. Resolve changing Issue, PR, branch and commit facts at use time. Put one-off instructions in the live task rather than a reusable prompt.

Protected-integration wording in reusable Game prompts is lifecycle intent only. Defer capability and primitive selection to the immutable bound META integration router. Do not pin a META policy version or REST endpoint in a reusable prompt, do not treat absence of a direct native primitive as a terminal blocker, and do not introduce generic auto-merge as a fallback. `BLOCKED_CAPABILITY_UNAVAILABLE` is valid only after the current bound router cannot freshly prove either its direct route or its delegated executor route.

Register every reusable or retired prompt in `PROMPT_LIFECYCLE.json`. A short alias selects a registered prompt; it grants no write, merge, production or cross-repository authority.
