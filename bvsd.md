Single agent work in this cloud session. Use the repository checkout you are in; build with CARGO_BUILD_JOBS=4. Load the repo skills oteryn-dev, oteryn-coding and oteryn-verify. Do not open a PR, do not label, do not merge. Final report at most 20 lines.

Repository Oteryn/Oteryn-Game. Start from `origin/claude/happy-davinci-ijz3wy-bvint` at exact commit `b97bf3aa2c2ee9bfef6a047817f898c087b230e2` (git fetch origin claude/happy-davinci-ijz3wy-bvint; git checkout -b claude/happy-davinci-ijz3wy-bvsd b97bf3aa2c2ee9bfef6a047817f898c087b230e2). This is the Bevy client package (apps/client/src/bevy_shell/**, feature `bevy-shell`); the Win32 UI is being removed. Parity reference is Tibia Global (current official client); OTClient-only extras are out of scope.

A Codex review found the findings below. Fix EVERY finding in this list. For each one add a regression test that goes through the REAL caller (the Bevy input route / observer / system that a player triggers, not a direct call into the helper the finding names) and fails before the fix and passes after; name it in the report. If a finding is wrong (already correct or unreachable), change nothing for it and give the traced reason. Touch only the files the findings need (plus their tests and the six-language client text catalogs if you add UI text). No stubs, no todo!/unimplemented!, no #[ignore], no empty match arms, no placeholder data; no test skipped, ignored, loosened or deleted. Do not touch .github/**, tools/repository/**, docs/agents/**.

Checks (iterate until green):
1. cargo fmt --all --check
2. cargo clippy --locked -p oteryn-client --features bevy-shell --all-targets -- -D warnings
3. cargo clippy --locked -p oteryn-client --all-targets -- -D warnings
4. cargo test --locked -p oteryn-client --features bevy-shell --lib -- bevy_shell::
5. cargo test --locked -p oteryn-client --lib
6. the client text catalog check if you touched UI text (find it with `rg -l "six-language" apps/client crates/client-ui | head`).

Commit on `claude/happy-davinci-ijz3wy-bvsd` with author `blakinio <75369544+blakinio@users.noreply.github.com>`, then `git push origin claude/happy-davinci-ijz3wy-bvsd`. No PR, no force, no other branch. Never end a turn with only a status: end only after a push or a verbatim hard blocker (disk full/OOM are not blockers: clean your own target, CARGO_BUILD_JOBS=2).
Final reply: branch, pushed SHA, base SHA, per finding fixed (file:line, test name) or declined (reason), per check its result line, push output.

FINDINGS:

1. **P1 — Undisclosed gold spending without confirmation.** `apps/client/src/bevy_shell/game_ui/bestiary.rs:147` immediately submits Remove and ResetAll; `apps/client/src/bevy_shell/game_ui/task_board/entries.rs:175` exposes BuyTalisman with no price. Real button activation reaches `charm_progression_request` or `task_board_full_request`, and the server deducts gold immediately. All three instances omit the gold cost before submission and offer no confirmation. For example, resetting at level 757 with Expansion costs 5,495,250 gold; buying a talisman costs 5,000 gold. Display the applicable cost and require confirmation before dispatch.

2. **P1 — Incorrect reset eligibility.** `apps/client/src/bevy_shell/game_ui/bestiary/widgets.rs:286` enables ResetAll only when an assignment exists. After unlocking charms and removing their assignments—or before assigning them—the real Bestiary UI disables reset, preventing the player from refunding spent points. The server resets unlocked stages as well as assignments. Enable reset when unlocked stages exist, including when every charm is unassigned.

3. **P1 — Missing production wiring for charm identification.** `apps/client/src/bevy_shell/game_ui/bestiary/widgets.rs:216` reads names from `panel.charm_browser.cards`, but no production caller requests that catalogue; `charm_catalogue_request` is called only by tests. Opening Bestiary therefore presents actionable “Charm N” placeholders. The renderer also never displays charm effect descriptions. Players can spend points or assign a charm without knowing its identity or effect, unlike Tibia Global. Load every catalogue page through the real opening path and render the selected charm’s description.

4. **P2 — Incorrect talisman bonus units.** `apps/client/src/bevy_shell/game_ui/task_board/entries.rs:185` prints `value.bonus` directly for all four paths: Damage, Life leech, Loot and Bestiary. Real server replies encode hundredths of one percent, so level-one values appear as `bonus 300` and `bonus 600` instead of `3%` and `6%`. Opening the Bounty Talisman tab reaches this renderer. Format these values as percentages; the new test’s synthetic `bonus: 1` does not exercise production units.
