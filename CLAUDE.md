# Claude Code instructions

@AGENTS.md

Claude Code reads this file at the start of every session; the repository rules live in
`AGENTS.md` (imported above) and the skills in `.claude/skills/`.

Codex review (owner rule, 2026-11-09; full text in `docs/agents/OWNER_FUNDED_AI_POLICY.md`,
*Owner review rule for every PR*, and `.claude/skills/oteryn-review` §5): every PR gets a
Codex review once after its first push, over the whole diff, at effort high. All findings of
the round are fixed in one push (fix work at effort high). A second, delta-only review runs
only when that repair is a substantial logic change; cosmetic or metadata-only repairs get none.
P0/P1 are fixed at once; P2/P3 go to the backlog. At most two reviews per PR; a P0/P1 after
the second means the design is wrong: stop and report to the owner.
