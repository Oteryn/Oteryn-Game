# NPCFIX continuation

AUTHORING on claude/happy-davinci-ijz3wy-npcfix. Recovered the 320-row audit report from 95accb923; head be6bca215 deleted the lane records. Status distinguishes prior authored work from qualified work. No completed implementation will be redone.

Owner now authorizes generic mechanisms, quest/NPC fixes, source research, commit and push after each green batch. Exclude Dawnport, W14, P23, X01-X06 and other lanes’ world changes. Run real PG RED/GREEN, fmt/check/clippy per batch. Single writer; no PR.

Next: shared atomic quest/NPC writer and first source-backed transaction batch. PostgreSQL 18.6 startup and connection succeeded via /workspace/oteryn-env/start.sh. Source env.sh for each build shell; never print its URL. Logs in /tmp/npcfix-evidence.

## Active prerequisite batch

Migration 1096 widens only the quest receipt effect bound from 8 to 16, aligned with the catalogue and NPC source planner. All other shape/deferred/fencing guards stay intact. Owning decision records the owner’s atomic eleven-write amendment.

Focused PostgreSQL RED: TooManyEffects, POSTGRES_STARTED marker, 4.39s. GREEN: 1 passed, all three DB cases executed, 15.01s. Verifies eleven writes/one receipt/one revision, exact receipt replay, and no writes or revision change for late stage/arithmetic refusal. Planner 22 passed; affected quest rules 248 passed. Full output in /tmp/npcfix-evidence/{atomic-red,atomic-green,planner-green,quests-green}.log. Initial out-of-range fixture used a statically invalid SET; corrected to runtime ADD overflow so it reaches the durable evaluator. No product assertion weakened.

Session 28398 runs sequential fmt, cargo check and clippy -D warnings for quests/dispatch/game-server, all targets/features. Inspect /tmp/npcfix-evidence/batch1-{check,clippy}.log, then commit/push on success. Recheck migration number with repository script immediately before push. No commit or push yet.

Seven remaining world/event/map entries are SKIPPED_OTHER_LANE under the owner’s world exclusions. No NPC has been newly marked QUALIFIED. Next NPC batch: generic guarded state dialogue on the existing quest_dialogue caller (Father’s Burden acceptance, Restless Soul atomic story, Strange Chalice topic chain). Donor snippets/URL+sha are in /tmp/npcfix-evidence; do not commit source copies.

Prerequisite batch QUALIFIED: all-target/all-feature cargo check PASS 5m22s; clippy -D warnings PASS 4m36s; fmt/diff checks PASS. Self-review found no issue. Migration 1096 rechecked free on origin/main and open PRs. Commit and push this batch now; then copy /tmp/npcfix-evidence/npcfix_state_dialogue_tests.rs into the mounted PG directory and run npcfix_state_postgres_ RED before adding the caller. The test draft uses existing fixtures and real content; three cases are isolated.
