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
pr: 1348
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
  - apps/game-server/src/node/serve.rs   # load the catalogue and bind the activated Content to it
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/src/content/reference_playable.rs   # shared: RewardClaimPlacement.achievement and its check only
  - apps/game-server/tests/content_reference_playable.rs   # shared: one RewardClaim case, one fixture field
  - apps/game-server/src/durability/account_achievement.rs   # entry grammar check, doc
  - apps/game-server/src/durability/reward_claim_mint.rs   # doc comment only
  - apps/game-server/tests/chest_use_postgres.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - tools/content-schema/achievement-authoring/validate_achievements.py
  - tools/content-schema/achievement-authoring/test_validate_achievements.py
  - tools/content-schema/achievement-authoring/README.md
  - docs/agents/tasks/archive/OTV2-20260930-chest-achievement-runtime.md
  - .github/workflows/achievement-authoring-schema.yml   # path filter only; owner authorized 2026-09-30 (answer 3a)
public_contracts: []
depends_on:
  - "docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md §2, §2.2, §3, §5 step 4"
  - "docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md §3, §4 (RewardClaim placements[].achievement)"
  - "ACHIEVEMENT step 4: reward-claim MINT grant (#1320)"
  - "CHEST-CONTENT part 1: RewardClaim Content family (#1334)"
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
  malformed catalogue refuses readiness (`BootError::ContentActivation("achievement catalogue")`),
  and so does activated Content whose RewardClaim names a key the catalogue lacks (see *Merge with
  main*). The chest `USE` has no production caller yet (control-wire lane).
- **Chest keys.** Superseded by the merge with #1334: the key comes from Content, not the caller.
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

## Merge with main (#1334)

#1334 (CHEST-CONTENT part 1) added the RewardClaim Content family and made D39 resolve the claim and
the reward from Content. `origin/main` was merged into this branch with a normal merge commit.

- **Conflicts.** `interaction/chest_use.rs`: `ChestUseRequest` keeps main's shape (it names only
  the chest); the caller-named `claim`, `reward_item`, `quantity` and `achievement` are gone, and
  the unit-test fixture follows main. `tests/support/chest_use_postgres_cases.rs`: main's
  `use_request(command, chest)`, `with_reward` and refusal table are kept, each call also passes
  the catalogue; the `catalogue()` helper is kept next to `with_reward`.
- **Key from Content.** `RewardClaimPlacement` gains `achievement: Option<String>`, the placement's
  `oteryn:achievement/<slug>` key (quest authoring format §4 `placements[].achievement`, bound at
  authoring by the contract §2.2 slug rule: `canary:achievement/annihilator` ->
  `oteryn:achievement/annihilator`). `resolve_chest` returns it with the claim and reward, and
  `prepare_chest_use` resolves its entry in the `AchievementCatalogue`. The caller names neither
  the key nor its entry, which closes the earlier trust note (caller-named key; owner question 1).
- **Content validation.** Link validation refuses a placement achievement that is not an
  `oteryn:achievement/` key, so an unbound source ref never reaches Content.
  `AchievementCatalogue::unbound_reward_claim_achievements` lists the keys a Content's RewardClaims
  name and the catalogue lacks (contract §3.3; a retired key binds), and `node/serve.rs` refuses a
  Content activation with any. CI runs it through `--lib` (key cases), `content_reference_playable`
  (the link check) and `chest_use_postgres` (Annihilator binds; an absent key is reported and still
  refused by the MINT). The authoring side stays `validate_achievements.py --chest-claims`.
- **End-to-end case.** The grant case now sets the achievement in Content: Annihilator on
  `OTHER_CHEST_PLACEMENT` grants; a replay grants nothing again; a later Content without the
  achievement conflicts under the same command; a retired key commits and records nothing; an
  absent key on a fourth claim is refused before any write.

## Architect choices for review

- The key is taken from the RewardClaim placement in Content (since #1334); the lookup, which
  decides the grant, is always the server's. Content carries the bound catalogue key, not the
  source ref: binding by slug happens once, at authoring, never at runtime.
- The loader is embedded at build time, not read from the activated Content package: the
  catalogue is not part of the content lock yet, and the spell book is the precedent.
- The catalogue lookup is part of the MINT intent binding (step 4): a replay after a catalogue
  revision change conflicts instead of replaying (documented in `chest_use.rs`).

## Validation (local)

- `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-game-server --all-targets -- -D
  warnings`: pass.
- `cargo test --locked -p oteryn-game-server --lib`: 1148 passed, 2 ignored (6 new unit tests:
  loader earnable/retired/absent, malformed, embedded catalogue, directory parity, unbound keys;
  chest lookup). `content_reference_playable`: 46 passed; `interaction_workflow`: 15 passed.
- PostgreSQL 17.6 (`postgres:17.6-bookworm` from `mirror.gcr.io`), after the merge with #1334:
  - `chest_use_postgres`: 759 passed, including
    `a_chest_with_an_achievement_grants_it_and_a_chest_without_one_grants_none`;
  - `reward_claim_mint_postgres`: 686 passed; `account_achievement_postgres`: 686 passed;
  - `character_authority_postgres`: 788 passed; `durability_postgres`: 767 passed (plus 1);
    `check_function_privileges_postgres`: 1 passed; `item_mint_postgres`: 700 passed.
- RED (mutation): dropping the Content achievement (`chest_achievement(achievements, None)` in
  `prepare_chest_use`) fails the PG case (no request and no fact for the Annihilator chest:
  `left: ([], [])`).
- Achievement authoring: `validate_achievements.py synthetic-valid-achievement.json`,
  `test_validate_achievements.py` (11 tests), `build_catalogue.py --check`, the catalogue with
  `--chest-claims` (1 ref, 0 unbound), ruff 0.16.1 check and format: pass.
- `validate_governance.py`, governance unit tests, `validate_repository_policy.py`,
  `git diff --check`: pass.

## Closeout

- Review: independent exact-head review routed by the lead on the frozen head. The worker
  triggered no owner-funded review.
- Merge commit/result: squash merge of the PR (resolve with `git log --grep`).

## Owner answers

- 2026-09-30, answer 3a: the Achievement Authoring Schema workflow also triggers on a hand edit of
  `tools/content-schema/quest-authoring/samples/chests/claims.json` (one path-filter line).
