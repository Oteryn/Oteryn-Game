RESTART NOTE (owner 2026-11-06): your previous run on this worktree was stopped only to switch the model setting to gpt-6.1-sol, reasoning effort medium. Continue from the CURRENT worktree state and your lane records (RESUME.md, QUEST-STATUS.json, FINAL.md under .npcfix/ if present). Do not redo finished work. Work as a SINGLE agent: never spawn subagents or parallel agent threads.

Reasoning effort: high.

NPCFIX: fix every current-playable NPC difference and missing NPC found by NPC-AUDIT-1, as one package (owner: finish all missing mechanics tonight; nothing deferred).

Worktree: /home/mole/oteryn-cache/rs-wt/npcfix, branch claude/happy-davinci-ijz3wy-npcfix from main ed788c4b. Read the repo skills oteryn-dev, oteryn-coding, oteryn-verify, then the audit (read-only, do not edit it):
/home/mole/oteryn-cache/rs-wt/npcaudit/.npcaudit/NPC-AUDIT.md, npc-audit.jsonl, FIX-GROUPS.md, research/*.md, results/*.json, RUNTIME-HANDLERS.md.

Work list: every row with status DIFF or MISSING. Skip rows whose only evidence is historical/test-server, and owner exclusions (Dawnport, OT-only W14, P23, X01-X06). For each remaining row fix the exact difference recorded in its research file (identity, position/placement, dialogue keywords and replies, shop offers and prices, services, travel, postage), with Wiki/transcript as the retail source and Canary/Crystal only as hints.

The three fix groups of FIX-GROUPS.md all go in this one package (their files are disjoint from each other, so one writer is fine):
1. WORLD-NPC-POPULATION: content/world/npc-placements/**, tools/content-schema/npc-authoring/npc_placement_scope.json; regenerate content/world/pins/* only with the repo tooling (regenerate_content.py --resolve --no-stage), never by hand.
2. NPC-NARRATIVE-SERVICES: content/npcs/definitions/**, content/dialogues/**, content/services/**.
3. NPC-CONDITIONAL-POSTAGE: character-conditioned postal pricing (completed Postman: 5/10 instead of 8/15; Robson explicit 15) through the existing npc trade path in crates/gameplay/content (project/v2.rs, npc_catalogue/service.rs), crates/gameplay/npc/src/npc_talk.rs, gameplay_transport/npc_trade_command.rs, durability/npc_trade.rs. Reuse existing quest-state reads; no new protocol code.
Do not edit quest lanes' modules (gameplay_transport/npc_command/questpc*.rs and quest content under content/quests/**). If a fix needs one, record it in the report for the coordinator instead.

For each fixed NPC add or extend a test (content validation test or unit/PG test of the trade/talk path) that fails before and passes after. Checks: cargo fmt, cargo check and clippy -D warnings of changed crates, the content/world validation tests (content_world_project_repository, npc tests of gameplay-content and gameplay-npc), pin check, architecture check. No full nextest, no full PG suite. Use only /home/mole/oteryn-cache/codex-bin/cargo, offline.

Do not commit, stage, push or stash. Work in slices and keep .npcfix/RESUME.md current so a restart continues. Write .npcfix/REPORT.md: per-NPC table (npc | town | audit status | fix | file:line | test | result), counts fixed/remaining with reason per remaining row, checks table PASS/FAIL with logs, and the line "ALL PASS" only when every check passed.
