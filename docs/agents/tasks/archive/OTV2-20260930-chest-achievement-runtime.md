# OTV2-20260930-chest-achievement-runtime

```yaml
task_id: OTV2-20260930-chest-achievement-runtime
title: ACHIEVEMENT - runtime catalogue loader and the chest USE grants its achievement
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chest-achievement-runtime
issue: 162
lane_id: ACHIEVEMENT
pr: null   # opened by the lead; recorded in the FREEZE_SHA packet on #162
base_sha: 1614042d
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "chest achievement runtime worker (claude-code-session-01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/achievement_catalogue.rs
  - apps/game-server/src/lib.rs   # module declaration only
  - apps/game-server/src/node/serve.rs   # load the catalogue at Content activation
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/src/durability/account_achievement.rs   # entry grammar check, doc
  - apps/game-server/src/durability/reward_claim_mint.rs   # doc comment only
  - apps/game-server/tests/chest_use_postgres.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - tools/content-schema/achievement-authoring/validate_achievements.py
  - tools/content-schema/achievement-authoring/test_validate_achievements.py
  - tools/content-schema/achievement-authoring/README.md
  - docs/agents/tasks/archive/OTV2-20260930-chest-achievement-runtime.md
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md §2, §2.2, §3, §5 step 4"
  - "docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md §3, §4 (RewardClaim placements[].achievement)"
  - "ACHIEVEMENT step 4: reward-claim MINT grant (#1320)"
blocks: ["ACHIEVEMENT: display of account achievements"]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner authorization: 2026-09-30, this session ("wszystkie 3", option a: a chest grants its
achievement in play).

- **Loader.** `apps/game-server/src/achievement_catalogue.rs` (top-level module, like
  `interaction_chest_use`). The two `content/achievements/achievements-*.json` shards are embedded
  with `include_str!`, as the V1 spell book embeds its bundles, and parsed into key -> (revision,
  retired). `AchievementCatalogue::lookup` returns `AchievementCatalogueLookup::Earnable { revision }`,
  `Retired` or `Absent`. Fail closed: a shard that is not JSON or not `{family, records}`, another
  family, a key or revision a grant request cannot carry (the step-3 grammar, through
  `account_achievement::valid_catalogue_entry`), a duplicate key or an empty catalogue refuses the
  whole catalogue. A unit test keeps the embedded list byte-equal to the directory.
- **Load point.** `node/serve.rs` loads it right after the V1 spell book, at Content activation; a
  malformed catalogue refuses readiness (`BootError::ContentActivation("achievement catalogue")`).
  Nothing consumes it there yet: the chest `USE` has no production caller (control-wire lane).
- **Chest keys.** Runtime Content has no RewardClaim family: the D39 caller names the claim and the
  reward. `ChestUseRequest` gains `achievement: Option<String>`, the chest placement's
  `placements[].achievement` key (quest authoring format §4), caller-named for the same reason. Its
  catalogue entry is never taken from the caller.
- **Wiring.** `prepare_chest_use` and `settle_chest_use` take `&AchievementCatalogue`; the MINT
  request carries `Some(RewardClaimAchievement { key, catalogue: lookup(key) })` when the chest has
  an achievement and `None` otherwise.
- **Content validation.** `validate_achievements.py` gains `bind_ref` (contract §2.2: a
  `canary:`/`crystal:`/`crystalserver:` ref binds by slug, `the_professors_nut` explicitly to
  `the_professor_s_nut`, an `oteryn:` key to itself; nothing is guessed), `chest_refs`,
  `unbound_refs` and `--chest-claims`. `test_validate_achievements.py` (run by the Achievement
  authoring CI) asserts that every chest-sample ref binds.
- **Mapping in content.** The chest sample (`quest-authoring/samples/chests/claims.json`) has one
  achievement ref on 4 placements of one claim: `canary:achievement/annihilator` (the Annihilator
  reward chests) -> `oteryn:achievement/annihilator` (revision 1, earnable). No chest ref is
  unbound. Across all quest samples 31 refs; the only one without a same-slug key is the
  interaction ref `the_professors_nut`, bound explicitly. No content file changed.

## Architect choices for review

- Caller-named key rather than a key on the runtime `PlacementRef`: the runtime placement has no
  RewardClaim binding at all (claim and reward are caller-named, a declared Content gap), and
  adding one is a Content-linker change beyond this slice. The lookup, which decides the grant,
  is always the server's.
- The loader is embedded at build time, not read from the activated Content package: the
  catalogue is not part of the content lock yet, and the spell book is the precedent.
- The catalogue lookup is part of the MINT intent binding (step 4): a replay after a catalogue
  revision change conflicts instead of replaying (documented in `chest_use.rs`).

## Validation (local)

- `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-game-server --all-targets -- -D
  warnings`: pass.
- `cargo test --locked -p oteryn-game-server --lib`: 1147 passed, 2 ignored (5 new unit tests:
  loader earnable/retired/absent, malformed, embedded catalogue, directory parity; chest lookup).
- PostgreSQL 17.6 (`postgres:17.6-bookworm` from `mirror.gcr.io`):
  - `chest_use_postgres`: 758 passed, including
    `a_chest_with_an_achievement_grants_it_and_a_chest_without_one_grants_none`;
  - `reward_claim_mint_postgres`: 686 passed; `account_achievement_postgres`: 686 passed;
  - `character_authority_postgres`: 788 passed; `durability_postgres`: 767 passed (plus 1);
    `check_function_privileges_postgres`: 1 passed; `item_mint_postgres`: 700 passed.
- RED (mutation): passing `None` again in `prepare_chest_use` fails the new PG case (no request and
  no fact for the Annihilator chest: `left: ([], [])`).
- Achievement authoring: `validate_achievements.py synthetic-valid-achievement.json`,
  `test_validate_achievements.py` (11 tests), `build_catalogue.py --check`, the catalogue with
  `--chest-claims` (1 ref, 0 unbound), ruff 0.16.1 check and format: pass.
- `validate_governance.py`, governance unit tests, `validate_repository_policy.py`,
  `git diff --check`: pass.

## Closeout

- Review: independent exact-head review routed by the lead on the frozen head. The worker
  triggered no owner-funded review.
- Merge commit/result: squash merge of the PR (resolve with `git log --grep`).
