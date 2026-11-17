Continue lane L3 in this worktree (single agent, reasoning medium, no subagents).
Read .questpc2/RESUME.md, .questpc2/L3-BLOCKERS.md, .questpc2/L3-REPORT.md and .questpc2/QUEST-STATUS.json first.
Owner rule: NO DEFERRING. The 35 UNFINISHED and 25 IN_PROGRESS entries in LANE-L3.json are NOT to be reported as blockers again.
Each blocker in L3-BLOCKERS.md names a missing composite mechanism (encounter windows, multi-player admission, wave rewards,
atomic offerings, item custody, seasonal stages, etc.). Implement the missing mechanism itself in the server code, generically and
reusable, then qualify the quests that need it. Missing NPCs/creatures/items are authored. Work blocker by blocker, smallest shared
mechanism first, batches of 3-6 entries; each batch: red-then-green focused tests incl. its own PG cases with proof they executed
(duration/marker), cargo check, clippy -D warnings of changed crates, fmt. No full nextest, no full PG suite.
Exclusions stay: Dawnport, OT-only W14, P23, X01-X06. Father's Burden: never split the 11-track write. Grants charge only on exact match.
Skip entries QUALIFIED in another still-running lane's QUEST-STATUS. Update QUEST-STATUS.json, L3-BLOCKERS.md (shrink it) and RESUME.md after every batch.
Do not commit or push. Never read credentials.
