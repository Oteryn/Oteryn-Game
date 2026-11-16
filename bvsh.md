Single agent work in this cloud session. Use the repository checkout you are in; build with CARGO_BUILD_JOBS=4. Load the repo skills oteryn-dev, oteryn-coding and oteryn-verify. Do not open a PR, do not label, do not merge. Final report at most 20 lines.

Repository Oteryn/Oteryn-Game. Start from `origin/claude/happy-davinci-ijz3wy-bvint` at exact commit `b97bf3aa2c2ee9bfef6a047817f898c087b230e2` (git fetch origin claude/happy-davinci-ijz3wy-bvint; git checkout -b claude/happy-davinci-ijz3wy-bvsh b97bf3aa2c2ee9bfef6a047817f898c087b230e2). This is the Bevy client package (apps/client/src/bevy_shell/**, feature `bevy-shell`); the Win32 UI is being removed. Parity reference is Tibia Global (current official client); OTClient-only extras are out of scope.

A Codex review found the findings below. Fix EVERY finding in this list. For each one add a regression test that goes through the REAL caller (the Bevy input route / observer / system that a player triggers, not a direct call into the helper the finding names) and fails before the fix and passes after; name it in the report. If a finding is wrong (already correct or unreachable), change nothing for it and give the traced reason. Touch only the files the findings need (plus their tests and the six-language client text catalogs if you add UI text). No stubs, no todo!/unimplemented!, no #[ignore], no empty match arms, no placeholder data; no test skipped, ignored, loosened or deleted. Do not touch .github/**, tools/repository/**, docs/agents/**.

Checks (iterate until green):
1. cargo fmt --all --check
2. cargo clippy --locked -p oteryn-client --features bevy-shell --all-targets -- -D warnings
3. cargo clippy --locked -p oteryn-client --all-targets -- -D warnings
4. cargo test --locked -p oteryn-client --features bevy-shell --lib -- bevy_shell::
5. cargo test --locked -p oteryn-client --lib
6. the client text catalog check if you touched UI text (find it with `rg -l "six-language" apps/client crates/client-ui | head`).

Commit on `claude/happy-davinci-ijz3wy-bvsh` with author `blakinio <75369544+blakinio@users.noreply.github.com>`, then `git push origin claude/happy-davinci-ijz3wy-bvsh`. No PR, no force, no other branch. Never end a turn with only a status: end only after a push or a verbatim hard blocker (disk full/OOM are not blockers: clean your own target, CARGO_BUILD_JOBS=2).
Final reply: branch, pushed SHA, base SHA, per finding fixed (file:line, test name) or declined (reason), per check its result line, push output.

FINDINGS:

1. **P1 — Unsettled donation input** — `apps/client/src/bevy_shell/game_ui/cyclopedia_map.rs:74`. With `500` committed in the editor and a replacement paste of `50` pending, clicking **Donate** immediately reads `gold.value()` and sends operation 4 with `500`. This real `Activate → cyclopedia_map_request` path can spend the previous amount. Defer submission until `EditableTextSystems` completes and pending edits/paste settle, as the generic forms do.

2. **P1 — Unreachable atlas floors** — `apps/client/src/bevy_shell/game_ui/cyclopedia_map.rs:180`. Clicking **Region atlas** loads a page and sets `panel.floor` from its first run; `region_atlas_text` displays only that floor. Neither the new controls nor **Next atlas page** provides floor selection. A valid multi-floor atlas therefore leaves areas on every other floor inaccessible. Wire floor navigation through the existing `select_floor` implementation.

3. **P1 — Stale reward state after collection** — `apps/client/src/bevy_shell/controls.rs:222`. The newly routed **Ctrl+Shift+1 → Boss rewards → Browse → Take item/Collect group/Collect all** path retains collected rows. On an accepted `CollectReply { remaining: 0, .. }`, `apply_result` clears pending status but leaves `panel.browse` unchanged, so the form still displays rewards that no longer exist. Invalidate or refresh the browsed page after collection; the added test applies this reply without checking the resulting rows.

4. **P2 — Donation draft discarded during redraw** — `apps/client/src/bevy_shell/game_ui/cyclopedia_map.rs:204`. Typed gold reaches `panel.fields[0]` only when **Donate** is clicked. Typing an amount and then clicking **Exploration status**, an area control, or **Region atlas** changes the panel snapshot, causing `project` to destroy the editor and recreate it from the previous submitted amount. Preserve the draft across panel updates, or synchronize edits before rebuilding the window.
