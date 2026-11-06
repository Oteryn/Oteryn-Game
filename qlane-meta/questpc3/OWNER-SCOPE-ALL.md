# OWNER DIRECTIVE 2026-11-06 (supersedes narrower scope)
The owner assigns THIS lane to finish ALL quests in the catalogue (the 373 pinned
titles and every reward chest/claim), not only the eight groups. Excluded only:
Dawnport, OT-only, Prey/Hunting Tasks (P23).
For every quest: full executable path on the server (start, quest log/missions,
NPC steps, doors/levers/step-in/multi-item callbacks, kills/boss steps, reward
chests with exact source rewards, completion) plus a PostgreSQL test with
POSTGRES_CASE_COMPLETED markers. Never stop on missing evidence: TibiaWiki
(MediaWiki API), donors in the repo and YouTube have the data; pick the
conservative reading and record the source. STOPPED is not an allowed final
status: choose a safe default and implement it.
Work order: finish the current run-7 items, then sweep the catalogue A..Z in
batches of 10-25 quests; after each batch write a checkpoint (report rows +
focused tests) so it can be committed and pushed. Migrations 0969 then 1000-1039.
A Codex Cloud audit (QALL-AUDIT) will drop a per-quest table into
.questpc/QALL/ when ready; use it if present.
