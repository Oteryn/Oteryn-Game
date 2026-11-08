Reasoning effort: medium.

Task NPCFIX continuation (fresh environment). Base: branch claude/happy-davinci-ijz3wy-npcfix (144 commits ahead of main). First `git fetch origin main && git merge origin/main` (main has #3034 encounters and #3044); resolve conflicts, regenerate generated files/pins with the repo tooling, never by hand.

Goal: finish the NPC service/travel slice (mechanics row S10: NPC speech, travel, Rashid, NPC services/missions). The previous lane dropped these NPCs as unpublished: GNOPS Member, Harlow (Vengoth), Captain Waverider, Maris (Mistrock), Ferryman Kamil (Meluna), Junkar (Mines). Their changes were only local and are lost; redo them from the Crystal donor if they are in scope, each as its own commit with its own test; drop any that fail rather than block the rest.

Validation (focused only): own tests red then green including own PG cases (prove they executed: duration/marker), cargo check, clippy of changed crates, fmt, content/authoring checks. Do NOT run tools/qualification/node_boot/run.sh here: this environment has no Platform checkout (it reported exact_platform_checkout_missing); CI runs node boot. If the disk fills, clean target/ of unrelated crates and continue.

Push after every green step to claude/happy-davinci-ijz3wy-npcfix (no force, no PR). Reply in at most 12 lines: SHA, NPCs done, NPCs dropped with reason, tests run.
Reasoning effort: medium.

STANDING RULES (from the 2026-11-08 night retro; apply to every push from now on, in addition to PREPUSH-2):
1. Order before every push: merge origin/main FIRST, then make your changes, then regenerate EVERY generated file/packet LAST (quest packets, NPC dialogue links, completion matrix, pins).
2. If you touched content/, quest data, NPC data or tools/content-schema: run locally the quest-authoring checks AND the quest-authoring unit-tests runner (same commands as the CI jobs 'quest-authoring (checks)' and 'quest-authoring (unit-tests)'), and the content-tree validation. Push only when they pass.
3. Sha-pinned proofs (e.g. samples/completion-matrix/reward-use-proofs pins test_sha256 of postgres_tests/qready1_reward_claims.rs): if you change a pinned test file, rerun its PG cases for real (duration > 0.01 s) and re-import with the proof tool (quest_reward_use_proofs.py import_log). Never hand-edit a sha.
4. If you touch node boot / serve / content activation: run the WHOLE node-boot script locally, every variant (including spell_input=full_manifest), not only the stage you fixed.
5. A branch with an open PR is FROZEN: push only CI fixes or a main merge I asked for.
No reply needed; keep working on your current task.


PC LANE NOTE: you run on the owner PC worktree (fresh origin/main). Do NOT commit and do NOT push (sandbox cannot write the git index); leave all changes in the worktree, the operator commits and pushes to the branch named above. Focused checks only (fmt, cargo check/clippy of changed crates, own tests incl. own PG cases if a PG route exists). End with at most 15 lines: files changed, tests run with results.
