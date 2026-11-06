# QUESTPC final local command evidence

| Exact command | Exit | Seconds | Evidence |
| --- | ---: | ---: | --- |
| `/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-content -p oteryn-gameplay-durability -p oteryn-gameplay-quests -p oteryn-gameplay-social -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --all-features --quiet -- -D warnings` | 0 | 38.75 | `.questpc/r5-clippy-final.log` |
| `/home/mole/oteryn-cache/codex-bin/cargo fmt --all --check` | 0 | 8.343 | `.questpc/r5-fmt-final.log` |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --lib quest_catalogue_boot_tests --quiet` | 0 | 21.23 | `.questpc/r5-boot-confirm.log` |
| `/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib reward_claim_mint::tests --quiet` | 0 | 10.665 | `.questpc/r5-fluid-unit-confirm.log` |
| `python3 tools/content-schema/quest-authoring/run_checks.py --part checks` | 0 | 188.299 | `.questpc/r5-authoring-checks.log` |
| `/home/mole/oteryn-cache/venv-ci312/bin/python tools/content-schema/quest-authoring/quest_donor_source_authoring.py --check` | 0 | 411.754 | `.questpc/r5-donor-check.log` |
| `python3 tools/content-schema/quest-authoring/run_checks.py --part unit-tests --jobs 2` | 1 | 333.296 | `.questpc/r5-authoring-units.log` |
| `QPC_SLOT=3 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres filled_fluid_claims_commit_root_and_children_and_replay_without_refilling -- --nocapture` | 101 | wall timer unavailable; see build/test duration | `.questpc/fluid-postgres.log` |
| `QPC_SLOT=2 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib quest_catalogue_chests_medusa_exact_crystal_reward_use_and_replay -- --nocapture` | 101 | 366.744 | `.questpc/chest-medusa.log` |

Authoring units:92/93modules pass; map decoder setup fails because zlib.h is absent.
PG commands: no POSTGRES_CASE_COMPLETED markers; Connection refused.
The fresh chest run stopped further PG filters after Medusa; inherited r3 runs selected both focused reward tests.
No whole-quest certification or new PG qualification.
