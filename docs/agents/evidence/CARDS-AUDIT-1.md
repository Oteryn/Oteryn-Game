# CARDS-AUDIT-1 — task-card reconciliation

Repository snapshot: `Oteryn/Oteryn-Game`, `origin/main` and worktree base `ac3f7fbc6279687c71d6ac3ec43eb27383451003`, audited 2026-10-23. Local documentation edits only; no commit, staging, publication, remote mutation or live deployment verification.

100 existing non-archived cards audited: **DONE 33, PARTIAL 39, OPEN 26, SUPERSEDED 2**. The one Soul War card is byte-identical. `TASK_TEMPLATE.md` and README/index files are not implementation cards. The new mechanic index has 36 requested areas and 153 resolving source/card links.

Each card was checked with `git log --oneline origin/main --grep <CARD-ID>` and the named current source. Where a long task ID had no log match, short IDs and file history identified the actual squash/carrier, notably #1675 (monster packets), #2005 (entry spell tiles), #1917 (source-lowered quest bindings), #2208 (bootstrap diagnostics), and #2309 (quest runtime source carried with DAILY-1a). A merged packet or removed source is not counted as a working mechanic. The lifecycle values remain those of the task template; full merged PR URLs are in tracking notes, and `pr:` remains numeric for terminal cards. Unfinished cards have `pr: null` rather than a terminal slice masquerading as an active candidate.

The archive-only reconciliation is within the archive README's exception for records that did not close in their own PR. Filesystem renames leave all changes unstaged: `git mv` would write the shared Git index outside this worktree directory. No Git mutation command was used.

## Validation

- `git diff --check`: PASS; new/untracked archive and index files also checked with read-only `git diff --no-index --check`.
- `python3 tools/agents/validate_governance.py`: PASS (22 required policy documents, 9 lanes).
- `python3 -m unittest discover -s tools/agents/tests`: PASS (124 tests).
- `python3 tools/repository/validate_repository_policy.py`: PASS (23 files, 68 workflows, routing regressions).
- `PYTHONPATH=tools/repository python3 -m unittest test_agent_governance_pr_targets.GovernanceTargets`: PASS (5 tests).
- `python3 -m unittest discover -s tools/repository -p 'test_*.py'`: NOT PASS (120 tests; 64 errors; workflow-expression tests require the unavailable `node` executable). No dependency installed, test skipped or protected file changed.
- Preservation checks: PASS for 187 acceptance/scope/design sections; relocated links are the only changes within those bodies. Every original card retained; 35 terminal cards moved; no stale moved-card links; Soul War unchanged; shared Git index unchanged.
- D1462 whole-diff review against the owner's `CODEX-CHECKLIST.md`: PASS for this documentation-only delta. No Rust enum, field, trait method or public function added. Cargo/clippy/fmt and PostgreSQL filters: NOT_APPLICABLE (no Rust, Cargo, SQL, workflow or content changes).

## Unfinished

The broader repository unittest discovery needs CI or an existing environment with Node.js. Implementation work named in PARTIAL/OPEN cards is outside this audit. No publication was requested or performed.

## Pass 2 — card authoring and closeout rules

Reviewed `git show --stat HEAD` first: pass 1 is `f1731bee8a0bc67e48b319aeaab7304f1f415830`. Resolved organization policy 3.1.0 at the immutable META binding `1bfb5ff98c8aa156e73669a14e083a1d464c29fb` through a read-only fetch. This pass changes only documentation and the existing Python governance validator/tests; no Rust, Soul War card, protected repository tool or workflow changes.

The [task template](../tasks/TASK_TEMPLATE.md), [task lifecycle process](../README.md#task-lifecycle) and [closeout procedure](../DELIVERY_COMPLETENESS_AND_CLOSEOUT.md#mechanic-cards) now require every mechanic card to declare `type: mechanic`, contain the Reference parity table and back every behaviour row with a table-driven test. Before work, agents must check MECHANICS_STATUS.md and the archive to avoid reimplementation; closeout updates the mechanic's index row in the same PR.

The existing validator is [tools/agents/validate_governance.py](../../../tools/agents/validate_governance.py), outside the prohibited `tools/repository/` tree. Its new check requires a real `## Reference parity` heading on explicitly typed mechanic cards whose status is DONE or completed, across root, active and archive task directories. `completed` remains the canonical lifecycle status. Historical cards without the new type field are not retroactively reclassified. The check also resolves the mechanic index's inline local file links relative to that index and rejects missing or out-of-repository destinations. It checks presence and link integrity; the implementer/reviewer must still verify reference correctness and table-driven coverage.

### Pass 2 validation

- `git diff --check`: PASS; new Python test file also checked with read-only `git diff --no-index --check`.
- `python3 tools/agents/validate_governance.py`: PASS (22 required policy documents, 9 lanes; all 153 index links resolve).
- `python3 -m unittest discover -s tools/agents/tests`: PASS (129 tests, including five new regression tests with table-driven heading/status/directory cases and link-resolution coverage).
- `python3 tools/repository/validate_repository_policy.py`: PASS (23 files, 68 workflows, routing regressions).
- `PYTHONPATH=tools/repository python3 -m unittest test_agent_governance_pr_targets.GovernanceTargets`: PASS (5 tests).
- `python3 -m unittest discover -s tools/repository -p 'test_*.py'`: NOT PASS (120 tests; 64 errors, all missing `node`, as in pass 1). No tests weakened, skipped or deleted; no dependencies downloaded.
- Whole-diff review against the owner's `CODEX-CHECKLIST.md`: PASS for this docs/process delta; no Rust enum variants, struct fields, trait methods or public functions added. Cargo test filters, clippy, rustfmt and PostgreSQL tests: NOT_APPLICABLE (no Rust, Cargo or runtime changes).
- Soul War card preservation: PASS (byte-identical to HEAD).

### Pass 2 Unfinished

The full repository unittest discovery still requires Node.js in CI or an existing authorized environment. No local installation or protected-file workaround was attempted. No commit, staging, push or publication was performed.

## Moved cards (35)

- `docs/agents/tasks/ADMIT-CAP-1.md` → [docs/agents/tasks/archive/ADMIT-CAP-1.md](../tasks/archive/ADMIT-CAP-1.md) — DONE.
- `docs/agents/tasks/BAGS-USE-1.md` → [docs/agents/tasks/archive/BAGS-USE-1.md](../tasks/archive/BAGS-USE-1.md) — DONE.
- `docs/agents/tasks/CI-PG-LIB-1.md` → [docs/agents/tasks/archive/CI-PG-LIB-1.md](../tasks/archive/CI-PG-LIB-1.md) — SUPERSEDED.
- `docs/agents/tasks/CI-PG-LIB-2.md` → [docs/agents/tasks/archive/CI-PG-LIB-2.md](../tasks/archive/CI-PG-LIB-2.md) — DONE.
- `docs/agents/tasks/CRAFT-CONTENT-1.md` → [docs/agents/tasks/archive/CRAFT-CONTENT-1.md](../tasks/archive/CRAFT-CONTENT-1.md) — DONE.
- `docs/agents/tasks/CREATURE-MOVE-1.md` → [docs/agents/tasks/archive/CREATURE-MOVE-1.md](../tasks/archive/CREATURE-MOVE-1.md) — DONE.
- `docs/agents/tasks/DEATH-ML-LOSS-1.md` → [docs/agents/tasks/archive/DEATH-ML-LOSS-1.md](../tasks/archive/DEATH-ML-LOSS-1.md) — DONE.
- `docs/agents/tasks/DISPATCH-WIRING-1.md` → [docs/agents/tasks/archive/DISPATCH-WIRING-1.md](../tasks/archive/DISPATCH-WIRING-1.md) — DONE.
- `docs/agents/tasks/GUILD-1.md` → [docs/agents/tasks/archive/GUILD-1.md](../tasks/archive/GUILD-1.md) — DONE.
- `docs/agents/tasks/ITEM-PIN-1.md` → [docs/agents/tasks/archive/ITEM-PIN-1.md](../tasks/archive/ITEM-PIN-1.md) — DONE.
- `docs/agents/tasks/LOOP-MONSTER-1.md` → [docs/agents/tasks/archive/LOOP-MONSTER-1.md](../tasks/archive/LOOP-MONSTER-1.md) — DONE.
- `docs/agents/tasks/MAP-PICKUP-1.md` → [docs/agents/tasks/archive/MAP-PICKUP-1.md](../tasks/archive/MAP-PICKUP-1.md) — DONE.
- `docs/agents/tasks/MAP-VIEWPORT-OPT-1.md` → [docs/agents/tasks/archive/MAP-VIEWPORT-OPT-1.md](../tasks/archive/MAP-VIEWPORT-OPT-1.md) — DONE.
- `docs/agents/tasks/MONK-SAVE-FLAKE-1.md` → [docs/agents/tasks/archive/MONK-SAVE-FLAKE-1.md](../tasks/archive/MONK-SAVE-FLAKE-1.md) — DONE.
- `docs/agents/tasks/NOQUEUE-1.md` → [docs/agents/tasks/archive/NOQUEUE-1.md](../tasks/archive/NOQUEUE-1.md) — DONE.
- `docs/agents/tasks/NPC-ECON-1.md` → [docs/agents/tasks/archive/NPC-ECON-1.md](../tasks/archive/NPC-ECON-1.md) — DONE.
- `docs/agents/tasks/OTV2-20261001-monster-loot-parser-completion-round7.md` → [docs/agents/tasks/archive/OTV2-20261001-monster-loot-parser-completion-round7.md](../tasks/archive/OTV2-20261001-monster-loot-parser-completion-round7.md) — DONE.
- `docs/agents/tasks/OTV2-20261001-monster-mitigation-round4.md` → [docs/agents/tasks/archive/OTV2-20261001-monster-mitigation-round4.md](../tasks/archive/OTV2-20261001-monster-mitigation-round4.md) — DONE.
- `docs/agents/tasks/OTV2-20261002-monster-completion-draft-publication.md` → [docs/agents/tasks/archive/OTV2-20261002-monster-completion-draft-publication.md](../tasks/archive/OTV2-20261002-monster-completion-draft-publication.md) — SUPERSEDED.
- `docs/agents/tasks/OTV2-20261006-quest-source-progress-overlay.md` → [docs/agents/tasks/archive/OTV2-20261006-quest-source-progress-overlay.md](../tasks/archive/OTV2-20261006-quest-source-progress-overlay.md) — DONE.
- `docs/agents/tasks/OTV2-20261007-game-preprod-topology-1.md` → [docs/agents/tasks/archive/OTV2-20261007-game-preprod-topology-1.md](../tasks/archive/OTV2-20261007-game-preprod-topology-1.md) — DONE.
- `docs/agents/tasks/OTV2-20261007-quest-source-lowered-binding-plan.md` → [docs/agents/tasks/archive/OTV2-20261007-quest-source-lowered-binding-plan.md](../tasks/archive/OTV2-20261007-quest-source-lowered-binding-plan.md) — DONE.
- `docs/agents/tasks/OTV2-20261007-shards-e1-max-health-admission.md` → [docs/agents/tasks/archive/OTV2-20261007-shards-e1-max-health-admission.md](../tasks/archive/OTV2-20261007-shards-e1-max-health-admission.md) — DONE.
- `docs/agents/tasks/OTV2-20261007-shards-i1-current-source-identity.md` → [docs/agents/tasks/archive/OTV2-20261007-shards-i1-current-source-identity.md](../tasks/archive/OTV2-20261007-shards-i1-current-source-identity.md) — DONE.
- `docs/agents/tasks/OTV2-20261011-entry-spell-tiles-1.md` → [docs/agents/tasks/archive/OTV2-20261011-entry-spell-tiles-1.md](../tasks/archive/OTV2-20261011-entry-spell-tiles-1.md) — DONE.
- `docs/agents/tasks/PIN-SCOPE-1.md` → [docs/agents/tasks/archive/PIN-SCOPE-1.md](../tasks/archive/PIN-SCOPE-1.md) — DONE.
- `docs/agents/tasks/PREM-2b-PREM-5.md` → [docs/agents/tasks/archive/PREM-2b-PREM-5.md](../tasks/archive/PREM-2b-PREM-5.md) — DONE.
- `docs/agents/tasks/PVP-1b.md` → [docs/agents/tasks/archive/PVP-1b.md](../tasks/archive/PVP-1b.md) — DONE.
- `docs/agents/tasks/QUEST-RT-1.md` → [docs/agents/tasks/archive/QUEST-RT-1.md](../tasks/archive/QUEST-RT-1.md) — DONE.
- `docs/agents/tasks/RUNE-CAST-1.md` → [docs/agents/tasks/archive/RUNE-CAST-1.md](../tasks/archive/RUNE-CAST-1.md) — DONE.
- `docs/agents/tasks/TUTORIAL-1.md` → [docs/agents/tasks/archive/TUTORIAL-1.md](../tasks/archive/TUTORIAL-1.md) — DONE.
- `docs/agents/tasks/WEATHER-1.md` → [docs/agents/tasks/archive/WEATHER-1.md](../tasks/archive/WEATHER-1.md) — DONE.
- `docs/agents/tasks/active/BUNDLE-MONSTER-AI-1.md` → [docs/agents/tasks/archive/BUNDLE-MONSTER-AI-1.md](../tasks/archive/BUNDLE-MONSTER-AI-1.md) — DONE.
- `docs/agents/tasks/active/OTV2-20260917-dur03-reference-one-item-resource-evidence-513.md` → [docs/agents/tasks/archive/OTV2-20260917-dur03-reference-one-item-resource-evidence-513.md](../tasks/archive/OTV2-20260917-dur03-reference-one-item-resource-evidence-513.md) — DONE.
- `docs/agents/tasks/active/OTV2-20260929-item-id-1-tibia-id-keys.md` → [docs/agents/tasks/archive/OTV2-20260929-item-id-1-tibia-id-keys.md](../tasks/archive/OTV2-20260929-item-id-1-tibia-id-keys.md) — DONE.

## Updated cards in place (65)

- [docs/agents/tasks/ACH-1.md](../tasks/ACH-1.md) — PARTIAL.
- [docs/agents/tasks/ADMIT-CAP-2.md](../tasks/ADMIT-CAP-2.md) — OPEN.
- [docs/agents/tasks/APP-1.md](../tasks/APP-1.md) — PARTIAL.
- [docs/agents/tasks/APP-2.md](../tasks/APP-2.md) — PARTIAL.
- [docs/agents/tasks/APP-3.md](../tasks/APP-3.md) — OPEN.
- [docs/agents/tasks/APP-4.md](../tasks/APP-4.md) — OPEN.
- [docs/agents/tasks/BED-1.md](../tasks/BED-1.md) — PARTIAL.
- [docs/agents/tasks/BED-REGEN-1.md](../tasks/BED-REGEN-1.md) — OPEN.
- [docs/agents/tasks/CRAFT-1.md](../tasks/CRAFT-1.md) — PARTIAL.
- [docs/agents/tasks/DAILY-1.md](../tasks/DAILY-1.md) — PARTIAL.
- [docs/agents/tasks/DAWNPORT-1.md](../tasks/DAWNPORT-1.md) — PARTIAL.
- [docs/agents/tasks/EQUIP-RT-1.md](../tasks/EQUIP-RT-1.md) — PARTIAL.
- [docs/agents/tasks/EVENTS-RAID-1.md](../tasks/EVENTS-RAID-1.md) — OPEN.
- [docs/agents/tasks/EVENTS-WORLD-1.md](../tasks/EVENTS-WORLD-1.md) — PARTIAL.
- [docs/agents/tasks/GM-1.md](../tasks/GM-1.md) — OPEN.
- [docs/agents/tasks/GUILD-BANK-1.md](../tasks/GUILD-BANK-1.md) — OPEN.
- [docs/agents/tasks/archive/GUILD-CHAT-1.md](../tasks/archive/GUILD-CHAT-1.md) — COMPLETE (#2326, archived).
- [docs/agents/tasks/GUILD-WIRE-1.md](../tasks/GUILD-WIRE-1.md) — OPEN.
- [docs/agents/tasks/GUILDHALL-1.md](../tasks/GUILDHALL-1.md) — OPEN.
- [docs/agents/tasks/HIRE-1.md](../tasks/HIRE-1.md) — OPEN.
- [docs/agents/tasks/HOUSE-1a.md](../tasks/HOUSE-1a.md) — OPEN.
- [docs/agents/tasks/HOUSE-1b.md](../tasks/HOUSE-1b.md) — OPEN.
- [docs/agents/tasks/HOUSE-ACL-1.md](../tasks/HOUSE-ACL-1.md) — OPEN.
- [docs/agents/tasks/HOUSE-RUNTIME-1a.md](../tasks/HOUSE-RUNTIME-1a.md) — PARTIAL.
- [docs/agents/tasks/HOUSE-RUNTIME-1b.md](../tasks/HOUSE-RUNTIME-1b.md) — OPEN.
- [docs/agents/tasks/HOUSE-RUNTIME-1c.md](../tasks/HOUSE-RUNTIME-1c.md) — OPEN.
- [docs/agents/tasks/HOUSE-WIRE-1.md](../tasks/HOUSE-WIRE-1.md) — OPEN.
- [docs/agents/tasks/HUNT-1.md](../tasks/HUNT-1.md) — OPEN.
- [docs/agents/tasks/ITEM-FORGE-1.md](../tasks/ITEM-FORGE-1.md) — PARTIAL.
- [docs/agents/tasks/ITEM-IMBUE-1.md](../tasks/ITEM-IMBUE-1.md) — OPEN.
- [docs/agents/tasks/ITEM-STORAGE-1.md](../tasks/ITEM-STORAGE-1.md) — OPEN.
- [docs/agents/tasks/ITEM-TIMED-1.md](../tasks/ITEM-TIMED-1.md) — PARTIAL.
- [docs/agents/tasks/LOOP-DAMAGE-1.md](../tasks/LOOP-DAMAGE-1.md) — PARTIAL.
- [docs/agents/tasks/LOOP-GROUND-1.md](../tasks/LOOP-GROUND-1.md) — PARTIAL.
- [docs/agents/tasks/LOOP-MAP-1.md](../tasks/LOOP-MAP-1.md) — PARTIAL.
- [docs/agents/tasks/LOOP-PROGRESS-1.md](../tasks/LOOP-PROGRESS-1.md) — PARTIAL.
- [docs/agents/tasks/LOOP-TRAIN-1.md](../tasks/LOOP-TRAIN-1.md) — OPEN.
- [docs/agents/tasks/LOOP-VITALS-1.md](../tasks/LOOP-VITALS-1.md) — PARTIAL.
- [docs/agents/tasks/MAGIC-SUMMON-1.md](../tasks/MAGIC-SUMMON-1.md) — PARTIAL.
- [docs/agents/tasks/MAIL-1.md](../tasks/MAIL-1.md) — PARTIAL.
- [docs/agents/tasks/MAP-CUTOVER-1c.md](../tasks/MAP-CUTOVER-1c.md) — OPEN.
- [docs/agents/tasks/archive/MAP-OVERLAY-1c.md](../tasks/archive/MAP-OVERLAY-1c.md) — COMPLETE (#2327, archived).
- [docs/agents/tasks/MARKET-1.md](../tasks/MARKET-1.md) — PARTIAL.
- [docs/agents/tasks/MONSTER-META-1.md](../tasks/MONSTER-META-1.md) — PARTIAL.
- [docs/agents/tasks/NPC-WORLD-1.md](../tasks/NPC-WORLD-1.md) — PARTIAL.
- [docs/agents/tasks/OTV2-20261001-monster-data-completion.md](../tasks/OTV2-20261001-monster-data-completion.md) — PARTIAL.
- [docs/agents/tasks/OTV2-20261001-monster-dependency-completion-round6.md](../tasks/OTV2-20261001-monster-dependency-completion-round6.md) — PARTIAL.
- [docs/agents/tasks/OTV2-20261006-quest-completion-all373-r2.md](../tasks/OTV2-20261006-quest-completion-all373-r2.md) — PARTIAL.
- [docs/agents/tasks/OTV2-20261011-boot-rat-policy-1.md](../tasks/OTV2-20261011-boot-rat-policy-1.md) — PARTIAL.
- [docs/agents/tasks/PARTY-1.md](../tasks/PARTY-1.md) — PARTIAL.
- [docs/agents/tasks/archive/PARTY-XP-1.md](../tasks/archive/PARTY-XP-1.md) — PARTIAL.
- [docs/agents/tasks/PROG-ADV-1.md](../tasks/PROG-ADV-1.md) — PARTIAL.
- [docs/agents/tasks/PVP-DEATH-1.md](../tasks/PVP-DEATH-1.md) — PARTIAL.
- [docs/agents/tasks/PVP-RT-1.md](../tasks/PVP-RT-1.md) — PARTIAL.
- [docs/agents/tasks/RANK-1.md](../tasks/RANK-1.md) — PARTIAL.
- [docs/agents/tasks/SPECTATE-1.md](../tasks/SPECTATE-1.md) — PARTIAL.
- [docs/agents/tasks/TRADE-PLAYER-1.md](../tasks/TRADE-PLAYER-1.md) — OPEN.
- [docs/agents/tasks/VIP-1.md](../tasks/VIP-1.md) — OPEN.
- [docs/agents/tasks/VIP-2.md](../tasks/VIP-2.md) — OPEN.
- [docs/agents/tasks/active/LOOP-WORLD-1.md](../tasks/active/LOOP-WORLD-1.md) — PARTIAL.
- [docs/agents/tasks/active/OTERYN-20261015-bootstrap-diagnostics-2206.md](../tasks/active/OTERYN-20261015-bootstrap-diagnostics-2206.md) — PARTIAL.
- [docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md](../tasks/active/OTV2-20260825-work-delivery-coordinator.md) — PARTIAL.
- [docs/agents/tasks/active/OTV2-20261001-charm-runtime-completion.md](../tasks/active/OTV2-20261001-charm-runtime-completion.md) — PARTIAL.
- [docs/agents/tasks/active/OTV2-20261015-oracle-arm64-deploy.md](../tasks/active/OTV2-20261015-oracle-arm64-deploy.md) — PARTIAL.
- [docs/agents/tasks/active/SEC-1.md](../tasks/active/SEC-1.md) — PARTIAL.

## Other changed files

- [docs/agents/README.md](../README.md).
- [docs/agents/evidence/CARDS-AUDIT-1.md](CARDS-AUDIT-1.md).
- [docs/agents/evidence/PREM-2b-PREM-5-local-validation.md](PREM-2b-PREM-5-local-validation.md).
- [docs/agents/tasks/MECHANICS_STATUS.md](../tasks/MECHANICS_STATUS.md).
- [docs/agents/tasks/active/README.md](../tasks/active/README.md).
- [docs/agents/tasks/archive/OTV2-20261003-active-packets-repoint.md](../tasks/archive/OTV2-20261003-active-packets-repoint.md).
- [docs/agents/tasks/archive/OTV2-20261011-accept-chat-attack-death-0.md](../tasks/archive/OTV2-20261011-accept-chat-attack-death-0.md).
- [docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ROOT_PACKETS_2026-10-04.md](../../architecture/reviews/OTERYN_GAME_ARCH_BATCH_ROOT_PACKETS_2026-10-04.md).
- [docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md](../../architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md).
