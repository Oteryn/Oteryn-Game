# SPELL-ARCHIVE-DISPLAY-NAMES-20261112

```yaml
task_id: SPELL-ARCHIVE-DISPLAY-NAMES-20261112
title: Rune catalog display labels with saved-binding compatibility
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/spell-archive-display-names-20261112
pr: 3146
base_sha: 83be72ad403ebc221dadba4b161c85fa1d47219a
head_sha: null
tested_code_head: 33725bd6d22ece82deefa88bfba121260ebc19d4
final_head_sha: null
owner: owner-directed-spell-parity-worker
created_at: 2026-11-12T22:38:03Z
updated_at: 2026-11-12T22:38:03Z
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/spell_catalog.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/spell_catalog.rs
  - crates/client-ui/build_support/spell_metadata.rs
  - apps/client/src/features/0287-actor-spell-catalog/archive.rs
  - apps/client/src/features/0287-actor-spell-catalog/archive_tests.rs
  - docs/agents/tasks/archive/SPELL-ARCHIVE-DISPLAY-NAMES-20261112.md
```

## Outcome

PARTIAL: the catalog chooses an agreed existing conjuration label for rune use. The 32 unanimous source relationships produce 30 changed labels; 2 already match. 3 missing creators and 1 conflicting relationship keep their original label. Instants, keys, incantations, item identities, cast order, costs and eligibility retain their existing owners.

Four verified legacy spelling/name pairs and rune ASCII case differences still resolve saved bindings to a unique current cast index, with words/item exact. Unknown names, changed words/items, instant renames and ambiguous matches remain inactive. Generated UI profession aliases retain the rune-use entry's vocations and preserve old metadata rows.

## Authority and source of truth

- Owner-directed bounded allocation and amendments: https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-6835257232. One writer, one branch. It grants authoring/draft publication, not global coordination, merge or deployment.
- Admission main 83be72ad was synchronized by normal Git merge with repository merge drivers. Relevant open-PR path overlap was checked. No policy/Cargo/protocol/content/pin changes are included.
- Existing contracts: `docs/contracts/protocol-oteryn/v1/actor-spell-catalog-v1.md` and `spellbook-details-v1.md`; the name remains a bounded display string, with the same codec and indices.
- PROVEN: owner-authorized Tibia Test Server 15.33.dd0955 Magical Archive captures on 2026-11-12 show the four labels below. Screenshots remain in the owner's capture archive, not redistributed as original assets in this PR.
- PROVEN: the existing source catalog has 32 unanimous conjuration labels, 3 runes with no creator and 1 conflict. Both label producers use conservative unanimity over all source creators, including inactive aliases. The original source comparison remains pinned to `20d2c7c0e165f00577d99922e63730a57f5a2ffe`; this task does not relabel it as current-main evidence.
- UNKNOWN: deployed Oteryn active generation, original texture/animation fidelity and live-cast behavior. They are not qualified by a card preview or these display tests.

## Reference parity

| Item | Retained legacy label | Captured client label | Capture PNG SHA256 |
|---|---|---|---|
|3153|antidote rune|Cure Poison Rune|8330123575a142b40b8f85fef7f6678d81fcb9a5595f59483f5ced375493e0fc|
|3197|desintegrate rune|Disintegrate Rune|d551375fa0049d806ba42b9648fdb0b4562056345e6f76c554939ef9807b9a02|
|3149|energybomb rune|Energy Bomb Rune|da01ce81f8504105095ce4403cb66eda4b814a17e185cb9c39eb2c7f1e5ad1f6|
|3192|firebomb rune|Fire Bomb Rune|fc7759468b2d28b0818d2fa2a2769440a66a8dd161ea316f12578778e2113fb7|

The evidence class is owner-captured `CIPSOFT_OFFICIAL` Test Server UI, limited to the visible labels. The projection's 32-item expected-label table is independent of the implementation helper. Client filtering exercises every agreed-source label and compares its vocations with the original rune-use metadata.

## Acceptance and preserved boundaries

- [x] Agreed labels, identical creator names, missing creators and conflicting/inactive creator names are covered through the real projection caller.
- [x] Legacy/case-only bindings survive; mixed old/new current identities are refused when ambiguous.
- [x] All 32 agreed-source labels retain the rune-use professions, not the creator's professions; server eligibility remains authoritative.
- [x] The native generation test retains strict soul, mana, words, item, index, learned, eligibility and codec assertions.
- [x] No original asset redistribution, source-header renaming, casting/mana changes or new wire/schema fields.

High-risk authority/recovery qualification: NOT_APPLICABLE. This is a read-only projection plus closed compatibility for local display identities. It introduces no durable server write, session fence, authority issuance or value transfer. Existing current-catalog uniqueness and authoritative cast admission remain in place.

## Validation

- RED: dispatch regressions 1 pass/1 fail before projection; client regressions 4 pass/2 fail before binding/metadata fixes. Existing full native projection test also exposed the old raw-name oracle, then received the independent reference table without removing other assertions.
- `cargo fmt --all --check` — pass, local WSL Rust 1.99.1, after final Rust writes.
- `cargo test --locked -p oteryn-gameplay-dispatch --quiet` — pass, 3506 passed, 0 failed, 16 pre-existing ignored; local WSL.
- `cargo clippy --locked -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings` — pass; local WSL.
- `cargo test --locked -p oteryn-client --quiet` — pass, 883 passed across 32 targets, 0 failed, 0 ignored; Windows.
- `cargo clippy --locked -p oteryn-client --all-targets --quiet -- -D warnings` — pass; Windows.
- `cargo test --locked -p oteryn-client-ui --quiet` — pass, 656 passed across 26 targets, 0 failed, 0 ignored; Windows.
- `cargo clippy --locked -p oteryn-client-ui --all-targets --quiet -- -D warnings` — pass; Windows.
- `git diff --check` — pass.
- `python tools/agents/validate_governance.py` — pass.
- `python -m unittest discover -s tools/agents/tests` — pass, 129 tests.

Windows cannot compile dispatch's Unix-only dependency, so its checks ran in local WSL, not a Cloud executor. Fixture byte conversion in the initial Windows checkout caused digest failures; restoration from exact Git blobs repaired the environment without data/pin changes, and all content manifest input hashes matched. The final Windows run emitted two long-test notices in existing Guild/Boss cases and later completed exit 0; the delay's cause was not established. Existing third-party deprecation warnings remain outside scope.

## Rollout, limits and closeout

Release requires a coordinated client/server update. Old binaries still use exact saved names; deploying only the new server can deactivate their old bindings or hide the renamed entries under profession filters. This PR does not claim old-binary compatibility or readiness for unilateral deployment. Settings schema is unchanged; rollback to old binaries may need old saved labels.

This is not full 1:1 Spell Archive. Paralyse/Paralyze, conflicting Light Magic Missile labels, other current-name differences, descriptions, mana, original icons/effects, range/area and deployment remain separate evidence/implementation work.

Authoring slice completed in draft PR #3146; no merge or deployment performed. Independent review and repository CI remain pending on the final remote head. The final exact SHA belongs in the immutable PR/FREEZE readback after this record's commit, avoiding a self-referential commit. Ownership remains with the branch writer until the lead closes or reopens AUTHORING; this record grants no further authority.
