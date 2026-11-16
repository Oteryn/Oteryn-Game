Single agent work in this cloud session. Use the repository checkout you are in; build with CARGO_BUILD_JOBS=4. Load the repo skills oteryn-dev, oteryn-coding and oteryn-verify. Do not open a PR, do not label, do not merge. Final report at most 20 lines.

Repository Oteryn/Oteryn-Game. Start from `origin/claude/happy-davinci-ijz3wy-bvint` at exact commit `b97bf3aa2c2ee9bfef6a047817f898c087b230e2` (git fetch origin claude/happy-davinci-ijz3wy-bvint; git checkout -b claude/happy-davinci-ijz3wy-bvsb b97bf3aa2c2ee9bfef6a047817f898c087b230e2). This is the Bevy client package (apps/client/src/bevy_shell/**, feature `bevy-shell`); the Win32 UI is being removed. Parity reference is Tibia Global (current official client); OTClient-only extras are out of scope.

A Codex review found the findings below. Fix EVERY finding in this list. For each one add a regression test that goes through the REAL caller (the Bevy input route / observer / system that a player triggers, not a direct call into the helper the finding names) and fails before the fix and passes after; name it in the report. If a finding is wrong (already correct or unreachable), change nothing for it and give the traced reason. Touch only the files the findings need (plus their tests and the six-language client text catalogs if you add UI text). No stubs, no todo!/unimplemented!, no #[ignore], no empty match arms, no placeholder data; no test skipped, ignored, loosened or deleted. Do not touch .github/**, tools/repository/**, docs/agents/**.

Checks (iterate until green):
1. cargo fmt --all --check
2. cargo clippy --locked -p oteryn-client --features bevy-shell --all-targets -- -D warnings
3. cargo clippy --locked -p oteryn-client --all-targets -- -D warnings
4. cargo test --locked -p oteryn-client --features bevy-shell --lib -- bevy_shell::
5. cargo test --locked -p oteryn-client --lib
6. the client text catalog check if you touched UI text (find it with `rg -l "six-language" apps/client crates/client-ui | head`).

Commit on `claude/happy-davinci-ijz3wy-bvsb` with author `blakinio <75369544+blakinio@users.noreply.github.com>`, then `git push origin claude/happy-davinci-ijz3wy-bvsb`. No PR, no force, no other branch. Never end a turn with only a status: end only after a push or a verbatim hard blocker (disk full/OOM are not blockers: clean your own target, CARGO_BUILD_JOBS=2).
Final reply: branch, pushed SHA, base SHA, per finding fixed (file:line, test name) or declined (reason), per check its result line, push output.

FINDINGS:

1. **P1 — Wrong target after reopening a pending form.** Instances: [mail/input.rs:57](apps/client/src/features/0115-mail/input.rs:57), [timed-item/input.rs:51](apps/client/src/features/0039-timed-item/input.rs:51). Submit an operation for item A, close the form while awaiting its reply, then choose **Write…** or **Timed item…** on item B. The real inventory `Activate → choose` path closes B’s menu and reopens the form, but these pending branches retain A’s handle. After A’s reply, subsequent actions still target A. Reject the opener visibly or defer B’s selection until the pending request completes.

2. **P1 — Writing opens without the existing inscription.** [mail/input.rs:58](apps/client/src/features/0115-mail/input.rs:58). For an already-written, rewritable item, inventory **Write… → mail_write_item** only changes its handle; `edit` clears the text when the handle changes. No request loads the existing inscription. The player therefore receives a blank editor, and **Write** sends that buffer as the complete replacement, losing the previous text. Load the authoritative inscription before enabling submission, as Tibia Global’s writing window does.

3. **P1 — Timed-item details survive a target change and rejection.** [timed-item/input.rs:55](apps/client/src/features/0039-timed-item/input.rs:55). Inspect A successfully, close its window, then choose **Timed item…** on B. The inventory caller changes the handle and revision but retains A’s summary. If B’s inspection is rejected, the reducer updates only feedback, so B’s window continues displaying A’s charges, remaining duration, retirement state, and repair fee. Clear or invalidate the summary when changing targets.

4. **P2 — Expanded context menu exceeds its viewport clamp.** [inventory/dialogs.rs:103](apps/client/src/bevy_shell/game_ui/inventory/dialogs.rs:103). With capabilities 115 and 39 negotiated, right-clicking an inventory slot near the bottom reaches `actions::click`, which reserves only 150 logical pixels vertically. The eight buttons now require at least 184 pixels including gaps and padding. Consequently **Timed item…** extends below the viewport and **Cancel** falls outside it. Compute the clamp from the visible rows or constrain and scroll the menu.
