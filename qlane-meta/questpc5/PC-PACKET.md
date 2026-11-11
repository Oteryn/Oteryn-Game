# questpc5 round L5 (quest rebalance)

Single agent. Do not spawn subagents. Reasoning medium.

Your quest set for this round is ONLY the entries in .questpc5/LANE-L5.json (lane L5 of the owner-approved rebalance, file /home/mole/oteryn-cache/qsplit/PLAN.md). Each entry has canonical, titles, from_lanes, status, blocker and mechanism. Your lane owns these mechanisms whole; no other lane works on them.

Rules:
- NO DEFERRING. Fix every gap inside this round: missing NPCs are added, missing items/rewards/callbacks are implemented. Never write "follow-up".
- Excluded (do not touch): Dawnport, OT-only W14, P23, X01-X06.
- Father's Burden: never split the 11-track write. Grant charges only on exact match. Starting equipment comes from Newhaven.
- Some entries come from lanes still running (from_lanes). Before working such an entry, read that lane's status file under /home/mole/oteryn-cache/rs-wt/<lane>/.<lane>/QUEST-STATUS.json; if it is QUALIFIED there, mark it qualified-elsewhere and skip. Work those entries last.
- If an entry needs qengine2's engine (atomic NPC exchange, outfit grants) and it is not in this worktree, implement the minimal piece needed here in the same shape as the qengine2 packet (/home/mole/oteryn-cache/qengine2*.md) and note it in RESUME.
- Work in batches of 3-6 entries per mechanism: write red PG test first, then code, then green. Prove the test executed (duration, marker); a 0.00s pass is a skip.
- Focused checks only: cargo fmt, clippy of changed crates, the new tests and their PG cases. No full nextest, no full PG suite.
- After each batch update .questpc5/QUEST-STATUS.json (canonical, status QUALIFIED/UNFINISHED/BLOCKED with concrete blocker) and .questpc5/RESUME.md.
- Do not commit, push or open PRs (the operator commits; autopush snapshots every 30 min).
- Never read or print credentials, .env files, ~/.codex auth, or PG password files.

Finish with: count qualified / unfinished / blocked and the exact blocker for each non-qualified entry.
