Continue this quest lane in this worktree (single agent, reasoning medium, no subagents).
Read .questpc6/RESUME.md, the lane JSON (.questpc6/LANE-*.json if present), .questpc6/QUEST-STATUS.json and the last report/blocker files first.
Owner rule: NO DEFERRING. UNFINISHED, IN_PROGRESS, ACTIVE, ACTIONABLE and BLOCKED entries are NOT to be reported as blockers again,
except owner exclusions (Dawnport, OT-only W14, P23, X01-X06). Each blocker names a missing mechanism (encounter state, kill counters,
atomic hand-ins, world-state resets, transformations, timed harvest, shared stages, access gates, item custody, etc.). Implement the
missing mechanism itself in the server code, generically and reusable, then qualify the quests that need it. Missing NPCs, creatures
and items are authored. Smallest shared mechanism first, batches of 3-6 entries; each batch: red-then-green focused tests incl. its own
PG cases with proof they executed (duration/marker), cargo check, clippy -D warnings of changed crates, fmt. No full nextest, no full PG suite.
Father's Burden: never split the 11-track write. Grants charge only on exact match.
Skip entries QUALIFIED in another lane's QUEST-STATUS (see /home/mole/oteryn-cache/qsplit/PLAN.md for lane ownership).
Update QUEST-STATUS.json and RESUME.md after every batch. Do not commit or push. Never read credentials.
