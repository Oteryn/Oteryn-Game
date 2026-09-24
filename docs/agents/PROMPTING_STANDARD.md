# Game prompting extension

Game adopts the prompting standard at the immutable META revision in `META_AGENT_POLICY_BINDING.json`, path `docs/agents/policy/PROMPTING_STANDARD.md`.

Reusable Game prompts are task deltas. Retain aliases, outcome, writable scope, Game-specific invariants and dependencies, and acceptance evidence. Resolve changing Issue, PR, branch and commit facts at use time. Put one-off instructions in the live task rather than a reusable prompt.

Register every reusable or retired prompt in `PROMPT_LIFECYCLE.json`. A short alias selects a registered prompt; it grants no write, merge, production or cross-repository authority.
