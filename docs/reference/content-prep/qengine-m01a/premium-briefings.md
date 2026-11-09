# Selected Premium TALK briefings

The `m01a_talk_routes.py` generator binds the selected recipes' first TALK stages
for Forgotten Knowledge (Ivalisse) and Kilmaresh (Eshaya). The production consumer
is the existing authored `NpcDialogue` dispatcher; the preflight tool verifies
its registry entry before enabling each stage. These are opening briefings,
not whole-quest completion or original Lua dialogue parity.

Evidence: `content/quests/definitions/quests-00100-00199.json:1`, Forgotten
Knowledge record 45 and Kilmaresh record 65, selected recipe `stages[0]` and
`requirements.premium`; inspected at fb10e73fa42b346d3f64552fa7e4e56e6ef461fb.
Both stages require one TALK, no item transfer, no prerequisite quest, and current
Premium. The authored phrases are `lost paths briefing` and
`ambassador investigation`; replies name the selected recipe's next objective.
The generated callbacks preserve the exact 0-to-1 stage guard and are
nonterminal. Later stages retain their own unresolved bindings.

`qengine_m01a_premium_starters.rs` exercises each NPC through the real transport
and production catalogue, sharing the established Sven Premium story. It checks
wrong NPC, unavailable Premium authority, accepted current Premium, literal
request replay, refusal of a fresh repeated request, one durable stage write and
receipt, and absence of a completion receipt. No stage progress is seeded.

Publication checkpoint: authoring checks and all 117 authoring test modules,
content-tree validation, workspace/all-targets check, dispatch library Clippy
with warnings denied, formatting, architecture, duplicate migrations, and World
pin verification passed. The final four PostgreSQL cases are still rebuilding
at publication under PREPUSH-2's cadence exception. In the preceding run,
Ivalisse passed in 237.026648549s and Sven in 164.666181396s; imported TALK
failed with `fence` and Eshaya with `earned appearance catalogue did not arrive`.
The final rerun uses no competing validation jobs and adds an explicit
same-command successful replay assertion. This checkpoint is PARTIAL.
