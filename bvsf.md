Single agent work in this cloud session. Use the repository checkout you are in; build with CARGO_BUILD_JOBS=4. Load the repo skills oteryn-dev, oteryn-coding and oteryn-verify. Do not open a PR, do not label, do not merge. Final report at most 20 lines.

Repository Oteryn/Oteryn-Game. Start from `origin/claude/happy-davinci-ijz3wy-bvint` at exact commit `b97bf3aa2c2ee9bfef6a047817f898c087b230e2` (git fetch origin claude/happy-davinci-ijz3wy-bvint; git checkout -b claude/happy-davinci-ijz3wy-bvsf b97bf3aa2c2ee9bfef6a047817f898c087b230e2). This is the Bevy client package (apps/client/src/bevy_shell/**, feature `bevy-shell`); the Win32 UI is being removed. Parity reference is Tibia Global (current official client); OTClient-only extras are out of scope.

A Codex review found the findings below. Fix EVERY finding in this list. For each one add a regression test that goes through the REAL caller (the Bevy input route / observer / system that a player triggers, not a direct call into the helper the finding names) and fails before the fix and passes after; name it in the report. If a finding is wrong (already correct or unreachable), change nothing for it and give the traced reason. Touch only the files the findings need (plus their tests and the six-language client text catalogs if you add UI text). No stubs, no todo!/unimplemented!, no #[ignore], no empty match arms, no placeholder data; no test skipped, ignored, loosened or deleted. Do not touch .github/**, tools/repository/**, docs/agents/**.

Checks (iterate until green):
1. cargo fmt --all --check
2. cargo clippy --locked -p oteryn-client --features bevy-shell --all-targets -- -D warnings
3. cargo clippy --locked -p oteryn-client --all-targets -- -D warnings
4. cargo test --locked -p oteryn-client --features bevy-shell --lib -- bevy_shell::
5. cargo test --locked -p oteryn-client --lib
6. the client text catalog check if you touched UI text (find it with `rg -l "six-language" apps/client crates/client-ui | head`).

Commit on `claude/happy-davinci-ijz3wy-bvsf` with author `blakinio <75369544+blakinio@users.noreply.github.com>`, then `git push origin claude/happy-davinci-ijz3wy-bvsf`. No PR, no force, no other branch. Never end a turn with only a status: end only after a push or a verbatim hard blocker (disk full/OOM are not blockers: clean your own target, CARGO_BUILD_JOBS=2).
Final reply: branch, pushed SHA, base SHA, per finding fixed (file:line, test name) or declined (reason), per check its result line, push output.

FINDINGS:

1. **P1 — HUD controls obscured by default pane.** [hud_panes.rs:93](apps/client/src/bevy_shell/game_ui/hud_panes.rs:93): On a fresh login, `project` displays the default Status pane at `[8,8,900,100]`, above the main HUD at global layer 20. It covers the vitals and pane-toggle strip, intercepting their clicks. Reserve space for docked panes or position them below the header.

2. **P1 — Interface scale applied twice.** [hud_panes.rs:319](apps/client/src/bevy_shell/game_ui/hud_panes.rs:319): Selecting 150% interface scale through Options also sets Bevy’s `UiScale` to 1.5. `layout.rect` scales the bounds again, while `root` at line 155 uses the unadjusted window size. In a 1024-wide window, a right-docked Combat pane starts at rendered x=984 and extends to x=1524. This affects all four panes. Use one consistent coordinate space for projection, clamping, and drag persistence.

3. **P1 — Canceled drags permanently suspend pane projection.** [hud_panes.rs:270](apps/client/src/bevy_shell/game_ui/hud_panes.rs:270): Start dragging a title bar on a touchscreen, then cancel the touch. Bevy emits `PointerCancel` without `PointerDragEnd`, leaving `TitleBar.origin` populated. `project` at line 315 subsequently skips that pane indefinitely, including hide/reset updates. The handlers also lack modal, focus, window-size, and scale cancellation checks. Clear the gesture on cancellation/context changes and fence both movement and completion.

4. **P1 — Mirroring discards channel identity.** [console.rs:117](apps/client/src/features/0007-chat/console.rs:117): From a conversation or public-channel tab, right-click a message and select “Mirror this tab.” `message_action` calls `chat_console_mirror`, which stores only `ConsoleTab::Private`; `mirror_text` renders it without the original channel filter. Conversations consequently show other private messages, while public-channel messages disappear. Closing the original channel also leaves its copy open. Preserve the exact tab identity and its lifecycle for every `ChannelTab` variant.

5. **P1 — Mirror hides original transcript content.** [chat/widgets.rs:127](apps/client/src/bevy_shell/game_ui/chat/widgets.rs:127): Opening a mirror places an opaque absolute pane over the transcript’s right third, while `HistoryScroll` retains its full width. After switching the original to another tab, message suffixes beneath the mirror are inaccessible: horizontal scrolling still calculates its limit against the full viewport. Allocate separate widths to the original transcript and mirror.

6. **P1 — Invalid house row silently reuses the previous selection.** [house/view.rs:11](apps/client/src/features/0049-house/view.rs:11): With catalogue row 1 selected, enter `0` or nonnumeric text and click “Show entrance map.” `houses::finish` calls `house_map`, whose failed parse leaves `model.index` unchanged and displays row 1’s map without an error. Reject invalid row input and clear the map instead of showing a previous house.
